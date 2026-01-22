// AddGuardianModal Component for WRAITH Vault
// Modal for adding new guardians to the vault

import React, { useState } from "react";
import { useGuardianStore } from "../stores/guardianStore";

interface AddGuardianModalProps {
  onClose: () => void;
  onAdded: () => void;
}

export function AddGuardianModal({ onClose, onAdded }: AddGuardianModalProps) {
  const { addGuardian, loading, error, clearError } = useGuardianStore();

  const [name, setName] = useState("");
  const [peerId, setPeerId] = useState("");
  const [publicKey, setPublicKey] = useState("");
  const [notes, setNotes] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);
  const [scanningPeer, setScanningPeer] = useState(false);

  const validate = (): boolean => {
    setValidationError(null);

    if (!name.trim()) {
      setValidationError("Guardian name is required");
      return false;
    }

    if (!peerId.trim()) {
      setValidationError("Peer ID is required");
      return false;
    }

    if (!publicKey.trim()) {
      setValidationError("Public key is required");
      return false;
    }

    // Basic validation for peer ID format (should be base58 or similar)
    if (peerId.length < 20) {
      setValidationError("Invalid peer ID format");
      return false;
    }

    // Basic validation for public key (should be hex or base64)
    if (publicKey.length < 32) {
      setValidationError("Invalid public key format");
      return false;
    }

    return true;
  };

  const handleAdd = async () => {
    if (!validate()) return;

    try {
      await addGuardian(name, peerId, publicKey, notes || null);
      onAdded();
    } catch (err) {
      console.error("Failed to add guardian:", err);
    }
  };

  const handleScanPeer = async () => {
    setScanningPeer(true);
    // In a real implementation, this would trigger a P2P discovery or QR scan
    // For now, we'll simulate finding a peer
    await new Promise((resolve) => setTimeout(resolve, 1500));

    // Simulated peer discovery result
    // In production, this would come from DHT discovery or QR code
    setScanningPeer(false);
  };

  const handlePasteFromClipboard = async () => {
    try {
      const text = await navigator.clipboard.readText();
      // Try to parse as JSON guardian info
      try {
        const info = JSON.parse(text);
        if (info.peer_id) setPeerId(info.peer_id);
        if (info.public_key) setPublicKey(info.public_key);
        if (info.name) setName(info.name);
      } catch {
        // If not JSON, assume it's just a peer ID
        setPeerId(text.trim());
      }
    } catch (err) {
      console.error("Failed to read clipboard:", err);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-gray-900 rounded-lg w-full max-w-md max-h-[90vh] overflow-hidden">
        {/* Header */}
        <div className="p-4 border-b border-gray-800">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold text-white">Add Guardian</h2>
            <button
              onClick={onClose}
              className="text-gray-400 hover:text-white transition"
            >
              Close
            </button>
          </div>
          <p className="text-sm text-gray-400 mt-1">
            Add a trusted peer to hold secret shards
          </p>
        </div>

        {/* Content */}
        <div className="p-6 space-y-4">
          {/* Error Display */}
          {(validationError || error) && (
            <div className="p-3 bg-red-900/20 border border-red-800 rounded text-red-400 text-sm">
              {validationError || error}
            </div>
          )}

          {/* Quick Actions */}
          <div className="flex gap-2">
            <button
              onClick={handleScanPeer}
              disabled={scanningPeer}
              className="flex-1 px-3 py-2 bg-gray-800 text-gray-300 rounded hover:bg-gray-700 disabled:opacity-50 text-sm"
            >
              {scanningPeer ? "Scanning..." : "Scan QR Code"}
            </button>
            <button
              onClick={handlePasteFromClipboard}
              className="flex-1 px-3 py-2 bg-gray-800 text-gray-300 rounded hover:bg-gray-700 text-sm"
            >
              Paste from Clipboard
            </button>
          </div>

          <div className="relative">
            <div className="absolute inset-0 flex items-center">
              <div className="w-full border-t border-gray-700" />
            </div>
            <div className="relative flex justify-center text-sm">
              <span className="px-2 bg-gray-900 text-gray-500">
                or enter manually
              </span>
            </div>
          </div>

          {/* Name */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Guardian Name *
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Alice's Laptop"
              className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500"
            />
            <p className="text-xs text-gray-500 mt-1">
              A friendly name to identify this guardian
            </p>
          </div>

          {/* Peer ID */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Peer ID *
            </label>
            <input
              type="text"
              value={peerId}
              onChange={(e) => setPeerId(e.target.value)}
              placeholder="12D3KooW..."
              className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
            />
            <p className="text-xs text-gray-500 mt-1">
              The WRAITH peer ID of the guardian node
            </p>
          </div>

          {/* Public Key */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Public Key *
            </label>
            <textarea
              value={publicKey}
              onChange={(e) => setPublicKey(e.target.value)}
              placeholder="Paste guardian's public key (hex or base64)..."
              rows={3}
              className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
            />
            <p className="text-xs text-gray-500 mt-1">
              Used to encrypt shards for this guardian
            </p>
          </div>

          {/* Notes */}
          <div>
            <label className="block text-sm text-gray-400 mb-1">
              Notes (optional)
            </label>
            <textarea
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Any notes about this guardian..."
              rows={2}
              className="w-full px-3 py-2 bg-gray-800 border border-gray-700 rounded text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-purple-500"
            />
          </div>

          {/* Trust Info */}
          <div className="bg-gray-800 rounded-lg p-4">
            <h4 className="text-sm font-medium text-gray-300 mb-2">
              Trust Information
            </h4>
            <p className="text-xs text-gray-500">
              New guardians start with "Basic" trust level. Trust levels
              increase based on successful recoveries and reliability. You can
              manually adjust trust levels after adding the guardian.
            </p>
          </div>
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-gray-800 flex items-center justify-between">
          <button
            onClick={onClose}
            className="px-4 py-2 text-gray-400 hover:text-white transition"
          >
            Cancel
          </button>
          <button
            onClick={handleAdd}
            disabled={loading}
            className="px-4 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 disabled:opacity-50 transition"
          >
            {loading ? "Adding..." : "Add Guardian"}
          </button>
        </div>
      </div>
    </div>
  );
}
