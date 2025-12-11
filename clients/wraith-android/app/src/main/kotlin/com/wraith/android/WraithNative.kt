package com.wraith.android

/**
 * Native interface to WRAITH Protocol via JNI.
 *
 * This class provides low-level access to the Rust implementation
 * of the WRAITH protocol for Android applications.
 */
object WraithNative {
    init {
        System.loadLibrary("wraith_android")
    }

    /**
     * Initialize a WRAITH node.
     *
     * @param listenAddr The address to listen on (e.g., "0.0.0.0:0")
     * @param configJson JSON configuration for the node
     * @return Node handle (> 0 on success, -1 on error)
     */
    external fun initNode(listenAddr: String, configJson: String): Long

    /**
     * Shutdown the WRAITH node.
     *
     * @param handle The node handle from initNode
     */
    external fun shutdownNode(handle: Long)

    /**
     * Establish a session with a remote peer.
     *
     * @param handle The node handle
     * @param peerId The peer ID (hex-encoded)
     * @return JSON string with session info, or null on error
     */
    external fun establishSession(handle: Long, peerId: String): String?

    /**
     * Send a file to a peer.
     *
     * @param handle The node handle
     * @param peerId The peer ID (hex-encoded)
     * @param filePath Path to the file to send
     * @return JSON string with transfer info, or null on error
     */
    external fun sendFile(handle: Long, peerId: String, filePath: String): String?

    /**
     * Get the current node status.
     *
     * @param handle The node handle
     * @return JSON string with node status
     */
    external fun getNodeStatus(handle: Long): String?
}
