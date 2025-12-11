package com.wraith.android

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject

/**
 * High-level Kotlin wrapper for WRAITH Protocol.
 *
 * This class provides a more idiomatic Kotlin interface to the
 * WRAITH protocol, with proper error handling and coroutine support.
 */
class WraithClient {
    private var nodeHandle: Long = 0

    /**
     * Start the WRAITH node.
     *
     * @param listenAddr The address to listen on (default: "0.0.0.0:0")
     * @param config Optional node configuration
     * @throws WraithException if initialization fails
     */
    suspend fun start(
        listenAddr: String = "0.0.0.0:0",
        config: NodeConfig = NodeConfig()
    ) = withContext(Dispatchers.IO) {
        val configJson = config.toJson()
        nodeHandle = WraithNative.initNode(listenAddr, configJson)
        if (nodeHandle < 0) {
            throw WraithException("Failed to initialize WRAITH node")
        }
    }

    /**
     * Shutdown the WRAITH node.
     */
    suspend fun shutdown() = withContext(Dispatchers.IO) {
        if (nodeHandle > 0) {
            WraithNative.shutdownNode(nodeHandle)
            nodeHandle = 0
        }
    }

    /**
     * Establish a session with a remote peer.
     *
     * @param peerId The peer ID (hex string)
     * @return SessionInfo for the established session
     * @throws WraithException if session establishment fails
     */
    suspend fun establishSession(peerId: String): SessionInfo = withContext(Dispatchers.IO) {
        ensureStarted()
        val json = WraithNative.establishSession(nodeHandle, peerId)
            ?: throw WraithException("Failed to establish session with peer $peerId")
        SessionInfo.fromJson(json)
    }

    /**
     * Send a file to a peer.
     *
     * @param peerId The peer ID (hex string)
     * @param filePath Path to the file to send
     * @return TransferInfo for the file transfer
     * @throws WraithException if file send fails
     */
    suspend fun sendFile(peerId: String, filePath: String): TransferInfo = withContext(Dispatchers.IO) {
        ensureStarted()
        val json = WraithNative.sendFile(nodeHandle, peerId, filePath)
            ?: throw WraithException("Failed to send file $filePath to peer $peerId")
        TransferInfo.fromJson(json)
    }

    /**
     * Get the current node status.
     *
     * @return NodeStatus with current node information
     * @throws WraithException if status retrieval fails
     */
    suspend fun getStatus(): NodeStatus = withContext(Dispatchers.IO) {
        ensureStarted()
        val json = WraithNative.getNodeStatus(nodeHandle)
            ?: throw WraithException("Failed to get node status")
        NodeStatus.fromJson(json)
    }

    private fun ensureStarted() {
        if (nodeHandle <= 0) {
            throw WraithException("WRAITH node not started. Call start() first.")
        }
    }
}

/**
 * Configuration for a WRAITH node.
 */
data class NodeConfig(
    val maxSessions: Int = 100,
    val maxTransfers: Int = 10,
    val bufferSize: Int = 65536,
) {
    fun toJson(): String = JSONObject().apply {
        put("max_sessions", maxSessions)
        put("max_transfers", maxTransfers)
        put("buffer_size", bufferSize)
    }.toString()
}

/**
 * Information about an established session.
 */
data class SessionInfo(
    val sessionId: String,
    val peerId: String,
    val connected: Boolean,
) {
    companion object {
        fun fromJson(json: String): SessionInfo {
            val obj = JSONObject(json)
            return SessionInfo(
                sessionId = obj.getString("sessionId"),
                peerId = obj.getString("peerId"),
                connected = obj.getBoolean("connected"),
            )
        }
    }
}

/**
 * Information about a file transfer.
 */
data class TransferInfo(
    val transferId: String,
    val peerId: String,
    val filePath: String,
    val status: String,
) {
    companion object {
        fun fromJson(json: String): TransferInfo {
            val obj = JSONObject(json)
            return TransferInfo(
                transferId = obj.getString("transferId"),
                peerId = obj.getString("peerId"),
                filePath = obj.getString("filePath"),
                status = obj.getString("status"),
            )
        }
    }
}

/**
 * Current status of the WRAITH node.
 */
data class NodeStatus(
    val running: Boolean,
    val localPeerId: String,
    val sessionCount: Int,
    val activeTransfers: Int,
) {
    companion object {
        fun fromJson(json: String): NodeStatus {
            val obj = JSONObject(json)
            return NodeStatus(
                running = obj.getBoolean("running"),
                localPeerId = obj.getString("localPeerId"),
                sessionCount = obj.getInt("sessionCount"),
                activeTransfers = obj.getInt("activeTransfers"),
            )
        }
    }
}

/**
 * Exception thrown by WRAITH operations.
 */
class WraithException(message: String, cause: Throwable? = null) : Exception(message, cause)
