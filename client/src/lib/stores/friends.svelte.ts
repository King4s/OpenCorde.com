/**
 * @file Friends store — manages friend relationships and user search
 * @purpose Manage pending/accepted friendships, sending requests, blocking users
 * @depends api/client
 */
import api from "$lib/api/client";

export interface Relationship {
  id: string;
  from_user: string;
  to_user: string;
  status: string;
  other_username: string;
  other_avatar_url: string | null;
  created_at: string;
}

export interface UserSearchResult {
  id: string;
  username: string;
  avatar_url: string | null;
}

let friends = $state<Relationship[]>([]);
let incoming = $state<Relationship[]>([]);
let outgoing = $state<Relationship[]>([]);
let searchResults = $state<UserSearchResult[]>([]);
let loading = $state(false);

export const friendStore = {
  get friends() {
    return friends;
  },
  get incoming() {
    return incoming;
  },
  get outgoing() {
    return outgoing;
  },
  get searchResults() {
    return searchResults;
  },
  get loading() {
    return loading;
  },

  async fetchFriends() {
    loading = true;
    try {
      friends = await api.get<Relationship[]>("/friends");
    } finally {
      loading = false;
    }
  },

  async fetchPending() {
    const data = await api.get<{
      incoming: Relationship[];
      outgoing: Relationship[];
    }>("/friends/pending");
    incoming = data.incoming;
    outgoing = data.outgoing;
  },

  async sendRequest(userId: string) {
    const rel = await api.post<Relationship>("/friends/request", {
      user_id: userId,
    });
    outgoing = [...outgoing, rel];
    return rel;
  },

  async accept(relationshipId: string) {
    await api.put(`/friends/${relationshipId}/accept`);
    const accepted = incoming.find((r) => r.id === relationshipId);
    if (accepted) {
      incoming = incoming.filter((r) => r.id !== relationshipId);
      friends = [...friends, { ...accepted, status: "accepted" }];
    }
  },

  async remove(relationshipId: string) {
    await api.delete(`/friends/${relationshipId}`);
    friends = friends.filter((r) => r.id !== relationshipId);
    incoming = incoming.filter((r) => r.id !== relationshipId);
    outgoing = outgoing.filter((r) => r.id !== relationshipId);
  },

  async block(userId: string) {
    await api.post("/friends/block", { user_id: userId });
  },

  async search(query: string) {
    if (query.length < 2) {
      searchResults = [];
      return;
    }
    searchResults = await api.get<UserSearchResult[]>(
      `/users/search?q=${encodeURIComponent(query)}`,
    );
  },

  clearSearch() {
    searchResults = [];
  },
};
