/**
 * @file REST API client for OpenCorde backend
 * @purpose Typed HTTP client with auth token management
 */

const API_BASE = '/api/v1';

export interface ApiError {
  code: string;
  message: string;
}

class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE) {
    this.baseUrl = baseUrl;
  }

  setToken(token: string | null) {
    if (typeof localStorage !== 'undefined') {
      if (token) {
        localStorage.setItem('opencorde_token', token);
      } else {
        localStorage.removeItem('opencorde_token');
      }
    }
  }

  private getToken(): string | null {
    if (typeof localStorage !== 'undefined') {
      return localStorage.getItem('opencorde_token');
    }
    return null;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    const token = this.getToken();
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    const response = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
      credentials: 'include',
    });

    if (!response.ok) {
      const text = await response.text();
      const error: ApiError = text
        ? (JSON.parse(text) as ApiError)
        : { code: 'HTTP_ERROR', message: `HTTP ${response.status}` };
      throw error;
    }

    if (response.status === 204) {
      return undefined as T;
    }

    return response.json();
  }

  private async requestFormData<T>(
    method: string,
    path: string,
    body: FormData
  ): Promise<T> {
    const headers: Record<string, string> = {};

    const token = this.getToken();
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    const response = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers,
      body,
      credentials: 'include',
    });

    if (!response.ok) {
      const text = await response.text();
      const error: ApiError = text
        ? (JSON.parse(text) as ApiError)
        : { code: 'HTTP_ERROR', message: `HTTP ${response.status}` };
      throw error;
    }

    if (response.status === 204) {
      return undefined as T;
    }

    return response.json();
  }

  get<T>(path: string) { return this.request<T>('GET', path); }
  post<T>(path: string, body?: unknown) { return this.request<T>('POST', path, body); }
  postFormData<T>(path: string, body: FormData) { return this.requestFormData<T>('POST', path, body); }
  patch<T>(path: string, body?: unknown) { return this.request<T>('PATCH', path, body); }
  delete<T>(path: string, body?: unknown) { return this.request<T>('DELETE', path, body); }
  put<T>(path: string, body?: unknown) { return this.request<T>('PUT', path, body); }
}

export const api = new ApiClient();
export default api;
