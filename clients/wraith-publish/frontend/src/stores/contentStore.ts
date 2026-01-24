import { create } from 'zustand';
import type { Article, ArticleFilter } from '../types';
import * as api from '../lib/tauri';

interface DraftUpdate {
  title?: string;
  content?: string;
  tags?: string[];
  imageUrl?: string | null;
}

interface ContentState {
  // State
  articles: Article[];
  selectedArticle: Article | null;
  filter: ArticleFilter;
  searchQuery: string;
  loading: boolean;
  error: string | null;

  // Actions
  fetchArticles: () => Promise<void>;
  selectArticle: (article: Article | null) => void;
  setFilter: (filter: ArticleFilter) => void;
  setSearchQuery: (query: string) => void;
  createDraft: (title: string, content: string, tags: string[]) => Promise<Article>;
  updateDraft: (id: string, updates: DraftUpdate) => void;
  saveDraft: (id: string) => Promise<Article>;
  deleteDraft: (id: string) => Promise<void>;
  publishArticle: (id: string) => Promise<string>;
  unpublishArticle: (cid: string) => Promise<void>;
  searchArticles: (query: string) => Promise<void>;
  setShowPublishModal: (show: boolean) => void;
  clearError: () => void;
}

export const useContentStore = create<ContentState>((set, get) => ({
  // Initial state
  articles: [],
  selectedArticle: null,
  filter: 'all',
  searchQuery: '',
  loading: false,
  error: null,

  // Fetch all articles based on filter
  fetchArticles: async () => {
    set({ loading: true, error: null });
    try {
      const filter = get().filter;
      let articles: Article[];

      if (filter === 'drafts') {
        articles = await api.listDrafts();
      } else if (filter === 'published') {
        articles = await api.listPublished();
      } else {
        // Fetch both and merge
        const [drafts, published] = await Promise.all([
          api.listDrafts(),
          api.listPublished(),
        ]);
        articles = [...drafts, ...published];
      }

      // Filter archived if needed
      if (filter === 'archived') {
        articles = articles.filter((a) => a.status === 'archived');
      }

      // Sort by updated_at descending
      articles.sort((a, b) => b.updated_at - a.updated_at);

      set({ articles, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  // Select an article
  selectArticle: (article) => {
    set({ selectedArticle: article });
  },

  // Set filter
  setFilter: (filter) => {
    set({ filter });
    get().fetchArticles();
  },

  // Set search query
  setSearchQuery: (query) => {
    set({ searchQuery: query });
    if (query.trim()) {
      get().searchArticles(query);
    } else {
      get().fetchArticles();
    }
  },

  // Create a new draft
  createDraft: async (title, content, tags) => {
    set({ loading: true, error: null });
    try {
      const article = await api.createDraft(title, content, tags);
      await get().fetchArticles();
      set({ selectedArticle: article, loading: false });
      return article;
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  // Update a draft locally (no API call)
  updateDraft: (id, updates) => {
    set((state) => {
      const updatedArticles = state.articles.map((a) => {
        if (a.id !== id) return a;
        return {
          ...a,
          ...(updates.title !== undefined && { title: updates.title }),
          ...(updates.content !== undefined && { content: updates.content }),
          ...(updates.tags !== undefined && { tags: updates.tags }),
          ...(updates.imageUrl !== undefined && { image_url: updates.imageUrl }),
          updated_at: Math.floor(Date.now() / 1000),
        };
      });

      const updatedSelected =
        state.selectedArticle?.id === id
          ? updatedArticles.find((a) => a.id === id) || state.selectedArticle
          : state.selectedArticle;

      return {
        articles: updatedArticles,
        selectedArticle: updatedSelected,
      };
    });
  },

  // Save draft to backend
  saveDraft: async (id) => {
    const state = get();
    const article = state.articles.find((a) => a.id === id);
    if (!article) throw new Error('Article not found');

    set({ loading: true, error: null });
    try {
      const saved = await api.updateDraft(
        id,
        article.title,
        article.content,
        article.tags,
        article.image_url
      );
      set((state) => ({
        articles: state.articles.map((a) => (a.id === id ? saved : a)),
        selectedArticle:
          state.selectedArticle?.id === id ? saved : state.selectedArticle,
        loading: false,
      }));
      return saved;
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  // Delete a draft
  deleteDraft: async (id) => {
    set({ loading: true, error: null });
    try {
      await api.deleteDraft(id);
      set((state) => ({
        articles: state.articles.filter((a) => a.id !== id),
        selectedArticle:
          state.selectedArticle?.id === id ? null : state.selectedArticle,
        loading: false,
      }));
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  // Publish an article
  publishArticle: async (id) => {
    set({ loading: true, error: null });
    try {
      const result = await api.publishArticle(id);
      set((state) => ({
        articles: state.articles.map((a) =>
          a.id === id ? result.article : a
        ),
        selectedArticle:
          state.selectedArticle?.id === id ? result.article : state.selectedArticle,
        loading: false,
      }));
      return result.cid;
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  // Unpublish an article
  unpublishArticle: async (cid) => {
    set({ loading: true, error: null });
    try {
      const article = await api.unpublishArticle(cid);
      set((state) => ({
        articles: state.articles.map((a) =>
          a.cid === cid ? article : a
        ),
        selectedArticle:
          state.selectedArticle?.cid === cid ? article : state.selectedArticle,
        loading: false,
      }));
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  // Search articles
  searchArticles: async (query) => {
    set({ loading: true, error: null });
    try {
      const articles = await api.searchArticles(query, 50);
      set({ articles, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  // Set show publish modal (delegates to uiStore but kept for convenience)
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  setShowPublishModal: (_show: boolean) => {
    // This is a convenience method - actual state is in uiStore
    // Components should import from uiStore directly
  },

  // Clear error
  clearError: () => set({ error: null }),
}));
