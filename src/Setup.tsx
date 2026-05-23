import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  CaptureMonitor,
  ConfigSummary,
  MonitorInfo,
  Provider,
  ValidationResult,
} from "./types";
import {
  captureMonitorFromSelectValue,
  captureMonitorToSelectValue,
  getProviderOption,
  PROVIDERS,
} from "./types";
import "./Setup.css";

export function Setup() {
  const [provider, setProvider] = useState<Provider>("openai");
  const [apiKey, setApiKey] = useState("");
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [model, setModel] = useState("");
  const [captureSelect, setCaptureSelect] = useState("primary");
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const selected = getProviderOption(provider);

  useEffect(() => {
  async function loadSetup() {
      try {
        const [summary, displayList] = await Promise.all([
          invoke<ConfigSummary>("get_config_summary"),
          invoke<MonitorInfo[]>("list_displays"),
        ]);
        setMonitors(displayList);
        if (summary.configured) {
          setProvider(summary.provider);
          setCaptureSelect(captureMonitorToSelectValue(summary.capture_monitor));
        }
      } catch (err) {
        setStatus(String(err));
      }
    }
    void loadSetup();
  }, []);

  async function onSave(e: React.FormEvent) {
    e.preventDefault();
    if (!apiKey.trim()) {
      setStatus("Enter an API key.");
      return;
    }
    setBusy(true);
    setStatus("Validating…");
    try {
      const capture_monitor: CaptureMonitor =
        captureMonitorFromSelectValue(captureSelect);
      const config: AppConfig = {
        provider,
        api_key: apiKey.trim(),
        model: model.trim() || undefined,
        capture_monitor,
      };
      const result = await invoke<ValidationResult>("validate_and_save_config", {
        config,
      });
      if (result.ok) {
        setStatus(result.message);
        await invoke("finish_setup");
      } else {
        setStatus(result.message);
      }
    } catch (err) {
      setStatus(String(err));
    } finally {
      setBusy(false);
    }
  }

  const captureOptions: { value: string; label: string }[] = [
    { value: "primary", label: "Primary display" },
    { value: "all", label: "All displays (combined)" },
    ...monitors.map((m) => ({
      value: String(m.index),
      label: m.label,
    })),
  ];

  return (
    <div className="setup">
      <header className="setup__header">
        <h1>answerplz</h1>
        <p>
          Pick a provider and paste your API key. Saved locally at{" "}
          <code>~/.answerplz/config.json</code>
        </p>
      </header>

      <form className="setup__form" onSubmit={onSave}>
        <label className="setup__label">
          Provider
          <select
            value={provider}
            onChange={(e) => setProvider(e.target.value as Provider)}
          >
            {PROVIDERS.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
          </select>
        </label>

        <label className="setup__label">
          API key
          <input
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder={selected.keyHint}
            autoComplete="off"
          />
        </label>

        <label className="setup__label">
          Screenshot target
          <select
            value={captureSelect}
            onChange={(e) => setCaptureSelect(e.target.value)}
          >
            {captureOptions.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </label>

        <p className="setup__hint">
          Uses <strong>{selected.defaultModel}</strong> by default (vision). Choose
          which display to capture when you click <strong>answer plz</strong>.
        </p>

        <button
          type="button"
          className="setup__advanced-toggle"
          onClick={() => setShowAdvanced((v) => !v)}
          aria-expanded={showAdvanced}
        >
          {showAdvanced ? "Hide" : "Show"} advanced options
        </button>

        {showAdvanced && (
          <label className="setup__label">
            Model override
            <input
              type="text"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder={selected.defaultModel}
            />
          </label>
        )}

        <button type="submit" className="setup__submit" disabled={busy}>
          {busy ? "Validating…" : "Validate & save"}
        </button>

        {status && (
          <p
            className={
              status.startsWith("API key is valid") ? "setup__ok" : "setup__status"
            }
          >
            {status}
          </p>
        )}
      </form>
    </div>
  );
}
