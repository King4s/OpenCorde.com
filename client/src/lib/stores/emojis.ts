/**
 * @file Emoji store
 * @purpose Manage server emoji state and API operations
 */
import { writable } from "svelte/store";
import api from "$lib/api/client";

export interface EmojiRow {
  id: string;
  server_id: string;
  name: string;
  image_url: string;
  uploaded_by: string;
  created_at: string;
}

interface EmojiStore {
  emojis: EmojiRow[];
  loading: boolean;
  error: string;
}

function createEmojiStore() {
  const { subscribe, set, update } = writable<EmojiStore>({
    emojis: [],
    loading: false,
    error: "",
  });

  return {
    subscribe,

    async fetchEmojis(spaceId: string) {
      update((s) => ({ ...s, loading: true, error: "" }));
      try {
        const emojis = await api.get<EmojiRow[]>(`/servers/${spaceId}/emojis`);
        update((s) => ({ ...s, emojis, loading: false }));
      } catch (err: any) {
        const error = err.message ?? "Failed to fetch emojis";
        update((s) => ({ ...s, error, loading: false }));
        throw err;
      }
    },

    async uploadEmoji(spaceId: string, name: string, file: Blob) {
      const formData = new FormData();
      formData.append("name", name);
      formData.append("file", file);

      try {
        const response = await fetch(`/api/v1/servers/${spaceId}/emojis`, {
          method: "POST",
          body: formData,
        });

        if (!response.ok) {
          const error = await response.json();
          throw new Error(error.message ?? "Failed to upload emoji");
        }

        const emoji = (await response.json()) as EmojiRow;
        update((s) => ({ ...s, emojis: [...s.emojis, emoji] }));
        return emoji;
      } catch (err) {
        throw err;
      }
    },

    async deleteEmoji(spaceId: string, emojiId: string) {
      try {
        await api.delete(`/servers/${spaceId}/emojis/${emojiId}`);
        update((s) => ({
          ...s,
          emojis: s.emojis.filter((e) => e.id !== emojiId),
        }));
      } catch (err: any) {
        throw err;
      }
    },
  };
}

export const emojiStore = createEmojiStore();
