import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";

/** Matches `min_inner_size` on the overlay window in `lib.rs`. */
export const OVERLAY_MIN_WIDTH = 220;
export const OVERLAY_MIN_HEIGHT = 52;

const PADDING_PX = 10;

type FitOptions = {
  /** When true, only expand the window if content needs more space (keeps user enlargements). */
  growOnly?: boolean;
};

/** Size the native window to fit visible UI, respecting the minimum dimensions. */
export async function fitOverlayWindowToContent(
  root: HTMLElement,
  options: FitOptions = {},
): Promise<void> {
  const win = getCurrentWindow();
  if (win.label !== "overlay") {
    return;
  }

  const { width, height } = root.getBoundingClientRect();
  const neededW = Math.max(Math.ceil(width + PADDING_PX), OVERLAY_MIN_WIDTH);
  const neededH = Math.max(Math.ceil(height + PADDING_PX), OVERLAY_MIN_HEIGHT);

  if (options.growOnly) {
    const scale = await win.scaleFactor();
    const inner = await win.innerSize();
    const currentW = inner.width / scale;
    const currentH = inner.height / scale;
    await win.setSize(
      new LogicalSize(
        Math.max(neededW, currentW),
        Math.max(neededH, currentH),
      ),
    );
    return;
  }

  await win.setSize(new LogicalSize(neededW, neededH));
}

/** @deprecated Use fitOverlayWindowToContent */
export async function syncOverlayWindowSize(root: HTMLElement): Promise<void> {
  return fitOverlayWindowToContent(root);
}
