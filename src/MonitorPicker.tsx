import { invoke } from "@tauri-apps/api/core";
import type { MonitorInfo } from "./types";
import { computeMonitorLayout } from "./monitorLayout";
import "./MonitorPicker.css";

interface MonitorPickerProps {
  monitors: MonitorInfo[];
  value: string;
  onChange: (value: string) => void;
}

async function previewSelection(value: string) {
  try {
    await invoke("show_display_preview", { selection: value });
  } catch {
    // Preview is best-effort while picking a display.
  }
}

export function MonitorPicker({ monitors, value, onChange }: MonitorPickerProps) {
  const layout = computeMonitorLayout(monitors);

  function select(value: string) {
    onChange(value);
    void previewSelection(value);
  }

  return (
    <div className="monitor-picker">
      <div className="monitor-picker__modes">
        <button
          type="button"
          className={`monitor-picker__mode ${value === "primary" ? "monitor-picker__mode--active" : ""}`}
          onClick={() => select("primary")}
          onMouseEnter={() => void previewSelection("primary")}
        >
          Primary display
        </button>
        <button
          type="button"
          className={`monitor-picker__mode ${value === "all" ? "monitor-picker__mode--active" : ""}`}
          onClick={() => select("all")}
          onMouseEnter={() => void previewSelection("all")}
        >
          All displays
        </button>
        <button
          type="button"
          className="monitor-picker__identify"
          onClick={() => {
            void previewSelection("all");
            window.setTimeout(() => void previewSelection(value), 2500);
          }}
        >
          Identify
        </button>
      </div>

      {layout && layout.items.length > 0 && (
        <div
          className="monitor-picker__canvas"
          onMouseLeave={() => void previewSelection(value)}
          role="listbox"
          aria-label="Choose a display"
        >
          {layout.items.map((monitor) => {
            const monitorValue = String(monitor.index);
            const isActive = value === monitorValue;
            return (
              <button
                key={monitor.index}
                type="button"
                role="option"
                aria-selected={isActive}
                className={`monitor-picker__screen ${isActive ? "monitor-picker__screen--active" : ""}`}
                style={{
                  left: `${monitor.leftPct}%`,
                  top: `${monitor.topPct}%`,
                  width: `${monitor.widthPct}%`,
                  height: `${monitor.heightPct}%`,
                }}
                onClick={() => select(monitorValue)}
                onMouseEnter={() => void previewSelection(monitorValue)}
                title={monitor.label}
              >
                <span className="monitor-picker__screen-number">{monitor.index + 1}</span>
              </button>
            );
          })}
        </div>
      )}

      <p className="monitor-picker__hint">
        Hover or click a display to highlight it on your monitors, like Windows display
        settings.
      </p>
    </div>
  );
}
