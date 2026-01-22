// Article types
export interface Article {
  id: string;
  cid: string | null;
  title: string;
  content: string;
  author_id: string;
  author_name: string | null;
  created_at: number;
  updated_at: number;
  published_at: number | null;
  tags: string[];
  image_url: string | null;
  status: ArticleStatus;
}

export type ArticleStatus = 'draft' | 'published' | 'archived';

// Signature types
export interface SignedContent {
  content: number[];
  signature: number[];
  public_key: number[];
  cid: string;
  signed_at: number;
}

export interface SignatureInfo {
  valid: boolean;
  signer_fingerprint: string;
  signed_at: number;
  cid: string;
}

// Propagation types
export interface PropagationStatus {
  cid: string;
  state: PropagationState;
  replica_count: number;
  confirmed_replicas: number;
  target_replicas: number;
  progress: number;
  eta_seconds: number | null;
  error: string | null;
  updated_at: number;
  started_at: number;
}

export type PropagationState = 'uploading' | 'propagating' | 'confirmed' | 'failed';

// Storage types
export interface StorageStats {
  total_items: number;
  item_count: number;
  pinned_items: number;
  total_bytes: number;
  total_size: number;
  articles_size: number;
  images_size: number;
  cache_size: number;
  replication_factor: number;
}

// Fetched article with verification
export interface FetchedArticle {
  article: Article;
  signature_info: SignatureInfo | null;
  from_cache: boolean;
  fetched_at: number;
}

// Publish result
export interface PublishResult {
  article: Article;
  cid: string;
}

// Markdown metadata
export interface MarkdownMetadata {
  title: string | null;
  excerpt: string;
  word_count: number;
  reading_time: number;
  headings: Heading[];
}

export interface Heading {
  level: number;
  text: string;
}

// Image types
export interface StoredImage {
  cid: string;
  data: number[];
  mime_type: string;
  uploaded_at: number;
}

// View types
export type ViewMode = 'list' | 'editor' | 'reader' | 'settings';
export type EditorMode = 'edit' | 'preview' | 'split';

// Filter types
export type ArticleFilter = 'all' | 'drafts' | 'published' | 'archived';
