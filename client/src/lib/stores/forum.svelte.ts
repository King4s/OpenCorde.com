// @file Forum store — manages forum post state and API calls

import api from '$lib/api/client';

export interface ForumPost {
  id: string;
  channel_id: string;
  author_id: string;
  author_username: string;
  title: string;
  content: string;
  reply_count: number;
  pinned: boolean;
  created_at: string;
  last_reply_at: string;
}

export interface ForumReply {
  id: string;
  post_id: string;
  author_id: string;
  author_username: string;
  content: string;
  created_at: string;
}

export interface ForumStore {
  posts: ForumPost[];
  currentPost: ForumPost | null;
  replies: ForumReply[];
  loading: boolean;
  error: string | null;
}

let state = $state<ForumStore>({
  posts: [],
  currentPost: null,
  replies: [],
  loading: false,
  error: null
});

export const forumStore = {
  get posts() {
    return state.posts;
  },
  get currentPost() {
    return state.currentPost;
  },
  get replies() {
    return state.replies;
  },
  get loading() {
    return state.loading;
  },
  get error() {
    return state.error;
  },

  async fetchPosts(channelId: string, limit: number = 20) {
    state.loading = true;
    state.error = null;
    try {
      const posts = await api.get<ForumPost[]>(
        `/channels/${channelId}/posts?limit=${limit}`
      );
      state.posts = posts;
    } catch (e: unknown) {
      state.error = (e as { message?: string }).message ?? 'Failed to load posts';
    } finally {
      state.loading = false;
    }
  },

  async createPost(
    channelId: string,
    title: string,
    content: string
  ): Promise<ForumPost> {
    const post = await api.post<ForumPost>(
      `/channels/${channelId}/posts`,
      { title, content }
    );
    state.posts = [post, ...state.posts];
    return post;
  },

  async deletePost(postId: string) {
    await api.delete(`/posts/${postId}`);
    state.posts = state.posts.filter(p => p.id !== postId);
    if (state.currentPost?.id === postId) {
      state.currentPost = null;
      state.replies = [];
    }
  },

  async fetchPost(postId: string) {
    state.loading = true;
    state.error = null;
    try {
      const response = await api.get<{ post: ForumPost; replies: ForumReply[] }>(
        `/posts/${postId}`
      );
      state.currentPost = response.post;
      state.replies = response.replies;
    } catch (e: unknown) {
      state.error = (e as { message?: string }).message ?? 'Failed to load post';
    } finally {
      state.loading = false;
    }
  },

  async createReply(postId: string, content: string): Promise<ForumReply> {
    const reply = await api.post<ForumReply>(
      `/posts/${postId}/replies`,
      { content }
    );
    state.replies = [...state.replies, reply];
    if (state.currentPost) {
      state.currentPost.reply_count += 1;
      state.currentPost.last_reply_at = reply.created_at;
    }
    return reply;
  },

  async deleteReply(replyId: string) {
    await api.delete(`/replies/${replyId}`);
    state.replies = state.replies.filter(r => r.id !== replyId);
    if (state.currentPost) {
      state.currentPost.reply_count = Math.max(0, state.currentPost.reply_count - 1);
    }
  },

  clear() {
    state.posts = [];
    state.currentPost = null;
    state.replies = [];
    state.error = null;
  }
};
