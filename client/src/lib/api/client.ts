/**
 * @file REST API client for OpenCorde backend
 * @purpose Typed HTTP client with auth token management
 */

export interface ApiError {
  code: string;
  message: string;
}

/** Return the API base URL, preferring the user-configured server over relative path. */
function getApiBase(): string {
  if (typeof localStorage === "undefined") return "/api/v1";
  const server = localStorage.getItem("opencorde_server");
  if (server) return `${server.replace(/\/$/, "")}/api/v1`;
  return "/api/v1";
}

class ApiClient {
  setToken(token: string | null) {
    if (typeof localStorage !== "undefined") {
      if (token) {
        localStorage.setItem("opencorde_token", token);
      } else {
        localStorage.removeItem("opencorde_token");
      }
    }
  }

  private getToken(): string | null {
    if (typeof localStorage !== "undefined") {
      return localStorage.getItem("opencorde_token");
    }
    return null;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };

    const token = this.getToken();
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }

    const response = await fetch(`${getApiBase()}${path}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
      credentials: "include",
    });

    if (!response.ok) {
      const text = await response.text();
      const error: ApiError = text
        ? (JSON.parse(text) as ApiError)
        : { code: "HTTP_ERROR", message: `HTTP ${response.status}` };
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
    body: FormData,
  ): Promise<T> {
    const headers: Record<string, string> = {};

    const token = this.getToken();
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }

    const response = await fetch(`${getApiBase()}${path}`, {
      method,
      headers,
      body,
      credentials: "include",
    });

    if (!response.ok) {
      const text = await response.text();
      const error: ApiError = text
        ? (JSON.parse(text) as ApiError)
        : { code: "HTTP_ERROR", message: `HTTP ${response.status}` };
      throw error;
    }

    if (response.status === 204) {
      return undefined as T;
    }

    return response.json();
  }

  get<T>(path: string) {
    return this.request<T>("GET", path);
  }
  post<T>(path: string, body?: unknown) {
    return this.request<T>("POST", path, body);
  }
  postFormData<T>(path: string, body: FormData) {
    return this.requestFormData<T>("POST", path, body);
  }
  patch<T>(path: string, body?: unknown) {
    return this.request<T>("PATCH", path, body);
  }
  delete<T>(path: string, body?: unknown) {
    return this.request<T>("DELETE", path, body);
  }
  put<T>(path: string, body?: unknown) {
    return this.request<T>("PUT", path, body);
  }
}

export const api = new ApiClient();
export default api;

/** Save the server base URL (e.g. "http://192.168.1.10:8080") and reload. */
export function setServerUrl(url: string): void {
  localStorage.setItem("opencorde_server", url.replace(/\/$/, ""));
}

/** Get the currently configured server URL, or null if using relative paths. */
export function getServerUrl(): string | null {
  if (typeof localStorage === "undefined") return null;
  return localStorage.getItem("opencorde_server");
}
