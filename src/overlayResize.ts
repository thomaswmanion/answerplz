import { getCurrentWindow } from "@tauri-apps/api/window";

export function startOverlayResize(): void {
  void getCurrentWindow().startResizeDragging("SouthEast");
}
