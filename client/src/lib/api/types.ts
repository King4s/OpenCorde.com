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
  email: string | null;
  avatar_url: string | null;
  public_key: string;
  status: number;
  bio: string | null;
  status_message: string | null;
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
  nsfw: boolean;
  created_at: string;
}

export interface ReactionCount {
  emoji: string;
  count: number;
  reacted: boolean;
}

export interface ReplyContext {
  id: string;
  author_username: string;
  content: string;
}

export interface Message {
  id: string;
  channel_id: string;
  author_id: string;
  author_username: string;
  content: string;
  attachments: Attachment[];
  edited_at: string | null;
  created_at: string;
  reply_to_id?: string | null;
  reply_to?: ReplyContext | null;
  reactions?: ReactionCount[];
}

export interface Attachment {
  id: string;
  filename: string;
  content_type: string;
  size: number;
  url: string;
}

export interface DmChannel {
  id: string;
  other_user_id: string;
  other_username: string;
}

export interface Member {
  user_id: string;
  server_id: string;
  username: string;
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

export interface DmMessage {
  id: string;
  dm_id: string;
  author_id: string;
  author_username: string;
  content: string;
  attachments: unknown[];
  edited_at: string | null;
  created_at: string;
}

export interface StageSession {
  id: string;
  channel_id: string;
  topic: string | null;
  started_by: string;
  started_at: string;
}

export interface StageParticipant {
  id: string;
  user_id: string;
  username: string;
  role: 'speaker' | 'audience';
  hand_raised: boolean;
  joined_at: string;
}

export interface StageDetail {
  session: StageSession;
  participants: StageParticipant[];
}

export interface InstanceStats {
  total_users: number;
  total_servers: number;
  total_messages: number;
  total_channels: number;
  active_voice_sessions: number;
}

export interface AdminUserRow {
  id: string;
  username: string;
  email: string;
  created_at: string;
}

export interface AdminServerRow {
  id: string;
  name: string;
  owner_id: number;
  member_count: number;
  created_at: string;
}
