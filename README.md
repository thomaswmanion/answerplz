# answerplz

A small desktop overlay that sits on your screen, takes a screenshot when you click **answer plz**, and returns a **brief** answer from the question visible on screen — using **your own** AI API key.

Built with [Tauri 2](https://tauri.app/) (Rust) and React (TypeScript).

## Download

Installers are on **[GitHub Releases](https://github.com/thomaswmanion/answerplz/releases)**.

| Your system | File to download |
|-------------|------------------|
| Windows 64-bit | `answerplz_*_x64-setup.exe` |
| Mac (M1 / M2 / M3 / M4) | `answerplz_*_aarch64.dmg` |
| Mac (Intel) | `answerplz_*_x64.dmg` |
| Linux | `answerplz_*_amd64.AppImage` (or `.deb`) |

No API key is bundled — configure your provider on first launch.

### CI and releases

- **Build** ([`.github/workflows/build.yml`](.github/workflows/build.yml)) runs on every push to `main` — use this for fast iteration.
- **Release** ([`.github/workflows/release.yml`](.github/workflows/release.yml)) uploads installers when you push a version tag (`v0.1.0`, etc.) or run **Actions → Release → Run workflow**.

To publish a new version: bump the version in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`, then `git tag v0.1.0 && git push origin v0.1.0`.

Repo setting for uploads: **Settings → Actions → General → Workflow permissions → Read and write permissions**.

## Features

- **Bring your own key** — OpenAI, Anthropic, Google Gemini, or OpenRouter (one dropdown + API key)
- **Unified AI client** — [genai](https://github.com/jeremychone/rust-genai) handles provider APIs (similar idea to the Vercel AI SDK)
- **Quick validation** — checks your key before saving to `~/.answerplz/config.json`
- **Floating overlay** — always-on-top, draggable chip with **answer plz**, settings, and quit
- **Screenshot → vision model** — captures the primary display and asks for the shortest possible answer

## Prerequisites

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

**CI locally (Linux, matches GitHub Actions):** enable Docker in WSL, then:

```bash
./scripts/docker-linux-build.sh
```

**WSL:** You also need a GUI (WSLg on Windows 11, or an X server). Check `echo $DISPLAY` — it should not be empty when you run the app.

**Rust:**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Development

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

Installers/binaries are under `src-tauri/target/release/bundle/`.

## Usage

1. On first launch, pick a provider, paste your API key, and click **Validate & save**.
2. The overlay appears: drag via the **⋮⋮** grip, click **answer plz** when you want an answer.
3. A small bubble shows the short answer; dismiss with **×**.
4. **⚙** reopens setup (change provider/key). **×** on the bar quits the app.

Config path: `~/.answerplz/config.json` (mode `600` on Unix).

## Supported providers

Pick one in setup and paste that provider’s API key. Default vision models:

| Provider    | Default model              |
|------------|----------------------------|
| OpenAI     | `gpt-4o-mini`              |
| Anthropic  | `claude-3-5-haiku-latest`  |
| Google Gemini | `gemini-2.0-flash`      |
| OpenRouter | `openai/gpt-4o-mini`       |

Use **Advanced → Model override** only if you want a different model ID.

## Security note

Your API key is stored **locally in plaintext** in your home directory, like most BYOK CLI/desktop tools. Do not share that file.

## License

MIT — see [LICENSE](LICENSE).
