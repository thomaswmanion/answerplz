#!/usr/bin/env bash
# Reproduce the Linux GitHub Actions build in Docker (enable WSL Docker integration).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${ANSWERPLZ_CI_IMAGE:-answerplz-ci:ubuntu22}"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is not available. Enable Docker Desktop WSL integration, or rely on GitHub Actions build.yml on push."
  exit 1
fi

docker build -t "$IMAGE" -f "$ROOT/scripts/Dockerfile.ci" "$ROOT/scripts"

docker run --rm \
  -v "$ROOT:/workspace" \
  -w /workspace \
  "$IMAGE" \
  bash -lc 'set -euo pipefail && npm ci && npm run tauri -- build'

echo "Linux CI build succeeded."
