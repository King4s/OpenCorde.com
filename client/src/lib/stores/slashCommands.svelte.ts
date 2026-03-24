//! @file Slash Commands store — manages slash command registrations

import api from '$lib/api/client';

export interface SlashCommand {
  id: string;
  server_id: string;
  name: string;
  description: string;
  handler_url: string;
  created_by: string;
  created_at: string;
}

let commands = $state<SlashCommand[]>([]);
let loading = $state(false);
let error = $state('');

export const slashCommandsStore = {
  get commands() {
    return commands;
  },
  get loading() {
    return loading;
  },
  get error() {
    return error;
  },

  async fetchCommands(serverId: string) {
    loading = true;
    error = '';
    try {
      commands = await api.get<SlashCommand[]>(`/servers/${serverId}/commands`);
    } catch (e: any) {
      error = e.message ?? 'Failed to load commands';
      commands = [];
    } finally {
      loading = false;
    }
  },

  async createCommand(
    serverId: string,
    name: string,
    description: string,
    handler_url: string
  ) {
    const command = await api.post<SlashCommand>(`/servers/${serverId}/commands`, {
      name,
      description,
      handler_url,
    });
    commands = [...commands, command];
    return command;
  },

  async deleteCommand(commandId: string) {
    await api.delete(`/commands/${commandId}`);
    commands = commands.filter((c) => c.id !== commandId);
  },

  async dispatchCommand(channelId: string, commandText: string, args?: string[]) {
    return await api.post<any>(`/channels/${channelId}/interact`, {
      command: commandText,
      args: args || [],
    });
  },
};
