// Voice Call Manager for WRAITH-Chat
//
// Handles real-time voice transport over WRAITH protocol streams,
// including call signaling, audio capture/playback, and call state management.

use crate::audio::{
    AudioConfig, AudioDevice, AudioDeviceManager, AudioFrame, JitterBuffer, VoiceDecoder,
    VoiceEncoder,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Observer, Producer, Split};
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
    pub async fn toggle_speaker(&self, call_id: &str) -> Result<bool, VoiceCallError> {
        let calls = self.calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VoiceCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;
        call.info.speaker_on = !call.info.speaker_on;
        // TODO: Actually switch audio output device
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
    pub async fn set_input_device(&self, device_id: Option<String>) -> Result<(), VoiceCallError> {
        let mut device_manager = self.device_manager.write().await;
        device_manager.set_input_device(device_id);
        Ok(())
    }

    /// Set the output audio device
    pub async fn set_output_device(&self, device_id: Option<String>) -> Result<(), VoiceCallError> {
        let mut device_manager = self.device_manager.write().await;
        device_manager.set_output_device(device_id);
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

        let call_id = call_id.to_string();
        let outgoing_tx = self.outgoing_tx.clone();

        // Spawn audio capture task
        let call_arc_capture = call_arc.clone();
        tokio::spawn(async move {
            Self::audio_capture_loop(call_id, call_arc_capture, outgoing_tx).await;
        });

        // Spawn audio playback task
        let call_arc_playback = call_arc.clone();
        tokio::spawn(async move {
            Self::audio_playback_loop(call_arc_playback).await;
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
    ) {
        // Create ring buffer for audio samples - shared between async task and audio thread
        let rb = HeapRb::<i16>::new(4800); // 100ms buffer
        let (producer, consumer) = rb.split();

        // Wrap in Arc<Mutex> for thread-safe access
        let producer = Arc::new(std::sync::Mutex::new(producer));
        let consumer = Arc::new(std::sync::Mutex::new(consumer));

        // Flag to signal the audio thread to stop
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let producer_clone = producer.clone();

        // Spawn the audio input stream in a blocking thread (since Stream is !Send)
        let audio_handle = std::thread::spawn(move || {
            let host = Self::get_preferred_host();
            let device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    log::error!("No input audio device available");
                    return;
                }
            };

            let config = cpal::StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(48000),
                buffer_size: cpal::BufferSize::Fixed(960),
            };

            let err_fn = |err| log::error!("Audio capture error: {}", err);

            let stream = device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Convert f32 to i16 and push to ring buffer
                    if let Ok(mut prod) = producer_clone.lock() {
                        for &sample in data {
                            let sample_i16 = (sample * 32767.0) as i16;
                            let _ = prod.try_push(sample_i16);
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
                    return;
                }
            };

            if let Err(e) = stream.play() {
                log::error!("Failed to start input stream: {}", e);
                return;
            }

            // Keep the stream alive while running
            while running_clone.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            drop(stream);
        });

        let mut frame_buffer = vec![0i16; 960];
        let mut encode_buffer = vec![0u8; 4000];

        // Main async loop: read from ring buffer, encode, and send
        loop {
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

    async fn audio_playback_loop(call_arc: Arc<Mutex<Call>>) {
        // Create ring buffer for playback - shared between async task and audio thread
        let rb = HeapRb::<i16>::new(9600); // 200ms buffer
        let (producer, consumer) = rb.split();

        // Wrap in Arc<Mutex> for thread-safe access
        let producer = Arc::new(std::sync::Mutex::new(producer));
        let consumer = Arc::new(std::sync::Mutex::new(consumer));

        // Flag to signal the audio thread to stop
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let consumer_clone = consumer.clone();

        // Spawn the audio output stream in a blocking thread (since Stream is !Send)
        let audio_handle = std::thread::spawn(move || {
            let host = Self::get_preferred_host();
            let device = match host.default_output_device() {
                Some(d) => d,
                None => {
                    log::error!("No output audio device available");
                    return;
                }
            };

            let config = cpal::StreamConfig {
                channels: 1,
                sample_rate: cpal::SampleRate(48000),
                buffer_size: cpal::BufferSize::Fixed(960),
            };

            let err_fn = |err| log::error!("Audio playback error: {}", err);

            let stream = device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut cons) = consumer_clone.lock() {
                        for sample in data.iter_mut() {
                            *sample = cons.try_pop().map(|s| s as f32 / 32767.0).unwrap_or(0.0);
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
                    return;
                }
            };

            if let Err(e) = stream.play() {
                log::error!("Failed to start output stream: {}", e);
                return;
            }

            // Keep the stream alive while running
            while running_clone.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            drop(stream);
        });

        // Main async loop: feed audio from jitter buffer to the ring buffer
        loop {
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
}
