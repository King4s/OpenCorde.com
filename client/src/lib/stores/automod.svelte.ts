//! @file AutoMod store — manages keyword filter rules

import api from "$lib/api/client";

export interface AutomodRule {
  id: string;
  server_id: string;
  name: string;
  keywords: string[];
  enabled: boolean;
  action: string;
  created_at: string;
}

let rules = $state<AutomodRule[]>([]);

export const automodStore = {
  get rules() {
    return rules;
  },

  async fetch(spaceId: string) {
    rules = await api.get<AutomodRule[]>(`/servers/${spaceId}/automod`);
  },

  async create(
    spaceId: string,
    keywords: string[],
    name?: string,
    action?: string,
  ) {
    const rule = await api.post<AutomodRule>(`/servers/${spaceId}/automod`, {
      name: name ?? "Keyword Filter",
      keywords,
      action: action ?? "delete",
    });
    rules = [...rules, rule];
    return rule;
  },

  async update(
    ruleId: string,
    patch: Partial<{
      name: string;
      keywords: string[];
      enabled: boolean;
      action: string;
    }>,
  ) {
    await api.patch(`/automod/${ruleId}`, patch);
    rules = rules.map((r) => (r.id === ruleId ? { ...r, ...patch } : r));
  },

  async remove(ruleId: string) {
    await api.delete(`/automod/${ruleId}`);
    rules = rules.filter((r) => r.id !== ruleId);
  },
};
