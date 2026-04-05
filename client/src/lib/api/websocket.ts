/**
 * @file WebSocket client for gateway connection
 * @purpose Handles connection lifecycle, heartbeat, reconnection, event dispatch
 * @depends stores/auth
 */

type EventHandler = (data: unknown) => void;

export class GatewayClient {
  private ws: WebSocket | null = null;
  private url: string;
  private token: string = "";
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  private handlers: Map<string, EventHandler[]> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;

  constructor(url?: string) {
    this.url = url ?? "";
  }

  connect(token: string): void {
    this.token = token;
    if (!this.url) {
      const server =
        typeof localStorage !== "undefined"
          ? localStorage.getItem("opencorde_server")
          : null;
      if (server) {
        const wsProto = server.startsWith("https:") ? "wss:" : "ws:";
        this.url = `${wsProto}${server.replace(/^https?/, "")}/api/v1/gateway`;
      } else {
        const proto = location.protocol === "https:" ? "wss:" : "ws:";
        this.url = `${proto}//${location.host}/api/v1/gateway`;
      }
    }
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      console.log("[WS] Connected");
      this.reconnectAttempts = 0;
    };

    this.ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      this.handleMessage(msg);
    };

    this.ws.onclose = () => {
      console.log("[WS] Disconnected");
      this.cleanup();
      this.tryReconnect();
    };

    this.ws.onerror = (error) => {
      console.error("[WS] Error:", error);
    };
  }

  private handleMessage(msg: { type: string; data?: unknown }): void {
    switch (msg.type) {
      case "Hello":
        // Send IDENTIFY
        this.send({ type: "Identify", data: { token: this.token } });
        // Start heartbeat
        const interval =
          (msg.data as { heartbeat_interval: number })?.heartbeat_interval ||
          30000;
        this.startHeartbeat(interval);
        break;

      case "Ready":
        console.log("[WS] Ready:", msg.data);
        this.emit("Ready", msg.data);
        break;

      case "Heartbeat":
        this.send({ type: "HeartbeatAck" });
        break;

      default:
        this.emit(msg.type, msg.data);
        break;
    }
  }

  on(event: string, handler: EventHandler): void {
    if (!this.handlers.has(event)) {
      this.handlers.set(event, []);
    }
    this.handlers.get(event)!.push(handler);
  }

  off(event: string, handler: EventHandler): void {
    const list = this.handlers.get(event);
    if (list) {
      this.handlers.set(
        event,
        list.filter((h) => h !== handler),
      );
    }
  }

  private emit(event: string, data: unknown): void {
    const list = this.handlers.get(event);
    if (list) {
      list.forEach((h) => h(data));
    }
  }

  send(data: unknown): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data));
    }
  }

  private startHeartbeat(interval: number): void {
    this.heartbeatTimer = setInterval(() => {
      // Server sends heartbeat, we just ack
    }, interval);
  }

  private cleanup(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  private tryReconnect(): void {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = Math.min(1000 * 2 ** this.reconnectAttempts, 30000);
      console.log(
        `[WS] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`,
      );
      setTimeout(() => this.connect(this.token), delay);
    }
  }

  disconnect(): void {
    this.cleanup();
    this.ws?.close();
    this.ws = null;
  }
}

export const gateway = new GatewayClient();
