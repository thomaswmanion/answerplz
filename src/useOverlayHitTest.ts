import { useEffect, type RefObject } from "react";
import { PhysicalPosition, cursorPosition, getCurrentWindow } from "@tauri-apps/api/window";
import { setClickThroughImmediate } from "./windowTraits";

function isWindowsDesktop(): boolean {
  return navigator.userAgent.includes("Windows");
}

const INTERACTIVE_SELECTOR =
  ".overlay-bar, .overlay-ask, .overlay-answer, .overlay-resize-handle, button, input, [data-tauri-drag-region]";

function rectsOverlap(
  x: number,
  y: number,
  rect: DOMRect,
  padding = 6,
): boolean {
  return (
    x >= rect.left - padding &&
    x <= rect.right + padding &&
    y >= rect.top - padding &&
    y <= rect.bottom + padding
  );
}

function collectInteractiveRects(): DOMRect[] {
  return Array.from(document.querySelectorAll(INTERACTIVE_SELECTOR)).map((el) =>
    el.getBoundingClientRect(),
  );
}

async function pointerHitsInteractive(clientX: number, clientY: number): Promise<boolean> {
  const el = document.elementFromPoint(clientX, clientY);
  if (el?.closest(INTERACTIVE_SELECTOR)) {
    return true;
  }
  return collectInteractiveRects().some((rect) => rectsOverlap(clientX, clientY, rect));
}

async function updateWindowsClickThrough(root: HTMLElement): Promise<void> {
  const win = getCurrentWindow();
  if (win.label !== "overlay") {
    return;
  }

  const [outer, cursor, scale] = await Promise.all([
    win.outerPosition(),
    cursorPosition(),
    win.scaleFactor(),
  ]);

  const local = screenToClient(cursor, outer, scale);
  const { width, height } = root.getBoundingClientRect();

  const insideOverlay =
    local.x >= 0 &&
    local.y >= 0 &&
    local.x <= width &&
    local.y <= height;

  if (!insideOverlay) {
    await setClickThroughImmediate(true);
    return;
  }

  const interactive = await pointerHitsInteractive(local.x, local.y);
  await setClickThroughImmediate(!interactive);

  if (interactive) {
    void win.setFocus();
  }
}

function screenToClient(
  cursor: PhysicalPosition,
  outer: PhysicalPosition,
  scale: number,
): { x: number; y: number } {
  return {
    x: (cursor.x - outer.x) / scale,
    y: (cursor.y - outer.y) / scale,
  };
}

/** Windows: pass clicks through transparent chrome; only the pill UI captures input. */
export function useOverlayHitTest(rootRef: RefObject<HTMLElement | null>) {
  useEffect(() => {
    let interval: ReturnType<typeof setInterval> | undefined;

    async function start() {
      if (!isWindowsDesktop()) {
        await setClickThroughImmediate(false);
        return;
      }

      await setClickThroughImmediate(true);

      interval = setInterval(() => {
        const root = rootRef.current;
        if (!root) {
          return;
        }
        void updateWindowsClickThrough(root);
      }, 48);
    }

    void start();

    return () => {
      if (interval !== undefined) {
        clearInterval(interval);
      }
    };
  }, [rootRef]);
}
