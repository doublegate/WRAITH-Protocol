# WRAITH Share

Group file sharing with granular access control using the WRAITH protocol.

## Overview

WRAITH Share is a secure file sharing application that provides:

- **Group Management**: Create groups, invite members, manage roles (admin/editor/viewer)
- **Cryptographic Access Control**: Capability-based encryption with per-file symmetric keys
- **File Versioning**: Up to 10 versions per file with restore capability
- **Activity Logging**: Track all group activities (last 1000 events)
- **Link Sharing**: Share files via links with optional password protection and expiration

## Architecture

### Backend (Rust/Tauri)

The backend is built with Tauri 2.x and provides:

- **SQLite Database**: Local storage for groups, files, members, and activities
- **Ed25519 Signatures**: Authentication and invitation signing
- **X25519 Key Exchange**: Per-member encryption key derivation
- **XChaCha20-Poly1305**: File content encryption
- **BLAKE3 Hashing**: File integrity verification
- **Argon2 Password Hashing**: Share link protection

### Modules

| Module | Description |
|--------|-------------|
| `database.rs` | SQLite schema and CRUD operations |
| `state.rs` | Application state and identity management |
| `group.rs` | Group creation, membership, invitations |
| `access_control.rs` | Capability-based encryption system |
| `file_transfer.rs` | File upload/download with encryption |
| `versioning.rs` | File version management |
| `activity.rs` | Activity logging and search |
| `link_share.rs` | Public share link management |
| `commands.rs` | Tauri IPC command handlers |

## Security Model

### Capability-Based Access Control

Each file is encrypted with a unique symmetric key (file capability). This key is then encrypted for each group member using X25519 key exchange:

1. File is encrypted with random XChaCha20-Poly1305 key
2. For each member, derive shared secret via X25519 (member's public key + ephemeral private key)
3. Encrypt file key with derived shared secret
4. Store encrypted key exchange for each member

### Role-Based Permissions

| Role | Permissions |
|------|-------------|
| Admin | Full access, manage members, delete group |
| Editor | Upload, download, delete files |
| Viewer | Download files only |

### Share Links

- Optional password protection (Argon2 hashed)
- Configurable expiration time
- Maximum download limits
- Revokable at any time

## Development

### Prerequisites

- Rust 1.88+ (2024 Edition)
- Node.js 18+
- Tauri CLI 2.x

### Build

```bash
# Install dependencies
npm install

# Development mode
npm run tauri:dev

# Production build
npm run tauri:build
```

### Testing

```bash
cd src-tauri
cargo test
cargo clippy -- -D warnings
```

## Tauri Commands

### Group Commands
- `create_group(name, description?)` - Create a new group
- `delete_group(group_id)` - Delete a group
- `get_group(group_id)` - Get group by ID
- `list_groups()` - List all groups
- `invite_member(group_id, peer_id?, role)` - Invite a member
- `accept_invitation(invitation)` - Accept an invitation
- `remove_member(group_id, peer_id)` - Remove a member
- `set_member_role(group_id, peer_id, role)` - Update member role
- `list_members(group_id)` - List group members
- `get_group_info(group_id)` - Get extended group info

### File Commands
- `upload_file(group_id, path, data)` - Upload a file
- `download_file(file_id)` - Download a file
- `delete_file(file_id)` - Delete a file
- `list_files(group_id)` - List files in a group
- `search_files(group_id, query)` - Search files

### Version Commands
- `get_file_versions(file_id)` - Get all versions
- `restore_version(file_id, version)` - Restore a version
- `get_version_summary(file_id)` - Get version summary

### Activity Commands
- `get_activity_log(group_id, limit, offset)` - Get activity log
- `get_recent_activity(limit)` - Get recent activity
- `search_activity(group_id, query, limit)` - Search activity
- `get_activity_stats(group_id)` - Get activity statistics

### Link Sharing Commands
- `create_share_link(file_id, expires_in_hours?, password?, max_downloads?)` - Create share link
- `get_share_link(link_id)` - Get share link info
- `revoke_share_link(link_id)` - Revoke a share link
- `download_via_link(link_id, password?)` - Download via share link
- `list_file_share_links(file_id)` - List links for a file
- `link_requires_password(link_id)` - Check if password required

### Identity Commands
- `get_peer_id()` - Get local peer ID
- `get_display_name()` - Get display name
- `set_display_name(name)` - Set display name

## License

MIT License - See LICENSE file for details.
