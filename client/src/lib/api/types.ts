/**
 * @file API response types matching backend schemas
 * @purpose Shared type definitions for API responses
 */

export interface UserInfo {
  id: string;
  username: string;
  email: string;
}

export interface UserProfile {
  id: string;
  username: string;
  email: string;
  avatar_url: string | null;
  status: number;
}

export interface AuthResponse {
  user: UserInfo;
  access_token: string;
  expires_in: number;
}

export interface Server {
  id: string;
  name: string;
  owner_id: string;
  icon_url: string | null;
  description: string | null;
  created_at: string;
}

export interface Channel {
  id: string;
  server_id: string;
  name: string;
  channel_type: number;
  topic: string | null;
  position: number;
  parent_id: string | null;
  created_at: string;
}

export interface Message {
  id: string;
  channel_id: string;
  author_id: string;
  content: string;
  attachments: unknown[];
  edited_at: string | null;
  created_at: string;
}

export interface Member {
  user_id: string;
  server_id: string;
  nickname: string | null;
  joined_at: string;
}

export interface VoiceState {
  user_id: string;
  channel_id: string;
  session_id: string;
  self_mute: boolean;
  self_deaf: boolean;
  joined_at: string;
}

export interface Role {
  id: string;
  server_id: string;
  name: string;
  permissions: number;
  color: number | null;
  position: number;
  mentionable: boolean;
  created_at: string;
}
