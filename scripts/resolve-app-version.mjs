import { execSync } from "node:child_process";

/** Tag if HEAD is tagged, else current branch name, else `dev`. */
export function resolveAppVersion() {
  const fromEnv = process.env.VITE_APP_VERSION;
  if (fromEnv && fromEnv.trim()) {
    return fromEnv.trim();
  }

  const fromGh = process.env.GITHUB_REF_NAME;
  if (fromGh && fromGh.trim()) {
    return fromGh.trim();
  }

  try {
    execSync("git rev-parse --is-inside-work-tree", { stdio: "ignore" });
    try {
      return execSync("git describe --tags --exact-match 2>/dev/null", {
        encoding: "utf8",
      }).trim();
    } catch {
      return execSync("git rev-parse --abbrev-ref HEAD", {
        encoding: "utf8",
      }).trim();
    }
  } catch {
    return "dev";
  }
}
