import type { MonitorInfo } from "./types";

export interface MonitorLayoutItem extends MonitorInfo {
  leftPct: number;
  topPct: number;
  widthPct: number;
  heightPct: number;
}

export interface MonitorLayout {
  items: MonitorLayoutItem[];
}

export function computeMonitorLayout(monitors: MonitorInfo[]): MonitorLayout | null {
  if (monitors.length === 0) {
    return null;
  }

  const minX = Math.min(...monitors.map((m) => m.x));
  const minY = Math.min(...monitors.map((m) => m.y));
  const maxX = Math.max(...monitors.map((m) => m.x + m.width));
  const maxY = Math.max(...monitors.map((m) => m.y + m.height));
  const totalW = Math.max(maxX - minX, 1);
  const totalH = Math.max(maxY - minY, 1);

  return {
    items: monitors.map((monitor) => ({
      ...monitor,
      leftPct: ((monitor.x - minX) / totalW) * 100,
      topPct: ((monitor.y - minY) / totalH) * 100,
      widthPct: (monitor.width / totalW) * 100,
      heightPct: (monitor.height / totalH) * 100,
    })),
  };
}
