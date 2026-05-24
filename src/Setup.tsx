import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  CaptureMonitor,
  ConfigSummary,
  HistoryEntry,
  MonitorInfo,
  Provider,
  SaveConfigRequest,
  ValidationResult,
} from "./types";
import {
  captureMonitorFromSelectValue,
  captureMonitorToSelectValue,
  DEFAULT_HOTKEY,
  getProviderOption,
  PROVIDERS,
} from "./types";
import { MonitorPicker } from "./MonitorPicker";
import { getAboutLabel } from "./appVersion";
import { checkForUpdates } from "./checkForUpdates";
import { formatHistoryDate } from "./formatHistoryDate";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./Setup.css";

export function Setup() {
  const [isConfigured, setIsConfigured] = useState(false);
  const [provider, setProvider] = useState<Provider>("openai");
  const [apiKey, setApiKey] = useState("");
  const [showApiKeyPanel, setShowApiKeyPanel] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [model, setModel] = useState("");
  const [captureSelect, setCaptureSelect] = useState("primary");
  const [hotkey, setHotkey] = useState(DEFAULT_HOTKEY);
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [busy, setBusy] = useState(false);

  const selected = getProviderOption(provider);

  useEffect(() => {
    async function loadSetup() {
      try {
        const [summary, displayList, historyList] = await Promise.all([
          invoke<ConfigSummary>("get_config_summary"),
          invoke<MonitorInfo[]>("list_displays"),
          invoke<HistoryEntry[]>("list_answer_history"),
        ]);
        setMonitors(displayList);
        setHistory(historyList);
        setIsConfigured(summary.configured);
        setHotkey(summary.hotkey || DEFAULT_HOTKEY);
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
      const trimmedHotkey = hotkey.trim() || DEFAULT_HOTKEY;
      try {
        await invoke<string>("validate_hotkey", { hotkey: trimmedHotkey });
      } catch (err) {
        setStatus(String(err));
        setBusy(false);
        return;
      }

      const request: SaveConfigRequest = {
        provider,
        model: model.trim() || undefined,
        capture_monitor,
        hotkey: trimmedHotkey,
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

  async function onCheckUpdates() {
    setCheckingUpdates(true);
    setUpdateStatus(null);
    try {
      const result = await checkForUpdates();
      setUpdateStatus(result.message);
      if (result.updateAvailable) {
        await openUrl(result.releaseUrl);
      }
    } catch (err) {
      setUpdateStatus(String(err));
    } finally {
      setCheckingUpdates(false);
    }
  }

  async function onClearHistory() {
    try {
      await invoke("clear_answer_history");
      setHistory([]);
    } catch (err) {
      setStatus(String(err));
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
            Used when you click <strong>answer plz</strong> or press your global hotkey.
          </p>
        </section>

        <section className="setup__section">
          <h2 className="setup__section-title">Global hotkey</h2>
          <label className="setup__label">
            Shortcut
            <input
              type="text"
              value={hotkey}
              onChange={(e) => setHotkey(e.target.value)}
              placeholder={DEFAULT_HOTKEY}
              spellCheck={false}
            />
          </label>
          <p className="setup__hint">
            Triggers screenshot + answer from anywhere. Examples:{" "}
            <code>Ctrl+Shift+A</code>, <code>Command+Shift+A</code> (macOS).
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

      {isConfigured && (
        <section className="setup__section setup__history">
          <div className="setup__history-header">
            <h2 className="setup__section-title">Recent answers</h2>
            {history.length > 0 && (
              <button
                type="button"
                className="setup__history-clear"
                onClick={() => void onClearHistory()}
              >
                Clear
              </button>
            )}
          </div>
          {history.length === 0 ? (
            <p className="setup__hint">Answers from the overlay appear here.</p>
          ) : (
            <ul className="setup__history-list">
              {history.slice(0, 8).map((entry, i) => (
                <li key={`${entry.at}-${i}`} className="setup__history-item">
                  <span className="setup__history-meta">
                    {formatHistoryDate(entry.at)} · {entry.source}
                  </span>
                  <p className="setup__history-preview">{entry.preview}</p>
                  <p className="setup__history-answer">{entry.answer}</p>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      <footer className="setup__about">
        <p className="setup__about-title">About</p>
        <p className="setup__about-version">{getAboutLabel()}</p>
        <button
          type="button"
          className="setup__update-btn"
          onClick={() => void onCheckUpdates()}
          disabled={checkingUpdates}
        >
          {checkingUpdates ? "Checking…" : "Check for updates"}
        </button>
        {updateStatus && <p className="setup__update-status">{updateStatus}</p>}
      </footer>
    </div>
  );
}
