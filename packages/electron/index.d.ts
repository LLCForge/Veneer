import type { Profile } from '@llcforge/veneer';

export interface WebContentsLike {
  debugger: {
    attach(protocolVersion?: string): void;
    detach(): void;
    isAttached(): boolean;
    sendCommand(method: string, commandParams?: Record<string, unknown>, sessionId?: string): Promise<unknown>;
  };
}
export interface SessionLike {
  setUserAgent(userAgent: string, acceptLanguages?: string): void;
}

/** Apply `veneer.cdpCommands(profile)` to a webContents; resolves to a detach function. */
export function applyToWebContents(webContents: WebContentsLike, profile: Profile): Promise<() => void>;
/** Session-wide user agent and Accept-Language. */
export function applyToElectronSession(session: SessionLike, profile: Profile): void;
/** `[name, value]` pairs for `app.commandLine.appendSwitch`. */
export function commandLineSwitches(profile: Profile): Array<[string, string]>;
