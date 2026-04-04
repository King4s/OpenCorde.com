/**
 * @file Events store — manages server scheduled events
 * @purpose Fetch, create, RSVP, and manage events
 * @depends api/client
 */
import api from '$lib/api/client';

export interface ServerEvent {
  id: string;
  server_id: string;
  channel_id: string | null;
  creator_id: string;
  creator_username: string;
  title: string;
  description: string | null;
  location_type: string;
  location_name: string | null;
  starts_at: string;
  ends_at: string | null;
  status: string;
  rsvp_count: number;
  created_at: string;
}

let events = $state<ServerEvent[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);

export const eventStore = {
  get events() { return events; },
  get loading() { return loading; },
  get error() { return error; },

  async fetchForServer(spaceId: string, past = false) {
    loading = true;
    error = null;
    try {
      const query = past ? '?past=true' : '';
      events = await api.get<ServerEvent[]>(`/servers/${spaceId}/events${query}`);
    } catch (e: any) {
      error = e.message ?? 'Failed to fetch events';
      events = [];
    } finally {
      loading = false;
    }
  },

  async create(spaceId: string, data: {
    title: string;
    description?: string;
    location_name?: string;
    location_type?: string;
    channel_id?: string;
    starts_at: string;
    ends_at?: string;
  }): Promise<ServerEvent> {
    const event = await api.post<ServerEvent>(`/servers/${spaceId}/events`, data);
    events = [event, ...events];
    return event;
  },

  async rsvp(eventId: string) {
    await api.post(`/events/${eventId}/rsvp`);
    events = events.map(e =>
      e.id === eventId ? { ...e, rsvp_count: e.rsvp_count + 1 } : e
    );
  },

  async unRsvp(eventId: string) {
    await api.delete(`/events/${eventId}/rsvp`);
    events = events.map(e =>
      e.id === eventId ? { ...e, rsvp_count: Math.max(0, e.rsvp_count - 1) } : e
    );
  },

  async updateStatus(eventId: string, status: 'active' | 'completed' | 'cancelled') {
    await api.patch(`/events/${eventId}`, { status });
    events = events.map(e =>
      e.id === eventId ? { ...e, status } : e
    );
  },

  async cancel(eventId: string) {
    await eventStore.updateStatus(eventId, 'cancelled');
    events = events.filter(e => e.id !== eventId);
  },

  async deleteEvent(eventId: string) {
    await api.delete(`/events/${eventId}`);
    events = events.filter(e => e.id !== eventId);
  }
};
