// Audio Processing Module for WRAITH-Chat Voice Calling
//
// Provides Opus codec encoding/decoding, echo cancellation, and noise suppression.

use audiopus::{
    Bitrate, Channels, MutSignals, SampleRate,
    coder::{Decoder as OpusDecoder, Encoder as OpusEncoder},
    packet::Packet,
};
#[cfg(target_os = "linux")]
use nnnoiseless::DenoiseState;
use std::collections::VecDeque;
use thiserror::Error;

/// Suppress stderr output during a function call on Linux.
///
/// ALSA's libasound library probes various plugins during device enumeration,
/// including JACK and OSS plugins. When these backends aren't available, they
/// write error messages directly to stderr (bypassing Rust's logging). This
/// helper temporarily redirects stderr to /dev/null to suppress these messages.
///
/// On non-Linux platforms, this simply executes the closure without redirection.
#[cfg(target_os = "linux")]
fn with_suppressed_stderr<T, F: FnOnce() -> T>(f: F) -> T {
    use std::fs::File;
    use std::os::unix::io::AsRawFd;

    // Save the current stderr file descriptor
    let stderr_fd = std::io::stderr().as_raw_fd();
    let saved_stderr = unsafe { libc::dup(stderr_fd) };

    if saved_stderr != -1 {
        // Open /dev/null and redirect stderr to it
        if let Ok(devnull) = File::open("/dev/null") {
            let devnull_fd = devnull.as_raw_fd();
            unsafe {
                libc::dup2(devnull_fd, stderr_fd);
            }
        }

        // Execute the function
        let result = f();

        // Restore stderr
        unsafe {
            libc::dup2(saved_stderr, stderr_fd);
            libc::close(saved_stderr);
        }

        result
    } else {
        // Failed to dup stderr, just run without suppression
        f()
    }
}

#[cfg(not(target_os = "linux"))]
fn with_suppressed_stderr<T, F: FnOnce() -> T>(f: F) -> T {
    f()
}

/// Audio processing errors
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Opus encoder error: {0}")]
    EncoderError(String),

    #[error("Opus decoder error: {0}")]
    DecoderError(String),

    #[error("Audio device error: {0}")]
    DeviceError(String),

    #[error("Invalid sample rate: {0}")]
    InvalidSampleRate(u32),

    #[error("Buffer overflow")]
    BufferOverflow,

    #[error("Buffer underflow")]
    BufferUnderflow,
}

/// Supported sample rates for voice calling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceSampleRate {
    /// 8kHz - narrowband (minimum quality)
    Narrowband,
    /// 16kHz - wideband (good quality)
    Wideband,
    /// 48kHz - fullband (best quality, recommended)
    Fullband,
}

impl VoiceSampleRate {
    pub fn as_hz(&self) -> u32 {
        match self {
            VoiceSampleRate::Narrowband => 8000,
            VoiceSampleRate::Wideband => 16000,
            VoiceSampleRate::Fullband => 48000,
        }
    }

    fn to_opus_rate(self) -> SampleRate {
        match self {
            VoiceSampleRate::Narrowband => SampleRate::Hz8000,
            VoiceSampleRate::Wideband => SampleRate::Hz16000,
            VoiceSampleRate::Fullband => SampleRate::Hz48000,
        }
    }
}

/// Configuration for the audio codec
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Sample rate for audio processing
    pub sample_rate: VoiceSampleRate,
    /// Bitrate in bps (default: 64000 for good quality voice)
    pub bitrate: u32,
    /// Frame size in samples (20ms at 48kHz = 960 samples)
    pub frame_size: usize,
    /// Enable noise suppression
    pub enable_noise_suppression: bool,
    /// Enable voice activity detection
    pub enable_vad: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: VoiceSampleRate::Fullband,
            bitrate: 64000,  // 64 kbps - good balance of quality and bandwidth
            frame_size: 960, // 20ms at 48kHz
            enable_noise_suppression: true,
            enable_vad: true,
        }
    }
}

/// Opus encoder wrapper for voice encoding
pub struct VoiceEncoder {
    encoder: OpusEncoder,
    config: AudioConfig,
    #[cfg(target_os = "linux")]
    denoise: Option<Box<DenoiseState<'static>>>,
    /// Buffer for resampling (for noise suppression at 48kHz)
    #[cfg(target_os = "linux")]
    resample_buffer: Vec<f32>,
}

// Mark VoiceEncoder as Send + Sync (encoder is internally thread-safe)
unsafe impl Send for VoiceEncoder {}
unsafe impl Sync for VoiceEncoder {}

impl VoiceEncoder {
    /// Create a new voice encoder with the given configuration
    pub fn new(config: AudioConfig) -> Result<Self, AudioError> {
        let mut encoder = OpusEncoder::new(
            config.sample_rate.to_opus_rate(),
            Channels::Mono,
            audiopus::Application::Voip,
        )
        .map_err(|e| AudioError::EncoderError(format!("{:?}", e)))?;

        // Set bitrate
        encoder
            .set_bitrate(Bitrate::BitsPerSecond(config.bitrate as i32))
            .map_err(|e| AudioError::EncoderError(format!("Failed to set bitrate: {:?}", e)))?;

        // Enable FEC for packet loss resilience
        encoder
            .set_inband_fec(true)
            .map_err(|e| AudioError::EncoderError(format!("Failed to enable FEC: {:?}", e)))?;

        // Set DTX (discontinuous transmission) for bandwidth efficiency
        encoder
            .set_dtx(true)
            .map_err(|e| AudioError::EncoderError(format!("Failed to enable DTX: {:?}", e)))?;

        // Initialize noise suppression if enabled (Linux only - uses nnnoiseless/RNNoise)
        #[cfg(target_os = "linux")]
        let denoise = if config.enable_noise_suppression {
            Some(DenoiseState::new())
        } else {
            None
        };

        // RNNoise requires 480 samples at 48kHz (10ms frames)
        #[cfg(target_os = "linux")]
        let resample_buffer = vec![0.0f32; 480];

        Ok(Self {
            encoder,
            config,
            #[cfg(target_os = "linux")]
            denoise,
            #[cfg(target_os = "linux")]
            resample_buffer,
        })
    }

    /// Encode PCM audio samples to Opus
    ///
    /// # Arguments
    /// * `pcm` - Input PCM samples (i16, mono)
    /// * `output` - Output buffer for encoded Opus data (should be at least 4000 bytes)
    ///
    /// # Returns
    /// Number of bytes written to output, or 0 if VAD detected silence
    pub fn encode(&mut self, pcm: &[i16], output: &mut [u8]) -> Result<usize, AudioError> {
        // Apply noise suppression if enabled (Linux only - uses nnnoiseless/RNNoise)
        #[cfg(target_os = "linux")]
        let processed_pcm: Vec<i16> = if self.denoise.is_some() {
            // We need to take denoise out temporarily to avoid borrow issues
            let mut denoise = self.denoise.take().unwrap();
            let result = self.apply_noise_suppression(pcm, &mut denoise);
            self.denoise = Some(denoise);
            result
        } else {
            pcm.to_vec()
        };

        // On non-Linux platforms, noise suppression is not available
        #[cfg(not(target_os = "linux"))]
        let processed_pcm: Vec<i16> = pcm.to_vec();

        let pcm_to_encode = &processed_pcm;

        // Simple VAD: check if audio is mostly silent
        if self.config.enable_vad {
            let energy: i64 = pcm_to_encode.iter().map(|&s| (s as i64).abs()).sum();
            let avg_energy = energy / pcm_to_encode.len().max(1) as i64;
            if avg_energy < 100 {
                // Very quiet, skip encoding
                return Ok(0);
            }
        }

        // Encode with Opus - the encoder takes &[i16] input and &mut [u8] output
        let len = self
            .encoder
            .encode(pcm_to_encode, output)
            .map_err(|e| AudioError::EncoderError(format!("{:?}", e)))?;

        Ok(len)
    }

    /// Apply RNNoise noise suppression to PCM samples (Linux only)
    #[cfg(target_os = "linux")]
    fn apply_noise_suppression(&mut self, pcm: &[i16], denoise: &mut DenoiseState) -> Vec<i16> {
        let mut output = Vec::with_capacity(pcm.len());

        // RNNoise requires 480-sample chunks at 48kHz (10ms)
        // Process in chunks, handling the last partial chunk
        for chunk in pcm.chunks(480) {
            // Convert i16 to f32 (RNNoise expects [-1.0, 1.0] range scaled to [-32768, 32767])
            for (i, &sample) in chunk.iter().enumerate() {
                if i < self.resample_buffer.len() {
                    self.resample_buffer[i] = sample as f32;
                }
            }

            // Pad remaining samples with zeros if chunk is smaller than 480
            for i in chunk.len()..self.resample_buffer.len() {
                self.resample_buffer[i] = 0.0;
            }

            // Apply noise suppression
            let mut denoised = [0.0f32; 480];
            denoise.process_frame(&mut denoised, &self.resample_buffer);

            // Convert back to i16 and add to output
            for &sample in denoised.iter().take(chunk.len()) {
                let clamped = sample.clamp(-32768.0, 32767.0);
                output.push(clamped as i16);
            }
        }

        output
    }

    /// Get the recommended frame size for this encoder
    pub fn frame_size(&self) -> usize {
        self.config.frame_size
    }

    /// Get the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.as_hz()
    }
}

/// Opus decoder wrapper for voice decoding
pub struct VoiceDecoder {
    decoder: OpusDecoder,
    config: AudioConfig,
    /// Last good packet for PLC (Packet Loss Concealment)
    last_packet: Option<Vec<u8>>,
}

// Mark VoiceDecoder as Send + Sync
unsafe impl Send for VoiceDecoder {}
unsafe impl Sync for VoiceDecoder {}

impl VoiceDecoder {
    /// Create a new voice decoder with the given configuration
    pub fn new(config: AudioConfig) -> Result<Self, AudioError> {
        let decoder = OpusDecoder::new(config.sample_rate.to_opus_rate(), Channels::Mono)
            .map_err(|e| AudioError::DecoderError(format!("{:?}", e)))?;

        Ok(Self {
            decoder,
            config,
            last_packet: None,
        })
    }

    /// Decode Opus data to PCM samples
    ///
    /// # Arguments
    /// * `opus_data` - Encoded Opus data
    /// * `output` - Output buffer for PCM samples
    /// * `use_fec` - Whether to use FEC for lost packet recovery
    ///
    /// # Returns
    /// Number of samples written to output
    pub fn decode(
        &mut self,
        opus_data: &[u8],
        output: &mut [i16],
        use_fec: bool,
    ) -> Result<usize, AudioError> {
        // Create MutSignals wrapper for output buffer
        let output_signals = MutSignals::try_from(output)
            .map_err(|e| AudioError::DecoderError(format!("Invalid output buffer: {:?}", e)))?;

        let samples = if use_fec && opus_data.is_empty() {
            // Packet loss - use FEC from last packet if available
            if let Some(ref last) = self.last_packet {
                let packet = Packet::try_from(last.as_slice())
                    .map_err(|e| AudioError::DecoderError(format!("Invalid packet: {:?}", e)))?;
                self.decoder
                    .decode(Some(packet), output_signals, true)
                    .map_err(|e| AudioError::DecoderError(format!("FEC decode error: {:?}", e)))?
            } else {
                // No FEC available, use PLC (pass None for packet loss concealment)
                self.decoder
                    .decode(None, output_signals, false)
                    .map_err(|e| AudioError::DecoderError(format!("PLC error: {:?}", e)))?
            }
        } else {
            // Normal decode
            let packet = Packet::try_from(opus_data)
                .map_err(|e| AudioError::DecoderError(format!("Invalid packet: {:?}", e)))?;
            let samples = self
                .decoder
                .decode(Some(packet), output_signals, false)
                .map_err(|e| AudioError::DecoderError(format!("{:?}", e)))?;

            // Store packet for potential FEC use
            self.last_packet = Some(opus_data.to_vec());

            samples
        };

        Ok(samples)
    }

    /// Get the recommended output buffer size
    pub fn output_buffer_size(&self) -> usize {
        self.config.frame_size * 2 // Double for safety
    }

    /// Get the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.as_hz()
    }
}

/// Jitter buffer for handling network latency variations
pub struct JitterBuffer {
    /// Queue for audio frames
    buffer: VecDeque<AudioFrame>,
    /// Target latency in milliseconds
    target_latency_ms: u32,
    /// Sample rate for timing calculations
    sample_rate: u32,
    /// Maximum buffer capacity
    capacity: usize,
    /// Statistics for adaptive buffering
    stats: JitterStats,
}

/// Audio frame with timestamp
#[derive(Clone)]
pub struct AudioFrame {
    /// PCM samples
    pub samples: Vec<i16>,
    /// Sequence number for ordering
    pub sequence: u32,
    /// Timestamp in samples
    pub timestamp: u64,
    /// Whether this frame was synthesized (PLC/FEC)
    pub synthesized: bool,
}

/// Jitter buffer statistics
#[derive(Default, Clone)]
pub struct JitterStats {
    /// Number of frames received
    pub frames_received: u64,
    /// Number of frames dropped (too late)
    pub frames_dropped: u64,
    /// Number of frames synthesized (missing)
    pub frames_synthesized: u64,
    /// Current buffer depth in frames
    pub buffer_depth: usize,
    /// Average jitter in milliseconds
    pub avg_jitter_ms: f32,
}

impl JitterBuffer {
    /// Create a new jitter buffer
    ///
    /// # Arguments
    /// * `target_latency_ms` - Target latency in milliseconds (typically 60-200ms)
    /// * `sample_rate` - Audio sample rate
    /// * `frame_size` - Number of samples per frame
    pub fn new(target_latency_ms: u32, sample_rate: u32, frame_size: usize) -> Self {
        // Calculate buffer capacity based on target latency
        let frames_per_second = sample_rate as usize / frame_size;
        let capacity = (frames_per_second * target_latency_ms as usize / 1000) * 3; // 3x headroom

        Self {
            buffer: VecDeque::with_capacity(capacity.max(32)),
            target_latency_ms,
            sample_rate,
            capacity: capacity.max(32),
            stats: JitterStats::default(),
        }
    }

    /// Push a frame into the buffer
    pub fn push(&mut self, frame: AudioFrame) -> Result<(), AudioError> {
        self.stats.frames_received += 1;

        if self.buffer.len() >= self.capacity {
            // Drop oldest frame
            self.buffer.pop_front();
            self.stats.frames_dropped += 1;
        }

        self.buffer.push_back(frame);
        self.stats.buffer_depth = self.buffer.len();
        Ok(())
    }

    /// Pop a frame from the buffer
    ///
    /// Returns None if buffer is empty
    pub fn pop(&mut self) -> Option<AudioFrame> {
        let frame = self.buffer.pop_front();
        self.stats.buffer_depth = self.buffer.len();
        frame
    }

    /// Get current statistics
    pub fn stats(&self) -> JitterStats {
        self.stats.clone()
    }

    /// Get current buffer depth in frames
    pub fn depth(&self) -> usize {
        self.buffer.len()
    }

    /// Check if buffer has enough frames to start playback
    pub fn ready(&self) -> bool {
        let frame_size = (self.sample_rate / 50) as usize; // 20ms
        let target_frames =
            (self.sample_rate / 1000 * self.target_latency_ms / 2) as usize / frame_size;
        self.buffer.len() >= target_frames.max(1)
    }
}

/// Audio device manager for input/output device handling
pub struct AudioDeviceManager {
    /// Currently selected input device ID
    input_device: Option<String>,
    /// Currently selected output device ID
    output_device: Option<String>,
}

impl AudioDeviceManager {
    /// Create a new audio device manager
    pub fn new() -> Self {
        Self {
            input_device: None,
            output_device: None,
        }
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
                if host_id == HostId::Alsa
                    && let Ok(host) = cpal::host_from_id(host_id)
                {
                    return host;
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

    /// List available input devices
    ///
    /// This function enumerates audio input devices while suppressing spurious
    /// error messages from unavailable audio backends (like JACK when not running).
    /// Stderr is temporarily redirected to /dev/null during device enumeration
    /// to prevent ALSA plugin errors from cluttering the console.
    pub fn list_input_devices(&self) -> Result<Vec<AudioDevice>, AudioError> {
        // Wrap the entire enumeration in stderr suppression to catch ALSA plugin errors
        with_suppressed_stderr(|| {
            let host = Self::get_preferred_host();

            let devices = match host.input_devices() {
                Ok(d) => d,
                Err(e) => {
                    log::warn!("Failed to enumerate input devices: {}", e);
                    return Ok(Vec::new());
                }
            };

            let mut result = Vec::new();
            for device in devices {
                if let Ok(description) = device.description() {
                    let name = description.name().to_string();
                    // Filter out virtual/null devices that aren't useful for voice
                    if !name.to_lowercase().contains("null")
                        && !name.to_lowercase().contains("dummy")
                    {
                        result.push(AudioDevice {
                            id: name.clone(),
                            name,
                            is_default: false, // We'll mark this later
                        });
                    }
                }
            }

            // Mark default device
            if let Some(default) = host.default_input_device()
                && let Ok(default_description) = default.description()
            {
                let default_name = default_description.name();
                for device in &mut result {
                    if device.name == default_name {
                        device.is_default = true;
                        break;
                    }
                }
            }

            Ok(result)
        })
    }

    /// List available output devices
    ///
    /// This function enumerates audio output devices while suppressing spurious
    /// error messages from unavailable audio backends (like JACK when not running).
    /// Stderr is temporarily redirected to /dev/null during device enumeration
    /// to prevent ALSA plugin errors from cluttering the console.
    pub fn list_output_devices(&self) -> Result<Vec<AudioDevice>, AudioError> {
        // Wrap the entire enumeration in stderr suppression to catch ALSA plugin errors
        with_suppressed_stderr(|| {
            let host = Self::get_preferred_host();

            let devices = match host.output_devices() {
                Ok(d) => d,
                Err(e) => {
                    log::warn!("Failed to enumerate output devices: {}", e);
                    return Ok(Vec::new());
                }
            };

            let mut result = Vec::new();
            for device in devices {
                if let Ok(description) = device.description() {
                    let name = description.name().to_string();
                    // Filter out virtual/null devices that aren't useful for voice
                    if !name.to_lowercase().contains("null")
                        && !name.to_lowercase().contains("dummy")
                    {
                        result.push(AudioDevice {
                            id: name.clone(),
                            name,
                            is_default: false,
                        });
                    }
                }
            }

            // Mark default device
            if let Some(default) = host.default_output_device()
                && let Ok(default_description) = default.description()
            {
                let default_name = default_description.name();
                for device in &mut result {
                    if device.name == default_name {
                        device.is_default = true;
                        break;
                    }
                }
            }

            Ok(result)
        })
    }

    /// Set the active input device
    pub fn set_input_device(&mut self, device_id: Option<String>) {
        self.input_device = device_id;
    }

    /// Set the active output device
    pub fn set_output_device(&mut self, device_id: Option<String>) {
        self.output_device = device_id;
    }

    /// Get the currently selected input device ID
    pub fn input_device(&self) -> Option<&str> {
        self.input_device.as_deref()
    }

    /// Get the currently selected output device ID
    pub fn output_device(&self) -> Option<&str> {
        self.output_device.as_deref()
    }

    /// Find an input device by its ID/name
    ///
    /// Returns the cpal device if found, or None if not found or if device_id is None
    /// (in which case the caller should use the default device).
    pub fn find_input_device(&self, device_id: Option<&str>) -> Option<cpal::Device> {
        let device_id = device_id?;

        with_suppressed_stderr(|| {
            let host = Self::get_preferred_host();
            let devices = host.input_devices().ok()?;

            for device in devices {
                if let Ok(description) = device.description()
                    && description.name() == device_id
                {
                    return Some(device);
                }
            }
            None
        })
    }

    /// Find an output device by its ID/name
    ///
    /// Returns the cpal device if found, or None if not found or if device_id is None
    /// (in which case the caller should use the default device).
    pub fn find_output_device(&self, device_id: Option<&str>) -> Option<cpal::Device> {
        let device_id = device_id?;

        with_suppressed_stderr(|| {
            let host = Self::get_preferred_host();
            let devices = host.output_devices().ok()?;

            for device in devices {
                if let Ok(description) = device.description()
                    && description.name() == device_id
                {
                    return Some(device);
                }
            }
            None
        })
    }

    /// Get the input device to use - either the selected one or the default
    pub fn get_active_input_device(&self) -> Option<cpal::Device> {
        with_suppressed_stderr(|| {
            // Try to get the selected device first
            if let Some(device) = self.find_input_device(self.input_device.as_deref()) {
                return Some(device);
            }

            // Fall back to default device
            let host = Self::get_preferred_host();
            host.default_input_device()
        })
    }

    /// Get the output device to use - either the selected one or the default
    pub fn get_active_output_device(&self) -> Option<cpal::Device> {
        with_suppressed_stderr(|| {
            // Try to get the selected device first
            if let Some(device) = self.find_output_device(self.output_device.as_deref()) {
                return Some(device);
            }

            // Fall back to default device
            let host = Self::get_preferred_host();
            host.default_output_device()
        })
    }
}

impl Default for AudioDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Audio device information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioDevice {
    /// Unique device identifier
    pub id: String,
    /// Human-readable device name
    pub name: String,
    /// Whether this is the system default device
    pub is_default: bool,
}

use cpal::traits::{DeviceTrait, HostTrait};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate.as_hz(), 48000);
        assert_eq!(config.bitrate, 64000);
        assert_eq!(config.frame_size, 960);
    }

    #[test]
    fn test_voice_encoder_creation() {
        let config = AudioConfig::default();
        let encoder = VoiceEncoder::new(config);
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_voice_decoder_creation() {
        let config = AudioConfig::default();
        let decoder = VoiceDecoder::new(config);
        assert!(decoder.is_ok());
    }

    #[test]
    fn test_jitter_buffer() {
        let mut buffer = JitterBuffer::new(100, 48000, 960);
        assert_eq!(buffer.depth(), 0);

        let frame = AudioFrame {
            samples: vec![0i16; 960],
            sequence: 0,
            timestamp: 0,
            synthesized: false,
        };

        buffer.push(frame.clone()).unwrap();
        assert_eq!(buffer.depth(), 1);

        let popped = buffer.pop();
        assert!(popped.is_some());
        assert_eq!(buffer.depth(), 0);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let config = AudioConfig {
            enable_noise_suppression: false,
            enable_vad: false,
            ..Default::default()
        };

        let mut encoder = VoiceEncoder::new(config.clone()).unwrap();
        let mut decoder = VoiceDecoder::new(config).unwrap();

        // Generate test audio (sine wave)
        let mut pcm = vec![0i16; 960];
        for (i, sample) in pcm.iter_mut().enumerate() {
            let t = i as f32 / 48000.0;
            *sample = (f32::sin(2.0 * std::f32::consts::PI * 440.0 * t) * 16000.0) as i16;
        }

        // Encode
        let mut opus_data = vec![0u8; 4000];
        let encoded_len = encoder.encode(&pcm, &mut opus_data).unwrap();
        assert!(encoded_len > 0);

        // Decode
        let mut decoded = vec![0i16; 1920];
        let decoded_samples = decoder
            .decode(&opus_data[..encoded_len], &mut decoded, false)
            .unwrap();
        assert!(decoded_samples > 0);
    }
}
