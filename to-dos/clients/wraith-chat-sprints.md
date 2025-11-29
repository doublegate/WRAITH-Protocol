# WRAITH-Chat Client - Sprint Planning

**Client Name:** WRAITH-Chat
**Tier:** 1 (High Priority)
**Description:** Secure end-to-end encrypted messaging application
**Target Platforms:** Windows, macOS, Linux, iOS, Android
**UI Framework:** React Native + Tauri (desktop), React Native (mobile)
**Timeline:** 14 weeks (3.5 sprints × 4 weeks)
**Total Story Points:** 182

---

## Overview

WRAITH-Chat is a secure messaging application leveraging the WRAITH protocol for encrypted peer-to-peer and group communications. It provides Signal-level security with traffic obfuscation and decentralized architecture.

**Core Value Proposition:**
- End-to-end encrypted 1:1 and group chats
- Disappearing messages with forward secrecy
- Voice/video calls over WRAITH protocol
- No phone number or email required
- Cross-platform sync without central servers

---

## Success Criteria

**User Experience:**
- [ ] Send message in <500ms latency (local network)
- [ ] Message delivery confirmation within 1 second
- [ ] Group chat supports 250+ members
- [ ] Voice call quality: clear audio at 64 kbps
- [ ] Video call quality: 720p at 1.5 Mbps

**Performance:**
- [ ] Message database handles 1M+ messages
- [ ] Search 100k messages in <200ms
- [ ] Application startup in <1.5 seconds
- [ ] <100 MB memory baseline
- [ ] <50 MB disk per 10k messages

**Platform Support:**
- [ ] Desktop: Windows 10+, macOS 11+, Linux (Ubuntu/Fedora)
- [ ] Mobile: iOS 14+, Android 10+
- [ ] Seamless sync across all devices
- [ ] Push notifications on mobile
- [ ] Background message sync

**Security:**
- [ ] Double ratchet algorithm (Signal protocol)
- [ ] Sealed sender (metadata protection)
- [ ] Contact verification via safety numbers
- [ ] Message expiration (5 seconds to 1 week)
- [ ] Screenshot detection/warning

---

## Dependencies

**Protocol Dependencies:**
- WRAITH protocol Phases 1-6 (weeks 1-36)
- Real-time bidirectional streaming support
- Group message routing in DHT
- Mobile-optimized obfuscation profiles

**External Dependencies:**
- React Native 0.73+
- Tauri 2.x (desktop)
- SQLCipher for encrypted database
- WebRTC for voice/video calls
- FCM/APNs for push notifications

**Team Dependencies:**
- Mobile developer (iOS/Android)
- Backend developer (notification service)
- UI/UX designer (messaging patterns)
- Security auditor (cryptography review)

---

## Deliverables

**Sprint 1 (Weeks 37-40): Core Messaging**
1. React Native/Tauri project setup
2. Encrypted SQLite database schema
3. 1:1 text messaging UI and logic
4. Message encryption (Double Ratchet)
5. Contact management (add/verify/block)
6. Message delivery receipts
7. Typing indicators
8. Read receipts

**Sprint 2 (Weeks 41-44): Group Chat & Media**
1. Group chat creation and management
2. Group message encryption (Sender Keys)
3. Admin controls (add/remove members)
4. Media attachments (images, videos, files)
5. Media encryption and thumbnails
6. Voice messages (record/send/play)
7. Link previews
8. Emoji reactions

**Sprint 3 (Weeks 45-48): Voice/Video Calls**
1. WebRTC integration
2. 1:1 voice calls
3. 1:1 video calls
4. Call signaling over WRAITH
5. STUN/TURN fallback
6. In-call UI (mute, camera toggle, end)
7. Call history
8. Ringtones and notifications

**Sprint 4 (Weeks 49-50): Polish & Distribution**
1. Push notifications (FCM/APNs)
2. Background sync service
3. Message search and filtering
4. Disappearing messages
5. App lock (PIN/biometric)
6. Export chat history (encrypted backup)
7. Platform-specific builds (IPA, APK, MSI, DMG, AppImage)
8. App store submission (iOS/Android)

---

## Sprint 1: Core Messaging (Weeks 37-40)

### Sprint Goal
Implement encrypted 1:1 text messaging with contact management and delivery confirmation.

**Total Story Points:** 52

---

### S1.1: Project Setup (5 points)

**Task:** Initialize React Native project with Tauri desktop wrapper.

**Acceptance Criteria:**
- [ ] React Native 0.73+ project created
- [ ] Tauri wrapper for desktop builds
- [ ] TypeScript configured
- [ ] Hot reloading works on all platforms
- [ ] Build targets: iOS, Android, Windows, macOS, Linux

**Project Initialization:**
```bash
# Create React Native project
npx react-native@latest init WraithChat --template react-native-template-typescript

cd WraithChat

# Install dependencies
npm install @react-navigation/native @react-navigation/stack
npm install react-native-gesture-handler react-native-reanimated
npm install react-native-safe-area-context react-native-screens
npm install react-native-vector-icons
npm install @react-native-async-storage/async-storage

# SQLCipher for encrypted storage
npm install react-native-sqlcipher-storage

# Install Tauri CLI for desktop builds
npm install --save-dev @tauri-apps/cli
```

**Tauri Configuration (Desktop):**
```json
// tauri.conf.json
{
  "productName": "WRAITH Chat",
  "version": "0.1.0",
  "identifier": "com.wraith.chat",
  "build": {
    "beforeDevCommand": "npm start",
    "beforeBuildCommand": "npm run build",
    "devPath": "http://localhost:8081",
    "distDir": "../build"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "notification": { "all": true },
      "fs": {
        "scope": ["$APPDATA/*"],
        "readFile": true,
        "writeFile": true
      },
      "path": { "all": true },
      "dialog": { "all": true }
    },
    "windows": [{
      "title": "WRAITH Chat",
      "width": 1000,
      "height": 700,
      "minWidth": 800,
      "minHeight": 600,
      "resizable": true
    }]
  }
}
```

---

### S1.2: Encrypted Database Schema (8 points)

**Task:** Design and implement SQLCipher database for messages, contacts, and keys.

**Acceptance Criteria:**
- [ ] Database encrypted with AES-256
- [ ] Schema supports messages, contacts, groups, media
- [ ] Indexes for fast message search
- [ ] Migration system for schema updates
- [ ] Auto-vacuum and integrity checks

**Schema Definition:**
```sql
-- Contacts table
CREATE TABLE contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    peer_id TEXT UNIQUE NOT NULL,
    display_name TEXT,
    identity_key BLOB NOT NULL,
    safety_number TEXT NOT NULL,
    verified INTEGER DEFAULT 0,
    blocked INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    last_seen INTEGER
);

-- Conversations table (1:1 and groups)
CREATE TABLE conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL CHECK(type IN ('direct', 'group')),
    peer_id TEXT, -- For direct chats
    group_id TEXT, -- For group chats
    display_name TEXT,
    avatar BLOB,
    muted INTEGER DEFAULT 0,
    archived INTEGER DEFAULT 0,
    last_message_id INTEGER,
    last_message_at INTEGER,
    unread_count INTEGER DEFAULT 0,
    FOREIGN KEY (last_message_id) REFERENCES messages(id)
);

-- Messages table
CREATE TABLE messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id INTEGER NOT NULL,
    sender_peer_id TEXT NOT NULL,
    content_type TEXT NOT NULL CHECK(content_type IN ('text', 'media', 'voice', 'file')),
    body TEXT, -- Encrypted message body
    media_path TEXT,
    media_mime_type TEXT,
    media_size INTEGER,
    timestamp INTEGER NOT NULL,
    sent INTEGER DEFAULT 0,
    delivered INTEGER DEFAULT 0,
    read INTEGER DEFAULT 0,
    expires_at INTEGER,
    direction TEXT NOT NULL CHECK(direction IN ('incoming', 'outgoing')),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

-- Group members table
CREATE TABLE group_members (
    group_id TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('admin', 'member')),
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (group_id, peer_id)
);

-- Cryptographic keys table (Double Ratchet state)
CREATE TABLE ratchet_states (
    peer_id TEXT PRIMARY KEY,
    root_key BLOB NOT NULL,
    sending_chain_key BLOB NOT NULL,
    receiving_chain_key BLOB NOT NULL,
    sending_chain_index INTEGER NOT NULL,
    receiving_chain_index INTEGER NOT NULL,
    dh_sending_key BLOB NOT NULL,
    dh_receiving_key BLOB,
    previous_sending_chain_length INTEGER,
    skipped_keys BLOB -- Serialized map of skipped keys
);

-- Indexes for performance
CREATE INDEX idx_messages_conversation ON messages(conversation_id, timestamp DESC);
CREATE INDEX idx_messages_sender ON messages(sender_peer_id);
CREATE INDEX idx_messages_search ON messages(body); -- Full-text search
CREATE INDEX idx_contacts_peer_id ON contacts(peer_id);
CREATE INDEX idx_group_members_group ON group_members(group_id);
```

**TypeScript Database Interface:**
```typescript
// src/database/Database.ts
import SQLite from 'react-native-sqlcipher-storage';

const DB_NAME = 'wraith_chat.db';
const DB_VERSION = 1;

export class Database {
  private db: SQLite.SQLiteDatabase | null = null;

  async open(password: string): Promise<void> {
    this.db = await SQLite.openDatabase({
      name: DB_NAME,
      location: 'default',
      key: password, // SQLCipher encryption key
    });

    await this.initialize();
  }

  private async initialize(): Promise<void> {
    if (!this.db) throw new Error('Database not opened');

    await this.db.executeSql(`
      PRAGMA cipher_page_size = 4096;
      PRAGMA kdf_iter = 64000;
      PRAGMA cipher_hmac_algorithm = HMAC_SHA512;
      PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512;
    `);

    // Create tables if not exists
    await this.db.executeSql(/* CREATE TABLE contacts ... */);
    await this.db.executeSql(/* CREATE TABLE conversations ... */);
    await this.db.executeSql(/* CREATE TABLE messages ... */);
    await this.db.executeSql(/* CREATE TABLE group_members ... */);
    await this.db.executeSql(/* CREATE TABLE ratchet_states ... */);

    // Create indexes
    await this.db.executeSql(/* CREATE INDEX ... */);
  }

  async insertMessage(message: Message): Promise<number> {
    if (!this.db) throw new Error('Database not opened');

    const result = await this.db.executeSql(
      `INSERT INTO messages (conversation_id, sender_peer_id, content_type, body, timestamp, direction)
       VALUES (?, ?, ?, ?, ?, ?)`,
      [message.conversationId, message.senderPeerId, message.contentType, message.body, message.timestamp, message.direction]
    );

    return result[0].insertId;
  }

  async getConversationMessages(conversationId: number, limit: number = 50, offset: number = 0): Promise<Message[]> {
    if (!this.db) throw new Error('Database not opened');

    const result = await this.db.executeSql(
      `SELECT * FROM messages WHERE conversation_id = ? ORDER BY timestamp DESC LIMIT ? OFFSET ?`,
      [conversationId, limit, offset]
    );

    return result[0].rows.raw() as Message[];
  }

  async searchMessages(query: string): Promise<Message[]> {
    if (!this.db) throw new Error('Database not opened');

    const result = await this.db.executeSql(
      `SELECT * FROM messages WHERE body LIKE ? ORDER BY timestamp DESC LIMIT 100`,
      [`%${query}%`]
    );

    return result[0].rows.raw() as Message[];
  }

  async close(): Promise<void> {
    if (this.db) {
      await this.db.close();
      this.db = null;
    }
  }
}

export interface Message {
  id?: number;
  conversationId: number;
  senderPeerId: string;
  contentType: 'text' | 'media' | 'voice' | 'file';
  body?: string;
  mediaPath?: string;
  mediaMimeType?: string;
  mediaSize?: number;
  timestamp: number;
  sent?: boolean;
  delivered?: boolean;
  read?: boolean;
  expiresAt?: number;
  direction: 'incoming' | 'outgoing';
}
```

---

### S1.3: 1:1 Text Messaging UI (8 points)

**Task:** Build conversation list and chat screen UI.

**Acceptance Criteria:**
- [ ] Conversation list shows recent chats
- [ ] Chat screen displays messages in chronological order
- [ ] Message bubbles styled (sent vs received)
- [ ] Text input with send button
- [ ] Scroll to bottom on new message
- [ ] Pull-to-refresh for older messages

**Conversation List Component:**
```tsx
// src/screens/ConversationListScreen.tsx
import React, { useState, useEffect } from 'react';
import { FlatList, View, Text, TouchableOpacity, Image } from 'react-native';
import { useNavigation } from '@react-navigation/native';
import { Database, Conversation } from '../database';

export function ConversationListScreen() {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const navigation = useNavigation();
  const db = new Database();

  useEffect(() => {
    loadConversations();
  }, []);

  const loadConversations = async () => {
    await db.open(/* password */);
    const convos = await db.getConversations();
    setConversations(convos);
  };

  const renderConversation = ({ item }: { item: Conversation }) => (
    <TouchableOpacity
      style={styles.conversationItem}
      onPress={() => navigation.navigate('Chat', { conversationId: item.id })}
    >
      <Image source={{ uri: item.avatar }} style={styles.avatar} />
      <View style={styles.conversationInfo}>
        <Text style={styles.displayName}>{item.displayName}</Text>
        <Text style={styles.lastMessage} numberOfLines={1}>
          {item.lastMessageBody}
        </Text>
      </View>
      <View style={styles.metaInfo}>
        <Text style={styles.timestamp}>{formatTimestamp(item.lastMessageAt)}</Text>
        {item.unreadCount > 0 && (
          <View style={styles.unreadBadge}>
            <Text style={styles.unreadCount}>{item.unreadCount}</Text>
          </View>
        )}
      </View>
    </TouchableOpacity>
  );

  return (
    <View style={styles.container}>
      <FlatList
        data={conversations}
        renderItem={renderConversation}
        keyExtractor={item => item.id.toString()}
        ItemSeparatorComponent={() => <View style={styles.separator} />}
      />
    </View>
  );
}

function formatTimestamp(ts: number): string {
  const now = Date.now();
  const diff = now - ts;

  if (diff < 60000) return 'Just now';
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h`;

  const date = new Date(ts);
  return `${date.getMonth() + 1}/${date.getDate()}`;
}
```

**Chat Screen Component:**
```tsx
// src/screens/ChatScreen.tsx
import React, { useState, useEffect, useRef } from 'react';
import { View, FlatList, TextInput, TouchableOpacity, KeyboardAvoidingView, Platform } from 'react-native';
import { Message } from '../database';
import { MessageBubble } from '../components/MessageBubble';
import { WraithClient } from '../wraith';

interface ChatScreenProps {
  route: { params: { conversationId: number } };
}

export function ChatScreen({ route }: ChatScreenProps) {
  const { conversationId } = route.params;
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputText, setInputText] = useState('');
  const [sending, setSending] = useState(false);
  const flatListRef = useRef<FlatList>(null);
  const wraith = new WraithClient();

  useEffect(() => {
    loadMessages();
    subscribeToNewMessages();
  }, [conversationId]);

  const loadMessages = async () => {
    const msgs = await db.getConversationMessages(conversationId);
    setMessages(msgs.reverse()); // Oldest first
  };

  const subscribeToNewMessages = () => {
    wraith.onMessage((msg: Message) => {
      if (msg.conversationId === conversationId) {
        setMessages(prev => [...prev, msg]);
        flatListRef.current?.scrollToEnd();
      }
    });
  };

  const sendMessage = async () => {
    if (!inputText.trim() || sending) return;

    setSending(true);
    try {
      const message: Message = {
        conversationId,
        senderPeerId: wraith.localPeerId,
        contentType: 'text',
        body: inputText.trim(),
        timestamp: Date.now(),
        direction: 'outgoing',
      };

      // Encrypt and send via WRAITH protocol
      await wraith.sendMessage(conversationId, message);

      // Save to database
      await db.insertMessage(message);

      setMessages(prev => [...prev, message]);
      setInputText('');
      flatListRef.current?.scrollToEnd();
    } catch (error) {
      console.error('Failed to send message:', error);
    } finally {
      setSending(false);
    }
  };

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      keyboardVerticalOffset={90}
    >
      <FlatList
        ref={flatListRef}
        data={messages}
        renderItem={({ item }) => <MessageBubble message={item} />}
        keyExtractor={item => item.id!.toString()}
        onContentSizeChange={() => flatListRef.current?.scrollToEnd()}
      />

      <View style={styles.inputContainer}>
        <TextInput
          style={styles.input}
          value={inputText}
          onChangeText={setInputText}
          placeholder="Type a message..."
          multiline
          maxLength={10000}
        />
        <TouchableOpacity
          style={[styles.sendButton, !inputText.trim() && styles.sendButtonDisabled]}
          onPress={sendMessage}
          disabled={!inputText.trim() || sending}
        >
          <Text style={styles.sendButtonText}>Send</Text>
        </TouchableOpacity>
      </View>
    </KeyboardAvoidingView>
  );
}
```

**Message Bubble Component:**
```tsx
// src/components/MessageBubble.tsx
import React from 'react';
import { View, Text } from 'react-native';
import { Message } from '../database';

interface MessageBubbleProps {
  message: Message;
}

export function MessageBubble({ message }: MessageBubbleProps) {
  const isOutgoing = message.direction === 'outgoing';

  return (
    <View style={[styles.container, isOutgoing ? styles.outgoing : styles.incoming]}>
      <View style={[styles.bubble, isOutgoing ? styles.bubbleOutgoing : styles.bubbleIncoming]}>
        <Text style={styles.bodyText}>{message.body}</Text>
        <View style={styles.metaInfo}>
          <Text style={styles.timestamp}>{formatTime(message.timestamp)}</Text>
          {isOutgoing && (
            <Text style={styles.status}>
              {message.read ? '✓✓' : message.delivered ? '✓' : '○'}
            </Text>
          )}
        </View>
      </View>
    </View>
  );
}

function formatTime(ts: number): string {
  const date = new Date(ts);
  return `${date.getHours()}:${String(date.getMinutes()).padStart(2, '0')}`;
}

const styles = StyleSheet.create({
  container: {
    paddingHorizontal: 12,
    paddingVertical: 4,
  },
  outgoing: {
    alignItems: 'flex-end',
  },
  incoming: {
    alignItems: 'flex-start',
  },
  bubble: {
    maxWidth: '75%',
    padding: 12,
    borderRadius: 18,
  },
  bubbleOutgoing: {
    backgroundColor: '#0084FF',
  },
  bubbleIncoming: {
    backgroundColor: '#E4E6EB',
  },
  bodyText: {
    fontSize: 16,
    color: '#000',
  },
  metaInfo: {
    flexDirection: 'row',
    justifyContent: 'flex-end',
    marginTop: 4,
  },
  timestamp: {
    fontSize: 11,
    color: '#65676B',
  },
  status: {
    fontSize: 11,
    color: '#65676B',
    marginLeft: 4,
  },
});
```

---

### S1.4: Double Ratchet Encryption (13 points)

**Task:** Implement Signal's Double Ratchet algorithm for message encryption.

**Acceptance Criteria:**
- [ ] X3DH key agreement for initial key exchange
- [ ] Symmetric-key ratchet (KDF chains)
- [ ] Diffie-Hellman ratchet (ECDH on Curve25519)
- [ ] Out-of-order message handling
- [ ] Skipped message keys stored securely
- [ ] Forward secrecy and post-compromise security

**Double Ratchet Implementation:**
```typescript
// src/crypto/DoubleRatchet.ts
import nacl from 'tweetnacl';
import { hkdf } from '@noble/hashes/hkdf';
import { sha256 } from '@noble/hashes/sha256';

const HEADER_SIZE = 32 + 4; // DH public key + message number

export class DoubleRatchet {
  private rootKey: Uint8Array;
  private sendingChainKey: Uint8Array;
  private receivingChainKey: Uint8Array | null = null;
  private dhSendingKey: nacl.BoxKeyPair;
  private dhReceivingKey: Uint8Array | null = null;
  private sendingChainIndex = 0;
  private receivingChainIndex = 0;
  private skippedKeys = new Map<string, Uint8Array>(); // (dh_public || message_index) -> message_key

  constructor(sharedSecret: Uint8Array, remotePublicKey?: Uint8Array) {
    this.rootKey = hkdf(sha256, sharedSecret, undefined, undefined, 32);
    this.dhSendingKey = nacl.box.keyPair();

    if (remotePublicKey) {
      this.dhReceive(remotePublicKey);
    }
  }

  encrypt(plaintext: Uint8Array): { ciphertext: Uint8Array; header: Uint8Array } {
    // Derive message key from sending chain
    const messageKey = this.kdfChain(this.sendingChainKey);

    // Encrypt plaintext with message key
    const nonce = nacl.randomBytes(24);
    const ciphertext = nacl.secretbox(plaintext, nonce, messageKey);

    // Construct header: DH public key || message number
    const header = new Uint8Array(HEADER_SIZE);
    header.set(this.dhSendingKey.publicKey, 0);
    new DataView(header.buffer).setUint32(32, this.sendingChainIndex, false);

    this.sendingChainIndex++;

    return { ciphertext: new Uint8Array([...nonce, ...ciphertext]), header };
  }

  decrypt(header: Uint8Array, ciphertext: Uint8Array): Uint8Array {
    const dhPublicKey = header.slice(0, 32);
    const messageIndex = new DataView(header.buffer).getUint32(32, false);

    // Check if we need to perform DH ratchet step
    if (!this.dhReceivingKey || !this.arraysEqual(dhPublicKey, this.dhReceivingKey)) {
      this.dhReceive(dhPublicKey);
    }

    // Handle out-of-order messages (skip keys)
    if (messageIndex > this.receivingChainIndex) {
      this.skipMessageKeys(messageIndex);
    } else if (messageIndex < this.receivingChainIndex) {
      // Use skipped key
      const skippedKey = this.skippedKeys.get(this.keyId(dhPublicKey, messageIndex));
      if (!skippedKey) throw new Error('Skipped message key not found');

      return this.decryptWithKey(ciphertext, skippedKey);
    }

    // Derive message key from receiving chain
    const messageKey = this.kdfChain(this.receivingChainKey!);
    this.receivingChainIndex++;

    return this.decryptWithKey(ciphertext, messageKey);
  }

  private dhReceive(remotePublicKey: Uint8Array): void {
    // Save previous receiving chain for skipped keys
    if (this.dhReceivingKey && this.receivingChainKey) {
      // Store old chain for potential out-of-order messages
    }

    this.dhReceivingKey = remotePublicKey;

    // Perform DH and derive new root key and receiving chain key
    const dhOutput = nacl.scalarMult(this.dhSendingKey.secretKey, remotePublicKey);
    const [newRootKey, newReceivingChainKey] = this.kdfRatchet(this.rootKey, dhOutput);

    this.rootKey = newRootKey;
    this.receivingChainKey = newReceivingChainKey;
    this.receivingChainIndex = 0;

    // Generate new DH sending key and derive sending chain
    this.dhSendingKey = nacl.box.keyPair();
    const dhOutput2 = nacl.scalarMult(this.dhSendingKey.secretKey, remotePublicKey);
    const [newRootKey2, newSendingChainKey] = this.kdfRatchet(this.rootKey, dhOutput2);

    this.rootKey = newRootKey2;
    this.sendingChainKey = newSendingChainKey;
    this.sendingChainIndex = 0;
  }

  private kdfRatchet(rootKey: Uint8Array, dhOutput: Uint8Array): [Uint8Array, Uint8Array] {
    const output = hkdf(sha256, dhOutput, rootKey, new Uint8Array([0x01]), 64);
    return [output.slice(0, 32), output.slice(32)];
  }

  private kdfChain(chainKey: Uint8Array): Uint8Array {
    const output = hkdf(sha256, chainKey, undefined, new Uint8Array([0x02]), 64);
    this.sendingChainKey = output.slice(0, 32); // Update chain key
    return output.slice(32); // Return message key
  }

  private skipMessageKeys(untilIndex: number): void {
    if (!this.receivingChainKey) throw new Error('No receiving chain key');

    while (this.receivingChainIndex < untilIndex) {
      const messageKey = this.kdfChain(this.receivingChainKey);
      const keyId = this.keyId(this.dhReceivingKey!, this.receivingChainIndex);
      this.skippedKeys.set(keyId, messageKey);
      this.receivingChainIndex++;

      // Limit skipped keys storage to prevent DoS
      if (this.skippedKeys.size > 1000) {
        const firstKey = this.skippedKeys.keys().next().value;
        this.skippedKeys.delete(firstKey);
      }
    }
  }

  private decryptWithKey(ciphertext: Uint8Array, messageKey: Uint8Array): Uint8Array {
    const nonce = ciphertext.slice(0, 24);
    const encrypted = ciphertext.slice(24);

    const plaintext = nacl.secretbox.open(encrypted, nonce, messageKey);
    if (!plaintext) throw new Error('Decryption failed');

    return plaintext;
  }

  private keyId(dhPublicKey: Uint8Array, index: number): string {
    return `${Buffer.from(dhPublicKey).toString('hex')}_${index}`;
  }

  private arraysEqual(a: Uint8Array, b: Uint8Array): boolean {
    return a.length === b.length && a.every((val, idx) => val === b[idx]);
  }

  // Serialize state for storage
  serialize(): string {
    return JSON.stringify({
      rootKey: Buffer.from(this.rootKey).toString('hex'),
      sendingChainKey: Buffer.from(this.sendingChainKey).toString('hex'),
      receivingChainKey: this.receivingChainKey ? Buffer.from(this.receivingChainKey).toString('hex') : null,
      dhSendingSecretKey: Buffer.from(this.dhSendingKey.secretKey).toString('hex'),
      dhSendingPublicKey: Buffer.from(this.dhSendingKey.publicKey).toString('hex'),
      dhReceivingKey: this.dhReceivingKey ? Buffer.from(this.dhReceivingKey).toString('hex') : null,
      sendingChainIndex: this.sendingChainIndex,
      receivingChainIndex: this.receivingChainIndex,
      skippedKeys: Array.from(this.skippedKeys.entries()).map(([k, v]) => [k, Buffer.from(v).toString('hex')]),
    });
  }

  // Deserialize state from storage
  static deserialize(data: string): DoubleRatchet {
    const obj = JSON.parse(data);
    // Implementation omitted for brevity
    return new DoubleRatchet(Buffer.from(obj.rootKey, 'hex'));
  }
}
```

**Integration with WRAITH Client:**
```typescript
// src/wraith/WraithClient.ts
import { DoubleRatchet } from '../crypto/DoubleRatchet';
import { Database } from '../database';

export class WraithClient {
  private ratchets = new Map<string, DoubleRatchet>();
  private db: Database;

  async sendMessage(peerId: string, plaintext: string): Promise<void> {
    let ratchet = this.ratchets.get(peerId);

    if (!ratchet) {
      // Perform X3DH key agreement and initialize ratchet
      const sharedSecret = await this.performX3DH(peerId);
      ratchet = new DoubleRatchet(sharedSecret);
      this.ratchets.set(peerId, ratchet);
    }

    const { ciphertext, header } = ratchet.encrypt(Buffer.from(plaintext, 'utf8'));

    // Send via WRAITH protocol
    await this.wraithSend(peerId, header, ciphertext);

    // Save ratchet state
    await this.db.saveRatchetState(peerId, ratchet.serialize());
  }

  async receiveMessage(peerId: string, header: Uint8Array, ciphertext: Uint8Array): Promise<string> {
    let ratchet = this.ratchets.get(peerId);

    if (!ratchet) {
      // Load ratchet state from database
      const state = await this.db.loadRatchetState(peerId);
      ratchet = DoubleRatchet.deserialize(state);
      this.ratchets.set(peerId, ratchet);
    }

    const plaintext = ratchet.decrypt(header, ciphertext);

    // Save updated ratchet state
    await this.db.saveRatchetState(peerId, ratchet.serialize());

    return Buffer.from(plaintext).toString('utf8');
  }

  private async performX3DH(peerId: string): Promise<Uint8Array> {
    // X3DH key agreement implementation
    // Returns shared secret for Double Ratchet initialization
    throw new Error('Not implemented');
  }

  private async wraithSend(peerId: string, header: Uint8Array, ciphertext: Uint8Array): Promise<void> {
    // Send encrypted message via WRAITH protocol
    throw new Error('Not implemented');
  }
}
```

---

### S1.5-S1.8: Additional Tasks (18 points total)

- **S1.5:** Contact Management (5 pts) - Add/delete contacts, verify identity keys, block/unblock
- **S1.6:** Message Delivery Receipts (3 pts) - Sent/delivered/read status, acknowledgment messages
- **S1.7:** Typing Indicators (2 pts) - Real-time typing status, timeout after 5 seconds
- **S1.8:** Read Receipts (3 pts) - Mark messages as read, notify sender

---

## Sprint 2-4 Summary

**Sprint 2 (Weeks 41-44):** Group messaging, media attachments, voice messages
**Sprint 3 (Weeks 45-48):** WebRTC voice/video calls
**Sprint 4 (Weeks 49-50):** Push notifications, disappearing messages, app stores

---

## Completion Checklist

- [ ] All platforms build successfully
- [ ] End-to-end encryption verified (security audit)
- [ ] 1:1 messaging functional
- [ ] Group messaging functional
- [ ] Voice/video calls working
- [ ] Push notifications delivering
- [ ] App submitted to iOS App Store
- [ ] App submitted to Google Play Store
- [ ] Desktop installers published

**Target Release Date:** Week 50 (14 weeks from protocol completion)

---

*WRAITH-Chat Sprint Planning v1.0.0*
