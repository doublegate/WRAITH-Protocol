import { invoke } from '@tauri-apps/api/core';
import type {
  Article,
  FetchedArticle,
  MarkdownMetadata,
  PropagationStatus,
  PublishResult,
  SignatureInfo,
  SignedContent,
  StorageStats,
  StoredImage,
} from '../types';

// =============================================================================
// Identity Commands
// =============================================================================

export async function getPeerId(): Promise<string> {
  return invoke('get_peer_id');
}

export async function getDisplayName(): Promise<string> {
  return invoke('get_display_name');
}

export async function setDisplayName(name: string): Promise<void> {
  return invoke('set_display_name', { name });
}

// =============================================================================
// Draft Commands
// =============================================================================

export async function createDraft(
  title: string,
  content: string,
  tags: string[]
): Promise<Article> {
  return invoke('create_draft', { title, content, tags });
}

export async function updateDraft(
  id: string,
  title: string,
  content: string,
  tags: string[],
  imageUrl: string | null
): Promise<Article> {
  return invoke('update_draft', { id, title, content, tags, imageUrl });
}

export async function deleteDraft(id: string): Promise<void> {
  return invoke('delete_draft', { id });
}

export async function listDrafts(): Promise<Article[]> {
  return invoke('list_drafts');
}

export async function getDraft(id: string): Promise<Article | null> {
  return invoke('get_draft', { id });
}

// =============================================================================
// Publish Commands
// =============================================================================

export async function publishArticle(id: string): Promise<PublishResult> {
  return invoke('publish_article', { id });
}

export async function unpublishArticle(cid: string): Promise<Article> {
  return invoke('unpublish_article', { cid });
}

export async function listPublished(): Promise<Article[]> {
  return invoke('list_published');
}

export async function getArticle(id: string): Promise<Article | null> {
  return invoke('get_article', { id });
}

// =============================================================================
// Content Commands
// =============================================================================

export async function fetchContent(cid: string): Promise<FetchedArticle | null> {
  return invoke('fetch_content', { cid });
}

export async function verifyContent(id: string): Promise<boolean> {
  return invoke('verify_content', { id });
}

export async function searchArticles(
  query: string,
  limit?: number
): Promise<Article[]> {
  return invoke('search_articles', { query, limit });
}

// =============================================================================
// Signature Commands
// =============================================================================

export async function signContent(content: number[]): Promise<SignedContent> {
  return invoke('sign_content', { content });
}

export async function verifySignature(
  signedContent: SignedContent
): Promise<SignatureInfo> {
  return invoke('verify_signature', { signedContent });
}

// =============================================================================
// Storage Commands
// =============================================================================

export async function pinContent(cid: string): Promise<boolean> {
  return invoke('pin_content', { cid });
}

export async function unpinContent(cid: string): Promise<boolean> {
  return invoke('unpin_content', { cid });
}

export async function listPinned(): Promise<string[]> {
  return invoke('list_pinned');
}

export async function getStorageStats(): Promise<StorageStats> {
  return invoke('get_storage_stats');
}

// =============================================================================
// Propagation Commands
// =============================================================================

export async function getPropagationStatus(
  cid: string
): Promise<PropagationStatus | null> {
  return invoke('get_propagation_status', { cid });
}

export async function listActivePropagations(): Promise<PropagationStatus[]> {
  return invoke('list_active_propagations');
}

// =============================================================================
// RSS Commands
// =============================================================================

export async function generateRssFeed(): Promise<string> {
  return invoke('generate_rss_feed');
}

export async function generateAuthorFeed(authorId: string): Promise<string> {
  return invoke('generate_author_feed', { authorId });
}

export async function generateTagFeed(tag: string): Promise<string> {
  return invoke('generate_tag_feed', { tag });
}

// =============================================================================
// Markdown Commands
// =============================================================================

export async function renderMarkdown(markdown: string): Promise<string> {
  return invoke('render_markdown', { markdown });
}

export async function extractMetadata(markdown: string): Promise<MarkdownMetadata> {
  return invoke('extract_metadata', { markdown });
}

// =============================================================================
// Image Commands
// =============================================================================

export async function uploadImage(
  data: number[],
  mimeType: string
): Promise<string> {
  return invoke('upload_image', { data, mimeType });
}

export async function getImage(cid: string): Promise<StoredImage | null> {
  return invoke('get_image', { cid });
}

export async function deleteImage(cid: string): Promise<void> {
  return invoke('delete_image', { cid });
}
