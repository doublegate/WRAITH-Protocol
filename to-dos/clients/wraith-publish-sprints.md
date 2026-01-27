# WRAITH-Publish Client - Sprint Planning

**Client Name:** WRAITH-Publish
**Tier:** 3 (Lower Priority)
**Description:** Decentralized content publishing platform
**Target Platforms:** Web (PWA), Desktop (Electron)
**UI Framework:** Next.js + React
**Timeline:** 6 weeks (1.5 sprints)
**Total Story Points:** 78

---

## Overview

WRAITH-Publish is a censorship-resistant publishing platform for blogs, articles, and long-form content. Think "Medium + IPFS" where content is distributed peer-to-peer and cannot be taken down.

**Core Value Proposition:**
- Publish articles without centralized hosting
- Content addressed by cryptographic hash
- No platform can censor or remove content
- Built-in monetization (optional reader payments)
- Beautiful reading experience

---

## Success Criteria

**Publishing:**
- [x] Markdown editor with live preview
- [x] Image/media upload and embedding
- [x] Draft saving and versioning
- [x] Publish to DHT in <5 seconds
- [x] Custom domain mapping (via DNS TXT records)

**Reading:**
- [x] Fast article loading (<2s)
- [x] Offline reading support
- [x] Full-text search across all publications
- [x] RSS feed generation
- [x] Reader comments (decentralized)

---

## Sprint 1: Editor & Publishing (Weeks 49-52)

### S1.1: Markdown Editor (13 points)

**Task:** Build rich markdown editor with live preview and XSS protection.

**Implementation:**
```tsx
// src/components/MarkdownEditor.tsx
import React, { useState } from 'react';
import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkRehype from 'remark-rehype';
import rehypeStringify from 'rehype-stringify';
import rehypeSanitize from 'rehype-sanitize';
import DOMPurify from 'dompurify';

interface EditorProps {
  initialContent?: string;
  onSave?: (content: string) => void;
}

export function MarkdownEditor({ initialContent = '', onSave }: EditorProps) {
  const [markdown, setMarkdown] = useState(initialContent);
  const [html, setHtml] = useState('');

  const updatePreview = async (md: string) => {
    const result = await unified()
      .use(remarkParse)
      .use(remarkRehype)
      .use(rehypeSanitize) // Server-side sanitization
      .use(rehypeStringify)
      .process(md);

    // Additional client-side sanitization with DOMPurify
    const sanitized = DOMPurify.sanitize(String(result), {
      ALLOWED_TAGS: ['p', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote',
                     'code', 'pre', 'strong', 'em', 'ul', 'ol', 'li', 'a', 'img'],
      ALLOWED_ATTR: ['href', 'src', 'alt', 'title', 'class'],
      ALLOW_DATA_ATTR: false,
    });

    setHtml(sanitized);
  };

  const handleChange = (value: string) => {
    setMarkdown(value);
    updatePreview(value);
  };

  return (
    <div className="editor-container">
      <div className="editor-pane">
        <textarea
          value={markdown}
          onChange={e => handleChange(e.target.value)}
          placeholder="Write your article in Markdown..."
          className="markdown-input"
        />
      </div>

      <div className="preview-pane">
        <div
          className="markdown-preview"
          dangerouslySetInnerHTML={{ __html: html }}
        />
      </div>

      <div className="editor-toolbar">
        <button onClick={() => onSave?.(markdown)}>Save Draft</button>
        <button onClick={() => publishArticle(markdown)}>Publish</button>
      </div>
    </div>
  );
}

async function publishArticle(markdown: string): Promise<void> {
  // Convert to HTML with sanitization
  const html = await unified()
    .use(remarkParse)
    .use(remarkRehype)
    .use(rehypeSanitize)
    .use(rehypeStringify)
    .process(markdown);

  // Additional sanitization before storage
  const sanitized = DOMPurify.sanitize(String(html), {
    ALLOWED_TAGS: ['p', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote',
                   'code', 'pre', 'strong', 'em', 'ul', 'ol', 'li', 'a', 'img'],
    ALLOWED_ATTR: ['href', 'src', 'alt', 'title', 'class'],
  });

  // Create article metadata
  const article = {
    title: extractTitle(markdown),
    content: sanitized,
    markdown,
    author: 'author-peer-id',
    publishedAt: Date.now(),
    tags: extractTags(markdown),
  };

  // Publish to DHT
  const { invoke } = await import('@tauri-apps/api/tauri');
  const articleId = await invoke<string>('publish_article', { article });

  console.log(`Published: wraith://article/${articleId}`);
}

function extractTitle(markdown: string): string {
  const match = markdown.match(/^#\s+(.+)$/m);
  return match ? match[1] : 'Untitled';
}

function extractTags(markdown: string): string[] {
  const match = markdown.match(/tags:\s*(.+)$/m);
  return match ? match[1].split(',').map(t => t.trim()) : [];
}
```

---

### S1.2: Content Storage & Distribution (13 points)

**Task:** Implement DHT-based content storage with redundancy.

**Implementation:**
```rust
// src-tauri/src/publishing.rs
use wraith_core::DhtNode;
use serde::{Serialize, Deserialize};
use blake3;

#[derive(Serialize, Deserialize, Clone)]
pub struct Article {
    pub title: String,
    pub content: String, // Sanitized HTML
    pub markdown: String,
    pub author: String,
    pub published_at: u64,
    pub tags: Vec<String>,
    pub images: Vec<String>, // Content hashes
}

#[derive(Serialize, Deserialize)]
pub struct ArticleMetadata {
    pub id: String,
    pub title: String,
    pub author: String,
    pub published_at: u64,
    pub tags: Vec<String>,
    pub content_hash: String,
}

pub struct Publisher {
    dht: DhtNode,
}

impl Publisher {
    pub fn new(dht: DhtNode) -> Self {
        Self { dht }
    }

    pub async fn publish_article(&self, article: Article) -> Result<String, String> {
        // Serialize article
        let article_bytes = serde_json::to_vec(&article)
            .map_err(|e| e.to_string())?;

        // Compute content hash
        let content_hash = blake3::hash(&article_bytes);
        let article_id = hex::encode(content_hash.as_bytes());

        // Store article in DHT with replication
        let key = format!("article:{}", article_id);
        self.dht.store(&key, article_bytes, 10).await // Replicate to 10 nodes
            .map_err(|e| e.to_string())?;

        // Store metadata separately for discovery
        let metadata = ArticleMetadata {
            id: article_id.clone(),
            title: article.title.clone(),
            author: article.author.clone(),
            published_at: article.published_at,
            tags: article.tags.clone(),
            content_hash: article_id.clone(),
        };

        let metadata_bytes = serde_json::to_vec(&metadata)
            .map_err(|e| e.to_string())?;

        // Store in author's index
        let author_key = format!("author:{}:articles", article.author);
        self.dht.append(&author_key, metadata_bytes).await
            .map_err(|e| e.to_string())?;

        // Store in tag indexes
        for tag in &article.tags {
            let tag_key = format!("tag:{}:articles", tag);
            self.dht.append(&tag_key, metadata_bytes.clone()).await
                .map_err(|e| e.to_string())?;
        }

        Ok(article_id)
    }

    pub async fn retrieve_article(&self, article_id: &str) -> Result<Article, String> {
        let key = format!("article:{}", article_id);

        let article_bytes = self.dht.retrieve(&key).await
            .map_err(|e| e.to_string())?;

        let article: Article = serde_json::from_slice(&article_bytes)
            .map_err(|e| e.to_string())?;

        Ok(article)
    }

    pub async fn search_by_tag(&self, tag: &str) -> Result<Vec<ArticleMetadata>, String> {
        let key = format!("tag:{}:articles", tag);

        let results = self.dht.retrieve(&key).await
            .map_err(|e| e.to_string())?;

        // Parse concatenated metadata entries
        let mut articles = Vec::new();
        let mut offset = 0;

        while offset < results.len() {
            let len = u32::from_be_bytes([
                results[offset],
                results[offset + 1],
                results[offset + 2],
                results[offset + 3],
            ]) as usize;

            offset += 4;

            let metadata: ArticleMetadata = serde_json::from_slice(&results[offset..offset + len])
                .map_err(|e| e.to_string())?;

            articles.push(metadata);
            offset += len;
        }

        Ok(articles)
    }
}

#[tauri::command]
pub async fn publish_article(
    article: Article,
    publisher: tauri::State<'_, Publisher>
) -> Result<String, String> {
    publisher.publish_article(article).await
}

#[tauri::command]
pub async fn get_article(
    article_id: String,
    publisher: tauri::State<'_, Publisher>
) -> Result<Article, String> {
    publisher.retrieve_article(&article_id).await
}

#[tauri::command]
pub async fn search_articles(
    tag: String,
    publisher: tauri::State<'_, Publisher>
) -> Result<Vec<ArticleMetadata>, String> {
    publisher.search_by_tag(&tag).await
}
```

---

### S1.3: Article Reader UI (13 points)

**Task:** Build beautiful article reading interface with proper sanitization.

**Implementation:**
```tsx
// src/pages/article/[id].tsx
import React, { useEffect, useState } from 'react';
import { useRouter } from 'next/router';
import { invoke } from '@tauri-apps/api/tauri';
import DOMPurify from 'dompurify';

interface Article {
  title: string;
  content: string; // Pre-sanitized HTML from server
  author: string;
  publishedAt: number;
  tags: string[];
}

export default function ArticlePage() {
  const router = useRouter();
  const { id } = router.query;
  const [article, setArticle] = useState<Article | null>(null);
  const [sanitizedContent, setSanitizedContent] = useState('');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!id) return;

    loadArticle(id as string);
  }, [id]);

  const loadArticle = async (articleId: string) => {
    setLoading(true);
    try {
      const data = await invoke<Article>('get_article', { articleId });

      // Re-sanitize content on client side for defense in depth
      const sanitized = DOMPurify.sanitize(data.content, {
        ALLOWED_TAGS: ['p', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote',
                       'code', 'pre', 'strong', 'em', 'ul', 'ol', 'li', 'a', 'img'],
        ALLOWED_ATTR: ['href', 'src', 'alt', 'title', 'class'],
        ALLOW_DATA_ATTR: false,
      });

      setArticle(data);
      setSanitizedContent(sanitized);
    } catch (error) {
      console.error('Failed to load article:', error);
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return <div className="loading">Loading article...</div>;
  }

  if (!article) {
    return <div className="error">Article not found</div>;
  }

  return (
    <div className="article-page">
      <article className="article-content">
        <header>
          <h1>{article.title}</h1>
          <div className="meta">
            <span className="author">By {article.author}</span>
            <span className="date">{formatDate(article.publishedAt)}</span>
          </div>
          <div className="tags">
            {article.tags.map(tag => (
              <span key={tag} className="tag">#{tag}</span>
            ))}
          </div>
        </header>

        <div
          className="content"
          dangerouslySetInnerHTML={{ __html: sanitizedContent }}
        />
      </article>

      <aside className="sidebar">
        <div className="share-section">
          <h3>Share</h3>
          <button onClick={() => copyLink(id as string)}>
            Copy Link
          </button>
        </div>

        <div className="author-section">
          <h3>About the Author</h3>
          <p>{article.author}</p>
        </div>
      </aside>
    </div>
  );
}

function formatDate(timestamp: number): string {
  const date = new Date(timestamp);
  return date.toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

function copyLink(articleId: string): void {
  const url = `wraith://article/${articleId}`;
  navigator.clipboard.writeText(url);
}
```

**CSS for Reading Experience:**
```css
/* Article typography optimized for reading */
.article-content {
  max-width: 680px;
  margin: 0 auto;
  padding: 60px 20px;
  font-family: 'Georgia', serif;
  font-size: 20px;
  line-height: 1.6;
  color: #333;
}

.article-content h1 {
  font-size: 42px;
  font-weight: 700;
  margin-bottom: 20px;
  line-height: 1.2;
}

.article-content .meta {
  color: #666;
  font-size: 16px;
  margin-bottom: 30px;
}

.article-content .content p {
  margin-bottom: 1.5em;
}

.article-content .content img {
  max-width: 100%;
  height: auto;
  margin: 30px 0;
  border-radius: 4px;
}

.article-content .content blockquote {
  border-left: 4px solid #ddd;
  padding-left: 20px;
  margin: 30px 0;
  font-style: italic;
  color: #555;
}

.article-content .content code {
  background: #f5f5f5;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: 'Monaco', monospace;
  font-size: 16px;
}

.article-content .content pre {
  background: #2d2d2d;
  color: #f8f8f2;
  padding: 20px;
  border-radius: 6px;
  overflow-x: auto;
  margin: 30px 0;
}
```

---

### Additional Tasks:

- **S1.4:** Image Upload & Hosting (8 pts) - Upload images to DHT, embed in articles
- **S1.5:** Draft Management (5 pts) - Save/load drafts, version history
- **S1.6:** Comment System (8 pts) - Decentralized comments via WRAITH protocol
- **S1.7:** RSS Feed Generation (5 pts) - Generate RSS feed for author's articles
- **S1.8:** Search & Discovery (8 pts) - Full-text search, trending articles

---

## Sprint 2: Monetization & Features (Weeks 53-54)

### Tasks:
- Reader payments (optional tips)
- Subscription model (recurring access)
- Analytics (view counts, reader engagement)
- Custom domain mapping
- SEO optimization
- PWA deployment

---

## Completion Checklist

- [x] Markdown editor functional with XSS protection
- [x] Articles published to DHT
- [x] Reader UI beautiful and responsive
- [x] Search and discovery working
- [x] Comments system functional
- [x] PWA installable
- [x] All user input properly sanitized

**Target Release Date:** Week 54

---

*WRAITH-Publish Sprint Planning v1.0.0*
