# WRAITH-Stream Client - Sprint Planning

**Client Name:** WRAITH-Stream
**Tier:** 3 (Lower Priority)
**Description:** Encrypted peer-to-peer media streaming
**Target Platforms:** Windows, macOS, Linux, Web, Mobile
**UI Framework:** Electron + React (desktop), React (web/mobile)
**Timeline:** 6 weeks (1.5 sprints × 4 weeks)
**Total Story Points:** 78

---

## Overview

WRAITH-Stream enables encrypted streaming of video, audio, and live content over the WRAITH protocol. Think "private YouTube" where content is distributed peer-to-peer without centralized servers.

**Core Value Proposition:**
- Stream encrypted video/audio to peers
- No content monitoring or censorship
- Adaptive bitrate streaming (240p - 4K)
- Live streaming support
- Decentralized content distribution (seeders)

---

## Success Criteria

**Performance:**
- [ ] 1080p playback at 5 Mbps
- [ ] Sub-3-second startup latency
- [ ] Adaptive bitrate switches in <500ms
- [ ] Supports 100+ concurrent viewers per stream

**Features:**
- [ ] On-demand video streaming (HLS/DASH)
- [ ] Live streaming with <5s latency
- [ ] Multi-quality transcoding (240p/480p/720p/1080p/4K)
- [ ] Subtitle support (SRT, VTT)
- [ ] Seek to any position in <1s

---

## Sprint 1: Core Streaming (Weeks 45-48)

### S1.1: Video Transcoding Pipeline (13 points)

**Task:** Implement FFmpeg-based transcoding to multiple bitrates.

**Implementation:**
```typescript
// src/transcoder/VideoTranscoder.ts
import ffmpeg from 'fluent-ffmpeg';
import path from 'path';

export interface TranscodeProfile {
  name: string;
  width: number;
  height: number;
  bitrate: string;
  audioBitrate: string;
}

const PROFILES: TranscodeProfile[] = [
  { name: '240p', width: 426, height: 240, bitrate: '400k', audioBitrate: '64k' },
  { name: '480p', width: 854, height: 480, bitrate: '1000k', audioBitrate: '96k' },
  { name: '720p', width: 1280, height: 720, bitrate: '2500k', audioBitrate: '128k' },
  { name: '1080p', width: 1920, height: 1080, bitrate: '5000k', audioBitrate: '192k' },
  { name: '4k', width: 3840, height: 2160, bitrate: '15000k', audioBitrate: '256k' },
];

export class VideoTranscoder {
  async transcode(inputPath: string, outputDir: string): Promise<string[]> {
    const outputs: string[] = [];

    // Create HLS master playlist
    const masterPlaylist = path.join(outputDir, 'master.m3u8');
    let masterContent = '#EXTM3U\n#EXT-X-VERSION:3\n';

    for (const profile of PROFILES) {
      const outputPath = path.join(outputDir, `${profile.name}.m3u8`);

      await new Promise<void>((resolve, reject) => {
        ffmpeg(inputPath)
          .outputOptions([
            '-c:v libx264',
            '-c:a aac',
            `-b:v ${profile.bitrate}`,
            `-b:a ${profile.audioBitrate}`,
            `-s ${profile.width}x${profile.height}`,
            '-hls_time 6',
            '-hls_playlist_type vod',
            `-hls_segment_filename ${path.join(outputDir, `${profile.name}_%03d.ts`)}`,
            '-f hls',
          ])
          .output(outputPath)
          .on('end', () => resolve())
          .on('error', reject)
          .run();
      });

      masterContent += `#EXT-X-STREAM-INF:BANDWIDTH=${parseInt(profile.bitrate) * 1000},RESOLUTION=${profile.width}x${profile.height}\n`;
      masterContent += `${profile.name}.m3u8\n`;

      outputs.push(outputPath);
    }

    // Write master playlist
    await fs.promises.writeFile(masterPlaylist, masterContent);

    return [masterPlaylist, ...outputs];
  }

  async transcodeLive(rtmpUrl: string, outputDir: string): Promise<void> {
    // Live transcoding with FFmpeg
    ffmpeg(rtmpUrl)
      .inputOptions(['-re'])
      .outputOptions([
        '-c:v libx264',
        '-preset veryfast',
        '-tune zerolatency',
        '-b:v 2500k',
        '-maxrate 2500k',
        '-bufsize 5000k',
        '-c:a aac',
        '-b:a 128k',
        '-hls_time 2',
        '-hls_list_size 10',
        '-hls_flags delete_segments',
        '-f hls',
      ])
      .output(path.join(outputDir, 'live.m3u8'))
      .run();
  }
}
```

---

### S1.2: Encrypted Segment Storage (13 points)

**Task:** Encrypt HLS segments and distribute via WRAITH protocol.

**Implementation:**
```typescript
// src/stream/SegmentStorage.ts
import { XChaCha20Poly1305 } from '../crypto';
import { WraithClient } from '../wraith';
import * as fs from 'fs/promises';
import * as path from 'path';

export class SegmentStorage {
  constructor(private wraith: WraithClient) {}

  async uploadStream(streamId: string, hlsDir: string): Promise<void> {
    const segments = await fs.readdir(hlsDir);

    for (const segment of segments) {
      if (!segment.endsWith('.ts')) continue;

      const segmentPath = path.join(hlsDir, segment);
      const data = await fs.readFile(segmentPath);

      // Encrypt segment
      const key = this.deriveSegmentKey(streamId, segment);
      const encrypted = XChaCha20Poly1305.encrypt(data, key);

      // Store in DHT with content addressing
      const hash = await this.wraith.hash(encrypted);
      await this.wraith.dhtStore(`stream:${streamId}:${segment}`, encrypted);

      console.log(`Uploaded segment ${segment} (${encrypted.length} bytes)`);
    }

    // Upload playlist files
    for (const file of segments) {
      if (file.endsWith('.m3u8')) {
        const playlistPath = path.join(hlsDir, file);
        const playlist = await fs.readFile(playlistPath, 'utf8');

        // Encrypt playlist
        const key = this.deriveSegmentKey(streamId, file);
        const encrypted = XChaCha20Poly1305.encrypt(
          Buffer.from(playlist, 'utf8'),
          key
        );

        await this.wraith.dhtStore(`stream:${streamId}:${file}`, encrypted);
      }
    }
  }

  async downloadSegment(streamId: string, segmentName: string): Promise<Buffer> {
    const encrypted = await this.wraith.dhtRetrieve(`stream:${streamId}:${segmentName}`);

    const key = this.deriveSegmentKey(streamId, segmentName);
    const decrypted = XChaCha20Poly1305.decrypt(encrypted, key);

    return Buffer.from(decrypted);
  }

  private deriveSegmentKey(streamId: string, segmentName: string): Uint8Array {
    const input = `${streamId}:${segmentName}`;
    return this.wraith.kdf(input, 32);
  }
}
```

---

### S1.3: Video Player UI (13 points)

**Task:** Build video player with HLS support and adaptive bitrate.

**Implementation:**
```tsx
// src/components/VideoPlayer.tsx
import React, { useRef, useEffect } from 'react';
import Hls from 'hls.js';

interface VideoPlayerProps {
  streamId: string;
  onError?: (error: Error) => void;
}

export function VideoPlayer({ streamId, onError }: VideoPlayerProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const hlsRef = useRef<Hls | null>(null);

  useEffect(() => {
    if (!videoRef.current) return;

    if (Hls.isSupported()) {
      const hls = new Hls({
        xhrSetup: (xhr, url) => {
          // Intercept segment requests and fetch from WRAITH
          xhr.addEventListener('load', async function() {
            if (xhr.status === 200) {
              const segmentName = url.split('/').pop()!;
              const data = await downloadSegment(streamId, segmentName);
              // Replace response with decrypted data
              Object.defineProperty(this, 'response', {
                writable: false,
                value: data.buffer
              });
            }
          });
        },
      });

      hls.loadSource(`wraith://stream/${streamId}/master.m3u8`);
      hls.attachMedia(videoRef.current);

      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        videoRef.current?.play();
      });

      hls.on(Hls.Events.ERROR, (event, data) => {
        if (data.fatal) {
          onError?.(new Error(`HLS error: ${data.type}`));
        }
      });

      hlsRef.current = hls;
    } else if (videoRef.current.canPlayType('application/vnd.apple.mpegurl')) {
      // Native HLS support (Safari)
      videoRef.current.src = `wraith://stream/${streamId}/master.m3u8`;
    }

    return () => {
      hlsRef.current?.destroy();
    };
  }, [streamId]);

  return (
    <div className="video-player">
      <video
        ref={videoRef}
        controls
        style={{ width: '100%', maxHeight: '80vh' }}
      />
    </div>
  );
}

async function downloadSegment(streamId: string, segmentName: string): Promise<Buffer> {
  // Call WRAITH client to download encrypted segment
  const { invoke } = await import('@tauri-apps/api/tauri');
  return invoke('download_segment', { streamId, segmentName });
}
```

---

### Additional Sprint 1 Tasks:

- **S1.4:** Live Streaming (13 pts) - RTMP ingest, real-time transcoding
- **S1.5:** Subtitle Support (5 pts) - SRT/VTT parsing and overlay
- **S1.6:** Seek Optimization (5 pts) - Keyframe indexing for instant seek
- **S1.7:** Content Discovery (8 pts) - Browse streams, search, categories
- **S1.8:** Viewer Analytics (5 pts) - View count, watch time tracking

---

## Sprint 2: Polish & Distribution (Weeks 49-50)

### Tasks:
- Live streaming UI (start/stop broadcast)
- Chat overlay for live streams
- Quality selector in player
- Offline viewing (download for later)
- Creator dashboard (upload, analytics)
- Desktop/web builds

---

## Completion Checklist

- [ ] Video transcoding works for all profiles
- [ ] HLS playback smooth on all platforms
- [ ] Live streaming <5s latency
- [ ] Adaptive bitrate switching functional
- [ ] Content discovery working
- [ ] Desktop/web apps published

**Target Release Date:** Week 50

---

*WRAITH-Stream Sprint Planning v1.0.0*
