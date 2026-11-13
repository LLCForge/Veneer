import type { Profile, CdpCommand } from '@llcforge/veneer';

export type Send = (method: string, params?: Record<string, unknown>) => Promise<unknown>;

export interface CdpClient {
  send(method: string, params?: Record<string, unknown>, sessionId?: string): Promise<unknown>;
  close(): void;
}

export interface ConnectOptions {
  /** WebSocket constructor; defaults to the global one (Node 22+, browsers). */
  WebSocket?: new (url: string) => WebSocket;
  /** Per-command timeout, default 15000 ms. */
  timeoutMs?: number;
}

/** Open a CDP connection to a browser or page WebSocket endpoint. */
export function connect(wsUrl: string, options?: ConnectOptions): Promise<CdpClient>;
/** Apply a Chromium profile through any `send(method, params)` function. */
export function applyToSession(send: Send, profile: Profile): Promise<void>;
/** Attach to a page target over a browser connection (flat session) and apply; returns the session id. */
export function attachAndApply(client: CdpClient, targetId: string, profile: Profile): Promise<string>;
/** Connect to a page target WebSocket URL, apply the profile, and return the open client. */
export function applyToTarget(wsUrl: string, profile: Profile, options?: ConnectOptions): Promise<CdpClient>;
export function cdpCommands(profile: Profile): CdpCommand[];
