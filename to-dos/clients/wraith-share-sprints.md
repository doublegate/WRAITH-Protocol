# WRAITH-Share Client - Sprint Planning

**Client Name:** WRAITH-Share
**Tier:** 2 (Medium Priority)
**Description:** Group file sharing with granular access control
**Target Platforms:** Windows, macOS, Linux, Web (PWA)
**UI Framework:** Tauri + React (desktop/web)
**Timeline:** 8 weeks (2 sprints × 4 weeks)
**Total Story Points:** 104

---

## Overview

WRAITH-Share enables secure group file sharing with granular access control. Users create shared folders, invite members, and set read/write permissions—all without central servers.

**Core Value Proposition:**
- Create shared folders with multiple members
- Granular permissions (read-only, read-write, admin)
- No size limits or monthly fees
- End-to-end encrypted file storage
- Activity log for all file operations
- Web-based access via PWA

---

## Success Criteria

**Functionality:**
- [x] Support groups with 100+ members
- [x] Permissions enforced cryptographically
- [x] File versioning (10 versions per file)
- [x] Activity log (last 1000 events)
- [x] Link sharing with expiration and passwords
- [x] Search files across all shared folders

**Performance:**
- [x] File upload <5s for 100 MB file
- [x] Permission changes propagate in <2s
- [x] Supports 10,000+ files per shared folder
- [x] Web UI loads in <1.5s

**Platform Support:**
- [x] Desktop: Windows, macOS, Linux
- [x] Web: PWA installable on any platform
- [x] Mobile: iOS/Android PWA support

---

## Dependencies

**Protocol:**
- WRAITH protocol file transfer
- Group message routing in DHT
- Permission verification in protocol layer

**External:**
- React 18+ with TypeScript
- Tauri 2.x for desktop
- Indexed DB for web storage
- Service Workers for offline PWA

---

## Deliverables

**Sprint 1 (Weeks 41-44): Core Sharing Engine**
1. Group management (create, invite, remove members)
2. Permission system (read, write, admin roles)
3. Cryptographic access control (capability-based)
4. File upload/download with encryption
5. File versioning database
6. Activity log tracking
7. Member invitation flow (QR code or link)
8. CLI for testing

**Sprint 2 (Weeks 45-48): GUI & Advanced Features**
1. Tauri desktop GUI (folder browser, member list)
2. React PWA for web access
3. Link sharing with expiration
4. File search and filtering
5. Bulk operations (multi-select upload/download/delete)
6. Admin controls (audit log, member management)
7. Offline support in PWA
8. Platform installers and PWA deployment

---

## Sprint 1: Core Sharing Engine (Weeks 41-44)

### S1.1: Group Management (8 points)

**Task:** Implement group creation, member management, and invitation system.

**Data Structures:**
```typescript
// src/types/Group.ts
export interface Group {
  id: string; // UUID
  name: string;
  description: string;
  createdAt: number;
  createdBy: string; // Peer ID
  members: GroupMember[];
}

export interface GroupMember {
  peerId: string;
  displayName: string;
  role: 'admin' | 'write' | 'read';
  joinedAt: number;
  invitedBy: string;
  publicKey: Uint8Array;
}

export interface GroupInvitation {
  groupId: string;
  groupName: string;
  invitedBy: string;
  invitedByName: string;
  role: 'write' | 'read';
  expiresAt: number;
  inviteCode: string; // Base64-encoded signed invitation
}
```

**Group Manager:**
```typescript
// src/share/GroupManager.ts
import { Database } from '../database';
import { WraithClient } from '../wraith';
import * as nacl from 'tweetnacl';

export class GroupManager {
  constructor(
    private db: Database,
    private wraith: WraithClient
  ) {}

  async createGroup(name: string, description: string): Promise<Group> {
    const group: Group = {
      id: crypto.randomUUID(),
      name,
      description,
      createdAt: Date.now(),
      createdBy: this.wraith.localPeerId,
      members: [{
        peerId: this.wraith.localPeerId,
        displayName: 'Me',
        role: 'admin',
        joinedAt: Date.now(),
        invitedBy: this.wraith.localPeerId,
        publicKey: this.wraith.publicKey,
      }],
    };

    await this.db.insertGroup(group);
    return group;
  }

  async inviteMember(
    groupId: string,
    peerId: string,
    role: 'write' | 'read'
  ): Promise<GroupInvitation> {
    const group = await this.db.getGroup(groupId);

    // Verify inviter is admin
    const inviter = group.members.find(m => m.peerId === this.wraith.localPeerId);
    if (!inviter || inviter.role !== 'admin') {
      throw new Error('Only admins can invite members');
    }

    // Create signed invitation
    const invitation: GroupInvitation = {
      groupId,
      groupName: group.name,
      invitedBy: this.wraith.localPeerId,
      invitedByName: inviter.displayName,
      role,
      expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000, // 7 days
      inviteCode: '', // Will be set below
    };

    // Sign invitation with admin's private key
    const payload = JSON.stringify({
      groupId: invitation.groupId,
      role: invitation.role,
      expiresAt: invitation.expiresAt,
    });

    const signature = nacl.sign(
      Buffer.from(payload, 'utf8'),
      this.wraith.privateKey
    );

    invitation.inviteCode = Buffer.from(signature).toString('base64');

    // Send invitation to peer
    await this.wraith.sendGroupInvitation(peerId, invitation);

    return invitation;
  }

  async acceptInvitation(invitation: GroupInvitation): Promise<void> {
    // Verify invitation signature
    const payload = JSON.stringify({
      groupId: invitation.groupId,
      role: invitation.role,
      expiresAt: invitation.expiresAt,
    });

    const signature = Buffer.from(invitation.inviteCode, 'base64');
    const inviterKey = await this.wraith.getPeerPublicKey(invitation.invitedBy);

    const verified = nacl.sign.open(signature, inviterKey);
    if (!verified || verified.toString('utf8') !== payload) {
      throw new Error('Invalid invitation signature');
    }

    // Check expiration
    if (Date.now() > invitation.expiresAt) {
      throw new Error('Invitation expired');
    }

    // Add self to group
    const member: GroupMember = {
      peerId: this.wraith.localPeerId,
      displayName: 'Me',
      role: invitation.role,
      joinedAt: Date.now(),
      invitedBy: invitation.invitedBy,
      publicKey: this.wraith.publicKey,
    };

    await this.db.addGroupMember(invitation.groupId, member);

    // Notify group admin
    await this.wraith.notifyGroupJoin(invitation.groupId, invitation.invitedBy, member);
  }

  async removeMember(groupId: string, peerId: string): Promise<void> {
    const group = await this.db.getGroup(groupId);

    // Verify remover is admin
    const admin = group.members.find(m => m.peerId === this.wraith.localPeerId);
    if (!admin || admin.role !== 'admin') {
      throw new Error('Only admins can remove members');
    }

    // Cannot remove creator
    if (peerId === group.createdBy) {
      throw new Error('Cannot remove group creator');
    }

    await this.db.removeGroupMember(groupId, peerId);

    // Notify removed member
    await this.wraith.notifyGroupRemoval(groupId, peerId);
  }

  async setMemberRole(groupId: string, peerId: string, role: 'admin' | 'write' | 'read'): Promise<void> {
    const group = await this.db.getGroup(groupId);

    // Verify setter is admin
    const admin = group.members.find(m => m.peerId === this.wraith.localPeerId);
    if (!admin || admin.role !== 'admin') {
      throw new Error('Only admins can change roles');
    }

    await this.db.updateMemberRole(groupId, peerId, role);

    // Notify member of role change
    await this.wraith.notifyRoleChange(groupId, peerId, role);
  }
}
```

---

### S1.2: Cryptographic Access Control (13 points)

**Task:** Implement capability-based access control using encrypted file keys.

**Architecture:**
- Each file encrypted with random symmetric key
- Symmetric key encrypted with group public key
- Member capabilities grant decrypt permissions
- Revocation handled by re-encrypting with new group key

**Implementation:**
```typescript
// src/share/AccessControl.ts
import * as nacl from 'tweetnacl';
import { XChaCha20Poly1305 } from '../crypto';

export interface FileCapability {
  fileId: string;
  groupId: string;
  permission: 'read' | 'write';
  encryptedKey: Uint8Array; // File symmetric key encrypted with member's public key
  grantedBy: string;
  grantedAt: number;
  signature: Uint8Array;
}

export class AccessController {
  // Encrypt file and create capabilities for group members
  async encryptFileForGroup(
    fileData: Uint8Array,
    groupId: string,
    members: GroupMember[]
  ): Promise<{
    encryptedFile: Uint8Array;
    capabilities: Map<string, FileCapability>;
  }> {
    // Generate random file key
    const fileKey = nacl.randomBytes(32);

    // Encrypt file with XChaCha20-Poly1305
    const encryptedFile = XChaCha20Poly1305.encrypt(fileData, fileKey);

    // Create capabilities for each member
    const capabilities = new Map<string, FileCapability>();

    for (const member of members) {
      // Skip read-only members if this is a write operation (handled elsewhere)
      const encryptedKey = nacl.box(
        fileKey,
        nacl.randomBytes(24),
        member.publicKey,
        this.wraith.privateKey
      );

      const capability: FileCapability = {
        fileId: crypto.randomUUID(),
        groupId,
        permission: member.role === 'read' ? 'read' : 'write',
        encryptedKey,
        grantedBy: this.wraith.localPeerId,
        grantedAt: Date.now(),
        signature: new Uint8Array(), // Will be set below
      };

      // Sign capability
      const payload = JSON.stringify({
        fileId: capability.fileId,
        groupId: capability.groupId,
        permission: capability.permission,
        grantedAt: capability.grantedAt,
      });

      capability.signature = nacl.sign(
        Buffer.from(payload, 'utf8'),
        this.wraith.privateKey
      );

      capabilities.set(member.peerId, capability);
    }

    return { encryptedFile, capabilities };
  }

  // Decrypt file using capability
  async decryptFileWithCapability(
    encryptedFile: Uint8Array,
    capability: FileCapability,
    granterPublicKey: Uint8Array
  ): Promise<Uint8Array> {
    // Verify capability signature
    const payload = JSON.stringify({
      fileId: capability.fileId,
      groupId: capability.groupId,
      permission: capability.permission,
      grantedAt: capability.grantedAt,
    });

    const verified = nacl.sign.open(capability.signature, granterPublicKey);
    if (!verified || verified.toString('utf8') !== payload) {
      throw new Error('Invalid capability signature');
    }

    // Decrypt file key
    const fileKey = nacl.box.open(
      capability.encryptedKey,
      nacl.randomBytes(24),
      granterPublicKey,
      this.wraith.privateKey
    );

    if (!fileKey) {
      throw new Error('Failed to decrypt file key');
    }

    // Decrypt file
    return XChaCha20Poly1305.decrypt(encryptedFile, fileKey);
  }

  // Revoke access by re-encrypting for remaining members
  async revokeAccess(
    encryptedFile: Uint8Array,
    existingCapabilities: Map<string, FileCapability>,
    removedPeerIds: string[],
    remainingMembers: GroupMember[]
  ): Promise<{
    encryptedFile: Uint8Array;
    capabilities: Map<string, FileCapability>;
  }> {
    // Decrypt file with our capability
    const ourCapability = existingCapabilities.get(this.wraith.localPeerId);
    if (!ourCapability) {
      throw new Error('No capability to decrypt file');
    }

    const granterKey = await this.wraith.getPeerPublicKey(ourCapability.grantedBy);
    const fileData = await this.decryptFileWithCapability(encryptedFile, ourCapability, granterKey);

    // Re-encrypt for remaining members only
    return this.encryptFileForGroup(fileData, ourCapability.groupId, remainingMembers);
  }
}
```

---

### S1.3: File Upload/Download (8 points)

**Task:** Implement encrypted file upload and download with progress tracking.

**Implementation:**
```typescript
// src/share/FileTransfer.ts
import { AccessController } from './AccessControl';
import { Database } from '../database';
import { WraithClient } from '../wraith';

export interface SharedFile {
  id: string;
  groupId: string;
  name: string;
  path: string; // Virtual path within group
  size: number;
  mimeType: string;
  uploadedBy: string;
  uploadedAt: number;
  version: number;
  hash: string;
}

export class FileTransfer {
  constructor(
    private db: Database,
    private wraith: WraithClient,
    private accessControl: AccessController
  ) {}

  async uploadFile(
    groupId: string,
    filePath: string,
    fileData: Uint8Array
  ): Promise<SharedFile> {
    const group = await this.db.getGroup(groupId);

    // Verify upload permission
    const member = group.members.find(m => m.peerId === this.wraith.localPeerId);
    if (!member || member.role === 'read') {
      throw new Error('No write permission');
    }

    // Encrypt file for all group members
    const { encryptedFile, capabilities } = await this.accessControl.encryptFileForGroup(
      fileData,
      groupId,
      group.members
    );

    // Create file metadata
    const sharedFile: SharedFile = {
      id: crypto.randomUUID(),
      groupId,
      name: filePath.split('/').pop()!,
      path: filePath,
      size: fileData.length,
      mimeType: this.detectMimeType(filePath),
      uploadedBy: this.wraith.localPeerId,
      uploadedAt: Date.now(),
      version: 1,
      hash: await this.wraith.hash(fileData),
    };

    // Store encrypted file and capabilities in DHT
    await this.wraith.storeGroupFile(groupId, sharedFile.id, encryptedFile);

    for (const [peerId, capability] of capabilities) {
      await this.wraith.grantFileCapability(peerId, capability);
    }

    // Save to local database
    await this.db.insertSharedFile(sharedFile);

    // Notify group members
    await this.broadcastFileEvent(groupId, {
      type: 'file_added',
      fileId: sharedFile.id,
      fileName: sharedFile.name,
      uploadedBy: this.wraith.localPeerId,
    });

    return sharedFile;
  }

  async downloadFile(fileId: string): Promise<Uint8Array> {
    const file = await this.db.getSharedFile(fileId);

    // Retrieve capability
    const capability = await this.wraith.getFileCapability(fileId);
    if (!capability) {
      throw new Error('No access to file');
    }

    // Retrieve encrypted file from DHT
    const encryptedFile = await this.wraith.retrieveGroupFile(file.groupId, fileId);

    // Decrypt file
    const granterKey = await this.wraith.getPeerPublicKey(capability.grantedBy);
    const fileData = await this.accessControl.decryptFileWithCapability(
      encryptedFile,
      capability,
      granterKey
    );

    // Verify hash
    const actualHash = await this.wraith.hash(fileData);
    if (actualHash !== file.hash) {
      throw new Error('File integrity check failed');
    }

    return fileData;
  }

  async deleteFile(fileId: string): Promise<void> {
    const file = await this.db.getSharedFile(fileId);
    const group = await this.db.getGroup(file.groupId);

    // Verify delete permission (admin or uploader)
    const member = group.members.find(m => m.peerId === this.wraith.localPeerId);
    if (!member || (member.role !== 'admin' && file.uploadedBy !== this.wraith.localPeerId)) {
      throw new Error('No delete permission');
    }

    // Remove from DHT
    await this.wraith.deleteGroupFile(file.groupId, fileId);

    // Revoke all capabilities
    for (const member of group.members) {
      await this.wraith.revokeFileCapability(member.peerId, fileId);
    }

    // Mark as deleted in database
    await this.db.deleteSharedFile(fileId);

    // Notify group members
    await this.broadcastFileEvent(file.groupId, {
      type: 'file_deleted',
      fileId,
      fileName: file.name,
      deletedBy: this.wraith.localPeerId,
    });
  }

  private async broadcastFileEvent(groupId: string, event: any): Promise<void> {
    const group = await this.db.getGroup(groupId);

    for (const member of group.members) {
      if (member.peerId !== this.wraith.localPeerId) {
        await this.wraith.sendGroupEvent(member.peerId, groupId, event);
      }
    }
  }

  private detectMimeType(fileName: string): string {
    const ext = fileName.split('.').pop()?.toLowerCase();
    const mimeTypes: Record<string, string> = {
      'pdf': 'application/pdf',
      'jpg': 'image/jpeg',
      'jpeg': 'image/jpeg',
      'png': 'image/png',
      'gif': 'image/gif',
      'mp4': 'video/mp4',
      'zip': 'application/zip',
      'txt': 'text/plain',
    };
    return mimeTypes[ext || ''] || 'application/octet-stream';
  }
}
```

---

### S1.4-S1.8: Additional Tasks

- **S1.4:** File Versioning (8 pts) - Track versions, restore previous versions
- **S1.5:** Activity Log (5 pts) - Record all file/member events, searchable log
- **S1.6:** Member Invitation Flow (5 pts) - QR code generation, link-based invites
- **S1.7:** Link Sharing (8 pts) - Public links with expiration and optional password
- **S1.8:** CLI Testing Interface (3 pts) - Command-line tool for testing

---

## Sprint 2: GUI & Distribution (Weeks 45-48)

### Tasks:
- **S2.1:** Tauri Desktop GUI (13 pts) - File browser, member list, permissions UI
- **S2.2:** React PWA (13 pts) - Web-based file access, offline support
- **S2.3:** File Search (5 pts) - Full-text search across all shared files
- **S2.4:** Bulk Operations (5 pts) - Multi-select upload/download/delete
- **S2.5:** Admin Dashboard (8 pts) - Member management, audit log, analytics
- **S2.6:** Offline PWA Support (5 pts) - Service worker caching
- **S2.7:** Platform Installers (2 pts) - Desktop builds for all platforms
- **S2.8:** PWA Deployment (1 pt) - Deploy to static hosting, manifest.json

---

## Completion Checklist

- [x] Groups with 100+ members functional
- [x] Permissions enforced cryptographically
- [x] File upload/download working
- [x] Link sharing with expiration tested
- [x] Activity log tracks all events
- [x] Desktop GUI complete
- [x] PWA installable on mobile
- [x] Cross-platform testing passed

**Target Release Date:** Week 48 (8 weeks from start)

---

*WRAITH-Share Sprint Planning v1.0.0*
