import { getAppVersion } from "./appVersion";

const GITHUB_REPO = "thomaswmanion/answerplz";
const RELEASES_URL = `https://github.com/${GITHUB_REPO}/releases`;

export interface UpdateCheckResult {
  currentVersion: string;
  latestVersion: string | null;
  updateAvailable: boolean;
  releaseUrl: string;
  message: string;
}

interface GitHubRelease {
  tag_name: string;
  html_url: string;
}

/** Strip leading "v" and compare dot-separated numeric segments. */
export function isNewerVersion(latest: string, current: string): boolean {
  const parse = (v: string) =>
    v
      .replace(/^v/i, "")
      .split(/[.-]/)
      .map((part) => Number.parseInt(part, 10) || 0);

  const a = parse(latest);
  const b = parse(current);
  const len = Math.max(a.length, b.length);
  for (let i = 0; i < len; i += 1) {
    const diff = (a[i] ?? 0) - (b[i] ?? 0);
    if (diff > 0) return true;
    if (diff < 0) return false;
  }
  return false;
}

export async function checkForUpdates(): Promise<UpdateCheckResult> {
  const currentVersion = getAppVersion();
  const base: UpdateCheckResult = {
    currentVersion,
    latestVersion: null,
    updateAvailable: false,
    releaseUrl: RELEASES_URL,
    message: "",
  };

  try {
    const res = await fetch(
      `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`,
      { headers: { Accept: "application/vnd.github+json" } },
    );
    if (!res.ok) {
      return {
        ...base,
        message: `Could not check for updates (${res.status}).`,
      };
    }
    const data = (await res.json()) as GitHubRelease;
    const latestVersion = data.tag_name;
    const releaseUrl = data.html_url || RELEASES_URL;
    const updateAvailable = isNewerVersion(latestVersion, currentVersion);
    return {
      currentVersion,
      latestVersion,
      updateAvailable,
      releaseUrl,
      message: updateAvailable
        ? `Update available: ${latestVersion} (you have ${currentVersion}).`
        : `You're on the latest release (${currentVersion}).`,
    };
  } catch (err) {
    return {
      ...base,
      message: `Could not check for updates: ${String(err)}`,
    };
  }
}
