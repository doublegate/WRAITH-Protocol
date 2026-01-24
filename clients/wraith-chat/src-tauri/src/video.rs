// Video Processing Module for WRAITH-Chat Video Calling
//
// Provides VP9/VP8 video codec encoding/decoding, camera capture, screen capture,
// and adaptive bitrate control for real-time video communication.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use thiserror::Error;

/// Video processing errors
#[derive(Debug, Error)]
pub enum VideoError {
    #[error("Codec error: {0}")]
    CodecError(String),

    #[error("Camera error: {0}")]
    CameraError(String),

    #[error("Screen capture error: {0}")]
    ScreenCaptureError(String),

    #[error("No camera available")]
    NoCameraAvailable,

    #[error("Invalid resolution: {0}x{1}")]
    InvalidResolution(u32, u32),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Capture error: {0}")]
    CaptureError(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Decoding error: {0}")]
    DecodingError(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Buffer overflow")]
    BufferOverflow,
}

/// Video resolution presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VideoResolution {
    /// 240p - Ultra low bandwidth mode
    UltraLow, // 426x240
    /// 360p - Low bandwidth mode (300kbps target)
    Low, // 640x360
    /// 480p - Medium quality (600kbps target)
    Medium, // 854x480
    /// 720p - HD quality (1.5Mbps target)
    #[default]
    Hd, // 1280x720
    /// 1080p - Full HD (3Mbps target)
    FullHd, // 1920x1080
}

impl VideoResolution {
    /// Get width and height for this resolution
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            VideoResolution::UltraLow => (426, 240),
            VideoResolution::Low => (640, 360),
            VideoResolution::Medium => (854, 480),
            VideoResolution::Hd => (1280, 720),
            VideoResolution::FullHd => (1920, 1080),
        }
    }

    /// Get target bitrate in bps for this resolution
    pub fn target_bitrate(&self) -> u32 {
        match self {
            VideoResolution::UltraLow => 150_000, // 150 kbps
            VideoResolution::Low => 300_000,      // 300 kbps
            VideoResolution::Medium => 600_000,   // 600 kbps
            VideoResolution::Hd => 1_500_000,     // 1.5 Mbps
            VideoResolution::FullHd => 3_000_000, // 3 Mbps
        }
    }

    /// Get minimum bitrate in bps for this resolution
    pub fn min_bitrate(&self) -> u32 {
        self.target_bitrate() / 2
    }

    /// Get maximum bitrate in bps for this resolution
    pub fn max_bitrate(&self) -> u32 {
        self.target_bitrate() * 3 / 2
    }

    /// Get recommended framerate for this resolution
    pub fn recommended_fps(&self) -> u32 {
        match self {
            VideoResolution::UltraLow => 15,
            VideoResolution::Low => 24,
            VideoResolution::Medium | VideoResolution::Hd | VideoResolution::FullHd => 30,
        }
    }

    /// Get the next lower resolution for quality degradation
    pub fn lower(&self) -> Option<VideoResolution> {
        match self {
            VideoResolution::FullHd => Some(VideoResolution::Hd),
            VideoResolution::Hd => Some(VideoResolution::Medium),
            VideoResolution::Medium => Some(VideoResolution::Low),
            VideoResolution::Low => Some(VideoResolution::UltraLow),
            VideoResolution::UltraLow => None,
        }
    }

    /// Get the next higher resolution for quality improvement
    pub fn higher(&self) -> Option<VideoResolution> {
        match self {
            VideoResolution::UltraLow => Some(VideoResolution::Low),
            VideoResolution::Low => Some(VideoResolution::Medium),
            VideoResolution::Medium => Some(VideoResolution::Hd),
            VideoResolution::Hd => Some(VideoResolution::FullHd),
            VideoResolution::FullHd => None,
        }
    }
}

/// Video codec type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VideoCodec {
    /// VP8 - Good compatibility, lower quality
    Vp8,
    /// VP9 - Better quality, higher CPU usage
    #[default]
    Vp9,
}

/// Video configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    /// Video codec to use
    pub codec: VideoCodec,
    /// Resolution preset
    pub resolution: VideoResolution,
    /// Target framerate (frames per second)
    pub framerate: u32,
    /// Target bitrate in bps (overrides resolution default if set)
    pub bitrate: Option<u32>,
    /// Enable adaptive bitrate
    pub adaptive_bitrate: bool,
    /// Keyframe interval in frames
    pub keyframe_interval: u32,
    /// Enable hardware acceleration (if available)
    pub hardware_acceleration: bool,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            codec: VideoCodec::default(),
            resolution: VideoResolution::Hd,
            framerate: 30,
            bitrate: None,
            adaptive_bitrate: true,
            keyframe_interval: 60, // Keyframe every 2 seconds at 30fps
            hardware_acceleration: true,
        }
    }
}

impl VideoConfig {
    /// Get the effective bitrate (custom or resolution default)
    pub fn effective_bitrate(&self) -> u32 {
        self.bitrate
            .unwrap_or_else(|| self.resolution.target_bitrate())
    }

    /// Create a low bandwidth configuration
    pub fn low_bandwidth() -> Self {
        Self {
            codec: VideoCodec::Vp8,
            resolution: VideoResolution::Low,
            framerate: 24,
            bitrate: Some(300_000),
            adaptive_bitrate: true,
            keyframe_interval: 48,
            hardware_acceleration: true,
        }
    }

    /// Create a high quality configuration
    pub fn high_quality() -> Self {
        Self {
            codec: VideoCodec::Vp9,
            resolution: VideoResolution::Hd,
            framerate: 30,
            bitrate: Some(1_500_000),
            adaptive_bitrate: true,
            keyframe_interval: 60,
            hardware_acceleration: true,
        }
    }
}

/// Video frame data
#[derive(Clone)]
pub struct VideoFrame {
    /// Raw pixel data (RGBA format)
    pub data: Vec<u8>,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Frame timestamp in microseconds
    pub timestamp_us: u64,
    /// Whether this is a keyframe
    pub is_keyframe: bool,
}

impl VideoFrame {
    /// Create a new video frame
    pub fn new(data: Vec<u8>, width: u32, height: u32, timestamp_us: u64) -> Self {
        Self {
            data,
            width,
            height,
            timestamp_us,
            is_keyframe: false,
        }
    }

    /// Create a blank frame (useful for testing)
    pub fn blank(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize; // RGBA
        Self {
            data: vec![0u8; size],
            width,
            height,
            timestamp_us: 0,
            is_keyframe: false,
        }
    }

    /// Get the expected data size for a resolution
    pub fn expected_size(width: u32, height: u32) -> usize {
        (width * height * 4) as usize // RGBA format
    }
}

/// Encoded video frame
#[derive(Clone, Serialize, Deserialize)]
pub struct EncodedVideoFrame {
    /// Encoded data (VP9/VP8)
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Timestamp in microseconds
    pub timestamp_us: u64,
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

/// Simulated video encoder (placeholder for actual vpx integration)
///
/// Note: In production, this would use libvpx bindings (vpx-rs crate) for actual
/// VP8/VP9 encoding. This implementation provides the interface and simulates
/// encoding for testing purposes.
pub struct VideoEncoder {
    config: VideoConfig,
    frame_count: u64,
    current_bitrate: AtomicU32,
    /// Simulated encoded data for testing
    last_frame_size: AtomicU32,
}

impl VideoEncoder {
    /// Create a new video encoder
    pub fn new(config: VideoConfig) -> Result<Self, VideoError> {
        let bitrate = config.effective_bitrate();

        Ok(Self {
            config,
            frame_count: 0,
            current_bitrate: AtomicU32::new(bitrate),
            last_frame_size: AtomicU32::new(0),
        })
    }

    /// Encode a raw video frame
    ///
    /// Returns the encoded data or an error.
    pub fn encode(&mut self, frame: &VideoFrame) -> Result<EncodedVideoFrame, VideoError> {
        let is_keyframe = self.frame_count == 0
            || self
                .frame_count
                .is_multiple_of(self.config.keyframe_interval as u64);

        // Calculate simulated encoded size based on bitrate
        // Real VP9 would compress significantly more
        let bitrate = self.current_bitrate.load(Ordering::Relaxed);
        let bits_per_frame = bitrate / self.config.framerate;
        let bytes_per_frame = (bits_per_frame / 8) as usize;

        // Keyframes are typically 2-5x larger
        let encoded_size = if is_keyframe {
            bytes_per_frame * 3
        } else {
            bytes_per_frame
        };

        // Simulate encoding - in production this would call libvpx
        // For now, create a header + placeholder data
        let mut encoded_data = Vec::with_capacity(encoded_size);

        // Simple header (8 bytes): magic + frame info
        encoded_data.extend_from_slice(b"WVID"); // Magic
        encoded_data.push(if is_keyframe { 0x01 } else { 0x00 }); // Frame type
        encoded_data.push(self.config.codec as u8); // Codec
        encoded_data.push((frame.width >> 8) as u8); // Width high byte
        encoded_data.push((frame.width & 0xFF) as u8); // Width low byte

        // Add simulated compressed data (in production: actual VP9/VP8 bitstream)
        // For testing, we'll add some placeholder bytes
        let payload_size = encoded_size.saturating_sub(8);
        encoded_data.resize(8 + payload_size, 0);

        // Simple "compression": copy some samples from input
        if !frame.data.is_empty() && payload_size > 0 {
            let sample_step = frame.data.len() / payload_size.max(1);
            for (i, byte) in encoded_data[8..].iter_mut().enumerate() {
                let src_idx = (i * sample_step) % frame.data.len();
                *byte = frame.data[src_idx];
            }
        }

        self.frame_count += 1;
        self.last_frame_size
            .store(encoded_data.len() as u32, Ordering::Relaxed);

        Ok(EncodedVideoFrame {
            data: encoded_data,
            width: frame.width,
            height: frame.height,
            timestamp_us: frame.timestamp_us,
            is_keyframe,
            codec: self.config.codec,
        })
    }

    /// Update encoder bitrate (for adaptive bitrate)
    pub fn set_bitrate(&mut self, bitrate: u32) {
        self.current_bitrate.store(bitrate, Ordering::Relaxed);
        log::debug!("Video encoder bitrate set to {} bps", bitrate);
    }

    /// Get current configuration
    pub fn config(&self) -> &VideoConfig {
        &self.config
    }

    /// Force next frame to be a keyframe
    pub fn force_keyframe(&mut self) {
        // Reset frame count to trigger keyframe
        self.frame_count = 0;
    }

    /// Get the last encoded frame size
    pub fn last_frame_size(&self) -> u32 {
        self.last_frame_size.load(Ordering::Relaxed)
    }
}

/// Video decoder
pub struct VideoDecoder {
    config: VideoConfig,
    /// Whether we have received a keyframe
    has_keyframe: bool,
}

impl VideoDecoder {
    /// Create a new video decoder
    pub fn new(config: VideoConfig) -> Result<Self, VideoError> {
        Ok(Self {
            config,
            has_keyframe: false,
        })
    }

    /// Decode an encoded video frame
    pub fn decode(&mut self, encoded: &EncodedVideoFrame) -> Result<VideoFrame, VideoError> {
        // Validate header
        if encoded.data.len() < 8 {
            return Err(VideoError::DecodingError("Frame too small".to_string()));
        }

        if &encoded.data[0..4] != b"WVID" {
            return Err(VideoError::DecodingError(
                "Invalid frame header".to_string(),
            ));
        }

        let is_keyframe = encoded.data[4] == 0x01;

        // Must receive keyframe first
        if !self.has_keyframe && !is_keyframe {
            return Err(VideoError::DecodingError(
                "Waiting for keyframe".to_string(),
            ));
        }

        if is_keyframe {
            self.has_keyframe = true;
        }

        // Simulate decoding - in production this would call libvpx decoder
        let frame_size = VideoFrame::expected_size(encoded.width, encoded.height);
        let mut decoded_data = vec![0u8; frame_size];

        // Simple "decompression": expand samples back
        let payload = &encoded.data[8..];
        if !payload.is_empty() {
            let expand_step = frame_size / payload.len().max(1);
            for (i, &byte) in payload.iter().enumerate() {
                let start = i * expand_step;
                let end = ((i + 1) * expand_step).min(frame_size);
                for pixel in decoded_data[start..end].iter_mut() {
                    *pixel = byte;
                }
            }
        }

        Ok(VideoFrame {
            data: decoded_data,
            width: encoded.width,
            height: encoded.height,
            timestamp_us: encoded.timestamp_us,
            is_keyframe,
        })
    }

    /// Reset decoder state (call when seeking or after packet loss)
    pub fn reset(&mut self) {
        self.has_keyframe = false;
    }

    /// Get current configuration
    pub fn config(&self) -> &VideoConfig {
        &self.config
    }
}

/// Camera device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDevice {
    /// Unique device identifier
    pub id: String,
    /// Human-readable device name
    pub name: String,
    /// Whether this is the default device
    pub is_default: bool,
    /// Whether this is the front camera (mobile)
    pub is_front: bool,
    /// Supported resolutions
    pub supported_resolutions: Vec<(u32, u32)>,
}

/// Camera capture manager
pub struct CameraCapture {
    /// Currently selected device
    current_device: Option<String>,
    /// Current capture configuration
    config: VideoConfig,
    /// Whether capture is active
    running: Arc<AtomicBool>,
    /// Frame counter
    frame_count: u64,
}

impl CameraCapture {
    /// Create a new camera capture instance
    pub fn new(config: VideoConfig) -> Self {
        Self {
            current_device: None,
            config,
            running: Arc::new(AtomicBool::new(false)),
            frame_count: 0,
        }
    }

    /// List available camera devices
    pub fn list_devices() -> Result<Vec<CameraDevice>, VideoError> {
        // In production, this would use platform-specific APIs:
        // - Linux: v4l2
        // - macOS: AVFoundation
        // - Windows: Media Foundation

        // For now, return simulated devices for testing
        Ok(vec![
            CameraDevice {
                id: "camera-0".to_string(),
                name: "Default Camera".to_string(),
                is_default: true,
                is_front: true,
                supported_resolutions: vec![(640, 360), (854, 480), (1280, 720), (1920, 1080)],
            },
            CameraDevice {
                id: "camera-1".to_string(),
                name: "Back Camera".to_string(),
                is_default: false,
                is_front: false,
                supported_resolutions: vec![(640, 360), (1280, 720), (1920, 1080)],
            },
        ])
    }

    /// Select a camera device
    pub fn select_device(&mut self, device_id: &str) -> Result<(), VideoError> {
        // Validate device exists
        let devices = Self::list_devices()?;
        if !devices.iter().any(|d| d.id == device_id) {
            return Err(VideoError::DeviceNotFound(device_id.to_string()));
        }

        self.current_device = Some(device_id.to_string());
        log::info!("Selected camera device: {}", device_id);
        Ok(())
    }

    /// Get the currently selected device
    pub fn current_device(&self) -> Option<&str> {
        self.current_device.as_deref()
    }

    /// Start capturing
    pub fn start(&mut self) -> Result<(), VideoError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        if self.current_device.is_none() {
            // Select default device
            let devices = Self::list_devices()?;
            if let Some(default) = devices.iter().find(|d| d.is_default) {
                self.current_device = Some(default.id.clone());
            } else if let Some(first) = devices.first() {
                self.current_device = Some(first.id.clone());
            } else {
                return Err(VideoError::NoCameraAvailable);
            }
        }

        self.running.store(true, Ordering::SeqCst);
        log::info!(
            "Camera capture started on device: {:?}",
            self.current_device
        );
        Ok(())
    }

    /// Stop capturing
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        log::info!("Camera capture stopped");
    }

    /// Check if capture is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Capture a frame (for testing/simulation)
    ///
    /// In production, this would be called by a capture thread/callback
    pub fn capture_frame(&mut self) -> Result<VideoFrame, VideoError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(VideoError::CaptureError("Camera not running".to_string()));
        }

        let (width, height) = self.config.resolution.dimensions();
        let timestamp_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        // Generate a test pattern frame
        let frame = generate_test_frame(width, height, self.frame_count);
        self.frame_count += 1;

        Ok(VideoFrame {
            data: frame,
            width,
            height,
            timestamp_us,
            is_keyframe: false,
        })
    }

    /// Update configuration
    pub fn set_config(&mut self, config: VideoConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &VideoConfig {
        &self.config
    }

    /// Switch between front and back camera
    pub fn switch_camera(&mut self) -> Result<(), VideoError> {
        let devices = Self::list_devices()?;

        let current_is_front = self
            .current_device
            .as_ref()
            .and_then(|id| devices.iter().find(|d| &d.id == id))
            .is_none_or(|d| d.is_front);

        // Find a camera of the opposite type
        let new_device = devices
            .iter()
            .find(|d| d.is_front != current_is_front)
            .or_else(|| devices.first());

        if let Some(device) = new_device {
            self.current_device = Some(device.id.clone());
            log::info!("Switched to camera: {}", device.name);
            Ok(())
        } else {
            Err(VideoError::NoCameraAvailable)
        }
    }
}

/// Generate a test pattern frame
fn generate_test_frame(width: u32, height: u32, frame_num: u64) -> Vec<u8> {
    let size = (width * height * 4) as usize;
    let mut data = vec![0u8; size];

    // Create a simple moving gradient pattern
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;

            // Moving gradient based on frame number
            let offset = (frame_num * 2) as u32;
            let r = ((x + offset) % 256) as u8;
            let g = ((y + offset) % 256) as u8;
            let b = (((x + y + offset) / 2) % 256) as u8;

            data[idx] = r; // R
            data[idx + 1] = g; // G
            data[idx + 2] = b; // B
            data[idx + 3] = 255; // A (fully opaque)
        }
    }

    data
}

/// Screen capture source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSource {
    /// Unique source identifier
    pub id: String,
    /// Source name (window title or display name)
    pub name: String,
    /// Whether this is a full screen (vs window)
    pub is_screen: bool,
    /// Thumbnail preview (optional)
    pub thumbnail: Option<Vec<u8>>,
    /// Source dimensions
    pub width: u32,
    pub height: u32,
}

/// Screen capture manager (desktop only)
pub struct ScreenCapture {
    /// Currently selected source
    current_source: Option<String>,
    /// Current capture configuration
    config: VideoConfig,
    /// Whether capture is active
    running: Arc<AtomicBool>,
    /// Frame counter
    frame_count: u64,
}

impl ScreenCapture {
    /// Create a new screen capture instance
    pub fn new(config: VideoConfig) -> Self {
        Self {
            current_source: None,
            config,
            running: Arc::new(AtomicBool::new(false)),
            frame_count: 0,
        }
    }

    /// List available screen capture sources
    pub fn list_sources() -> Result<Vec<ScreenSource>, VideoError> {
        // In production, this would use platform-specific APIs:
        // - Linux: X11/Wayland (pipewire), xdg-desktop-portal
        // - macOS: ScreenCaptureKit, CGWindowList
        // - Windows: DXGI Desktop Duplication, GDI

        // Return simulated sources for testing
        Ok(vec![
            ScreenSource {
                id: "screen-0".to_string(),
                name: "Primary Display".to_string(),
                is_screen: true,
                thumbnail: None,
                width: 1920,
                height: 1080,
            },
            ScreenSource {
                id: "screen-1".to_string(),
                name: "Secondary Display".to_string(),
                is_screen: true,
                thumbnail: None,
                width: 1920,
                height: 1080,
            },
            ScreenSource {
                id: "window-1".to_string(),
                name: "WRAITH-Chat".to_string(),
                is_screen: false,
                thumbnail: None,
                width: 1200,
                height: 800,
            },
        ])
    }

    /// Select a screen capture source
    pub fn select_source(&mut self, source_id: &str) -> Result<(), VideoError> {
        let sources = Self::list_sources()?;
        if !sources.iter().any(|s| s.id == source_id) {
            return Err(VideoError::DeviceNotFound(source_id.to_string()));
        }

        self.current_source = Some(source_id.to_string());
        log::info!("Selected screen capture source: {}", source_id);
        Ok(())
    }

    /// Get the currently selected source
    pub fn current_source(&self) -> Option<&str> {
        self.current_source.as_deref()
    }

    /// Request screen capture permission
    pub fn request_permission() -> Result<bool, VideoError> {
        // In production:
        // - macOS: ScreenCaptureKit.requestPermission()
        // - Linux: xdg-desktop-portal
        // - Windows: Usually granted

        // Simulate permission granted for now
        log::info!("Screen capture permission requested");
        Ok(true)
    }

    /// Start capturing
    pub fn start(&mut self) -> Result<(), VideoError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        if self.current_source.is_none() {
            let sources = Self::list_sources()?;
            if let Some(screen) = sources.iter().find(|s| s.is_screen) {
                self.current_source = Some(screen.id.clone());
            } else if let Some(first) = sources.first() {
                self.current_source = Some(first.id.clone());
            } else {
                return Err(VideoError::ScreenCaptureError(
                    "No capture sources available".to_string(),
                ));
            }
        }

        self.running.store(true, Ordering::SeqCst);
        log::info!(
            "Screen capture started on source: {:?}",
            self.current_source
        );
        Ok(())
    }

    /// Stop capturing
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        log::info!("Screen capture stopped");
    }

    /// Check if capture is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Capture a frame
    pub fn capture_frame(&mut self) -> Result<VideoFrame, VideoError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(VideoError::CaptureError(
                "Screen capture not running".to_string(),
            ));
        }

        // Get source dimensions (or use config resolution)
        let sources = Self::list_sources()?;
        let source = self
            .current_source
            .as_ref()
            .and_then(|id| sources.iter().find(|s| &s.id == id));

        let (width, height) = if let Some(src) = source {
            // Scale to config resolution if needed
            let (target_w, target_h) = self.config.resolution.dimensions();
            if src.width > target_w || src.height > target_h {
                (target_w, target_h)
            } else {
                (src.width, src.height)
            }
        } else {
            self.config.resolution.dimensions()
        };

        let timestamp_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        // Generate test frame (in production: actual screen capture)
        let frame = generate_test_frame(width, height, self.frame_count);
        self.frame_count += 1;

        Ok(VideoFrame {
            data: frame,
            width,
            height,
            timestamp_us,
            is_keyframe: false,
        })
    }

    /// Update configuration
    pub fn set_config(&mut self, config: VideoConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &VideoConfig {
        &self.config
    }
}

/// Network quality level for adaptive bitrate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkQuality {
    Excellent,
    Good,
    Fair,
    Poor,
    Critical,
}

impl NetworkQuality {
    /// Estimate quality from packet loss and RTT
    pub fn estimate(packet_loss_percent: f32, rtt_ms: f32) -> Self {
        if packet_loss_percent < 1.0 && rtt_ms < 100.0 {
            NetworkQuality::Excellent
        } else if packet_loss_percent < 3.0 && rtt_ms < 200.0 {
            NetworkQuality::Good
        } else if packet_loss_percent < 5.0 && rtt_ms < 300.0 {
            NetworkQuality::Fair
        } else if packet_loss_percent < 10.0 && rtt_ms < 500.0 {
            NetworkQuality::Poor
        } else {
            NetworkQuality::Critical
        }
    }

    /// Get recommended resolution for this quality level
    pub fn recommended_resolution(&self) -> VideoResolution {
        match self {
            NetworkQuality::Excellent => VideoResolution::Hd,
            NetworkQuality::Good => VideoResolution::Medium,
            NetworkQuality::Fair => VideoResolution::Low,
            NetworkQuality::Poor => VideoResolution::Low,
            NetworkQuality::Critical => VideoResolution::UltraLow,
        }
    }
}

/// Adaptive bitrate controller
pub struct AdaptiveBitrateController {
    /// Current target bitrate
    current_bitrate: u32,
    /// Minimum allowed bitrate
    min_bitrate: u32,
    /// Maximum allowed bitrate
    max_bitrate: u32,
    /// Target resolution
    target_resolution: VideoResolution,
    /// Recent bandwidth measurements (bps)
    bandwidth_history: VecDeque<u32>,
    /// Recent packet loss measurements (%)
    loss_history: VecDeque<f32>,
    /// Recent RTT measurements (ms)
    rtt_history: VecDeque<f32>,
    /// Whether adaptation is enabled
    enabled: bool,
    /// Frames since last quality change
    frames_since_change: u32,
    /// Minimum frames between quality changes (hysteresis)
    min_frames_between_changes: u32,
}

impl AdaptiveBitrateController {
    /// Create a new adaptive bitrate controller
    pub fn new(initial_resolution: VideoResolution) -> Self {
        let bitrate = initial_resolution.target_bitrate();

        Self {
            current_bitrate: bitrate,
            min_bitrate: VideoResolution::UltraLow.min_bitrate(),
            max_bitrate: VideoResolution::Hd.max_bitrate(),
            target_resolution: initial_resolution,
            bandwidth_history: VecDeque::with_capacity(30),
            loss_history: VecDeque::with_capacity(30),
            rtt_history: VecDeque::with_capacity(30),
            enabled: true,
            frames_since_change: 0,
            min_frames_between_changes: 60, // ~2 seconds at 30fps
        }
    }

    /// Enable or disable adaptation
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Update with new network statistics
    pub fn update(
        &mut self,
        bandwidth_bps: u32,
        packet_loss: f32,
        rtt_ms: f32,
    ) -> AdaptationResult {
        self.frames_since_change += 1;

        // Add to history
        if self.bandwidth_history.len() >= 30 {
            self.bandwidth_history.pop_front();
        }
        self.bandwidth_history.push_back(bandwidth_bps);

        if self.loss_history.len() >= 30 {
            self.loss_history.pop_front();
        }
        self.loss_history.push_back(packet_loss);

        if self.rtt_history.len() >= 30 {
            self.rtt_history.pop_front();
        }
        self.rtt_history.push_back(rtt_ms);

        if !self.enabled || self.frames_since_change < self.min_frames_between_changes {
            return AdaptationResult::NoChange;
        }

        // Calculate averages
        let avg_bandwidth =
            self.bandwidth_history.iter().sum::<u32>() / self.bandwidth_history.len().max(1) as u32;
        let avg_loss =
            self.loss_history.iter().sum::<f32>() / self.loss_history.len().max(1) as f32;
        let avg_rtt = self.rtt_history.iter().sum::<f32>() / self.rtt_history.len().max(1) as f32;

        let quality = NetworkQuality::estimate(avg_loss, avg_rtt);
        let recommended = quality.recommended_resolution();

        // Decide on adaptation
        if recommended != self.target_resolution {
            let new_bitrate = recommended.target_bitrate().min(avg_bandwidth * 90 / 100);
            let old_bitrate = self.current_bitrate;

            self.current_bitrate = new_bitrate.clamp(self.min_bitrate, self.max_bitrate);
            self.target_resolution = recommended;
            self.frames_since_change = 0;

            if new_bitrate > old_bitrate {
                AdaptationResult::BitrateIncreased {
                    old_bitrate,
                    new_bitrate: self.current_bitrate,
                    resolution: recommended,
                }
            } else {
                AdaptationResult::BitrateDecreased {
                    old_bitrate,
                    new_bitrate: self.current_bitrate,
                    resolution: recommended,
                }
            }
        } else if avg_bandwidth > self.current_bitrate * 120 / 100 {
            // Bandwidth increased significantly, try to increase bitrate
            let new_bitrate = (self.current_bitrate * 110 / 100).min(self.max_bitrate);
            if new_bitrate > self.current_bitrate {
                let old = self.current_bitrate;
                self.current_bitrate = new_bitrate;
                self.frames_since_change = 0;
                AdaptationResult::BitrateIncreased {
                    old_bitrate: old,
                    new_bitrate,
                    resolution: self.target_resolution,
                }
            } else {
                AdaptationResult::NoChange
            }
        } else if avg_bandwidth < self.current_bitrate * 80 / 100 {
            // Bandwidth dropped, decrease bitrate
            let new_bitrate = (self.current_bitrate * 90 / 100).max(self.min_bitrate);
            if new_bitrate < self.current_bitrate {
                let old = self.current_bitrate;
                self.current_bitrate = new_bitrate;
                self.frames_since_change = 0;
                AdaptationResult::BitrateDecreased {
                    old_bitrate: old,
                    new_bitrate,
                    resolution: self.target_resolution,
                }
            } else {
                AdaptationResult::NoChange
            }
        } else {
            AdaptationResult::NoChange
        }
    }

    /// Get current target bitrate
    pub fn current_bitrate(&self) -> u32 {
        self.current_bitrate
    }

    /// Get current target resolution
    pub fn target_resolution(&self) -> VideoResolution {
        self.target_resolution
    }

    /// Set maximum allowed bitrate
    pub fn set_max_bitrate(&mut self, max: u32) {
        self.max_bitrate = max;
        self.current_bitrate = self.current_bitrate.min(max);
    }

    /// Set minimum allowed bitrate
    pub fn set_min_bitrate(&mut self, min: u32) {
        self.min_bitrate = min;
        self.current_bitrate = self.current_bitrate.max(min);
    }

    /// Force a specific bitrate (disables adaptation temporarily)
    pub fn force_bitrate(&mut self, bitrate: u32, resolution: VideoResolution) {
        self.current_bitrate = bitrate.clamp(self.min_bitrate, self.max_bitrate);
        self.target_resolution = resolution;
        self.frames_since_change = 0;
    }

    /// Get network quality estimate
    pub fn network_quality(&self) -> NetworkQuality {
        let avg_loss =
            self.loss_history.iter().sum::<f32>() / self.loss_history.len().max(1) as f32;
        let avg_rtt = self.rtt_history.iter().sum::<f32>() / self.rtt_history.len().max(1) as f32;
        NetworkQuality::estimate(avg_loss, avg_rtt)
    }

    /// Set minimum frames between quality changes (for testing)
    #[cfg(test)]
    pub fn set_hysteresis(&mut self, frames: u32) {
        self.min_frames_between_changes = frames;
    }
}

/// Result of bitrate adaptation
#[derive(Debug, Clone)]
pub enum AdaptationResult {
    NoChange,
    BitrateIncreased {
        old_bitrate: u32,
        new_bitrate: u32,
        resolution: VideoResolution,
    },
    BitrateDecreased {
        old_bitrate: u32,
        new_bitrate: u32,
        resolution: VideoResolution,
    },
}

/// Video frame buffer (jitter buffer for video)
pub struct VideoFrameBuffer {
    /// Buffered frames ordered by timestamp
    frames: VecDeque<EncodedVideoFrame>,
    /// Maximum buffer size
    capacity: usize,
    /// Target buffer depth in frames
    target_depth: usize,
    /// Last played frame timestamp
    last_timestamp: u64,
    /// Whether we have a keyframe
    has_keyframe: bool,
}

impl VideoFrameBuffer {
    /// Create a new video frame buffer
    pub fn new(target_depth: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(target_depth * 3),
            capacity: target_depth * 3,
            target_depth,
            last_timestamp: 0,
            has_keyframe: false,
        }
    }

    /// Push a frame into the buffer
    pub fn push(&mut self, frame: EncodedVideoFrame) -> Result<(), VideoError> {
        if self.frames.len() >= self.capacity {
            // Buffer full, drop oldest non-keyframe
            let mut dropped = false;
            for i in 0..self.frames.len() {
                if !self.frames[i].is_keyframe {
                    self.frames.remove(i);
                    dropped = true;
                    break;
                }
            }
            if !dropped {
                self.frames.pop_front();
            }
        }

        if frame.is_keyframe {
            self.has_keyframe = true;
        }

        // Insert in timestamp order
        let insert_pos = self
            .frames
            .iter()
            .position(|f| f.timestamp_us > frame.timestamp_us)
            .unwrap_or(self.frames.len());

        self.frames.insert(insert_pos, frame);

        Ok(())
    }

    /// Pop the next frame to display
    pub fn pop(&mut self) -> Option<EncodedVideoFrame> {
        if !self.has_keyframe {
            return None;
        }

        // Need at least target_depth frames buffered before starting playback
        if self.frames.len() < self.target_depth && self.last_timestamp == 0 {
            return None;
        }

        let frame = self.frames.pop_front();
        if let Some(ref f) = frame {
            self.last_timestamp = f.timestamp_us;
        }
        frame
    }

    /// Get current buffer depth
    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Check if buffer is ready for playback
    pub fn ready(&self) -> bool {
        self.has_keyframe && self.frames.len() >= self.target_depth
    }

    /// Reset the buffer
    pub fn reset(&mut self) {
        self.frames.clear();
        self.last_timestamp = 0;
        self.has_keyframe = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_resolution() {
        let res = VideoResolution::Hd;
        assert_eq!(res.dimensions(), (1280, 720));
        assert_eq!(res.target_bitrate(), 1_500_000);
        assert_eq!(res.recommended_fps(), 30);

        assert_eq!(res.lower(), Some(VideoResolution::Medium));
        assert_eq!(res.higher(), Some(VideoResolution::FullHd));

        assert_eq!(VideoResolution::UltraLow.lower(), None);
        assert_eq!(VideoResolution::FullHd.higher(), None);
    }

    #[test]
    fn test_video_config_default() {
        let config = VideoConfig::default();
        assert_eq!(config.codec, VideoCodec::Vp9);
        assert_eq!(config.resolution, VideoResolution::Hd);
        assert_eq!(config.framerate, 30);
        assert!(config.adaptive_bitrate);
    }

    #[test]
    fn test_video_encoder_creation() {
        let config = VideoConfig::default();
        let encoder = VideoEncoder::new(config);
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_video_encode_decode() {
        let config = VideoConfig {
            resolution: VideoResolution::Low,
            ..Default::default()
        };

        let mut encoder = VideoEncoder::new(config.clone()).unwrap();
        let mut decoder = VideoDecoder::new(config).unwrap();

        // Create test frame
        let frame = VideoFrame::blank(640, 360);

        // Encode
        let encoded = encoder.encode(&frame).unwrap();
        assert!(encoded.is_keyframe); // First frame should be keyframe
        assert!(!encoded.data.is_empty());

        // Decode
        let decoded = decoder.decode(&encoded).unwrap();
        assert_eq!(decoded.width, 640);
        assert_eq!(decoded.height, 360);
    }

    #[test]
    fn test_camera_list_devices() {
        let devices = CameraCapture::list_devices().unwrap();
        assert!(!devices.is_empty());
        assert!(devices.iter().any(|d| d.is_default));
    }

    #[test]
    fn test_camera_capture() {
        let config = VideoConfig::default();
        let mut capture = CameraCapture::new(config);

        // Start capture
        capture.start().unwrap();
        assert!(capture.is_running());

        // Capture a frame
        let frame = capture.capture_frame().unwrap();
        assert!(!frame.data.is_empty());

        // Stop capture
        capture.stop();
        assert!(!capture.is_running());
    }

    #[test]
    fn test_screen_capture() {
        let config = VideoConfig::default();
        let mut capture = ScreenCapture::new(config);

        let sources = ScreenCapture::list_sources().unwrap();
        assert!(!sources.is_empty());

        capture.start().unwrap();
        assert!(capture.is_running());

        let frame = capture.capture_frame().unwrap();
        assert!(!frame.data.is_empty());

        capture.stop();
        assert!(!capture.is_running());
    }

    #[test]
    fn test_adaptive_bitrate_controller() {
        let mut controller = AdaptiveBitrateController::new(VideoResolution::Hd);

        assert_eq!(controller.current_bitrate(), 1_500_000);
        assert_eq!(controller.target_resolution(), VideoResolution::Hd);

        // Simulate good network
        for _ in 0..100 {
            controller.update(2_000_000, 0.5, 50.0);
        }
        assert_eq!(controller.network_quality(), NetworkQuality::Excellent);

        // Simulate poor network
        let mut abc = AdaptiveBitrateController::new(VideoResolution::Hd);
        abc.min_frames_between_changes = 0; // Disable hysteresis for test

        for _ in 0..100 {
            let result = abc.update(300_000, 8.0, 400.0);
            if matches!(result, AdaptationResult::BitrateDecreased { .. }) {
                break;
            }
        }

        // Should have decreased bitrate due to poor network
        assert!(abc.current_bitrate() < 1_500_000);
    }

    #[test]
    fn test_video_frame_buffer() {
        let mut buffer = VideoFrameBuffer::new(3);
        assert!(!buffer.ready());

        // Add keyframe
        buffer
            .push(EncodedVideoFrame {
                data: vec![b'W', b'V', b'I', b'D', 0x01, 0, 5, 0],
                width: 1280,
                height: 720,
                timestamp_us: 0,
                is_keyframe: true,
                codec: VideoCodec::Vp9,
            })
            .unwrap();

        // Add more frames
        for i in 1..5 {
            buffer
                .push(EncodedVideoFrame {
                    data: vec![b'W', b'V', b'I', b'D', 0x00, 0, 5, 0],
                    width: 1280,
                    height: 720,
                    timestamp_us: i * 33333, // ~30fps
                    is_keyframe: false,
                    codec: VideoCodec::Vp9,
                })
                .unwrap();
        }

        assert!(buffer.ready());
        assert!(buffer.depth() >= 3);

        // Pop frames
        let frame = buffer.pop().unwrap();
        assert!(frame.is_keyframe);
    }

    #[test]
    fn test_network_quality_estimation() {
        assert_eq!(
            NetworkQuality::estimate(0.5, 50.0),
            NetworkQuality::Excellent
        );
        assert_eq!(NetworkQuality::estimate(2.0, 150.0), NetworkQuality::Good);
        assert_eq!(NetworkQuality::estimate(4.0, 250.0), NetworkQuality::Fair);
        assert_eq!(NetworkQuality::estimate(8.0, 400.0), NetworkQuality::Poor);
        assert_eq!(
            NetworkQuality::estimate(15.0, 600.0),
            NetworkQuality::Critical
        );
    }
}
