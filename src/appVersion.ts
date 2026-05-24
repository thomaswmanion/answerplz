/** Build-time label: release tag (e.g. v0.2.3) or git branch (e.g. main). */
export function getAppVersion(): string {
  return import.meta.env.VITE_APP_VERSION || "dev";
}

export function getAboutLabel(): string {
  const version = getAppVersion();
  return `answerplz ${version}`;
}
