export type Provider = "openai" | "anthropic" | "google" | "openrouter";

export interface AppConfig {
  provider: Provider;
  api_key: string;
  model?: string;
  base_url?: string;
}

export interface ConfigSummary {
  provider: Provider;
  model: string;
  configured: boolean;
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
