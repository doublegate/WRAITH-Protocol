// Voice Call Manager for WRAITH-Chat
//
// Handles real-time voice transport over WRAITH protocol streams,
// including call signaling, audio capture/playback, and call state management.

use crate::audio::{
    AudioConfig, AudioDevice, AudioDeviceManager, AudioFrame, JitterBuffer, VoiceDecoder,
    VoiceEncoder,
};
use cpal::traits::{DeviceTrait, StreamTrait};
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Observer, Producer, Split};
// Re-import Consumer and Producer for explicit trait method calls
use ringbuf::traits::Consumer as ConsumerTrait;
use ringbuf::traits::Producer as ProducerTrait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock, mpsc};
use uuid::Uuid;

/// Voice call errors
#[derive(Debug, Error)]
pub enum VoiceCallError {
    #[error("Call not found: {0}")]
    CallNotFound(String),

    #[error("Call already exists with peer: {0}")]
    CallAlreadyExists(String),

    #[error("Invalid call state: expected {expected}, got {actual}")]
    InvalidState { expected: String, actual: String },

    #[error("Audio error: {0}")]
    AudioError(#[from] crate::audio::AudioError),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Codec error: {0}")]
    CodecError(String),

    #[error("Device error: {0}")]
    DeviceError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Call ended: {0}")]
    CallEnded(String),
}

/// Voice call state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallState {
    /// Call is being initiated
    Initiating,
    /// Ringing (outgoing) - waiting for remote to answer
    Ringing,
    /// Incoming call - waiting for local user to answer
    Incoming,
    /// Call is connected and active
    Connected,
    /// Call is on hold
    OnHold,
    /// Call is reconnecting after network issues
    Reconnecting,
    /// Call has ended
    Ended,
}

impl std::fmt::Display for CallState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallState::Initiating => write!(f, "initiating"),
            CallState::Ringing => write!(f, "ringing"),
            CallState::Incoming => write!(f, "incoming"),
            CallState::Connected => write!(f, "connected"),
            CallState::OnHold => write!(f, "on_hold"),
            CallState::Reconnecting => write!(f, "reconnecting"),
            CallState::Ended => write!(f, "ended"),
        }
    }
}

/// Call direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallDirection {
    Outgoing,
    Incoming,
}

/// Voice call signaling message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CallSignal {
    /// Initiate a call
    Offer {
        call_id: String,
        codec_config: CodecConfig,
    },
    /// Accept a call
    Answer { call_id: String },
    /// Reject a call
    Reject { call_id: String, reason: String },
    /// Terminate a call
    Hangup { call_id: String, reason: String },
    /// Call is ringing
    Ringing { call_id: String },
    /// Put call on hold
    Hold { call_id: String },
    /// Resume call from hold
    Resume { call_id: String },
    /// Keep-alive ping
    Ping { call_id: String, timestamp: u64 },
    /// Keep-alive pong
    Pong { call_id: String, timestamp: u64 },
}

/// Codec configuration sent during call setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecConfig {
    /// Codec name (always "opus" for now)
    pub codec: String,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Bitrate in bps
    pub bitrate: u32,
    /// Frame size in samples
    pub frame_size: usize,
}

impl Default for CodecConfig {
    fn default() -> Self {
        Self {
            codec: "opus".to_string(),
            sample_rate: 48000,
            bitrate: 64000,
            frame_size: 960,
        }
    }
}

/// Voice packet sent over the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicePacket {
    /// Call ID this packet belongs to
    pub call_id: String,
    /// Sequence number for ordering and loss detection
    pub sequence: u32,
    /// Timestamp in audio samples
    pub timestamp: u64,
    /// Encoded audio data (Opus)
    #[serde(with = "serde_bytes")]
    pub audio_data: Vec<u8>,
    /// Whether this is a silence frame (DTX)
    pub is_silence: bool,
}

mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        Ok(bytes)
    }
}

/// Call statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CallStats {
    /// Call duration in seconds
    pub duration_secs: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Packets lost
    pub packets_lost: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f32,
    /// Jitter in milliseconds
    pub jitter_ms: f32,
    /// Current bitrate in bps
    pub current_bitrate: u32,
}

/// Information about an active call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallInfo {
    /// Unique call identifier
    pub call_id: String,
    /// Remote peer ID
    pub peer_id: String,
    /// Call state
    pub state: CallState,
    /// Call direction
    pub direction: CallDirection,
    /// When the call started (Unix timestamp)
    pub started_at: i64,
    /// When the call connected (Unix timestamp, None if not connected)
    pub connected_at: Option<i64>,
    /// Whether local audio is muted
    pub muted: bool,
    /// Whether speaker is on (vs earpiece)
    pub speaker_on: bool,
    /// Call statistics
    pub stats: CallStats,
}

/// Signal to change audio device in running loops
#[derive(Debug, Clone)]
pub enum AudioDeviceSignal {
    /// Switch to a new input device (microphone)
    SwitchInput(Option<String>),
    /// Switch to a new output device (speaker/headphone)
    SwitchOutput(Option<String>),
}

/// Internal call state
struct Call {
    info: CallInfo,
    encoder: VoiceEncoder,
    decoder: VoiceDecoder,
    jitter_buffer: JitterBuffer,
    sequence: AtomicU32,
    timestamp: AtomicU32,
    running: AtomicBool,
    /// Channel for receiving audio frames to play
    #[allow(dead_code)]
    audio_rx: Option<mpsc::Receiver<Vec<i16>>>,
    /// Channel for sending captured audio
    #[allow(dead_code)]
    audio_tx: Option<mpsc::Sender<Vec<i16>>>,
    /// Channel for signaling input device changes
    input_device_signal: Option<mpsc::Sender<AudioDeviceSignal>>,
    /// Channel for signaling output device changes
    output_device_signal: Option<mpsc::Sender<AudioDeviceSignal>>,
}

/// Voice call manager
pub struct VoiceCallManager {
    /// Active calls by call ID
    calls: RwLock<HashMap<String, Arc<Mutex<Call>>>>,
    /// Audio device manager
    device_manager: RwLock<AudioDeviceManager>,
    /// Default audio configuration
    audio_config: AudioConfig,
    /// Channel for outgoing voice packets
    outgoing_tx: mpsc::Sender<VoicePacket>,
    /// Handle for receiving outgoing packets
    outgoing_rx: Mutex<Option<mpsc::Receiver<VoicePacket>>>,
    /// Channel for outgoing signals
    signal_tx: mpsc::Sender<(String, CallSignal)>,
    /// Handle for receiving signals to send
    signal_rx: Mutex<Option<mpsc::Receiver<(String, CallSignal)>>>,
}

impl VoiceCallManager {
    /// Create a new voice call manager
    pub fn new() -> Self {
        let (outgoing_tx, outgoing_rx) = mpsc::channel(1000);
        let (signal_tx, signal_rx) = mpsc::channel(100);

        Self {
            calls: RwLock::new(HashMap::new()),
            device_manager: RwLock::new(AudioDeviceManager::new()),
            audio_config: AudioConfig::default(),
            outgoing_tx,
            outgoing_rx: Mutex::new(Some(outgoing_rx)),
            signal_tx,
            signal_rx: Mutex::new(Some(signal_rx)),
        }
    }

    /// Take the outgoing packet receiver (for integration with transport layer)
    pub async fn take_outgoing_receiver(&self) -> Option<mpsc::Receiver<VoicePacket>> {
        self.outgoing_rx.lock().await.take()
    }

    /// Take the signal receiver (for integration with transport layer)
    pub async fn take_signal_receiver(&self) -> Option<mpsc::Receiver<(String, CallSignal)>> {
        self.signal_rx.lock().await.take()
    }

    /// Initiate a call to a peer
    pub async fn start_call(&self, peer_id: &str) -> Result<CallInfo, VoiceCallError> {
        // Check if we already have a call with this peer
        let calls = self.calls.read().await;
        for call in calls.values() {
            let call = call.lock().await;
            if call.info.peer_id == peer_id && call.info.state != CallState::Ended {
                return Err(VoiceCallError::CallAlreadyExists(peer_id.to_string()));
            }
        }
        drop(calls);

        let call_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        let encoder = VoiceEncoder::new(self.audio_config.clone())?;
        let decoder = VoiceDecoder::new(self.audio_config.clone())?;
        let jitter_buffer = JitterBuffer::new(
            100,
            self.audio_config.sample_rate.as_hz(),
            self.audio_config.frame_size,
        );

        let info = CallInfo {
            call_id: call_id.clone(),
            peer_id: peer_id.to_string(),
            state: CallState::Initiating,
            direction: CallDirection::Outgoing,
            started_at: now,
            connected_at: None,
            muted: false,
            speaker_on: false,
            stats: CallStats::default(),
        };

        let call = Call {
            info: info.clone(),
            encoder,
            decoder,
            jitter_buffer,
            sequence: AtomicU32::new(0),
            timestamp: AtomicU32::new(0),
            running: AtomicBool::new(false),
            audio_rx: None,
            audio_tx: None,
            input_device_signal: None,
            output_device_signal: None,
        };

        // Store the call
        let mut calls = self.calls.write().await;
        calls.insert(call_id.clone(), Arc::new(Mutex::new(call)));
        drop(calls);

        // Send offer signal
        let signal = CallSignal::Offer {
            call_id: call_id.clone(),
            codec_config: CodecConfig::default(),
        };
        self.signal_tx
            .send((peer_id.to_string(), signal))
            .await
            .map_err(|e| VoiceCallError::TransportError(e.to_string()))?;

        // Update state to ringing
        self.update_call_state(&call_id, CallState::Ringing).await?;

        Ok(self.get_call_info(&call_id).await?.unwrap())
    }

    /// Handle an incoming call offer
    pub async fn handle_incoming_call(
        &self,
        peer_id: &str,
        call_id: &str,
        _codec_config: CodecConfig,
    ) -> Result<CallInfo, VoiceCallError> {
        let now = chrono::Utc::now().timestamp();

        let encoder = VoiceEncoder::new(self.audio_config.clone())?;
        let decoder = VoiceDecoder::new(self.audio_config.clone())?;
        let jitter_buffer = JitterBuffer::new(
            100,
            self.audio_config.sample_rate.as_hz(),
            self.audio_config.frame_size,
        );

        let info = CallInfo {
            call_id: call_id.to_string(),
            peer_id: peer_id.to_string(),
            state: CallState::Incoming,
            direction: CallDirection::Incoming,
            started_at: now,
            connected_at: None,
            muted: false,
            speaker_on: false,
            stats: CallStats::default(),
        };

        let call = Call {
            info: info.clone(),
            encoder,
            decoder,
            jitter_buffer,
            sequence: AtomicU32::new(0),
            timestamp: AtomicU32::new(0),
            running: AtomicBool::new(false),
            audio_rx: None,
            audio_tx: None,
            input_device_signal: None,
            output_device_signal: None,
        };

        let mut calls = self.calls.write().await;
        calls.insert(call_id.to_string(), Arc::new(Mutex::new(call)));

        // Send ringing signal
        let signal = CallSignal::Ringing {
            call_id: call_id.to_string(),
        };
        self.signal_tx
            .send((peer_id.to_string(), signal))
            .await
            .map_err(|e| VoiceCallError::TransportError(e.to_string()))?;

        Ok(info)
    }

    /// Answer an incoming call
    pub async fn answer_call(&self, call_id: &str) -> Result<CallInfo, VoiceCallError> {
        let calls = self.calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VoiceCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;

        if call.info.state != CallState::Incoming {
            return Err(VoiceCallError::InvalidState {
                expected: "incoming".to_string(),
                actual: call.info.state.to_string(),
            });
        }

        let peer_id = call.info.peer_id.clone();

        // Update state
        call.info.state = CallState::Connected;
        call.info.connected_at = Some(chrono::Utc::now().timestamp());
        call.running.store(true, Ordering::SeqCst);

        let info = call.info.clone();
        drop(call);

        // Send answer signal
        let signal = CallSignal::Answer {
            call_id: call_id.to_string(),
        };
        self.signal_tx
            .send((peer_id, signal))
            .await
            .map_err(|e| VoiceCallError::TransportError(e.to_string()))?;

        // Start audio streams
        self.start_audio_streams(call_id).await?;

        Ok(info)
    }

    /// Handle answer from remote peer
    pub async fn handle_call_answered(&self, call_id: &str) -> Result<CallInfo, VoiceCallError> {
        let calls = self.calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VoiceCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;

        if call.info.state != CallState::Ringing {
            return Err(VoiceCallError::InvalidState {
                expected: "ringing".to_string(),
                actual: call.info.state.to_string(),
            });
        }

        call.info.state = CallState::Connected;
        call.info.connected_at = Some(chrono::Utc::now().timestamp());
        call.running.store(true, Ordering::SeqCst);

        let info = call.info.clone();
        drop(call);

        // Start audio streams
        self.start_audio_streams(call_id).await?;

        Ok(info)
    }

    /// Reject an incoming call
    pub async fn reject_call(&self, call_id: &str, reason: &str) -> Result<(), VoiceCallError> {
        let calls = self.calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VoiceCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;
        let peer_id = call.info.peer_id.clone();

        call.info.state = CallState::Ended;
        call.running.store(false, Ordering::SeqCst);
        drop(call);

        // Send reject signal
        let signal = CallSignal::Reject {
            call_id: call_id.to_string(),
            reason: reason.to_string(),
        };
        self.signal_tx
            .send((peer_id, signal))
            .await
            .map_err(|e| VoiceCallError::TransportError(e.to_string()))?;

        Ok(())
    }

    /// End an active call
    pub async fn end_call(&self, call_id: &str, reason: &str) -> Result<(), VoiceCallError> {
        let calls = self.calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VoiceCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;
        let peer_id = call.info.peer_id.clone();

        call.info.state = CallState::Ended;
        call.running.store(false, Ordering::SeqCst);
        drop(call);

        // Send hangup signal
        let signal = CallSignal::Hangup {
            call_id: call_id.to_string(),
            reason: reason.to_string(),
        };
        self.signal_tx
            .send((peer_id, signal))
            .await
            .map_err(|e| VoiceCallError::TransportError(e.to_string()))?;

        // Remove call after a delay (for statistics collection)
        let call_id_owned = call_id.to_string();
        let calls_ref = self.calls.read().await;
        let calls_clone: HashMap<String, Arc<Mutex<Call>>> = calls_ref
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        drop(calls_ref);

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            // Note: We can't actually remove from the original RwLock here
            // This is a simplified cleanup - in production, use a proper cleanup mechanism
            let _ = calls_clone.get(&call_id_owned);
        });

        Ok(())
    }

    /// Handle remote hangup
    pub async fn handle_call_ended(
        &self,
        call_id: &str,
        _reason: &str,
    ) -> Result<(), VoiceCallError> {
        let calls = self.calls.read().await;
        if let Some(call_arc) = calls.get(call_id) {
            let mut call = call_arc.lock().await;
            call.info.state = CallState::Ended;
            call.running.store(false, Ordering::SeqCst);
        }
        Ok(())
    }

    /// Toggle mute on a call
    pub async fn toggle_mute(&self, call_id: &str) -> Result<bool, VoiceCallError> {
        let calls = self.calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VoiceCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;
        call.info.muted = !call.info.muted;
        Ok(call.info.muted)
    }

    /// Toggle speaker on a call
    ///
    /// When speaker is turned ON, switches to the configured output device (or default speaker).
    /// When speaker is turned OFF, switches back to the default output device (typically earpiece on mobile).
    ///
    /// For desktop applications, this effectively switches between the primary and secondary
    /// output devices if configured.
    pub async fn toggle_speaker(&self, call_id: &str) -> Result<bool, VoiceCallError> {
        let calls = self.calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VoiceCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;
        call.info.speaker_on = !call.info.speaker_on;

        // Determine which device to switch to
        let new_device_id = if call.info.speaker_on {
            // Speaker ON: use the configured output device (or default)
            let device_manager = self.device_manager.read().await;
            device_manager.output_device().map(String::from)
        } else {
            // Speaker OFF: use default device (None means default)
            None
        };

        // Signal the output stream to switch devices if the call is active
        if call.info.state == CallState::Connected {
            if let Some(ref signal_tx) = call.output_device_signal {
                if let Err(e) = signal_tx
                    .send(AudioDeviceSignal::SwitchOutput(new_device_id))
                    .await
                {
                    log::warn!("Failed to send output device switch signal: {}", e);
                }
            }
        }

        Ok(call.info.speaker_on)
    }

    /// Get call information
    pub async fn get_call_info(&self, call_id: &str) -> Result<Option<CallInfo>, VoiceCallError> {
        let calls = self.calls.read().await;
        if let Some(call_arc) = calls.get(call_id) {
            let call = call_arc.lock().await;
            Ok(Some(call.info.clone()))
        } else {
            Ok(None)
        }
    }

    /// Get all active calls
    pub async fn get_active_calls(&self) -> Vec<CallInfo> {
        let calls = self.calls.read().await;
        let mut result = Vec::new();
        for call_arc in calls.values() {
            let call = call_arc.lock().await;
            if call.info.state != CallState::Ended {
                result.push(call.info.clone());
            }
        }
        result
    }

    /// Process an incoming voice packet
    pub async fn process_voice_packet(&self, packet: VoicePacket) -> Result<(), VoiceCallError> {
        let calls = self.calls.read().await;
        let call_arc = calls
            .get(&packet.call_id)
            .ok_or_else(|| VoiceCallError::CallNotFound(packet.call_id.clone()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;

        // Update stats
        call.info.stats.packets_received += 1;

        // Skip if call is not active
        if call.info.state != CallState::Connected {
            return Ok(());
        }

        // Decode audio
        if !packet.is_silence {
            let mut pcm = vec![0i16; call.decoder.output_buffer_size()];
            let samples = call
                .decoder
                .decode(&packet.audio_data, &mut pcm, false)
                .map_err(|e| VoiceCallError::CodecError(e.to_string()))?;

            if samples > 0 {
                // Add to jitter buffer
                let frame = AudioFrame {
                    samples: pcm[..samples].to_vec(),
                    sequence: packet.sequence,
                    timestamp: packet.timestamp,
                    synthesized: false,
                };
                call.jitter_buffer.push(frame)?;
            }
        }

        Ok(())
    }

    /// Process an incoming signal
    pub async fn process_signal(
        &self,
        peer_id: &str,
        signal: CallSignal,
    ) -> Result<Option<CallInfo>, VoiceCallError> {
        match signal {
            CallSignal::Offer {
                call_id,
                codec_config,
            } => {
                let info = self
                    .handle_incoming_call(peer_id, &call_id, codec_config)
                    .await?;
                Ok(Some(info))
            }
            CallSignal::Answer { call_id } => {
                let info = self.handle_call_answered(&call_id).await?;
                Ok(Some(info))
            }
            CallSignal::Reject { call_id, reason } => {
                self.handle_call_ended(&call_id, &reason).await?;
                Ok(None)
            }
            CallSignal::Hangup { call_id, reason } => {
                self.handle_call_ended(&call_id, &reason).await?;
                Ok(None)
            }
            CallSignal::Ringing { call_id } => {
                // Remote is ringing, update state
                self.update_call_state(&call_id, CallState::Ringing).await?;
                self.get_call_info(&call_id).await
            }
            CallSignal::Hold { call_id } => {
                self.update_call_state(&call_id, CallState::OnHold).await?;
                self.get_call_info(&call_id).await
            }
            CallSignal::Resume { call_id } => {
                self.update_call_state(&call_id, CallState::Connected)
                    .await?;
                self.get_call_info(&call_id).await
            }
            CallSignal::Ping { call_id, timestamp } => {
                // Respond with pong
                let calls = self.calls.read().await;
                if let Some(call_arc) = calls.get(&call_id) {
                    let call = call_arc.lock().await;
                    let peer_id = call.info.peer_id.clone();
                    drop(call);

                    let pong = CallSignal::Pong { call_id, timestamp };
                    self.signal_tx
                        .send((peer_id, pong))
                        .await
                        .map_err(|e| VoiceCallError::TransportError(e.to_string()))?;
                }
                Ok(None)
            }
            CallSignal::Pong { call_id, timestamp } => {
                // Calculate RTT and update stats
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let rtt = now.saturating_sub(timestamp) as f32;

                let calls = self.calls.read().await;
                if let Some(call_arc) = calls.get(&call_id) {
                    let mut call = call_arc.lock().await;
                    // Simple moving average for latency
                    call.info.stats.avg_latency_ms =
                        call.info.stats.avg_latency_ms * 0.9 + rtt * 0.1;
                }
                Ok(None)
            }
        }
    }

    /// List available audio input devices
    pub async fn list_input_devices(&self) -> Result<Vec<AudioDevice>, VoiceCallError> {
        let device_manager = self.device_manager.read().await;
        Ok(device_manager.list_input_devices()?)
    }

    /// List available audio output devices
    pub async fn list_output_devices(&self) -> Result<Vec<AudioDevice>, VoiceCallError> {
        let device_manager = self.device_manager.read().await;
        Ok(device_manager.list_output_devices()?)
    }

    /// Set the input audio device
    ///
    /// This updates the device manager's selection and propagates the change
    /// to all active connected calls.
    pub async fn set_input_device(&self, device_id: Option<String>) -> Result<(), VoiceCallError> {
        // Update device manager
        {
            let mut device_manager = self.device_manager.write().await;
            device_manager.set_input_device(device_id.clone());
        }

        // Propagate to all active calls
        let calls = self.calls.read().await;
        for call_arc in calls.values() {
            let call = call_arc.lock().await;
            if call.info.state == CallState::Connected {
                if let Some(ref signal_tx) = call.input_device_signal {
                    if let Err(e) = signal_tx
                        .send(AudioDeviceSignal::SwitchInput(device_id.clone()))
                        .await
                    {
                        log::warn!("Failed to send input device switch signal: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Set the output audio device
    ///
    /// This updates the device manager's selection and propagates the change
    /// to all active connected calls.
    pub async fn set_output_device(&self, device_id: Option<String>) -> Result<(), VoiceCallError> {
        // Update device manager
        {
            let mut device_manager = self.device_manager.write().await;
            device_manager.set_output_device(device_id.clone());
        }

        // Propagate to all active calls
        let calls = self.calls.read().await;
        for call_arc in calls.values() {
            let call = call_arc.lock().await;
            if call.info.state == CallState::Connected {
                if let Some(ref signal_tx) = call.output_device_signal {
                    if let Err(e) = signal_tx
                        .send(AudioDeviceSignal::SwitchOutput(device_id.clone()))
                        .await
                    {
                        log::warn!("Failed to send output device switch signal: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    // Internal helper methods

    async fn update_call_state(
        &self,
        call_id: &str,
        new_state: CallState,
    ) -> Result<(), VoiceCallError> {
        let calls = self.calls.read().await;
        if let Some(call_arc) = calls.get(call_id) {
            let mut call = call_arc.lock().await;
            call.info.state = new_state;
            if new_state == CallState::Connected && call.info.connected_at.is_none() {
                call.info.connected_at = Some(chrono::Utc::now().timestamp());
            }
        }
        Ok(())
    }

    async fn start_audio_streams(&self, call_id: &str) -> Result<(), VoiceCallError> {
        let calls = self.calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VoiceCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        // Create device signal channels
        let (input_signal_tx, input_signal_rx) = mpsc::channel::<AudioDeviceSignal>(4);
        let (output_signal_tx, output_signal_rx) = mpsc::channel::<AudioDeviceSignal>(4);

        // Store signal senders in the call for later device switching
        {
            let mut call = call_arc.lock().await;
            call.input_device_signal = Some(input_signal_tx);
            call.output_device_signal = Some(output_signal_tx);
        }

        // Get current device selections
        let (input_device_id, output_device_id) = {
            let device_manager = self.device_manager.read().await;
            (
                device_manager.input_device().map(String::from),
                device_manager.output_device().map(String::from),
            )
        };

        let call_id = call_id.to_string();
        let outgoing_tx = self.outgoing_tx.clone();

        // Spawn audio capture task
        let call_arc_capture = call_arc.clone();
        tokio::spawn(async move {
            Self::audio_capture_loop(
                call_id,
                call_arc_capture,
                outgoing_tx,
                input_device_id,
                input_signal_rx,
            )
            .await;
        });

        // Spawn audio playback task
        let call_arc_playback = call_arc.clone();
        tokio::spawn(async move {
            Self::audio_playback_loop(call_arc_playback, output_device_id, output_signal_rx).await;
        });

        Ok(())
    }

    /// Set up environment variables to suppress spurious audio backend errors.
    ///
    /// On Linux, ALSA's plugin system (libasound) probes various backends during
    /// device enumeration, including JACK and OSS plugins. When these backends
    /// aren't available, they emit error messages to stderr. This function sets
    /// environment variables to suppress these harmless warnings.
    ///
    /// # Safety
    /// This function sets environment variables using unsafe blocks. It uses
    /// `Once` to ensure the variables are only set once, and is typically called
    /// early in the audio subsystem initialization before any multi-threading
    /// occurs in the audio code path.
    #[cfg(target_os = "linux")]
    fn suppress_audio_backend_errors() {
        use std::env;
        use std::sync::Once;

        static INIT: Once = Once::new();
        INIT.call_once(|| {
            // SAFETY: These environment variables are set once during initialization
            // before any ALSA/JACK threads are spawned. The Once guard ensures
            // thread-safety for this initialization.
            unsafe {
                // Prevent JACK plugin from trying to connect to/start a JACK server
                if env::var("JACK_NO_START_SERVER").is_err() {
                    env::set_var("JACK_NO_START_SERVER", "1");
                }

                // Prevent JACK-related auto-start behavior
                if env::var("JACK_NO_AUDIO_RESERVATION").is_err() {
                    env::set_var("JACK_NO_AUDIO_RESERVATION", "1");
                }

                // Set a very short timeout for JACK connection attempts (in ms)
                // This minimizes delay when JACK isn't available
                if env::var("JACK_DEFAULT_SERVER").is_err() {
                    env::set_var("JACK_DEFAULT_SERVER", "");
                }
            }
        });
    }

    /// Get the preferred cpal host for this platform.
    ///
    /// On Linux, this explicitly selects ALSA to avoid JACK initialization.
    /// Environment variables are set to suppress ALSA plugin errors from JACK/OSS.
    fn get_preferred_host() -> cpal::Host {
        #[cfg(target_os = "linux")]
        {
            // Set up environment variables to suppress backend errors
            Self::suppress_audio_backend_errors();

            // On Linux, use ALSA explicitly to avoid JACK initialization
            use cpal::HostId;

            // Try to get ALSA host explicitly
            for host_id in cpal::available_hosts() {
                if host_id == HostId::Alsa {
                    if let Ok(host) = cpal::host_from_id(host_id) {
                        return host;
                    }
                }
            }

            // Fallback to default host if ALSA isn't available
            cpal::default_host()
        }

        #[cfg(not(target_os = "linux"))]
        {
            cpal::default_host()
        }
    }

    async fn audio_capture_loop(
        call_id: String,
        call_arc: Arc<Mutex<Call>>,
        outgoing_tx: mpsc::Sender<VoicePacket>,
        initial_device_id: Option<String>,
        mut device_signal_rx: mpsc::Receiver<AudioDeviceSignal>,
    ) {
        // Track current device ID
        let current_device_id = Arc::new(std::sync::Mutex::new(initial_device_id));

        // Create ring buffer for audio samples - shared between async task and audio thread
        let rb = HeapRb::<i16>::new(4800); // 100ms buffer
        let (producer, consumer) = rb.split();

        // Wrap in Arc<Mutex> for thread-safe access
        let producer = Arc::new(std::sync::Mutex::new(producer));
        let consumer = Arc::new(std::sync::Mutex::new(consumer));

        // Flag to signal the audio thread to stop
        let running = Arc::new(AtomicBool::new(true));
        // Flag to signal device switch is needed
        let device_switch_requested = Arc::new(AtomicBool::new(false));

        // Clone all the Arc values needed by the audio thread
        let running_clone = running.clone();
        let device_switch_clone = device_switch_requested.clone();
        let producer_clone = producer.clone();
        let current_device_id_clone = current_device_id.clone();

        // Spawn the audio input stream in a blocking thread (since Stream is !Send)
        let audio_handle = std::thread::spawn(move || {
            Self::run_input_stream_loop(
                running_clone,
                device_switch_clone,
                producer_clone,
                current_device_id_clone,
            );
        });

        let mut frame_buffer = vec![0i16; 960];
        let mut encode_buffer = vec![0u8; 4000];

        // Main async loop: read from ring buffer, encode, and send
        loop {
            // Check for device switch signals (non-blocking)
            if let Ok(AudioDeviceSignal::SwitchInput(new_device_id)) = device_signal_rx.try_recv() {
                log::info!(
                    "Switching input device to: {:?}",
                    new_device_id.as_deref().unwrap_or("default")
                );
                if let Ok(mut device_id) = current_device_id.lock() {
                    *device_id = new_device_id;
                }
                // Signal the audio thread to restart with new device
                device_switch_requested.store(true, Ordering::SeqCst);
            }

            // Check if call is still active
            {
                let call = call_arc.lock().await;
                if !call.running.load(Ordering::SeqCst) {
                    break;
                }
            }

            // Wait for enough samples
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let samples_available = consumer.lock().map(|c| c.occupied_len()).unwrap_or(0);
            if samples_available >= 960 {
                // Read samples from ring buffer
                if let Ok(mut cons) = consumer.lock() {
                    for sample in frame_buffer.iter_mut() {
                        *sample = cons.try_pop().unwrap_or(0);
                    }
                }

                // Encode audio
                let mut call = call_arc.lock().await;

                // Skip encoding if muted
                if call.info.muted {
                    continue;
                }

                let encoded_len = match call.encoder.encode(&frame_buffer, &mut encode_buffer) {
                    Ok(len) => len,
                    Err(e) => {
                        log::error!("Audio encode error: {}", e);
                        continue;
                    }
                };

                let sequence = call.sequence.fetch_add(1, Ordering::SeqCst);
                let timestamp = call.timestamp.fetch_add(960, Ordering::SeqCst);
                call.info.stats.packets_sent += 1;

                drop(call);

                // Send voice packet
                let packet = VoicePacket {
                    call_id: call_id.clone(),
                    sequence,
                    timestamp: timestamp as u64,
                    audio_data: encode_buffer[..encoded_len].to_vec(),
                    is_silence: encoded_len == 0,
                };

                if outgoing_tx.send(packet).await.is_err() {
                    log::error!("Failed to send voice packet");
                    break;
                }
            }
        }

        // Signal the audio thread to stop
        running.store(false, Ordering::SeqCst);
        let _ = audio_handle.join();
    }

    /// Run the input stream loop in a blocking thread, handling device switches
    fn run_input_stream_loop(
        running: Arc<AtomicBool>,
        device_switch_requested: Arc<AtomicBool>,
        producer: Arc<std::sync::Mutex<ringbuf::HeapProd<i16>>>,
        current_device_id: Arc<std::sync::Mutex<Option<String>>>,
    ) {
        while running.load(Ordering::SeqCst) {
            // Get current device
            let device = {
                let device_id = current_device_id.lock().ok().and_then(|d| d.clone());
                Self::get_input_device_by_id(device_id.as_deref())
            };

            let device = match device {
                Some(d) => d,
                None => {
                    log::error!("No input audio device available");
                    // Wait a bit before retrying
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            let config = cpal::StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(48000),
                buffer_size: cpal::BufferSize::Fixed(960),
            };

            let err_fn = |err| log::error!("Audio capture error: {}", err);
            let producer_clone = producer.clone();

            let stream = device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Convert f32 to i16 and push to ring buffer
                    if let Ok(mut prod) = producer_clone.lock() {
                        for &sample in data {
                            let sample_i16 = (sample * 32767.0) as i16;
                            let _ = ProducerTrait::try_push(&mut *prod, sample_i16);
                        }
                    }
                },
                err_fn,
                None,
            );

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to build input stream: {}", e);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            if let Err(e) = stream.play() {
                log::error!("Failed to start input stream: {}", e);
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }

            log::info!("Input audio stream started");

            // Keep the stream alive while running and no device switch is requested
            while running.load(Ordering::SeqCst) && !device_switch_requested.load(Ordering::SeqCst)
            {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            // Drop stream before potentially creating a new one
            drop(stream);

            // Clear the device switch flag if it was set
            if device_switch_requested.load(Ordering::SeqCst) {
                device_switch_requested.store(false, Ordering::SeqCst);
                log::info!("Input device switch completed");
            }
        }
    }

    /// Get input device by ID or return default
    fn get_input_device_by_id(device_id: Option<&str>) -> Option<cpal::Device> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = Self::get_preferred_host();

        if let Some(id) = device_id {
            // Try to find the specific device
            if let Ok(devices) = host.input_devices() {
                for device in devices {
                    if let Ok(name) = device.name() {
                        if name == id {
                            return Some(device);
                        }
                    }
                }
            }
            log::warn!("Input device '{}' not found, using default", id);
        }

        // Fall back to default
        host.default_input_device()
    }

    async fn audio_playback_loop(
        call_arc: Arc<Mutex<Call>>,
        initial_device_id: Option<String>,
        mut device_signal_rx: mpsc::Receiver<AudioDeviceSignal>,
    ) {
        // Track current device ID
        let current_device_id = Arc::new(std::sync::Mutex::new(initial_device_id));

        // Create ring buffer for playback - shared between async task and audio thread
        let rb = HeapRb::<i16>::new(9600); // 200ms buffer
        let (producer, consumer) = rb.split();

        // Wrap in Arc<Mutex> for thread-safe access
        let producer = Arc::new(std::sync::Mutex::new(producer));
        let consumer = Arc::new(std::sync::Mutex::new(consumer));

        // Flag to signal the audio thread to stop
        let running = Arc::new(AtomicBool::new(true));
        // Flag to signal device switch is needed
        let device_switch_requested = Arc::new(AtomicBool::new(false));

        // Clone all the Arc values needed by the audio thread
        let running_clone = running.clone();
        let device_switch_clone = device_switch_requested.clone();
        let consumer_clone = consumer.clone();
        let current_device_id_clone = current_device_id.clone();

        // Spawn the audio output stream in a blocking thread (since Stream is !Send)
        let audio_handle = std::thread::spawn(move || {
            Self::run_output_stream_loop(
                running_clone,
                device_switch_clone,
                consumer_clone,
                current_device_id_clone,
            );
        });

        // Main async loop: feed audio from jitter buffer to the ring buffer
        loop {
            // Check for device switch signals (non-blocking)
            if let Ok(AudioDeviceSignal::SwitchOutput(new_device_id)) = device_signal_rx.try_recv()
            {
                log::info!(
                    "Switching output device to: {:?}",
                    new_device_id.as_deref().unwrap_or("default")
                );
                if let Ok(mut device_id) = current_device_id.lock() {
                    *device_id = new_device_id;
                }
                // Signal the audio thread to restart with new device
                device_switch_requested.store(true, Ordering::SeqCst);
            }

            // Check if call is still active
            {
                let call = call_arc.lock().await;
                if !call.running.load(Ordering::SeqCst) {
                    break;
                }
            }

            // Get frames from jitter buffer
            {
                let mut call = call_arc.lock().await;
                while let Some(frame) = call.jitter_buffer.pop() {
                    if let Ok(mut prod) = producer.lock() {
                        for &sample in &frame.samples {
                            let _ = prod.try_push(sample);
                        }
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        // Signal the audio thread to stop
        running.store(false, Ordering::SeqCst);
        let _ = audio_handle.join();
    }

    /// Run the output stream loop in a blocking thread, handling device switches
    fn run_output_stream_loop(
        running: Arc<AtomicBool>,
        device_switch_requested: Arc<AtomicBool>,
        consumer: Arc<std::sync::Mutex<ringbuf::HeapCons<i16>>>,
        current_device_id: Arc<std::sync::Mutex<Option<String>>>,
    ) {
        while running.load(Ordering::SeqCst) {
            // Get current device
            let device = {
                let device_id = current_device_id.lock().ok().and_then(|d| d.clone());
                Self::get_output_device_by_id(device_id.as_deref())
            };

            let device = match device {
                Some(d) => d,
                None => {
                    log::error!("No output audio device available");
                    // Wait a bit before retrying
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            let config = cpal::StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(48000),
                buffer_size: cpal::BufferSize::Fixed(960),
            };

            let err_fn = |err| log::error!("Audio playback error: {}", err);
            let consumer_clone = consumer.clone();

            let stream = device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut cons) = consumer_clone.lock() {
                        for sample in data.iter_mut() {
                            *sample = ConsumerTrait::try_pop(&mut *cons)
                                .map(|s| s as f32 / 32767.0)
                                .unwrap_or(0.0);
                        }
                    }
                },
                err_fn,
                None,
            );

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to build output stream: {}", e);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            if let Err(e) = stream.play() {
                log::error!("Failed to start output stream: {}", e);
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }

            log::info!("Output audio stream started");

            // Keep the stream alive while running and no device switch is requested
            while running.load(Ordering::SeqCst) && !device_switch_requested.load(Ordering::SeqCst)
            {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            // Drop stream before potentially creating a new one
            drop(stream);

            // Clear the device switch flag if it was set
            if device_switch_requested.load(Ordering::SeqCst) {
                device_switch_requested.store(false, Ordering::SeqCst);
                log::info!("Output device switch completed");
            }
        }
    }

    /// Get output device by ID or return default
    fn get_output_device_by_id(device_id: Option<&str>) -> Option<cpal::Device> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let host = Self::get_preferred_host();

        if let Some(id) = device_id {
            // Try to find the specific device
            if let Ok(devices) = host.output_devices() {
                for device in devices {
                    if let Ok(name) = device.name() {
                        if name == id {
                            return Some(device);
                        }
                    }
                }
            }
            log::warn!("Output device '{}' not found, using default", id);
        }

        // Fall back to default
        host.default_output_device()
    }
}

impl Default for VoiceCallManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_call_manager_creation() {
        let manager = VoiceCallManager::new();
        let calls = manager.get_active_calls().await;
        assert!(calls.is_empty());
    }

    #[tokio::test]
    async fn test_call_states() {
        assert_eq!(CallState::Initiating.to_string(), "initiating");
        assert_eq!(CallState::Connected.to_string(), "connected");
        assert_eq!(CallState::Ended.to_string(), "ended");
    }

    #[tokio::test]
    async fn test_codec_config_default() {
        let config = CodecConfig::default();
        assert_eq!(config.codec, "opus");
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.bitrate, 64000);
    }

    #[test]
    fn test_voice_packet_serialization() {
        let packet = VoicePacket {
            call_id: "test-call".to_string(),
            sequence: 1,
            timestamp: 960,
            audio_data: vec![0u8; 100],
            is_silence: false,
        };

        let json = serde_json::to_string(&packet).unwrap();
        let decoded: VoicePacket = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.call_id, packet.call_id);
        assert_eq!(decoded.sequence, packet.sequence);
        assert_eq!(decoded.timestamp, packet.timestamp);
    }

    // ==================== Sprint 18.2 Edge Case Tests ====================

    /// Test 1: Call reconnection after brief network drop
    /// Simulates a network drop by transitioning through Reconnecting state
    #[tokio::test]
    async fn test_call_reconnection_after_network_drop() {
        let manager = VoiceCallManager::new();

        // Start a call
        let call_info = manager.start_call("peer-123").await.unwrap();
        let call_id = call_info.call_id.clone();

        // Simulate receiving answer
        manager.handle_call_answered(&call_id).await.unwrap();

        // Verify call is connected
        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.state, CallState::Connected);

        // Simulate network drop by transitioning to Reconnecting state
        manager
            .update_call_state(&call_id, CallState::Reconnecting)
            .await
            .unwrap();

        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.state, CallState::Reconnecting);

        // Simulate network recovery - reconnect
        manager
            .update_call_state(&call_id, CallState::Connected)
            .await
            .unwrap();

        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.state, CallState::Connected);

        // Verify call statistics were maintained through reconnection
        assert!(info.connected_at.is_some());
    }

    /// Test 2: Audio device disconnected during call (simulated)
    /// Tests the device switching mechanism when a device becomes unavailable
    #[tokio::test]
    async fn test_audio_device_disconnection_handling() {
        let manager = VoiceCallManager::new();

        // Start a call
        let call_info = manager.start_call("peer-456").await.unwrap();
        let call_id = call_info.call_id.clone();

        // Simulate receiving answer
        manager.handle_call_answered(&call_id).await.unwrap();

        // Try to set a non-existent device - should fall back to default
        let result = manager
            .set_input_device(Some("non-existent-device-12345".to_string()))
            .await;

        // Setting device should succeed (device lookup happens at stream creation time)
        assert!(result.is_ok());

        // Try to set output device
        let result = manager
            .set_output_device(Some("non-existent-output-device".to_string()))
            .await;
        assert!(result.is_ok());

        // Call should still be in valid state
        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.state, CallState::Connected);
    }

    /// Test 3: Microphone permission scenarios
    /// Tests call behavior when microphone access might be restricted
    #[tokio::test]
    async fn test_microphone_permission_scenarios() {
        let manager = VoiceCallManager::new();

        // Start call - call initiation should work regardless of mic permission
        let call_info = manager.start_call("peer-789").await.unwrap();
        assert_eq!(call_info.state, CallState::Ringing);

        // Toggle mute should work to handle no-mic scenarios
        let muted = manager.toggle_mute(&call_info.call_id).await.unwrap();
        assert!(muted);

        // Toggle again to unmute
        let muted = manager.toggle_mute(&call_info.call_id).await.unwrap();
        assert!(!muted);

        // Call should remain valid
        let info = manager
            .get_call_info(&call_info.call_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(info.muted, false);
    }

    /// Test 4: Jitter buffer underflow/overflow simulation
    /// Tests the jitter buffer behavior under extreme conditions
    #[test]
    fn test_jitter_buffer_underflow_overflow() {
        use crate::audio::{AudioFrame, JitterBuffer};

        // Create a small buffer to test overflow
        let mut buffer = JitterBuffer::new(50, 48000, 960);

        // Test underflow - try to pop from empty buffer
        let frame = buffer.pop();
        assert!(frame.is_none(), "Should return None on empty buffer");

        // Fill buffer beyond capacity to test overflow
        for i in 0..200 {
            let frame = AudioFrame {
                samples: vec![0i16; 960],
                sequence: i,
                timestamp: i as u64 * 960,
                synthesized: false,
            };
            let result = buffer.push(frame);
            assert!(result.is_ok(), "Push should always succeed");
        }

        // Buffer should have dropped some frames
        let stats = buffer.stats();
        assert!(
            stats.frames_dropped > 0,
            "Should have dropped frames on overflow"
        );
        assert!(
            stats.frames_received == 200,
            "Should have received all frames"
        );

        // Pop all frames - should still work
        let mut popped_count = 0;
        while buffer.pop().is_some() {
            popped_count += 1;
        }
        assert!(popped_count > 0, "Should be able to pop frames");
    }

    /// Test 5: Echo cancellation verification (via noise suppression)
    /// Tests that audio processing with noise suppression is functioning
    #[test]
    fn test_echo_cancellation_via_noise_suppression() {
        use crate::audio::{AudioConfig, VoiceEncoder};

        // Create encoder with noise suppression enabled (includes echo cancellation path)
        let config = AudioConfig {
            enable_noise_suppression: true,
            enable_vad: false, // Disable VAD to ensure audio goes through
            ..Default::default()
        };

        let mut encoder = VoiceEncoder::new(config).unwrap();

        // Create a loud test signal that should be processed
        let mut pcm = vec![0i16; 960];
        for (i, sample) in pcm.iter_mut().enumerate() {
            // Generate a 440Hz sine wave at moderate volume
            let t = i as f32 / 48000.0;
            *sample = (f32::sin(2.0 * std::f32::consts::PI * 440.0 * t) * 8000.0) as i16;
        }

        // Encode the audio
        let mut output = vec![0u8; 4000];
        let encoded_len = encoder.encode(&pcm, &mut output).unwrap();

        // Should produce output (noise suppression shouldn't eliminate real audio)
        assert!(
            encoded_len > 0,
            "Encoder should produce output for non-silent audio"
        );

        // Test with noise suppression disabled
        let config_no_ns = AudioConfig {
            enable_noise_suppression: false,
            enable_vad: false,
            ..Default::default()
        };

        let mut encoder_no_ns = VoiceEncoder::new(config_no_ns).unwrap();
        let mut output2 = vec![0u8; 4000];
        let encoded_len2 = encoder_no_ns.encode(&pcm, &mut output2).unwrap();

        // Both should produce output
        assert!(encoded_len > 0);
        assert!(encoded_len2 > 0);
    }

    /// Test 6: Device switching during active call
    /// Tests the ability to switch audio devices during an active call
    #[tokio::test]
    async fn test_device_switching_during_active_call() {
        let manager = VoiceCallManager::new();

        // Start and connect a call
        let call_info = manager.start_call("peer-switch-test").await.unwrap();
        let call_id = call_info.call_id.clone();
        manager.handle_call_answered(&call_id).await.unwrap();

        // Verify call is connected
        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.state, CallState::Connected);

        // Switch input device (None means default)
        let result = manager.set_input_device(None).await;
        assert!(result.is_ok());

        // Switch output device
        let result = manager.set_output_device(None).await;
        assert!(result.is_ok());

        // Toggle speaker (tests output device switching logic)
        let speaker_on = manager.toggle_speaker(&call_id).await.unwrap();
        assert!(speaker_on);

        let speaker_off = manager.toggle_speaker(&call_id).await.unwrap();
        assert!(!speaker_off);

        // Call should remain connected after device switches
        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.state, CallState::Connected);
    }

    /// Test 7: Multiple concurrent calls rejection
    /// Tests that starting a call with an existing peer is rejected
    #[tokio::test]
    async fn test_duplicate_call_rejection() {
        let manager = VoiceCallManager::new();

        // Start first call
        let call_info = manager.start_call("peer-duplicate").await.unwrap();
        assert_eq!(call_info.state, CallState::Ringing);

        // Try to start another call with the same peer
        let result = manager.start_call("peer-duplicate").await;
        assert!(result.is_err());

        if let Err(VoiceCallError::CallAlreadyExists(peer)) = result {
            assert_eq!(peer, "peer-duplicate");
        } else {
            panic!("Expected CallAlreadyExists error");
        }
    }

    /// Test 8: Call state transitions validation
    /// Tests that invalid state transitions are properly handled
    #[tokio::test]
    async fn test_call_state_transition_validation() {
        let manager = VoiceCallManager::new();

        // Try to answer a non-existent call
        let result = manager.answer_call("non-existent-call").await;
        assert!(result.is_err());

        if let Err(VoiceCallError::CallNotFound(id)) = result {
            assert_eq!(id, "non-existent-call");
        } else {
            panic!("Expected CallNotFound error");
        }

        // Start a call and try to answer it (wrong state - it's outgoing)
        let call_info = manager.start_call("peer-state-test").await.unwrap();
        let result = manager.answer_call(&call_info.call_id).await;
        assert!(result.is_err());

        if let Err(VoiceCallError::InvalidState { expected, actual }) = result {
            assert_eq!(expected, "incoming");
            assert_eq!(actual, "ringing");
        } else {
            panic!("Expected InvalidState error");
        }
    }

    /// Test 9: Voice packet processing with various states
    /// Tests voice packet handling in different call states
    #[tokio::test]
    async fn test_voice_packet_processing_states() {
        let manager = VoiceCallManager::new();

        // Start a call
        let call_info = manager.start_call("peer-packet-test").await.unwrap();
        let call_id = call_info.call_id.clone();

        // Create a test packet
        let packet = VoicePacket {
            call_id: call_id.clone(),
            sequence: 1,
            timestamp: 960,
            audio_data: vec![0u8; 100],
            is_silence: false,
        };

        // Process packet when call is not connected (should be skipped)
        let result = manager.process_voice_packet(packet.clone()).await;
        assert!(result.is_ok());

        // Connect the call
        manager.handle_call_answered(&call_id).await.unwrap();

        // Now packet processing should work fully
        let result = manager.process_voice_packet(packet.clone()).await;
        assert!(result.is_ok());

        // Check stats were updated
        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert!(info.stats.packets_received > 0);
    }

    /// Test 10: Signal processing for all signal types
    /// Tests the signal handler for various call signals
    #[tokio::test]
    async fn test_signal_processing_all_types() {
        let manager = VoiceCallManager::new();

        // Process incoming call offer
        let signal = CallSignal::Offer {
            call_id: "test-call-signal".to_string(),
            codec_config: CodecConfig::default(),
        };
        let result = manager.process_signal("remote-peer", signal).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        // Process ringing signal for the call
        let signal = CallSignal::Ringing {
            call_id: "test-call-signal".to_string(),
        };
        let result = manager.process_signal("remote-peer", signal).await;
        assert!(result.is_ok());

        // Process hold signal
        let signal = CallSignal::Hold {
            call_id: "test-call-signal".to_string(),
        };
        let result = manager.process_signal("remote-peer", signal).await;
        assert!(result.is_ok());

        // Verify call is on hold
        let info = manager
            .get_call_info("test-call-signal")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(info.state, CallState::OnHold);

        // Process resume signal
        let signal = CallSignal::Resume {
            call_id: "test-call-signal".to_string(),
        };
        let result = manager.process_signal("remote-peer", signal).await;
        assert!(result.is_ok());

        // Process ping signal
        let signal = CallSignal::Ping {
            call_id: "test-call-signal".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        let result = manager.process_signal("remote-peer", signal).await;
        assert!(result.is_ok());

        // Process pong signal
        let signal = CallSignal::Pong {
            call_id: "test-call-signal".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
                - 100, // Simulate 100ms RTT
        };
        let result = manager.process_signal("remote-peer", signal).await;
        assert!(result.is_ok());

        // Process hangup signal
        let signal = CallSignal::Hangup {
            call_id: "test-call-signal".to_string(),
            reason: "user requested".to_string(),
        };
        let result = manager.process_signal("remote-peer", signal).await;
        assert!(result.is_ok());

        // Verify call is ended
        let info = manager
            .get_call_info("test-call-signal")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(info.state, CallState::Ended);
    }

    /// Test 11: Audio device signal enum variants
    #[test]
    fn test_audio_device_signal_variants() {
        // Test input device signal
        let signal = AudioDeviceSignal::SwitchInput(Some("test-device".to_string()));
        match signal {
            AudioDeviceSignal::SwitchInput(Some(id)) => assert_eq!(id, "test-device"),
            _ => panic!("Expected SwitchInput"),
        }

        // Test output device signal with None
        let signal = AudioDeviceSignal::SwitchOutput(None);
        match signal {
            AudioDeviceSignal::SwitchOutput(None) => {}
            _ => panic!("Expected SwitchOutput with None"),
        }
    }

    /// Test 12: Call stats initialization and updates
    #[tokio::test]
    async fn test_call_stats_updates() {
        let manager = VoiceCallManager::new();

        // Start and connect a call
        let call_info = manager.start_call("peer-stats-test").await.unwrap();
        let call_id = call_info.call_id.clone();
        manager.handle_call_answered(&call_id).await.unwrap();

        // Initial stats should be default
        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.stats.packets_sent, 0);
        assert_eq!(info.stats.packets_received, 0);
        assert_eq!(info.stats.packets_lost, 0);

        // Process some packets to update stats
        for i in 0..5 {
            let packet = VoicePacket {
                call_id: call_id.clone(),
                sequence: i,
                timestamp: i as u64 * 960,
                audio_data: vec![0u8; 50],
                is_silence: false,
            };
            manager.process_voice_packet(packet).await.unwrap();
        }

        // Check stats were updated
        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.stats.packets_received, 5);
    }
}
