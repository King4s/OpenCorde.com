// @file Thread store — manages thread state and API calls
import api from '$lib/api/client';

export interface Thread {
  id: string;
  channel_id: string;
  parent_msg_id: string | null;
  name: string;
  created_by: string;
  created_at: string;
  last_msg_at: string;
  msg_count: number;
}

export interface ThreadMessage {
  id: string;
  content: string;
  author_id: string;
  author_username: string;
  attachments: unknown[];
  created_at: string;
  edited_at: string | null;
  reply_to_id?: string | null;
}

export interface ThreadStore {
  activeThread: Thread | null;
  messages: ThreadMessage[];
  loading: boolean;
  error: string | null;
}

let state = $state<ThreadStore>({
  activeThread: null,
  messages: [],
  loading: false,
  error: null
});

export const threadStore = {
  get activeThread() {
    return state.activeThread;
  },
  get messages() {
    return state.messages;
  },
  get loading() {
    return state.loading;
  },
  get error() {
    return state.error;
  },

  async openThread(threadId: string) {
    state.loading = true;
    state.error = null;
    try {
      const [thread, messages] = await Promise.all([
        api.get<Thread>(`/threads/${threadId}`),
        api.get<ThreadMessage[]>(`/threads/${threadId}/messages`)
      ]);
      state.activeThread = thread;
      state.messages = messages;
    } catch (e: unknown) {
      state.error = (e as { message?: string }).message ?? 'Failed to load thread';
    } finally {
      state.loading = false;
    }
  },

  async createThread(
    channelId: string,
    messageId: string,
    name?: string
  ): Promise<Thread> {
    const thread = await api.post<Thread>(
      `/channels/${channelId}/messages/${messageId}/thread`,
      { name: name ?? 'Thread' }
    );
    state.activeThread = thread;
    state.messages = [];
    return thread;
  },

  async sendMessage(threadId: string, content: string) {
    const msg = await api.post<ThreadMessage>(
      `/threads/${threadId}/messages`,
      { content }
    );
    state.messages = [...state.messages, msg];
    return msg;
  },

  closeThread() {
    state.activeThread = null;
    state.messages = [];
    state.error = null;
  }
};
