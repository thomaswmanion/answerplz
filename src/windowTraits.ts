import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();
let toggleTimeout: ReturnType<typeof setTimeout> | undefined;
let clickThroughEnabled: boolean | null = null;

async function applyClickThrough(ignore: boolean) {
  if (clickThroughEnabled === ignore) {
    return;
  }
  clickThroughEnabled = ignore;
  await appWindow.setIgnoreCursorEvents(ignore);
}

/** Debounced click-through toggle so DWM/WebView2 don't desync on rapid changes. */
export function setClickThroughSafe(ignore: boolean) {
  if (toggleTimeout !== undefined) {
    clearTimeout(toggleTimeout);
  }

  toggleTimeout = window.setTimeout(async () => {
    toggleTimeout = undefined;
    try {
      await applyClickThrough(ignore);
    } catch (err) {
      console.error("DWM failed to update window bounds:", err);
    }
  }, 16);
}

/** Immediate toggle — used by the Windows hit-test loop. */
export async function setClickThroughImmediate(ignore: boolean) {
  if (toggleTimeout !== undefined) {
    clearTimeout(toggleTimeout);
    toggleTimeout = undefined;
  }
  try {
    await applyClickThrough(ignore);
  } catch (err) {
    console.error("DWM failed to update window bounds:", err);
  }
}

/** Release click-through immediately — call on overlay unmount / before quit. */
export async function releaseClickThrough() {
  if (toggleTimeout !== undefined) {
    clearTimeout(toggleTimeout);
    toggleTimeout = undefined;
  }
  try {
    clickThroughEnabled = null;
    await appWindow.setIgnoreCursorEvents(false);
  } catch (err) {
    console.error("DWM failed to release click-through:", err);
  }
}
