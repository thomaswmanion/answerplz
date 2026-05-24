import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  CaptureMonitor,
  ConfigSummary,
  MonitorInfo,
  Provider,
  SaveConfigRequest,
  ValidationResult,
} from "./types";
import {
  captureMonitorFromSelectValue,
  captureMonitorToSelectValue,
  getProviderOption,
  PROVIDERS,
} from "./types";
import { MonitorPicker } from "./MonitorPicker";
import "./Setup.css";

export function Setup() {
  const [isConfigured, setIsConfigured] = useState(false);
  const [provider, setProvider] = useState<Provider>("openai");
  const [apiKey, setApiKey] = useState("");
  const [showApiKeyPanel, setShowApiKeyPanel] = useState(false);
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
        setIsConfigured(summary.configured);
        if (summary.configured) {
          setProvider(summary.provider);
          setCaptureSelect(captureMonitorToSelectValue(summary.capture_monitor));
          setModel(summary.model_override ?? "");
          setShowApiKeyPanel(false);
        } else {
          setShowApiKeyPanel(true);
        }
      } catch (err) {
        setStatus(String(err));
      }
    }
    void loadSetup();
    return () => {
      void invoke("hide_display_preview");
    };
  }, []);

  useEffect(() => {
    if (monitors.length === 0) {
      return;
    }
    void invoke("show_display_preview", { selection: captureSelect });
  }, [monitors, captureSelect]);

  async function onSave(e: React.FormEvent) {
    e.preventDefault();
    const trimmedKey = apiKey.trim();
    if (!isConfigured && !trimmedKey) {
      setStatus("Enter an API key.");
      return;
    }

    setBusy(true);
    setStatus(isConfigured && !trimmedKey ? "Saving…" : "Validating…");
    try {
      const capture_monitor: CaptureMonitor =
        captureMonitorFromSelectValue(captureSelect);
      const request: SaveConfigRequest = {
        provider,
        model: model.trim() || undefined,
        capture_monitor,
        ...(trimmedKey ? { api_key: trimmedKey } : {}),
      };
      const result = await invoke<ValidationResult>("save_app_config", {
        request,
      });
      if (result.ok) {
        setStatus(result.message);
        setApiKey("");
        await invoke("hide_display_preview");
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

  async function onCancel() {
    if (!isConfigured) {
      return;
    }
    try {
      await invoke("hide_display_preview");
      await invoke("close_setup_window");
    } catch (err) {
      setStatus(String(err));
    }
  }

  const saveLabel = busy
    ? isConfigured && !apiKey.trim()
      ? "Saving…"
      : "Validating…"
    : isConfigured
      ? "Save settings"
      : "Validate & save";

  return (
    <div className="setup">
      <header className="setup__header">
        <div className="setup__brand">
          <img src="/logo.png" alt="" className="setup__logo" aria-hidden />
          <h1>{isConfigured ? "Settings" : "answerplz"}</h1>
        </div>
        <p>
          {isConfigured ? (
            "Change screenshot target and other options. Your API key stays saved unless you update it below."
          ) : (
            <>
              Pick a provider and paste your API key. Saved locally at{" "}
              <code>~/.answerplz/config.json</code>
            </>
          )}
        </p>
      </header>

      <form className="setup__form" onSubmit={onSave}>
        <section className="setup__section">
          <h2 className="setup__section-title">Screenshot</h2>
          <MonitorPicker
            monitors={monitors}
            value={captureSelect}
            onChange={setCaptureSelect}
          />
          <p className="setup__hint">
            Used when you click <strong>answer plz</strong>.
          </p>
        </section>

        <section className="setup__section">
          <h2 className="setup__section-title">AI provider</h2>
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
          <p className="setup__hint">
            Default model: <strong>{selected.defaultModel}</strong>
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
        </section>

        <section className="setup__section setup__section--key">
          {isConfigured ? (
            <>
              <button
                type="button"
                className="setup__panel-toggle"
                onClick={() => setShowApiKeyPanel((v) => !v)}
                aria-expanded={showApiKeyPanel}
              >
                {showApiKeyPanel ? "Hide API key" : "Change API key"}
              </button>
              {showApiKeyPanel && (
                <label className="setup__label">
                  New API key
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    placeholder={selected.keyHint}
                    autoComplete="off"
                  />
                  <span className="setup__field-note">
                    Leave blank to keep your current key. Filling this in re-validates
                    before saving.
                  </span>
                </label>
              )}
            </>
          ) : (
            <label className="setup__label">
              API key
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder={selected.keyHint}
                autoComplete="off"
                required
              />
            </label>
          )}
        </section>

        <div className="setup__actions">
          {isConfigured && (
            <button
              type="button"
              className="setup__cancel"
              onClick={() => void onCancel()}
              disabled={busy}
            >
              Cancel
            </button>
          )}
          <button type="submit" className="setup__submit" disabled={busy}>
            {saveLabel}
          </button>
        </div>

        {status && (
          <p
            className={
              status.startsWith("API key is valid") ||
              status.startsWith("Settings saved")
                ? "setup__ok"
                : "setup__status"
            }
          >
            {status}
          </p>
        )}
      </form>
    </div>
  );
}
