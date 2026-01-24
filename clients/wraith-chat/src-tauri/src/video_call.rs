// Video Call Manager for WRAITH-Chat
//
// Extends voice calling with video capabilities, including camera capture,
// screen sharing, and adaptive bitrate control for real-time video communication.

use crate::video::{
    AdaptationResult, AdaptiveBitrateController, CameraCapture, CameraDevice, EncodedVideoFrame,
    ScreenCapture, ScreenSource, VideoCodec, VideoConfig, VideoDecoder, VideoEncoder, VideoError,
    VideoFrame, VideoFrameBuffer, VideoResolution,
};
use crate::voice_call::{
    CallDirection, CallInfo as VoiceCallInfo, CallState, CallStats,
    CodecConfig as AudioCodecConfig, VoiceCallError, VoiceCallManager,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock, mpsc};

/// Video call errors
#[derive(Debug, Error)]
pub enum VideoCallError {
    #[error("Call not found: {0}")]
    CallNotFound(String),

    #[error("Call already exists with peer: {0}")]
    CallAlreadyExists(String),

    #[error("Invalid call state: expected {expected}, got {actual}")]
    InvalidState { expected: String, actual: String },

    #[error("Video error: {0}")]
    VideoError(#[from] VideoError),

    #[error("Voice call error: {0}")]
    VoiceError(#[from] VoiceCallError),

    #[error("Transport error: {0}")]
    TransportError(String),

    #[error("Codec error: {0}")]
    CodecError(String),

    #[error("No video stream")]
    NoVideoStream,

    #[error("Camera not available")]
    CameraNotAvailable,

    #[error("Screen capture not available")]
    ScreenCaptureNotAvailable,

    #[error("Video disabled for this call")]
    VideoDisabled,

    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Video source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VideoSource {
    /// Camera video
    #[default]
    Camera,
    /// Screen share
    Screen,
    /// No video (audio only)
    None,
}

/// Video call signaling message types (video-specific signals)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VideoCallSignal {
    /// Video offer (sent with or after voice offer)
    VideoOffer {
        call_id: String,
        video_config: VideoCodecConfig,
        source: VideoSource,
    },

    /// Accept video
    VideoAccept {
        call_id: String,
        video_config: VideoCodecConfig,
    },

    /// Reject video (continue as voice only)
    VideoReject { call_id: String, reason: String },

    /// Enable video during call
    VideoEnable {
        call_id: String,
        source: VideoSource,
    },

    /// Disable video during call
    VideoDisable { call_id: String },

    /// Switch video source
    VideoSourceSwitch {
        call_id: String,
        source: VideoSource,
    },

    /// Request keyframe (for recovery after packet loss)
    KeyframeRequest { call_id: String },

    /// Bandwidth estimation update
    BandwidthUpdate { call_id: String, estimated_bps: u32 },
}

/// Video codec configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCodecConfig {
    /// Codec type
    pub codec: VideoCodec,
    /// Resolution
    pub resolution: VideoResolution,
    /// Framerate
    pub framerate: u32,
    /// Initial bitrate in bps
    pub bitrate: u32,
    /// Enable adaptive bitrate
    pub adaptive: bool,
}

impl Default for VideoCodecConfig {
    fn default() -> Self {
        Self {
            codec: VideoCodec::Vp9,
            resolution: VideoResolution::Hd,
            framerate: 30,
            bitrate: 1_500_000,
            adaptive: true,
        }
    }
}

/// Video packet sent over the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPacket {
    /// Call ID this packet belongs to
    pub call_id: String,
    /// Sequence number for ordering and loss detection
    pub sequence: u32,
    /// Timestamp in microseconds
    pub timestamp_us: u64,
    /// Encoded video data
    #[serde(with = "serde_bytes")]
    pub video_data: Vec<u8>,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Whether this is a keyframe
    pub is_keyframe: bool,
    /// Codec used
    pub codec: VideoCodec,
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

/// Video call statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoCallStats {
    /// Voice call stats (audio)
    pub audio_stats: CallStats,
    /// Video frames sent
    pub video_frames_sent: u64,
    /// Video frames received
    pub video_frames_received: u64,
    /// Video frames dropped
    pub video_frames_dropped: u64,
    /// Current video bitrate in bps
    pub video_bitrate: u32,
    /// Current resolution
    pub current_resolution: VideoResolution,
    /// Average video latency in ms
    pub video_latency_ms: f32,
    /// Video FPS (actual)
    pub actual_fps: f32,
    /// Keyframes requested (packet loss indicator)
    pub keyframes_requested: u64,
}

/// Extended call information with video details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCallInfo {
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
    /// When the call connected (Unix timestamp)
    pub connected_at: Option<i64>,
    /// Whether local audio is muted
    pub audio_muted: bool,
    /// Whether local video is enabled
    pub video_enabled: bool,
    /// Current video source
    pub video_source: VideoSource,
    /// Whether remote video is enabled
    pub remote_video_enabled: bool,
    /// Whether speaker is on
    pub speaker_on: bool,
    /// Video call statistics
    pub stats: VideoCallStats,
}

impl From<VoiceCallInfo> for VideoCallInfo {
    fn from(voice: VoiceCallInfo) -> Self {
        Self {
            call_id: voice.call_id,
            peer_id: voice.peer_id,
            state: voice.state,
            direction: voice.direction,
            started_at: voice.started_at,
            connected_at: voice.connected_at,
            audio_muted: voice.muted,
            video_enabled: false,
            video_source: VideoSource::None,
            remote_video_enabled: false,
            speaker_on: voice.speaker_on,
            stats: VideoCallStats {
                audio_stats: voice.stats,
                ..Default::default()
            },
        }
    }
}

/// Internal video call state
struct VideoCall {
    /// Basic call info
    info: VideoCallInfo,
    /// Video encoder
    encoder: Option<VideoEncoder>,
    /// Video decoder
    decoder: Option<VideoDecoder>,
    /// Camera capture
    camera: Option<CameraCapture>,
    /// Screen capture
    screen_capture: Option<ScreenCapture>,
    /// Video frame buffer (jitter buffer)
    frame_buffer: VideoFrameBuffer,
    /// Adaptive bitrate controller
    abr_controller: AdaptiveBitrateController,
    /// Video sequence number
    sequence: AtomicU32,
    /// Whether video capture is running
    video_running: AtomicBool,
    /// Frames sent counter
    frames_sent: AtomicU64,
    /// Frames received counter
    frames_received: AtomicU64,
    /// Last frame timestamp for FPS calculation
    #[allow(dead_code)]
    last_frame_time: AtomicU64,
    /// FPS accumulator (frames in last second)
    fps_counter: AtomicU32,
}

impl VideoCall {
    fn new(call_id: String, peer_id: String, direction: CallDirection) -> Self {
        let now = chrono::Utc::now().timestamp();

        Self {
            info: VideoCallInfo {
                call_id,
                peer_id,
                state: CallState::Initiating,
                direction,
                started_at: now,
                connected_at: None,
                audio_muted: false,
                video_enabled: false,
                video_source: VideoSource::None,
                remote_video_enabled: false,
                speaker_on: false,
                stats: VideoCallStats::default(),
            },
            encoder: None,
            decoder: None,
            camera: None,
            screen_capture: None,
            frame_buffer: VideoFrameBuffer::new(3), // 3 frame buffer (~100ms at 30fps)
            abr_controller: AdaptiveBitrateController::new(VideoResolution::Hd),
            sequence: AtomicU32::new(0),
            video_running: AtomicBool::new(false),
            frames_sent: AtomicU64::new(0),
            frames_received: AtomicU64::new(0),
            last_frame_time: AtomicU64::new(0),
            fps_counter: AtomicU32::new(0),
        }
    }

    fn enable_video(&mut self, source: VideoSource, config: VideoConfig) -> Result<(), VideoError> {
        // Create encoder
        self.encoder = Some(VideoEncoder::new(config.clone())?);
        self.decoder = Some(VideoDecoder::new(config.clone())?);

        // Initialize capture based on source
        match source {
            VideoSource::Camera => {
                self.camera = Some(CameraCapture::new(config));
                self.screen_capture = None;
            }
            VideoSource::Screen => {
                self.screen_capture = Some(ScreenCapture::new(config));
                self.camera = None;
            }
            VideoSource::None => {
                self.camera = None;
                self.screen_capture = None;
            }
        }

        self.info.video_enabled = source != VideoSource::None;
        self.info.video_source = source;

        Ok(())
    }

    fn disable_video(&mut self) {
        // Stop capture
        if let Some(ref mut camera) = self.camera {
            camera.stop();
        }
        if let Some(ref mut screen) = self.screen_capture {
            screen.stop();
        }

        self.video_running.store(false, Ordering::SeqCst);
        self.info.video_enabled = false;
        self.info.video_source = VideoSource::None;
    }

    fn start_capture(&mut self) -> Result<(), VideoError> {
        self.video_running.store(true, Ordering::SeqCst);

        match self.info.video_source {
            VideoSource::Camera => {
                if let Some(ref mut camera) = self.camera {
                    camera.start()?;
                }
            }
            VideoSource::Screen => {
                if let Some(ref mut screen) = self.screen_capture {
                    screen.start()?;
                }
            }
            VideoSource::None => {}
        }

        Ok(())
    }

    fn stop_capture(&mut self) {
        self.video_running.store(false, Ordering::SeqCst);

        if let Some(ref mut camera) = self.camera {
            camera.stop();
        }
        if let Some(ref mut screen) = self.screen_capture {
            screen.stop();
        }
    }

    fn capture_frame(&mut self) -> Result<VideoFrame, VideoError> {
        match self.info.video_source {
            VideoSource::Camera => self
                .camera
                .as_mut()
                .ok_or(VideoError::CameraError("No camera".to_string()))?
                .capture_frame(),
            VideoSource::Screen => self
                .screen_capture
                .as_mut()
                .ok_or(VideoError::ScreenCaptureError(
                    "No screen capture".to_string(),
                ))?
                .capture_frame(),
            VideoSource::None => Err(VideoError::CaptureError("Video disabled".to_string())),
        }
    }

    fn encode_frame(&mut self, frame: &VideoFrame) -> Result<EncodedVideoFrame, VideoError> {
        self.encoder
            .as_mut()
            .ok_or(VideoError::EncodingError("No encoder".to_string()))?
            .encode(frame)
    }

    fn decode_frame(&mut self, encoded: &EncodedVideoFrame) -> Result<VideoFrame, VideoError> {
        self.decoder
            .as_mut()
            .ok_or(VideoError::DecodingError("No decoder".to_string()))?
            .decode(encoded)
    }

    fn update_stats(&mut self) {
        let frames_sent = self.frames_sent.load(Ordering::Relaxed);
        let frames_received = self.frames_received.load(Ordering::Relaxed);

        self.info.stats.video_frames_sent = frames_sent;
        self.info.stats.video_frames_received = frames_received;
        self.info.stats.video_bitrate = self.abr_controller.current_bitrate();
        self.info.stats.current_resolution = self.abr_controller.target_resolution();

        // Calculate actual FPS
        let fps = self.fps_counter.swap(0, Ordering::Relaxed) as f32;
        self.info.stats.actual_fps = fps;
    }
}

/// Video call manager
///
/// Extends VoiceCallManager with video capabilities.
pub struct VideoCallManager {
    /// Underlying voice call manager for audio
    voice_manager: Arc<VoiceCallManager>,
    /// Video-specific call state
    video_calls: RwLock<HashMap<String, Arc<Mutex<VideoCall>>>>,
    /// Default video configuration
    video_config: VideoConfig,
    /// Channel for outgoing video packets
    video_tx: mpsc::Sender<VideoPacket>,
    /// Handle for receiving outgoing video packets
    video_rx: Mutex<Option<mpsc::Receiver<VideoPacket>>>,
    /// Channel for outgoing video signals
    signal_tx: mpsc::Sender<(String, VideoCallSignal)>,
    /// Handle for receiving signals to send
    signal_rx: Mutex<Option<mpsc::Receiver<(String, VideoCallSignal)>>>,
}

impl VideoCallManager {
    /// Create a new video call manager
    pub fn new() -> Self {
        let (video_tx, video_rx) = mpsc::channel(500); // Lower buffer for video (larger packets)
        let (signal_tx, signal_rx) = mpsc::channel(100);

        Self {
            voice_manager: Arc::new(VoiceCallManager::new()),
            video_calls: RwLock::new(HashMap::new()),
            video_config: VideoConfig::default(),
            video_tx,
            video_rx: Mutex::new(Some(video_rx)),
            signal_tx,
            signal_rx: Mutex::new(Some(signal_rx)),
        }
    }

    /// Create with a custom voice manager
    pub fn with_voice_manager(voice_manager: Arc<VoiceCallManager>) -> Self {
        let (video_tx, video_rx) = mpsc::channel(500);
        let (signal_tx, signal_rx) = mpsc::channel(100);

        Self {
            voice_manager,
            video_calls: RwLock::new(HashMap::new()),
            video_config: VideoConfig::default(),
            video_tx,
            video_rx: Mutex::new(Some(video_rx)),
            signal_tx,
            signal_rx: Mutex::new(Some(signal_rx)),
        }
    }

    /// Get reference to the voice call manager
    pub fn voice_manager(&self) -> &Arc<VoiceCallManager> {
        &self.voice_manager
    }

    /// Take the outgoing video packet receiver
    pub async fn take_video_receiver(&self) -> Option<mpsc::Receiver<VideoPacket>> {
        self.video_rx.lock().await.take()
    }

    /// Take the signal receiver
    pub async fn take_signal_receiver(&self) -> Option<mpsc::Receiver<(String, VideoCallSignal)>> {
        self.signal_rx.lock().await.take()
    }

    /// Start a video call
    pub async fn start_video_call(
        &self,
        peer_id: &str,
        enable_video: bool,
    ) -> Result<VideoCallInfo, VideoCallError> {
        // Start voice call first
        let voice_info = self.voice_manager.start_call(peer_id).await?;

        // Create video call state
        let call_id = voice_info.call_id.clone();
        let mut video_call = VideoCall::new(
            call_id.clone(),
            peer_id.to_string(),
            CallDirection::Outgoing,
        );
        video_call.info.state = CallState::Ringing;

        // Enable video if requested
        if enable_video {
            video_call.enable_video(VideoSource::Camera, self.video_config.clone())?;
        }

        let info = video_call.info.clone();

        // Store video call state
        let mut calls = self.video_calls.write().await;
        calls.insert(call_id.clone(), Arc::new(Mutex::new(video_call)));
        drop(calls);

        // Send video offer if video is enabled
        if enable_video {
            let signal = VideoCallSignal::VideoOffer {
                call_id: call_id.clone(),
                video_config: VideoCodecConfig {
                    codec: self.video_config.codec,
                    resolution: self.video_config.resolution,
                    framerate: self.video_config.framerate,
                    bitrate: self.video_config.effective_bitrate(),
                    adaptive: self.video_config.adaptive_bitrate,
                },
                source: VideoSource::Camera,
            };
            self.signal_tx
                .send((peer_id.to_string(), signal))
                .await
                .map_err(|e| VideoCallError::TransportError(e.to_string()))?;
        }

        Ok(info)
    }

    /// Handle incoming video call
    pub async fn handle_incoming_video_call(
        &self,
        peer_id: &str,
        call_id: &str,
        video_config: Option<VideoCodecConfig>,
    ) -> Result<VideoCallInfo, VideoCallError> {
        // Handle voice portion (ignore result as we track video call info separately)
        let audio_config = AudioCodecConfig::default();
        self.voice_manager
            .handle_incoming_call(peer_id, call_id, audio_config)
            .await?;

        // Create video call state
        let mut video_call = VideoCall::new(
            call_id.to_string(),
            peer_id.to_string(),
            CallDirection::Incoming,
        );
        video_call.info.state = CallState::Incoming;

        // Set up video if offered
        if let Some(config) = video_config {
            video_call.info.remote_video_enabled = true;
            // Create decoder for remote video
            let decoder_config = VideoConfig {
                codec: config.codec,
                resolution: config.resolution,
                framerate: config.framerate,
                ..Default::default()
            };
            video_call.decoder = Some(VideoDecoder::new(decoder_config)?);
        }

        let info = video_call.info.clone();

        let mut calls = self.video_calls.write().await;
        calls.insert(call_id.to_string(), Arc::new(Mutex::new(video_call)));

        Ok(info)
    }

    /// Answer a video call
    pub async fn answer_video_call(
        &self,
        call_id: &str,
        enable_video: bool,
    ) -> Result<VideoCallInfo, VideoCallError> {
        // Answer voice call (ignore result as we track video call info separately)
        self.voice_manager.answer_call(call_id).await?;

        // Get video call state
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;

        if call.info.state != CallState::Incoming {
            return Err(VideoCallError::InvalidState {
                expected: "incoming".to_string(),
                actual: call.info.state.to_string(),
            });
        }

        call.info.state = CallState::Connected;
        call.info.connected_at = Some(chrono::Utc::now().timestamp());

        // Enable local video if requested
        if enable_video {
            call.enable_video(VideoSource::Camera, self.video_config.clone())?;

            // Send video accept
            let signal = VideoCallSignal::VideoAccept {
                call_id: call_id.to_string(),
                video_config: VideoCodecConfig {
                    codec: self.video_config.codec,
                    resolution: self.video_config.resolution,
                    framerate: self.video_config.framerate,
                    bitrate: self.video_config.effective_bitrate(),
                    adaptive: self.video_config.adaptive_bitrate,
                },
            };
            let peer_id = call.info.peer_id.clone();
            drop(call);

            self.signal_tx
                .send((peer_id, signal))
                .await
                .map_err(|e| VideoCallError::TransportError(e.to_string()))?;

            // Start video streams
            self.start_video_streams(call_id).await?;

            let calls = self.video_calls.read().await;
            let call = calls.get(call_id).unwrap().lock().await;
            Ok(call.info.clone())
        } else {
            let info = call.info.clone();
            drop(call);
            Ok(info)
        }
    }

    /// End a video call
    pub async fn end_video_call(&self, call_id: &str, reason: &str) -> Result<(), VideoCallError> {
        // End voice call
        self.voice_manager.end_call(call_id, reason).await?;

        // Stop video
        let calls = self.video_calls.read().await;
        if let Some(call_arc) = calls.get(call_id) {
            let mut call = call_arc.lock().await;
            call.stop_capture();
            call.info.state = CallState::Ended;
        }

        Ok(())
    }

    /// Enable video during a call
    pub async fn enable_video(
        &self,
        call_id: &str,
        source: VideoSource,
    ) -> Result<(), VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;

        if call.info.state != CallState::Connected {
            return Err(VideoCallError::InvalidState {
                expected: "connected".to_string(),
                actual: call.info.state.to_string(),
            });
        }

        call.enable_video(source, self.video_config.clone())?;

        let peer_id = call.info.peer_id.clone();
        drop(call);

        // Notify remote
        let signal = VideoCallSignal::VideoEnable {
            call_id: call_id.to_string(),
            source,
        };
        self.signal_tx
            .send((peer_id, signal))
            .await
            .map_err(|e| VideoCallError::TransportError(e.to_string()))?;

        // Start video streams
        self.start_video_streams(call_id).await?;

        Ok(())
    }

    /// Disable video during a call
    pub async fn disable_video(&self, call_id: &str) -> Result<(), VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;
        call.disable_video();

        let peer_id = call.info.peer_id.clone();
        drop(call);

        // Notify remote
        let signal = VideoCallSignal::VideoDisable {
            call_id: call_id.to_string(),
        };
        self.signal_tx
            .send((peer_id, signal))
            .await
            .map_err(|e| VideoCallError::TransportError(e.to_string()))?;

        Ok(())
    }

    /// Switch video source (camera to screen or vice versa)
    pub async fn switch_video_source(
        &self,
        call_id: &str,
        source: VideoSource,
    ) -> Result<(), VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;

        // Stop current capture
        call.stop_capture();

        // Enable new source
        call.enable_video(source, self.video_config.clone())?;
        call.start_capture()?;

        let peer_id = call.info.peer_id.clone();
        drop(call);

        // Notify remote
        let signal = VideoCallSignal::VideoSourceSwitch {
            call_id: call_id.to_string(),
            source,
        };
        self.signal_tx
            .send((peer_id, signal))
            .await
            .map_err(|e| VideoCallError::TransportError(e.to_string()))?;

        Ok(())
    }

    /// Switch camera (front/back)
    pub async fn switch_camera(&self, call_id: &str) -> Result<(), VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;

        if let Some(ref mut camera) = call.camera {
            camera.switch_camera()?;
        } else {
            return Err(VideoCallError::CameraNotAvailable);
        }

        Ok(())
    }

    /// Toggle audio mute
    pub async fn toggle_mute(&self, call_id: &str) -> Result<bool, VideoCallError> {
        let muted = self.voice_manager.toggle_mute(call_id).await?;

        // Update video call state
        let calls = self.video_calls.read().await;
        if let Some(call_arc) = calls.get(call_id) {
            let mut call = call_arc.lock().await;
            call.info.audio_muted = muted;
        }

        Ok(muted)
    }

    /// Process an incoming video packet
    pub async fn process_video_packet(&self, packet: VideoPacket) -> Result<(), VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(&packet.call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(packet.call_id.clone()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;

        // Update stats
        call.frames_received.fetch_add(1, Ordering::Relaxed);
        call.fps_counter.fetch_add(1, Ordering::Relaxed);

        // Create encoded frame
        let encoded = EncodedVideoFrame {
            data: packet.video_data,
            width: packet.width,
            height: packet.height,
            timestamp_us: packet.timestamp_us,
            is_keyframe: packet.is_keyframe,
            codec: packet.codec,
        };

        // Add to frame buffer
        call.frame_buffer.push(encoded)?;

        // Update ABR with network stats
        // In production, we'd measure actual bandwidth from packet timing
        let estimated_bandwidth = call.abr_controller.current_bitrate();
        let result = call.abr_controller.update(estimated_bandwidth, 0.0, 50.0);

        if let AdaptationResult::BitrateDecreased { new_bitrate, .. } = result {
            // Request keyframe after quality drop
            // Get new_bitrate before borrowing encoder to avoid simultaneous borrows
            let bitrate = new_bitrate;
            if let Some(ref mut encoder) = call.encoder {
                encoder.set_bitrate(bitrate);
            }
        }

        Ok(())
    }

    /// Process an incoming video signal
    pub async fn process_signal(
        &self,
        _peer_id: &str,
        signal: VideoCallSignal,
    ) -> Result<Option<VideoCallInfo>, VideoCallError> {
        match signal {
            VideoCallSignal::VideoOffer {
                call_id,
                video_config,
                source: _,
            } => {
                // Remote is offering video
                let calls = self.video_calls.read().await;
                if let Some(call_arc) = calls.get(&call_id) {
                    let mut call = call_arc.lock().await;
                    call.info.remote_video_enabled = true;

                    // Set up decoder for remote video
                    let decoder_config = VideoConfig {
                        codec: video_config.codec,
                        resolution: video_config.resolution,
                        framerate: video_config.framerate,
                        ..Default::default()
                    };
                    call.decoder = Some(VideoDecoder::new(decoder_config)?);
                    Ok(Some(call.info.clone()))
                } else {
                    Ok(None)
                }
            }
            VideoCallSignal::VideoAccept {
                call_id,
                video_config: _,
            } => {
                // Remote accepted our video
                let calls = self.video_calls.read().await;
                if let Some(call_arc) = calls.get(&call_id) {
                    let mut call = call_arc.lock().await;
                    call.info.remote_video_enabled = true;
                    Ok(Some(call.info.clone()))
                } else {
                    Ok(None)
                }
            }
            VideoCallSignal::VideoReject { call_id, reason: _ } => {
                // Remote rejected video, continue as voice only
                let calls = self.video_calls.read().await;
                if let Some(call_arc) = calls.get(&call_id) {
                    let mut call = call_arc.lock().await;
                    call.disable_video();
                    Ok(Some(call.info.clone()))
                } else {
                    Ok(None)
                }
            }
            VideoCallSignal::VideoEnable { call_id, source: _ } => {
                let calls = self.video_calls.read().await;
                if let Some(call_arc) = calls.get(&call_id) {
                    let mut call = call_arc.lock().await;
                    call.info.remote_video_enabled = true;
                    Ok(Some(call.info.clone()))
                } else {
                    Ok(None)
                }
            }
            VideoCallSignal::VideoDisable { call_id } => {
                let calls = self.video_calls.read().await;
                if let Some(call_arc) = calls.get(&call_id) {
                    let mut call = call_arc.lock().await;
                    call.info.remote_video_enabled = false;
                    Ok(Some(call.info.clone()))
                } else {
                    Ok(None)
                }
            }
            VideoCallSignal::VideoSourceSwitch { call_id, source: _ } => {
                // Remote switched source, we just note it
                let calls = self.video_calls.read().await;
                if let Some(call_arc) = calls.get(&call_id) {
                    let call = call_arc.lock().await;
                    Ok(Some(call.info.clone()))
                } else {
                    Ok(None)
                }
            }
            VideoCallSignal::KeyframeRequest { call_id } => {
                // Remote requested keyframe
                let calls = self.video_calls.read().await;
                if let Some(call_arc) = calls.get(&call_id) {
                    let mut call = call_arc.lock().await;
                    if let Some(ref mut encoder) = call.encoder {
                        encoder.force_keyframe();
                    }
                    call.info.stats.keyframes_requested += 1;
                }
                Ok(None)
            }
            VideoCallSignal::BandwidthUpdate {
                call_id,
                estimated_bps,
            } => {
                // Remote reported bandwidth estimate
                let calls = self.video_calls.read().await;
                if let Some(call_arc) = calls.get(&call_id) {
                    let mut call = call_arc.lock().await;
                    call.abr_controller.update(estimated_bps, 0.0, 50.0);
                    // Get bitrate before borrowing encoder to avoid simultaneous borrows
                    let new_bitrate = call.abr_controller.current_bitrate();
                    if let Some(ref mut encoder) = call.encoder {
                        encoder.set_bitrate(new_bitrate);
                    }
                }
                Ok(None)
            }
        }
    }

    /// Request keyframe from remote (after packet loss)
    pub async fn request_keyframe(&self, call_id: &str) -> Result<(), VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let call = call_arc.lock().await;
        let peer_id = call.info.peer_id.clone();
        drop(call);

        let signal = VideoCallSignal::KeyframeRequest {
            call_id: call_id.to_string(),
        };
        self.signal_tx
            .send((peer_id, signal))
            .await
            .map_err(|e| VideoCallError::TransportError(e.to_string()))?;

        Ok(())
    }

    /// Get video call info
    pub async fn get_call_info(
        &self,
        call_id: &str,
    ) -> Result<Option<VideoCallInfo>, VideoCallError> {
        let calls = self.video_calls.read().await;
        if let Some(call_arc) = calls.get(call_id) {
            let mut call = call_arc.lock().await;
            call.update_stats();
            Ok(Some(call.info.clone()))
        } else {
            Ok(None)
        }
    }

    /// Get all active video calls
    pub async fn get_active_calls(&self) -> Vec<VideoCallInfo> {
        let calls = self.video_calls.read().await;
        let mut result = Vec::new();

        for call_arc in calls.values() {
            let mut call = call_arc.lock().await;
            if call.info.state != CallState::Ended {
                call.update_stats();
                result.push(call.info.clone());
            }
        }

        result
    }

    /// List camera devices
    pub fn list_cameras() -> Result<Vec<CameraDevice>, VideoCallError> {
        Ok(CameraCapture::list_devices()?)
    }

    /// List screen capture sources
    pub fn list_screen_sources() -> Result<Vec<ScreenSource>, VideoCallError> {
        Ok(ScreenCapture::list_sources()?)
    }

    /// Select camera device for a call
    pub async fn select_camera(
        &self,
        call_id: &str,
        device_id: &str,
    ) -> Result<(), VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;
        if let Some(ref mut camera) = call.camera {
            camera.select_device(device_id)?;
        } else {
            return Err(VideoCallError::CameraNotAvailable);
        }

        Ok(())
    }

    /// Select screen capture source for a call
    pub async fn select_screen_source(
        &self,
        call_id: &str,
        source_id: &str,
    ) -> Result<(), VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;
        if let Some(ref mut screen) = call.screen_capture {
            screen.select_source(source_id)?;
        } else {
            return Err(VideoCallError::ScreenCaptureNotAvailable);
        }

        Ok(())
    }

    /// Set video quality
    pub async fn set_video_quality(
        &self,
        call_id: &str,
        resolution: VideoResolution,
    ) -> Result<(), VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;
        call.abr_controller
            .force_bitrate(resolution.target_bitrate(), resolution);

        if let Some(ref mut encoder) = call.encoder {
            encoder.set_bitrate(resolution.target_bitrate());
        }

        call.info.stats.current_resolution = resolution;

        Ok(())
    }

    /// Get the next frame to display for a call
    pub async fn get_next_frame(
        &self,
        call_id: &str,
    ) -> Result<Option<VideoFrame>, VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;

        if !call.frame_buffer.ready() {
            return Ok(None);
        }

        if let Some(encoded) = call.frame_buffer.pop() {
            match call.decode_frame(&encoded) {
                Ok(frame) => Ok(Some(frame)),
                Err(e) => {
                    log::warn!("Frame decode error: {}, requesting keyframe", e);
                    // Could request keyframe here
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Get local preview frame
    pub async fn get_local_preview(
        &self,
        call_id: &str,
    ) -> Result<Option<VideoFrame>, VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        let mut call = call_arc.lock().await;

        if !call.video_running.load(Ordering::SeqCst) {
            return Ok(None);
        }

        match call.capture_frame() {
            Ok(frame) => Ok(Some(frame)),
            Err(_) => Ok(None),
        }
    }

    // Internal helper methods

    async fn start_video_streams(&self, call_id: &str) -> Result<(), VideoCallError> {
        let calls = self.video_calls.read().await;
        let call_arc = calls
            .get(call_id)
            .ok_or_else(|| VideoCallError::CallNotFound(call_id.to_string()))?
            .clone();
        drop(calls);

        // Start capture
        {
            let mut call = call_arc.lock().await;
            call.start_capture()?;
        }

        // Spawn video capture task
        let call_arc_capture = call_arc.clone();
        let video_tx = self.video_tx.clone();
        let call_id_owned = call_id.to_string();

        tokio::spawn(async move {
            Self::video_capture_loop(call_id_owned, call_arc_capture, video_tx).await;
        });

        Ok(())
    }

    async fn video_capture_loop(
        call_id: String,
        call_arc: Arc<Mutex<VideoCall>>,
        video_tx: mpsc::Sender<VideoPacket>,
    ) {
        let framerate = {
            let call = call_arc.lock().await;
            call.encoder
                .as_ref()
                .map(|e| e.config().framerate)
                .unwrap_or(30)
        };

        let frame_interval = std::time::Duration::from_micros(1_000_000 / framerate as u64);

        loop {
            let start = std::time::Instant::now();

            // Check if call is still active
            let should_continue = {
                let call = call_arc.lock().await;
                call.video_running.load(Ordering::SeqCst) && call.info.state == CallState::Connected
            };

            if !should_continue {
                break;
            }

            // Capture and encode frame
            let packet = {
                let mut call = call_arc.lock().await;

                match call.capture_frame() {
                    Ok(frame) => match call.encode_frame(&frame) {
                        Ok(encoded) => {
                            let sequence = call.sequence.fetch_add(1, Ordering::SeqCst);
                            call.frames_sent.fetch_add(1, Ordering::Relaxed);

                            Some(VideoPacket {
                                call_id: call_id.clone(),
                                sequence,
                                timestamp_us: encoded.timestamp_us,
                                video_data: encoded.data,
                                width: encoded.width,
                                height: encoded.height,
                                is_keyframe: encoded.is_keyframe,
                                codec: encoded.codec,
                            })
                        }
                        Err(e) => {
                            log::error!("Video encode error: {}", e);
                            None
                        }
                    },
                    Err(e) => {
                        log::error!("Video capture error: {}", e);
                        None
                    }
                }
            };

            // Send packet
            if let Some(packet) = packet
                && video_tx.send(packet).await.is_err()
            {
                log::error!("Failed to send video packet");
                break;
            }

            // Sleep for remaining frame time
            let elapsed = start.elapsed();
            if elapsed < frame_interval {
                tokio::time::sleep(frame_interval - elapsed).await;
            }
        }

        log::info!("Video capture loop ended for call {}", call_id);
    }
}

impl Default for VideoCallManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_video_call_manager_creation() {
        let manager = VideoCallManager::new();
        let calls = manager.get_active_calls().await;
        assert!(calls.is_empty());
    }

    #[test]
    fn test_video_codec_config_default() {
        let config = VideoCodecConfig::default();
        assert_eq!(config.codec, VideoCodec::Vp9);
        assert_eq!(config.resolution, VideoResolution::Hd);
        assert_eq!(config.framerate, 30);
        assert!(config.adaptive);
    }

    #[test]
    fn test_video_packet_serialization() {
        let packet = VideoPacket {
            call_id: "test-call".to_string(),
            sequence: 1,
            timestamp_us: 33333,
            video_data: vec![0u8; 1000],
            width: 1280,
            height: 720,
            is_keyframe: true,
            codec: VideoCodec::Vp9,
        };

        let json = serde_json::to_string(&packet).unwrap();
        let decoded: VideoPacket = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.call_id, packet.call_id);
        assert_eq!(decoded.sequence, packet.sequence);
        assert_eq!(decoded.is_keyframe, packet.is_keyframe);
    }

    #[test]
    fn test_video_source() {
        assert_eq!(VideoSource::default(), VideoSource::Camera);
    }

    #[test]
    fn test_video_call_info_from_voice() {
        let voice_info = VoiceCallInfo {
            call_id: "test".to_string(),
            peer_id: "peer".to_string(),
            state: CallState::Connected,
            direction: CallDirection::Outgoing,
            started_at: 0,
            connected_at: Some(0),
            muted: false,
            speaker_on: false,
            stats: CallStats::default(),
        };

        let video_info = VideoCallInfo::from(voice_info);
        assert_eq!(video_info.call_id, "test");
        assert!(!video_info.video_enabled);
        assert_eq!(video_info.video_source, VideoSource::None);
    }

    #[test]
    fn test_video_call_stats_default() {
        let stats = VideoCallStats::default();
        assert_eq!(stats.video_frames_sent, 0);
        assert_eq!(stats.video_frames_received, 0);
        assert_eq!(stats.video_bitrate, 0);
    }

    // ==================== Sprint 18.2 Edge Case Tests ====================

    /// Test 1: Camera permission handling
    /// Tests video call behavior when camera access might be unavailable
    #[tokio::test]
    async fn test_camera_permission_handling() {
        let manager = VideoCallManager::new();

        // Start a video call without video initially (permission pending scenario)
        let call_info = manager
            .start_video_call("peer-camera-test", false)
            .await
            .unwrap();
        assert!(!call_info.video_enabled);
        assert_eq!(call_info.video_source, VideoSource::None);

        // List cameras (simulated permission check)
        let cameras = VideoCallManager::list_cameras().unwrap();
        assert!(
            !cameras.is_empty(),
            "Should have simulated cameras available"
        );

        // Now enable video (permission granted scenario)
        // First we need to simulate call being answered
        let calls = manager.video_calls.read().await;
        if let Some(call_arc) = calls.get(&call_info.call_id) {
            let mut call = call_arc.lock().await;
            call.info.state = CallState::Connected;
            call.info.connected_at = Some(chrono::Utc::now().timestamp());
        }
        drop(calls);

        let result = manager
            .enable_video(&call_info.call_id, VideoSource::Camera)
            .await;
        assert!(result.is_ok());

        let info = manager
            .get_call_info(&call_info.call_id)
            .await
            .unwrap()
            .unwrap();
        assert!(info.video_enabled);
        assert_eq!(info.video_source, VideoSource::Camera);
    }

    /// Test 2: Resolution change during call
    /// Tests dynamic resolution switching during an active video call
    #[tokio::test]
    async fn test_resolution_change_during_call() {
        // Create encoder with HD resolution
        let config_hd = VideoConfig {
            resolution: VideoResolution::Hd,
            ..Default::default()
        };
        let mut encoder = VideoEncoder::new(config_hd).unwrap();
        let initial_bitrate = encoder.config().effective_bitrate();
        assert_eq!(initial_bitrate, 1_500_000);

        // Simulate resolution downgrade (e.g., due to bandwidth)
        encoder.set_bitrate(600_000); // Medium bitrate

        // Create a frame and encode at new bitrate
        let frame = VideoFrame::blank(1280, 720);
        let encoded = encoder.encode(&frame).unwrap();

        // Frame should still encode successfully
        assert!(!encoded.data.is_empty());
        assert!(encoded.is_keyframe); // First frame is keyframe

        // Test decoder handles resolution change gracefully
        let config_new = VideoConfig {
            resolution: VideoResolution::Medium,
            ..Default::default()
        };
        let mut decoder = VideoDecoder::new(config_new).unwrap();

        // Decoder should handle frames from different resolutions
        let decoded = decoder.decode(&encoded);
        assert!(decoded.is_ok());
    }

    /// Test 3: Bandwidth adaptation under stress
    /// Tests the adaptive bitrate controller under various network conditions
    #[test]
    fn test_bandwidth_adaptation_under_stress() {
        let mut controller = AdaptiveBitrateController::new(VideoResolution::Hd);
        controller.set_hysteresis(0); // Disable hysteresis for testing

        // Initial state
        assert_eq!(controller.current_bitrate(), 1_500_000);
        assert_eq!(controller.target_resolution(), VideoResolution::Hd);

        // Simulate severe bandwidth drop
        let mut bitrate_decreased = false;
        for _ in 0..50 {
            let result = controller.update(200_000, 15.0, 500.0); // Very poor network
            if matches!(result, AdaptationResult::BitrateDecreased { .. }) {
                bitrate_decreased = true;
            }
        }

        assert!(
            bitrate_decreased,
            "Bitrate should decrease under poor network conditions"
        );
        assert!(
            controller.current_bitrate() < 1_500_000,
            "Bitrate should be lower than initial"
        );

        // Simulate network recovery
        let mut controller2 = AdaptiveBitrateController::new(VideoResolution::Low);
        controller2.set_hysteresis(0);

        let mut bitrate_increased = false;
        for _ in 0..50 {
            let result = controller2.update(5_000_000, 0.1, 20.0); // Excellent network
            if matches!(result, AdaptationResult::BitrateIncreased { .. }) {
                bitrate_increased = true;
            }
        }

        assert!(
            bitrate_increased,
            "Bitrate should increase under good network conditions"
        );
    }

    /// Test 4: Frame drop recovery
    /// Tests video decoder behavior when frames are dropped/lost
    #[test]
    fn test_frame_drop_recovery() {
        let config = VideoConfig {
            resolution: VideoResolution::Low,
            keyframe_interval: 30, // Keyframe every 30 frames
            ..Default::default()
        };

        let mut encoder = VideoEncoder::new(config.clone()).unwrap();
        let mut decoder = VideoDecoder::new(config).unwrap();

        // Encode first frame (keyframe)
        let frame0 = VideoFrame::blank(640, 360);
        let encoded0 = encoder.encode(&frame0).unwrap();
        assert!(encoded0.is_keyframe);

        // Decode keyframe successfully
        let decoded0 = decoder.decode(&encoded0);
        assert!(decoded0.is_ok());

        // Encode frames 1-5
        let mut encoded_frames = Vec::new();
        for i in 1..6 {
            let frame = VideoFrame::new(vec![i as u8; 640 * 360 * 4], 640, 360, i * 33333);
            encoded_frames.push(encoder.encode(&frame).unwrap());
        }

        // Simulate frame loss - skip frames 1-3, try to decode frame 4
        // Decoder should still work (P-frame can be decoded after keyframe)
        let decoded4 = decoder.decode(&encoded_frames[3]);
        assert!(decoded4.is_ok());

        // Reset decoder and try to decode without keyframe first
        decoder.reset();
        let result = decoder.decode(&encoded_frames[4]);
        assert!(result.is_err(), "Should fail without keyframe");
    }

    /// Test 5: Codec fallback (VP9 to VP8)
    /// Tests codec negotiation and fallback scenarios
    #[test]
    fn test_codec_fallback_vp9_to_vp8() {
        // Try VP9 first
        let config_vp9 = VideoConfig {
            codec: VideoCodec::Vp9,
            resolution: VideoResolution::Low,
            ..Default::default()
        };

        let mut encoder_vp9 = VideoEncoder::new(config_vp9).unwrap();
        let frame = VideoFrame::blank(640, 360);
        let encoded_vp9 = encoder_vp9.encode(&frame).unwrap();
        assert_eq!(encoded_vp9.codec, VideoCodec::Vp9);

        // Fallback to VP8
        let config_vp8 = VideoConfig {
            codec: VideoCodec::Vp8,
            resolution: VideoResolution::Low,
            ..Default::default()
        };

        let mut encoder_vp8 = VideoEncoder::new(config_vp8).unwrap();
        let encoded_vp8 = encoder_vp8.encode(&frame).unwrap();
        assert_eq!(encoded_vp8.codec, VideoCodec::Vp8);

        // Both should produce valid output
        assert!(!encoded_vp9.data.is_empty());
        assert!(!encoded_vp8.data.is_empty());

        // Create decoders for each codec
        let config_decoder_vp9 = VideoConfig {
            codec: VideoCodec::Vp9,
            resolution: VideoResolution::Low,
            ..Default::default()
        };
        let mut decoder_vp9 = VideoDecoder::new(config_decoder_vp9).unwrap();
        let decoded = decoder_vp9.decode(&encoded_vp9);
        assert!(decoded.is_ok());

        let config_decoder_vp8 = VideoConfig {
            codec: VideoCodec::Vp8,
            resolution: VideoResolution::Low,
            ..Default::default()
        };
        let mut decoder_vp8 = VideoDecoder::new(config_decoder_vp8).unwrap();
        let decoded = decoder_vp8.decode(&encoded_vp8);
        assert!(decoded.is_ok());
    }

    /// Test 6: Screen share on simulated multi-monitor
    /// Tests screen capture source selection with multiple displays
    #[test]
    fn test_screen_share_multi_monitor() {
        // List all screen sources (simulated multi-monitor)
        let sources = ScreenCapture::list_sources().unwrap();

        // Should have multiple sources (screens + windows)
        assert!(sources.len() >= 2, "Should have multiple capture sources");

        // Find screen sources specifically
        let screens: Vec<&ScreenSource> = sources.iter().filter(|s| s.is_screen).collect();
        assert!(screens.len() >= 2, "Should have at least 2 screen sources");

        // Find window sources
        let windows: Vec<&ScreenSource> = sources.iter().filter(|s| !s.is_screen).collect();
        assert!(!windows.is_empty(), "Should have window sources");

        // Create screen capture and select different sources
        let config = VideoConfig::default();
        let mut capture = ScreenCapture::new(config);

        // Select primary display
        let primary = sources
            .iter()
            .find(|s| s.is_screen && s.id == "screen-0")
            .unwrap();
        capture.select_source(&primary.id).unwrap();
        assert_eq!(capture.current_source(), Some(primary.id.as_str()));

        // Select secondary display
        let secondary = sources
            .iter()
            .find(|s| s.is_screen && s.id == "screen-1")
            .unwrap();
        capture.select_source(&secondary.id).unwrap();
        assert_eq!(capture.current_source(), Some(secondary.id.as_str()));

        // Select a window
        let window = windows.first().unwrap();
        capture.select_source(&window.id).unwrap();
        assert_eq!(capture.current_source(), Some(window.id.as_str()));

        // Start capture and verify it works
        capture.start().unwrap();
        assert!(capture.is_running());

        // Capture a frame
        let frame = capture.capture_frame().unwrap();
        assert!(!frame.data.is_empty());

        capture.stop();
        assert!(!capture.is_running());
    }

    /// Test 7: Video call signal handling - all signal types
    #[tokio::test]
    async fn test_video_signal_handling_all_types() {
        let manager = VideoCallManager::new();

        // Start a video call
        let call_info = manager
            .start_video_call("peer-signal-test", true)
            .await
            .unwrap();
        let call_id = call_info.call_id.clone();

        // Simulate call being answered
        {
            let calls = manager.video_calls.read().await;
            if let Some(call_arc) = calls.get(&call_id) {
                let mut call = call_arc.lock().await;
                call.info.state = CallState::Connected;
                call.info.connected_at = Some(chrono::Utc::now().timestamp());
            }
        }

        // Test VideoAccept signal
        let signal = VideoCallSignal::VideoAccept {
            call_id: call_id.clone(),
            video_config: VideoCodecConfig::default(),
        };
        let result = manager.process_signal("peer-signal-test", signal).await;
        assert!(result.is_ok());

        // Test VideoEnable signal
        let signal = VideoCallSignal::VideoEnable {
            call_id: call_id.clone(),
            source: VideoSource::Camera,
        };
        let result = manager.process_signal("peer-signal-test", signal).await;
        assert!(result.is_ok());

        let info = result.unwrap().unwrap();
        assert!(info.remote_video_enabled);

        // Test VideoDisable signal
        let signal = VideoCallSignal::VideoDisable {
            call_id: call_id.clone(),
        };
        let result = manager.process_signal("peer-signal-test", signal).await;
        assert!(result.is_ok());

        let info = result.unwrap().unwrap();
        assert!(!info.remote_video_enabled);

        // Test KeyframeRequest signal
        let signal = VideoCallSignal::KeyframeRequest {
            call_id: call_id.clone(),
        };
        let result = manager.process_signal("peer-signal-test", signal).await;
        assert!(result.is_ok());

        // Test BandwidthUpdate signal
        let signal = VideoCallSignal::BandwidthUpdate {
            call_id: call_id.clone(),
            estimated_bps: 1_000_000,
        };
        let result = manager.process_signal("peer-signal-test", signal).await;
        assert!(result.is_ok());
    }

    /// Test 8: Video frame buffer ordering and overflow
    #[test]
    fn test_video_frame_buffer_ordering_overflow() {
        let mut buffer = VideoFrameBuffer::new(3);

        // Add keyframe
        let keyframe = EncodedVideoFrame {
            data: vec![b'W', b'V', b'I', b'D', 0x01, 0, 5, 0],
            width: 1280,
            height: 720,
            timestamp_us: 0,
            is_keyframe: true,
            codec: VideoCodec::Vp9,
        };
        buffer.push(keyframe).unwrap();

        // Add frames out of order
        for ts in [100000u64, 33333, 66666, 133333, 166666] {
            let frame = EncodedVideoFrame {
                data: vec![b'W', b'V', b'I', b'D', 0x00, 0, 5, 0],
                width: 1280,
                height: 720,
                timestamp_us: ts,
                is_keyframe: false,
                codec: VideoCodec::Vp9,
            };
            buffer.push(frame).unwrap();
        }

        // Buffer should reorder by timestamp
        assert!(buffer.ready());

        // Pop and verify ordering
        let first = buffer.pop().unwrap();
        assert!(first.is_keyframe);
        assert_eq!(first.timestamp_us, 0);

        let second = buffer.pop().unwrap();
        assert_eq!(second.timestamp_us, 33333);

        let third = buffer.pop().unwrap();
        assert_eq!(third.timestamp_us, 66666);

        // Continue popping - should maintain order
        let fourth = buffer.pop().unwrap();
        assert_eq!(fourth.timestamp_us, 100000);
    }

    /// Test 9: Camera switching during active call
    #[tokio::test]
    async fn test_camera_switching_during_call() {
        let config = VideoConfig::default();
        let mut capture = CameraCapture::new(config);

        // List cameras
        let cameras = CameraCapture::list_devices().unwrap();
        assert!(cameras.len() >= 2, "Need at least 2 cameras for this test");

        // Start with default camera
        capture.start().unwrap();
        assert!(capture.is_running());

        // Get initial device
        let initial_device = capture.current_device().map(|s| s.to_string());

        // Switch camera
        capture.switch_camera().unwrap();
        let new_device = capture.current_device().map(|s| s.to_string());

        // Device should have changed
        assert_ne!(initial_device, new_device, "Camera should have switched");

        // Capture should still work
        let frame = capture.capture_frame().unwrap();
        assert!(!frame.data.is_empty());

        // Switch back
        capture.switch_camera().unwrap();
        let final_device = capture.current_device().map(|s| s.to_string());

        // Should be back to original (or different if only 2 cameras)
        assert!(final_device.is_some());

        capture.stop();
    }

    /// Test 10: Video quality settings persistence
    #[tokio::test]
    async fn test_video_quality_settings() {
        let manager = VideoCallManager::new();

        // Start a video call
        let call_info = manager
            .start_video_call("peer-quality-test", true)
            .await
            .unwrap();
        let call_id = call_info.call_id.clone();

        // Simulate call being connected
        {
            let calls = manager.video_calls.read().await;
            if let Some(call_arc) = calls.get(&call_id) {
                let mut call = call_arc.lock().await;
                call.info.state = CallState::Connected;
            }
        }

        // Set video quality to Low
        manager
            .set_video_quality(&call_id, VideoResolution::Low)
            .await
            .unwrap();

        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.stats.current_resolution, VideoResolution::Low);

        // Set to Medium
        manager
            .set_video_quality(&call_id, VideoResolution::Medium)
            .await
            .unwrap();

        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.stats.current_resolution, VideoResolution::Medium);

        // Set to HD
        manager
            .set_video_quality(&call_id, VideoResolution::Hd)
            .await
            .unwrap();

        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert_eq!(info.stats.current_resolution, VideoResolution::Hd);
    }

    /// Test 11: Video disable/enable toggle during call
    #[tokio::test]
    async fn test_video_disable_enable_toggle() {
        let manager = VideoCallManager::new();

        // Start a video call with video enabled
        let call_info = manager
            .start_video_call("peer-toggle-test", true)
            .await
            .unwrap();
        let call_id = call_info.call_id.clone();

        assert!(call_info.video_enabled);

        // Simulate call being connected
        {
            let calls = manager.video_calls.read().await;
            if let Some(call_arc) = calls.get(&call_id) {
                let mut call = call_arc.lock().await;
                call.info.state = CallState::Connected;
            }
        }

        // Disable video
        manager.disable_video(&call_id).await.unwrap();

        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert!(!info.video_enabled);
        assert_eq!(info.video_source, VideoSource::None);

        // Re-enable video
        manager
            .enable_video(&call_id, VideoSource::Camera)
            .await
            .unwrap();

        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert!(info.video_enabled);
        assert_eq!(info.video_source, VideoSource::Camera);

        // Switch to screen share
        manager
            .switch_video_source(&call_id, VideoSource::Screen)
            .await
            .unwrap();

        let info = manager.get_call_info(&call_id).await.unwrap().unwrap();
        assert!(info.video_enabled);
        assert_eq!(info.video_source, VideoSource::Screen);
    }

    /// Test 12: Network quality estimation accuracy
    #[test]
    fn test_network_quality_estimation_boundaries() {
        use crate::video::NetworkQuality;

        // Test exact boundary conditions
        // Excellent: <1% loss, <100ms RTT
        assert_eq!(
            NetworkQuality::estimate(0.9, 99.9),
            NetworkQuality::Excellent
        );
        assert_eq!(NetworkQuality::estimate(1.0, 100.0), NetworkQuality::Good); // At boundary

        // Good: <3% loss, <200ms RTT
        assert_eq!(NetworkQuality::estimate(2.9, 199.9), NetworkQuality::Good);
        assert_eq!(NetworkQuality::estimate(3.0, 200.0), NetworkQuality::Fair); // At boundary

        // Fair: <5% loss, <300ms RTT
        assert_eq!(NetworkQuality::estimate(4.9, 299.9), NetworkQuality::Fair);
        assert_eq!(NetworkQuality::estimate(5.0, 300.0), NetworkQuality::Poor); // At boundary

        // Poor: <10% loss, <500ms RTT
        assert_eq!(NetworkQuality::estimate(9.9, 499.9), NetworkQuality::Poor);
        assert_eq!(
            NetworkQuality::estimate(10.0, 500.0),
            NetworkQuality::Critical
        ); // At boundary

        // Critical: >=10% loss or >=500ms RTT
        assert_eq!(
            NetworkQuality::estimate(20.0, 800.0),
            NetworkQuality::Critical
        );

        // Edge case: good loss but bad RTT
        assert_eq!(
            NetworkQuality::estimate(0.5, 600.0),
            NetworkQuality::Critical
        );

        // Edge case: bad loss but good RTT
        assert_eq!(
            NetworkQuality::estimate(15.0, 50.0),
            NetworkQuality::Critical
        );
    }
}
