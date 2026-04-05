/**
 * @file Webhook store — manages webhook CRUD and display
 */
import api from "$lib/api/client";

export interface Webhook {
  id: string;
  channel_id: string;
  server_id: string;
  name: string;
  token: string;
  url: string;
  created_by: string;
  created_at: string;
}

let webhooks = $state<Webhook[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);

export const webhookStore = {
  get webhooks() {
    return webhooks;
  },
  get loading() {
    return loading;
  },
  get error() {
    return error;
  },

  async fetchForChannel(channelId: string) {
    loading = true;
    error = null;
    try {
      webhooks = await api.get<Webhook[]>(`/channels/${channelId}/webhooks`);
    } catch (e: unknown) {
      error = (e as { message?: string }).message ?? "Failed to load webhooks";
    } finally {
      loading = false;
    }
  },

  async create(channelId: string, name?: string): Promise<Webhook> {
    const wh = await api.post<Webhook>(`/channels/${channelId}/webhooks`, {
      name,
    });
    webhooks = [...webhooks, wh];
    return wh;
  },

  async remove(webhookId: string) {
    await api.delete(`/webhooks/${webhookId}`);
    webhooks = webhooks.filter((w) => w.id !== webhookId);
  },

  clear() {
    webhooks = [];
  },
};
