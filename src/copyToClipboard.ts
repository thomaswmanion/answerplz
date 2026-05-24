/** Copy text to the system clipboard from the overlay webview. */
export async function copyToClipboard(text: string): Promise<void> {
  if (!text.trim()) {
    throw new Error("Nothing to copy.");
  }
  await navigator.clipboard.writeText(text);
}
