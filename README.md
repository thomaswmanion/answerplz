# answerplz

<p align="center">
  <img src="logo.png" alt="answerplz logo" width="180">
</p>

A small desktop overlay that sits on your screen, takes a screenshot when you click **answer plz** (or press your global hotkey), and returns a **brief** answer from the question visible on screen — using **your own** AI API key.

Built with [Tauri 2](https://tauri.app/) (Rust) and React (TypeScript).

## Download

Installers are on **[GitHub Releases](https://github.com/thomaswmanion/answerplz/releases)**.

| Your system | File to download |
|-------------|------------------|
| Windows 64-bit | `answerplz_*_x64-setup.exe` |
| Mac (Apple Silicon) | `answerplz_*_aarch64.dmg` |
| Linux | `answerplz_*_amd64.AppImage` (or `.deb`) |

No API key is bundled — configure your provider on first launch.

### Unsigned builds (Windows & macOS)

Releases are **not code-signed yet**. Your OS or antivirus may warn because the installer comes from an unknown publisher. This is expected for a small open-source app — source is in this repo if you want to inspect or [build it yourself](#build).

**Windows**

1. Run the installer (`answerplz_*_x64-setup.exe`).
2. If SmartScreen shows **Windows protected your PC**, click **More info**, then **Run anyway**.
3. If your antivirus blocks it, allow the file or add an exception. You can verify the download came from [GitHub Releases](https://github.com/thomaswmanion/answerplz/releases) for this repository.

**macOS**

Browser downloads get a quarantine flag. Unsigned apps often show a misleading **"answerplz is damaged and can't be opened"** dialog with only **Move to Trash** and **Done**.

**That dialog cannot be bypassed** with right-click → Open or **Open Anyway** in System Settings. You need Terminal once:

1. Open the `.dmg` and copy **answerplz.app** to **Applications**.
2. Open **Terminal** (Applications → Utilities → Terminal).
3. Remove the quarantine flag:

```bash
xattr -dr com.apple.quarantine /Applications/answerplz.app
```

4. Open **answerplz** from Applications.

If the app is somewhere else, change the path (for example `~/Downloads/answerplz.app`).

After quarantine is cleared, macOS may show a **second**, softer warning — **"cannot be verified"** or **"Not Opened"**. That one *can* use the GUI:

- **macOS 14 and earlier:** right-click the app → **Open** → **Open** (once).
- **macOS 15 (Sequoia) and later:** try to open the app once, then go to **System Settings → Privacy & Security**, scroll down, click **Open Anyway**, and enter your password. Right-click → Open often no longer works on Sequoia.

If it still won't launch after removing quarantine, run both commands:

```bash
codesign --force --deep --sign - --timestamp=none /Applications/answerplz.app
xattr -dr com.apple.quarantine /Applications/answerplz.app
```

Linux packages (`.AppImage`, `.deb`) are not affected by Windows SmartScreen or macOS Gatekeeper.

## Features

- **Bring your own key** — OpenAI, Anthropic, Google Gemini, or OpenRouter (one dropdown + API key)
- **Floating overlay** — always-on-top, draggable, resizable chip with **answer plz**, settings, and hide
- **Global hotkey** — configurable shortcut (default `Ctrl+Shift+A`) triggers screenshot + answer from anywhere
- **Screenshot → vision model** — captures your chosen display (primary, a specific monitor, or all combined) and asks for the shortest possible answer
- **Type a question** — click **?**, ask anything, get a brief reply (no screenshot)
- **Clipboard answer** — click **⎘** to answer from whatever text you copied
- **Copy answer** — one click on the answer bubble
- **System tray** — show/hide overlay, open settings, or quit (the overlay **×** hides; quit is from the tray)
- **Recent answers** — last answers saved locally in settings
- **Check for updates** — compares your version to the latest GitHub release
- **Quick validation** — checks your key before saving to `~/.answerplz/config.json`

## Usage

1. On first launch, pick a provider, paste your API key, and click **Validate & save**.
2. The overlay appears: drag via the **⋮⋮** grip, click **answer plz** (or use your hotkey) when you want an answer.
3. A small bubble shows the short answer; copy with **⎘** or dismiss with **×**.
4. **⚙** reopens settings (provider, monitor, hotkey, etc.). **×** on the bar **hides** the overlay — use the **system tray** to show it again or quit.

Config path: `~/.answerplz/config.json` (mode `600` on Unix). Answer history: `~/.answerplz/history.json`.

### macOS permissions

Screenshots require **Screen Recording** in **System Settings → Privacy & Security → Screen Recording** (enable **answerplz**, then restart the app). Without it, captures can succeed but show blank content and the model will reply with useless answers like “No answer”.

**Accessibility** (for global hotkeys) may also be required in the same settings pane.

## Supported providers

Pick one in setup and paste that provider’s API key. Default vision models:

| Provider    | Default model              |
|------------|----------------------------|
| OpenAI     | `gpt-5.4-mini`              |
| Anthropic  | `claude-haiku-4-5`          |
| Google Gemini | `gemini-3.5-flash`       |
| OpenRouter | `openai/gpt-5.4-mini`       |

Use **Advanced → Model override** only if you want a different model ID.

## Security note

Your API key is stored **locally in plaintext** in your home directory, like most BYOK CLI/desktop tools. Do not share that file.

## License

MIT — see [LICENSE](LICENSE).

---

## For contributors

### Prerequisites

Install [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

**Linux (Debian/Ubuntu / WSL):**

Tauri links against system GTK/WebKit libraries. If `npm run tauri dev` fails during `cargo build` with `pkg-config` or `dbus-1` errors, install:

```bash
sudo apt update
sudo apt install -y \
  pkg-config \
  libdbus-1-dev \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl wget file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  libxcb1-dev \
  libxcb-randr0-dev
```

**WSL:** You also need a GUI (WSLg on Windows 11, or an X server). Check `echo $DISPLAY` — it should not be empty when you run the app.

**Rust:**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Development

```bash
npm install
npm run tauri dev
```

After updating `logo.png`, regenerate bundle and web icons:

```bash
npm run icons
```

### Build

```bash
npm run tauri build
```

Installers/binaries are under `src-tauri/target/release/bundle/`.

**CI locally (Linux, matches GitHub Actions):** enable Docker in WSL, then:

```bash
./scripts/docker-linux-build.sh
```

### CI and releases

- **Build** ([`.github/workflows/build.yml`](.github/workflows/build.yml)) runs on every push to `main` — use this for fast iteration.
- **Release** ([`.github/workflows/release.yml`](.github/workflows/release.yml)) uploads installers when you push a version tag (`v0.1.0`, etc.) or run **Actions → Release → Run workflow**.

To publish a new version (bumps all three version files, commits, tags, pushes, and waits for CI):

```bash
# GH_TOKEN or GITHUB_TOKEN required, or `gh auth login`
./scripts/release.sh 0.2.6
```

Use `--dry-run` to preview, or `-y` to skip the confirmation prompt.

Repo setting for uploads: **Settings → Actions → General → Workflow permissions → Read and write permissions**.

The app uses [genai](https://github.com/jeremychone/rust-genai) for provider APIs (similar idea to the Vercel AI SDK).
