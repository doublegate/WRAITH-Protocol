//! Packet handling for WRAITH nodes
//!
//! This module contains packet receive/dispatch logic extracted from node.rs
//! to reduce complexity and improve code organization.
//!
//! # Packet Flow
//!
//! ```text
//! UDP Socket → recv_from → handle_incoming_packet → dispatch_frame → handler
//!                                |
//!                                └→ handshake → SessionManager
//! ```

use crate::frame::{Frame, FrameBuilder, FrameType};
use crate::node::Node;
use crate::node::config::CoverTrafficDistribution;
use crate::node::error::{NodeError, Result};
use crate::node::file_transfer::FileTransferContext;
use crate::node::routing::extract_connection_id;
use crate::node::session::{HandshakePacket, PeerConnection};
use crate::transfer::TransferSession;
use crate::{ConnectionId, HandshakePhase, SessionState};
use getrandom::getrandom;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock, oneshot};
use wraith_files::chunker::FileChunker;
use wraith_transport::transport::Transport;

impl Node {
    /// Packet receive loop - main event loop for incoming packets
    ///
    /// Continuously receives packets from the transport layer and dispatches
    /// them for processing. Runs until the node is stopped.
    pub(crate) async fn packet_receive_loop(&self) {
        let mut buf = vec![0u8; 65536];
        loop {
            if !self.is_running() {
                break;
            }

            let transport = {
                let guard = self.inner.transport.lock().await;
                match guard.as_ref() {
                    Some(t) => Arc::clone(t),
                    None => break,
                }
            };

            match tokio::time::timeout(Duration::from_millis(100), transport.recv_from(&mut buf))
                .await
            {
                Ok(Ok((size, from))) => {
                    let packet_data = buf[..size].to_vec();
                    let node = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = node.handle_incoming_packet(packet_data, from).await {
                            tracing::debug!("Error handling packet from {}: {}", from, e);
                        }
                    });
                }
                Ok(Err(e)) => {
                    tracing::warn!("Error receiving packet: {}", e);
                }
                Err(_) => {
                    // Timeout - continue loop
                    continue;
                }
            }
        }
    }

    /// Cover traffic loop - generates dummy traffic to mask real activity
    ///
    /// Sends PAD frames to all active sessions at configured intervals
    /// to make traffic analysis more difficult.
    pub(crate) async fn cover_traffic_loop(&self) {
        let config = &self.inner.config.obfuscation.cover_traffic;
        let rate = config.rate;

        loop {
            if !self.is_running() {
                break;
            }

            let delay = match config.distribution {
                CoverTrafficDistribution::Constant => {
                    if rate > 0.0 {
                        Duration::from_secs_f64(1.0 / rate)
                    } else {
                        Duration::from_secs(1)
                    }
                }
                CoverTrafficDistribution::Poisson => {
                    use rand::Rng;
                    let u: f64 = rand::thread_rng().r#gen();
                    Duration::from_secs_f64((-u.ln() / rate).min(10.0))
                }
                CoverTrafficDistribution::Uniform { min_ms, max_ms } => {
                    use rand::Rng;
                    Duration::from_millis(rand::thread_rng().gen_range(min_ms..=max_ms))
                }
            };

            tokio::time::sleep(delay).await;

            // Send cover traffic to all active sessions
            for entry in self.inner.sessions.iter() {
                let connection = entry.value();
                let mut pad_data = vec![0u8; 64];
                if getrandom(&mut pad_data).is_err() {
                    continue;
                }

                let frame_bytes = match FrameBuilder::new()
                    .frame_type(FrameType::Pad)
                    .stream_id(0)
                    .payload(&pad_data)
                    .build(128)
                {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };

                let connection = Arc::clone(connection);
                let node = self.clone();
                tokio::spawn(async move {
                    let _ = node.send_encrypted_frame(&connection, &frame_bytes).await;
                });
            }
        }
    }

    /// Handle incoming packet from network
    ///
    /// Unwraps protocol obfuscation, routes packet by Connection ID,
    /// and dispatches to appropriate handler.
    pub(crate) async fn handle_incoming_packet(
        &self,
        data: Vec<u8>,
        from: SocketAddr,
    ) -> Result<()> {
        // Unwrap any protocol mimicry
        let unwrapped = self.unwrap_protocol(&data)?;

        // Check for pending handshake matching this source
        let matching_addr = self
            .inner
            .pending_handshakes
            .iter()
            .find(|entry| {
                let registered = entry.key();
                if registered.ip().is_unspecified() {
                    from.port() == registered.port()
                } else {
                    from == *registered
                }
            })
            .map(|entry| *entry.key());

        // If there's a pending handshake, forward the packet
        if let Some(addr) = matching_addr {
            if let Some((_addr, tx)) = self.inner.pending_handshakes.remove(&addr) {
                let packet = HandshakePacket {
                    data: unwrapped.clone(),
                    from,
                };
                let _ = tx.send(packet);
                return Ok(());
            }
        }

        // Route by Connection ID
        match extract_connection_id(&unwrapped) {
            Some(connection_id) => {
                if let Some(conn) = self.inner.routing.lookup(connection_id) {
                    conn.touch();
                    match conn.decrypt_frame(&unwrapped[8..]).await {
                        Ok(frame_bytes) => {
                            let node = self.clone();
                            tokio::spawn(async move {
                                if let Err(e) = node.dispatch_frame(frame_bytes).await {
                                    tracing::warn!("Error handling frame: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::warn!("Failed to decrypt packet from {}: {}", from, e);
                        }
                    }
                } else {
                    // Unknown Connection ID - might be a handshake initiation
                    if let Err(e) = self.handle_handshake_initiation(&unwrapped, from).await {
                        tracing::warn!("Handshake initiation failed from {}: {}", from, e);
                    }
                }
            }
            None => {
                // No Connection ID - likely a handshake initiation
                if let Err(e) = self.handle_handshake_initiation(&unwrapped, from).await {
                    tracing::warn!("Handshake initiation failed from {}: {}", from, e);
                }
            }
        }

        Ok(())
    }

    /// Dispatch frame to appropriate handler based on frame type
    pub(crate) async fn dispatch_frame(&self, frame_bytes: Vec<u8>) -> Result<()> {
        let frame = Frame::parse(&frame_bytes)
            .map_err(|e| NodeError::Other(format!("Failed to parse frame: {}", e)))?;

        match frame.frame_type() {
            FrameType::StreamOpen => self.handle_stream_open_frame(frame).await,
            FrameType::Data => self.handle_data_frame(frame).await,
            FrameType::StreamClose => {
                tracing::debug!("Received StreamClose frame");
                Ok(())
            }
            _ => {
                tracing::debug!("Unhandled frame type: {:?}", frame.frame_type());
                Ok(())
            }
        }
    }

    /// Handle handshake initiation (responder side)
    ///
    /// When a packet arrives that doesn't match a known Connection ID,
    /// it may be a Noise_XX handshake initiation.
    pub(crate) async fn handle_handshake_initiation(
        &self,
        msg1: &[u8],
        peer_addr: SocketAddr,
    ) -> Result<crate::node::session::SessionId> {
        let transport = self.get_transport().await?;

        tracing::info!(
            "Handling handshake initiation from {} ({} bytes)",
            peer_addr,
            msg1.len()
        );

        // Create channel for receiving msg3
        let (msg3_tx, msg3_rx) = oneshot::channel();
        self.inner.pending_handshakes.insert(peer_addr, msg3_tx);

        // Perform Noise_XX handshake as responder
        let handshake_result = crate::node::session::perform_handshake_responder(
            self.inner.identity.x25519_keypair(),
            msg1,
            peer_addr,
            transport.as_ref(),
            Some(msg3_rx),
        )
        .await;

        // Clean up pending handshake
        self.inner.pending_handshakes.remove(&peer_addr);

        let (crypto, session_id, peer_id) = handshake_result?;

        // Derive connection ID from session ID
        let mut connection_id_bytes = [0u8; 8];
        connection_id_bytes.copy_from_slice(&session_id[..8]);
        let connection_id = ConnectionId::from_bytes(connection_id_bytes);

        // Create connection
        let connection = PeerConnection::new(session_id, peer_id, peer_addr, connection_id, crypto);

        // Transition through handshake states
        connection
            .transition_to(SessionState::Handshaking(HandshakePhase::RespSent))
            .await?;
        connection.transition_to(SessionState::Established).await?;

        // Check for existing session
        if self.inner.sessions.contains_key(&peer_id) {
            if let Some(existing) = self.inner.sessions.get(&peer_id) {
                return Ok(existing.session_id);
            }
        }

        // Store session and route
        let connection_arc = Arc::new(connection);
        self.inner
            .sessions
            .insert(peer_id, Arc::clone(&connection_arc));

        let cid_u64 = u64::from_be_bytes(connection_id_bytes);
        self.inner.routing.add_route(cid_u64, connection_arc);

        tracing::info!(
            "Session established as responder with peer {}, session: {}, route: {:016x}",
            hex::encode(&peer_id[..8]),
            hex::encode(&session_id[..8]),
            cid_u64
        );

        Ok(session_id)
    }

    /// Handle StreamOpen frame (file transfer metadata)
    pub(crate) async fn handle_stream_open_frame(&self, frame: Frame<'_>) -> Result<()> {
        let metadata = crate::node::file_transfer::FileMetadata::deserialize(frame.payload())?;

        tracing::info!(
            "Received file transfer request: {} ({} bytes)",
            metadata.file_name,
            metadata.file_size
        );

        // Create receive transfer session
        let mut transfer = TransferSession::new_receive(
            metadata.transfer_id,
            std::path::PathBuf::from(&metadata.file_name),
            metadata.file_size,
            metadata.chunk_size as usize,
        );
        transfer.start();

        // Create file reassembler
        let reassembler = wraith_files::chunker::FileReassembler::new(
            &metadata.file_name,
            metadata.file_size,
            metadata.chunk_size as usize,
        )
        .map_err(NodeError::Io)?;

        // Create tree hash (root only for now)
        let tree_hash = wraith_files::tree_hash::FileTreeHash {
            root: metadata.root_hash,
            chunks: Vec::new(),
        };

        // Store transfer context
        let context = Arc::new(FileTransferContext::new_receive(
            metadata.transfer_id,
            Arc::new(RwLock::new(transfer)),
            Arc::new(Mutex::new(reassembler)),
            tree_hash,
        ));
        self.inner.transfers.insert(metadata.transfer_id, context);

        Ok(())
    }

    /// Handle Data frame (file chunk)
    pub(crate) async fn handle_data_frame(&self, frame: Frame<'_>) -> Result<()> {
        let chunk_index = frame.sequence() as u64;
        let chunk_data = frame.payload();

        // Find matching transfer by stream ID
        let mut matched_context = None;
        for entry in self.inner.transfers.iter() {
            let tid = entry.key();
            let stream_id = ((tid[0] as u16) << 8) | (tid[1] as u16);
            if stream_id == frame.stream_id() {
                matched_context = Some(entry.value().clone());
                break;
            }
        }

        let context = matched_context.ok_or_else(|| {
            NodeError::InvalidState(format!("No transfer for stream_id {}", frame.stream_id()))
        })?;
        let transfer_id = context.transfer_id;

        // Write chunk to reassembler
        if let Some(reassembler_arc) = &context.reassembler {
            reassembler_arc
                .lock()
                .await
                .write_chunk(chunk_index, chunk_data)
                .map_err(NodeError::Io)?;
        }

        // Verify chunk hash if available
        if chunk_index < context.tree_hash.chunks.len() as u64 {
            let computed_hash = blake3::hash(chunk_data);
            if computed_hash.as_bytes() != &context.tree_hash.chunks[chunk_index as usize] {
                return Err(NodeError::InvalidState(
                    "Chunk hash verification failed".to_string(),
                ));
            }
        }

        // Update transfer progress
        let mut transfer = context.transfer_session.write().await;
        transfer.mark_chunk_transferred(chunk_index, chunk_data.len());

        if transfer.is_complete() {
            tracing::info!(
                "File transfer {:?} completed ({} bytes)",
                hex::encode(&transfer_id[..8]),
                transfer.file_size
            );
        }

        Ok(())
    }

    /// Send file chunks to peer
    pub(crate) async fn send_file_chunks(
        &self,
        transfer_id: crate::node::identity::TransferId,
        file_path: std::path::PathBuf,
        stream_id: u16,
        connection: Arc<PeerConnection>,
    ) -> Result<()> {
        let context = self
            .inner
            .transfers
            .get(&transfer_id)
            .ok_or(NodeError::TransferNotFound(transfer_id))?
            .clone();

        let mut chunker = FileChunker::new(&file_path, self.inner.config.transfer.chunk_size)
            .map_err(NodeError::Io)?;

        let total_chunks = chunker.num_chunks();

        for chunk_index in 0..total_chunks {
            let chunk_data = chunker.read_chunk_at(chunk_index).map_err(NodeError::Io)?;
            let chunk_len = chunk_data.len();

            // Verify chunk hash
            if chunk_index < context.tree_hash.chunks.len() as u64 {
                let computed_hash = blake3::hash(&chunk_data);
                if computed_hash.as_bytes() != &context.tree_hash.chunks[chunk_index as usize] {
                    return Err(NodeError::InvalidState(
                        "Chunk hash verification failed".to_string(),
                    ));
                }
            }

            // Build and send chunk frame
            let chunk_frame =
                crate::node::file_transfer::build_chunk_frame(stream_id, chunk_index, &chunk_data)?;

            self.send_encrypted_frame(&connection, &chunk_frame).await?;

            // Update progress
            context
                .transfer_session
                .write()
                .await
                .mark_chunk_transferred(chunk_index, chunk_len);
        }

        tracing::info!(
            "File transfer {:?} completed ({} chunks sent)",
            hex::encode(&transfer_id[..8]),
            total_chunks
        );

        Ok(())
    }

    /// Send encrypted frame to peer
    #[allow(dead_code)]
    pub(crate) async fn send_encrypted_frame(
        &self,
        connection: &PeerConnection,
        frame_bytes: &[u8],
    ) -> Result<()> {
        // Encrypt the frame
        let encrypted = connection.encrypt_frame(frame_bytes).await?;
        let encrypted_len = encrypted.len();

        // Apply obfuscation
        let mut obfuscated = encrypted;
        self.apply_obfuscation(&mut obfuscated)?;

        // Wrap in protocol mimicry
        let wrapped = self.wrap_protocol(&obfuscated)?;

        // Apply timing delay
        let delay = self.get_timing_delay();
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }

        // Send via transport
        let transport = self.get_transport().await?;
        transport
            .send_to(&wrapped, connection.peer_addr)
            .await
            .map_err(|e| NodeError::Transport(format!("Failed to send packet: {}", e)))?;

        tracing::trace!(
            "Sent {} obfuscated bytes to {} (original: {} encrypted)",
            wrapped.len(),
            connection.peer_addr,
            encrypted_len
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cover_traffic_distribution_constant() {
        let rate = 10.0; // 10 packets per second
        let expected_delay = Duration::from_secs_f64(1.0 / rate);
        assert_eq!(expected_delay, Duration::from_millis(100));
    }
}
