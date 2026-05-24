export type Provider = "openai" | "anthropic" | "google" | "openrouter";

export type CaptureMonitor =
  | { mode: "primary" }
  | { mode: "all" }
  | { mode: "monitor"; index: number };

export interface AppConfig {
  provider: Provider;
  api_key: string;
  model?: string;
  base_url?: string;
  capture_monitor?: CaptureMonitor;
  /** Global shortcut, e.g. Ctrl+Shift+A */
  hotkey?: string;
}

export interface SaveConfigRequest {
  provider: Provider;
  model?: string;
  base_url?: string;
  capture_monitor: CaptureMonitor;
  /** Omit or leave empty to keep the stored key when updating settings. */
  api_key?: string;
  /** Global shortcut string, e.g. Ctrl+Shift+A */
  hotkey?: string;
}

export interface ConfigSummary {
  provider: Provider;
  model: string;
  model_override?: string;
  configured: boolean;
  capture_monitor: CaptureMonitor;
  hotkey: string;
}

export interface HistoryEntry {
  at: string;
  source: string;
  preview: string;
  answer: string;
}

export const DEFAULT_HOTKEY = "Ctrl+Shift+A";

export interface MonitorInfo {
  index: number;
  id: number;
  width: number;
  height: number;
  x: number;
  y: number;
  is_primary: boolean;
  label: string;
}

export interface ValidationResult {
  ok: boolean;
  message: string;
}

export interface ProviderOption {
  id: Provider;
  label: string;
  keyHint: string;
  defaultModel: string;
}

/** Common vision-capable providers — one API key each; Rust uses the `genai` crate. */
export const PROVIDERS: ProviderOption[] = [
  {
    id: "openai",
    label: "OpenAI",
    keyHint: "sk-… from platform.openai.com",
    defaultModel: "gpt-4o-mini",
  },
  {
    id: "anthropic",
    label: "Anthropic (Claude)",
    keyHint: "sk-ant-… from console.anthropic.com",
    defaultModel: "claude-3-5-haiku-latest",
  },
  {
    id: "google",
    label: "Google Gemini",
    keyHint: "API key from aistudio.google.com",
    defaultModel: "gemini-2.5-flash",
  },
  {
    id: "openrouter",
    label: "OpenRouter (many models)",
    keyHint: "sk-or-… from openrouter.ai",
    defaultModel: "openai/gpt-4o-mini",
  },
];

export function getProviderOption(id: Provider): ProviderOption {
  return PROVIDERS.find((p) => p.id === id) ?? PROVIDERS[0];
}

export function captureMonitorToSelectValue(target: CaptureMonitor): string {
  if (target.mode === "primary") return "primary";
  if (target.mode === "all") return "all";
  return String(target.index);
}

export function captureMonitorFromSelectValue(value: string): CaptureMonitor {
  if (value === "primary") return { mode: "primary" };
  if (value === "all") return { mode: "all" };
  return { mode: "monitor", index: Number.parseInt(value, 10) };
}

export const DEFAULT_CAPTURE_MONITOR: CaptureMonitor = { mode: "primary" };
