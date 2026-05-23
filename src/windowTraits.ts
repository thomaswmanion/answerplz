import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();
let toggleTimeout: ReturnType<typeof setTimeout> | undefined;

/** Debounced click-through toggle so DWM/WebView2 don't desync on rapid changes. */
export function setClickThroughSafe(ignore: boolean) {
  if (toggleTimeout !== undefined) {
    clearTimeout(toggleTimeout);
  }

  toggleTimeout = window.setTimeout(async () => {
    toggleTimeout = undefined;
    try {
      await appWindow.setIgnoreCursorEvents(ignore);
    } catch (err) {
      console.error("DWM failed to update window bounds:", err);
    }
  }, 16);
}

/** Release click-through immediately — call on overlay unmount / before quit. */
export async function releaseClickThrough() {
  if (toggleTimeout !== undefined) {
    clearTimeout(toggleTimeout);
    toggleTimeout = undefined;
  }
  try {
    await appWindow.setIgnoreCursorEvents(false);
  } catch (err) {
    console.error("DWM failed to release click-through:", err);
  }
}
