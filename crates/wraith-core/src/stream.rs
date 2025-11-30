//! Stream multiplexing within sessions.
//!
//! Streams are logical bidirectional byte channels within a session,
//! used for individual file transfers.

use crate::error::SessionError;
use std::collections::VecDeque;

/// Stream state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Stream created but not yet opened
    Idle,
    /// Stream is open and bidirectional
    Open,
    /// Stream half-closed (local end finished sending)
    HalfClosedLocal,
    /// Stream half-closed (remote end finished sending)
    HalfClosedRemote,
    /// All data sent, awaiting final ACK
    DataSent,
    /// Stream closed
    Closed,
}

/// A multiplexed stream within a session
pub struct Stream {
    /// Stream ID (odd = client-initiated, even = server-initiated)
    id: u16,
    /// Current stream state
    state: StreamState,
    /// Flow control window (bytes available to send)
    send_window: u64,
    /// Flow control window (bytes we can receive)
    recv_window: u64,
    /// Maximum window size
    max_window: u64,
    /// Buffered data to send
    send_buffer: VecDeque<Vec<u8>>,
    /// Buffered received data
    recv_buffer: VecDeque<Vec<u8>>,
    /// Total bytes sent
    bytes_sent: u64,
    /// Total bytes received
    bytes_received: u64,
    /// Whether FIN has been sent
    fin_sent: bool,
    /// Whether FIN has been received
    fin_received: bool,
}

impl Stream {
    /// Create a new stream with the given ID and initial window
    #[must_use]
    pub fn new(id: u16, initial_window: u64) -> Self {
        Self {
            id,
            state: StreamState::Idle,
            send_window: initial_window,
            recv_window: initial_window,
            max_window: initial_window * 16, // Allow growth up to 16x
            send_buffer: VecDeque::new(),
            recv_buffer: VecDeque::new(),
            bytes_sent: 0,
            bytes_received: 0,
            fin_sent: false,
            fin_received: false,
        }
    }

    /// Get the stream ID
    #[must_use]
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Get current stream state
    #[must_use]
    pub fn state(&self) -> StreamState {
        self.state
    }

    /// Get the current send window size
    #[must_use]
    pub fn send_window(&self) -> u64 {
        self.send_window
    }

    /// Get the current receive window size
    #[must_use]
    pub fn recv_window(&self) -> u64 {
        self.recv_window
    }

    /// Get bytes sent
    #[must_use]
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    /// Get bytes received
    #[must_use]
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }

    /// Check if this is a client-initiated stream (odd ID)
    #[must_use]
    pub fn is_client_initiated(&self) -> bool {
        self.id % 2 == 1
    }

    /// Check if stream can transition to new state
    #[must_use]
    pub fn can_transition(&self, to: StreamState) -> bool {
        match (self.state, to) {
            // From Idle
            (StreamState::Idle, StreamState::Open | StreamState::Closed) => true,

            // From Open
            (
                StreamState::Open,
                StreamState::HalfClosedLocal
                | StreamState::HalfClosedRemote
                | StreamState::DataSent
                | StreamState::Closed,
            ) => true,

            // From HalfClosedLocal
            (StreamState::HalfClosedLocal, StreamState::Closed) => true,

            // From HalfClosedRemote
            (StreamState::HalfClosedRemote, StreamState::Closed) => true,

            // From DataSent
            (StreamState::DataSent, StreamState::Closed) => true,

            // All other transitions invalid
            _ => false,
        }
    }

    /// Transition to new state
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InvalidState` if the transition is not allowed
    /// from the current state.
    pub fn transition_to(&mut self, new_state: StreamState) -> Result<(), SessionError> {
        if !self.can_transition(new_state) {
            return Err(SessionError::InvalidState);
        }

        self.state = new_state;

        // Cleanup on close
        if new_state == StreamState::Closed {
            self.send_buffer.clear();
            self.recv_buffer.clear();
        }

        Ok(())
    }

    /// Open the stream
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InvalidState` if the stream is not in Idle state.
    pub fn open(&mut self) -> Result<(), SessionError> {
        self.transition_to(StreamState::Open)
    }

    /// Close the stream
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InvalidState` if the stream cannot be closed from
    /// its current state.
    pub fn close(&mut self) -> Result<(), SessionError> {
        self.transition_to(StreamState::Closed)
    }

    /// Reset the stream (abrupt termination)
    ///
    /// # Errors
    ///
    /// This function is infallible but returns `Result` for API consistency.
    pub fn reset(&mut self) -> Result<(), SessionError> {
        self.state = StreamState::Closed;
        self.send_buffer.clear();
        self.recv_buffer.clear();
        Ok(())
    }

    /// Write data to send buffer
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InvalidState` if the stream is not open for writing
    /// (must be in `Open` or `HalfClosedRemote` state).
    pub fn write(&mut self, data: Vec<u8>) -> Result<(), SessionError> {
        if self.state != StreamState::Open && self.state != StreamState::HalfClosedRemote {
            return Err(SessionError::InvalidState);
        }

        self.send_buffer.push_back(data);
        Ok(())
    }

    /// Read data from receive buffer
    #[must_use]
    pub fn read(&mut self) -> Option<Vec<u8>> {
        self.recv_buffer.pop_front()
    }

    /// Peek at receive buffer without removing
    #[must_use]
    pub fn peek(&self) -> Option<&Vec<u8>> {
        self.recv_buffer.front()
    }

    /// Check if send buffer has data
    #[must_use]
    pub fn has_data_to_send(&self) -> bool {
        !self.send_buffer.is_empty()
    }

    /// Check if receive buffer has data
    #[must_use]
    pub fn has_received_data(&self) -> bool {
        !self.recv_buffer.is_empty()
    }

    /// Get send buffer size
    #[must_use]
    pub fn send_buffer_size(&self) -> usize {
        self.send_buffer.iter().map(Vec::len).sum()
    }

    /// Get receive buffer size
    #[must_use]
    pub fn recv_buffer_size(&self) -> usize {
        self.recv_buffer.iter().map(Vec::len).sum()
    }

    /// Consume send window (when sending data)
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InvalidState` if the requested bytes exceed
    /// the available send window.
    pub fn consume_send_window(&mut self, bytes: u64) -> Result<(), SessionError> {
        if bytes > self.send_window {
            return Err(SessionError::InvalidState);
        }

        self.send_window -= bytes;
        self.bytes_sent += bytes;
        Ok(())
    }

    /// Update send window (when receiving `WINDOW_UPDATE`)
    pub fn update_send_window(&mut self, additional: u64) {
        self.send_window = (self.send_window + additional).min(self.max_window);
    }

    /// Consume receive window (when receiving data)
    ///
    /// # Errors
    ///
    /// Returns `SessionError::InvalidState` if the requested bytes exceed
    /// the available receive window.
    pub fn consume_recv_window(&mut self, bytes: u64) -> Result<(), SessionError> {
        if bytes > self.recv_window {
            return Err(SessionError::InvalidState);
        }

        self.recv_window -= bytes;
        self.bytes_received += bytes;
        Ok(())
    }

    /// Update receive window (send `WINDOW_UPDATE` to peer)
    pub fn update_recv_window(&mut self, additional: u64) {
        self.recv_window = (self.recv_window + additional).min(self.max_window);
    }

    /// Mark FIN sent
    pub fn mark_fin_sent(&mut self) {
        self.fin_sent = true;
        if self.state == StreamState::Open {
            self.state = StreamState::HalfClosedLocal;
        }
    }

    /// Mark FIN received
    pub fn mark_fin_received(&mut self) {
        self.fin_received = true;
        if self.state == StreamState::Open {
            self.state = StreamState::HalfClosedRemote;
        } else if self.state == StreamState::HalfClosedLocal {
            self.state = StreamState::Closed;
        }
    }

    /// Check if both FINs exchanged
    #[must_use]
    pub fn is_fully_closed(&self) -> bool {
        self.fin_sent && self.fin_received
    }

    /// Check if stream can send data
    #[must_use]
    pub fn can_send(&self) -> bool {
        matches!(
            self.state,
            StreamState::Open | StreamState::HalfClosedRemote
        ) && !self.fin_sent
            && self.send_window > 0
    }

    /// Check if stream can receive data
    #[must_use]
    pub fn can_receive(&self) -> bool {
        matches!(self.state, StreamState::Open | StreamState::HalfClosedLocal)
            && !self.fin_received
            && self.recv_window > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INITIAL_WINDOW: u64 = 65536;

    #[test]
    fn test_stream_creation() {
        let stream = Stream::new(42, INITIAL_WINDOW);

        assert_eq!(stream.id(), 42);
        assert_eq!(stream.state(), StreamState::Idle);
        assert_eq!(stream.send_window(), INITIAL_WINDOW);
        assert_eq!(stream.recv_window(), INITIAL_WINDOW);
        assert_eq!(stream.bytes_sent(), 0);
        assert_eq!(stream.bytes_received(), 0);
        assert!(!stream.is_fully_closed());
    }

    #[test]
    fn test_stream_client_vs_server_initiated() {
        let client_stream = Stream::new(1, INITIAL_WINDOW);
        let server_stream = Stream::new(2, INITIAL_WINDOW);

        assert!(client_stream.is_client_initiated());
        assert!(!server_stream.is_client_initiated());
    }

    #[test]
    fn test_stream_open_transition() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);

        assert_eq!(stream.state(), StreamState::Idle);
        stream.open().unwrap();
        assert_eq!(stream.state(), StreamState::Open);
    }

    #[test]
    fn test_stream_close_transition() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        stream.close().unwrap();
        assert_eq!(stream.state(), StreamState::Closed);
    }

    #[test]
    fn test_stream_valid_state_transitions() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);

        // Idle -> Open
        assert!(stream.can_transition(StreamState::Open));
        stream.transition_to(StreamState::Open).unwrap();

        // Open -> HalfClosedLocal
        assert!(stream.can_transition(StreamState::HalfClosedLocal));
        stream.transition_to(StreamState::HalfClosedLocal).unwrap();

        // HalfClosedLocal -> Closed
        assert!(stream.can_transition(StreamState::Closed));
        stream.transition_to(StreamState::Closed).unwrap();
    }

    #[test]
    fn test_stream_invalid_state_transitions() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        // Can't go from Open to Idle
        assert!(!stream.can_transition(StreamState::Idle));
        assert!(stream.transition_to(StreamState::Idle).is_err());

        // Can't go from Open directly to DataSent without proper setup
        assert_eq!(stream.state(), StreamState::Open);
    }

    #[test]
    fn test_stream_half_close_local() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        stream.transition_to(StreamState::HalfClosedLocal).unwrap();
        assert_eq!(stream.state(), StreamState::HalfClosedLocal);

        // Can still receive data
        assert!(stream.can_transition(StreamState::Closed));
    }

    #[test]
    fn test_stream_half_close_remote() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        stream.transition_to(StreamState::HalfClosedRemote).unwrap();
        assert_eq!(stream.state(), StreamState::HalfClosedRemote);

        // Can still send data
        assert!(stream.can_transition(StreamState::Closed));
    }

    #[test]
    fn test_stream_write_data() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        let data = b"Hello, WRAITH!".to_vec();
        stream.write(data.clone()).unwrap();

        assert!(stream.has_data_to_send());
        assert_eq!(stream.send_buffer_size(), data.len());
    }

    #[test]
    fn test_stream_write_when_not_open() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);

        let data = b"Should fail".to_vec();
        assert!(stream.write(data).is_err());
    }

    #[test]
    fn test_stream_write_when_half_closed_remote() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();
        stream.transition_to(StreamState::HalfClosedRemote).unwrap();

        // Can still write when half-closed remote
        let data = b"Still writable".to_vec();
        stream.write(data.clone()).unwrap();
        assert_eq!(stream.send_buffer_size(), data.len());
    }

    #[test]
    fn test_stream_read_data() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        // Simulate receiving data by directly manipulating recv_buffer
        let data = b"Received data".to_vec();
        stream.recv_buffer.push_back(data.clone());

        assert!(stream.has_received_data());
        let read_data = stream.read().unwrap();
        assert_eq!(read_data, data);
        assert!(!stream.has_received_data());
    }

    #[test]
    fn test_stream_peek_data() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        let data = b"Peeked data".to_vec();
        stream.recv_buffer.push_back(data.clone());

        let peeked = stream.peek().unwrap();
        assert_eq!(peeked, &data);

        // Peek doesn't consume
        assert!(stream.has_received_data());
    }

    #[test]
    fn test_stream_consume_send_window() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        let bytes = 1024;
        stream.consume_send_window(bytes).unwrap();

        assert_eq!(stream.send_window(), INITIAL_WINDOW - bytes);
        assert_eq!(stream.bytes_sent(), bytes);
    }

    #[test]
    fn test_stream_consume_send_window_overflow() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        // Try to consume more than window allows
        assert!(stream.consume_send_window(INITIAL_WINDOW + 1).is_err());
    }

    #[test]
    fn test_stream_update_send_window() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        stream.update_send_window(1024);
        assert_eq!(stream.send_window(), INITIAL_WINDOW + 1024);
    }

    #[test]
    fn test_stream_send_window_max_limit() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        let max_window = stream.max_window;

        // Update beyond max should cap at max
        stream.update_send_window(max_window * 2);
        assert_eq!(stream.send_window(), max_window);
    }

    #[test]
    fn test_stream_consume_recv_window() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        let bytes = 2048;
        stream.consume_recv_window(bytes).unwrap();

        assert_eq!(stream.recv_window(), INITIAL_WINDOW - bytes);
        assert_eq!(stream.bytes_received(), bytes);
    }

    #[test]
    fn test_stream_consume_recv_window_overflow() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        assert!(stream.consume_recv_window(INITIAL_WINDOW + 1).is_err());
    }

    #[test]
    fn test_stream_update_recv_window() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        stream.update_recv_window(4096);
        assert_eq!(stream.recv_window(), INITIAL_WINDOW + 4096);
    }

    #[test]
    fn test_stream_fin_sent_transition() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        stream.mark_fin_sent();
        assert_eq!(stream.state(), StreamState::HalfClosedLocal);
        assert!(!stream.is_fully_closed());
    }

    #[test]
    fn test_stream_fin_received_transition() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        stream.mark_fin_received();
        assert_eq!(stream.state(), StreamState::HalfClosedRemote);
        assert!(!stream.is_fully_closed());
    }

    #[test]
    fn test_stream_both_fins_exchanged() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        stream.mark_fin_sent();
        assert_eq!(stream.state(), StreamState::HalfClosedLocal);

        stream.mark_fin_received();
        assert_eq!(stream.state(), StreamState::Closed);
        assert!(stream.is_fully_closed());
    }

    #[test]
    fn test_stream_reset() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        // Add some data
        stream.write(b"data".to_vec()).unwrap();
        stream.recv_buffer.push_back(b"received".to_vec());

        stream.reset().unwrap();

        assert_eq!(stream.state(), StreamState::Closed);
        assert!(!stream.has_data_to_send());
        assert!(!stream.has_received_data());
    }

    #[test]
    fn test_stream_can_send() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);

        // Can't send when idle
        assert!(!stream.can_send());

        stream.open().unwrap();
        assert!(stream.can_send());

        // Can send when half-closed remote
        stream.transition_to(StreamState::HalfClosedRemote).unwrap();
        assert!(stream.can_send());

        // Can't send when half-closed local
        let mut stream2 = Stream::new(2, INITIAL_WINDOW);
        stream2.open().unwrap();
        stream2.transition_to(StreamState::HalfClosedLocal).unwrap();
        assert!(!stream2.can_send());
    }

    #[test]
    fn test_stream_can_send_with_zero_window() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        // Exhaust send window
        stream.consume_send_window(INITIAL_WINDOW).unwrap();
        assert!(!stream.can_send());
    }

    #[test]
    fn test_stream_can_receive() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);

        // Can't receive when idle
        assert!(!stream.can_receive());

        stream.open().unwrap();
        assert!(stream.can_receive());

        // Can receive when half-closed local
        stream.transition_to(StreamState::HalfClosedLocal).unwrap();
        assert!(stream.can_receive());

        // Can't receive when half-closed remote
        let mut stream2 = Stream::new(2, INITIAL_WINDOW);
        stream2.open().unwrap();
        stream2
            .transition_to(StreamState::HalfClosedRemote)
            .unwrap();
        assert!(!stream2.can_receive());
    }

    #[test]
    fn test_stream_can_receive_with_zero_window() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        // Exhaust receive window
        stream.consume_recv_window(INITIAL_WINDOW).unwrap();
        assert!(!stream.can_receive());
    }

    #[test]
    fn test_stream_cleanup_on_close() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        // Add data to buffers
        stream.write(b"send data".to_vec()).unwrap();
        stream.recv_buffer.push_back(b"recv data".to_vec());

        assert!(stream.has_data_to_send());
        assert!(stream.has_received_data());

        stream.transition_to(StreamState::Closed).unwrap();

        // Buffers should be cleared
        assert!(!stream.has_data_to_send());
        assert!(!stream.has_received_data());
    }

    #[test]
    fn test_stream_buffer_sizes() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        assert_eq!(stream.send_buffer_size(), 0);
        assert_eq!(stream.recv_buffer_size(), 0);

        stream.write(b"123".to_vec()).unwrap();
        stream.write(b"45678".to_vec()).unwrap();

        assert_eq!(stream.send_buffer_size(), 8);

        stream.recv_buffer.push_back(b"abcd".to_vec());
        stream.recv_buffer.push_back(b"efgh".to_vec());

        assert_eq!(stream.recv_buffer_size(), 8);
    }

    #[test]
    fn test_stream_multiple_writes() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        let data1 = b"first".to_vec();
        let data2 = b"second".to_vec();
        let data3 = b"third".to_vec();

        stream.write(data1).unwrap();
        stream.write(data2).unwrap();
        stream.write(data3).unwrap();

        assert_eq!(stream.send_buffer_size(), 16); // 5 + 6 + 5
        assert_eq!(stream.send_buffer.len(), 3);
    }

    #[test]
    fn test_stream_fin_idempotent() {
        let mut stream = Stream::new(1, INITIAL_WINDOW);
        stream.open().unwrap();

        stream.mark_fin_sent();
        let state1 = stream.state();

        stream.mark_fin_sent(); // Call again
        let state2 = stream.state();

        assert_eq!(state1, state2);
        assert_eq!(state1, StreamState::HalfClosedLocal);
    }

    #[test]
    fn test_stream_max_window_enforced() {
        let stream = Stream::new(1, INITIAL_WINDOW);
        let max = stream.max_window;

        // Max window should be 16x initial
        assert_eq!(max, INITIAL_WINDOW * 16);
    }
}
