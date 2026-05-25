#!/usr/bin/env bash
# Bump app version, commit, tag, push, and wait for the GitHub Release workflow.
#
# Usage:
#   ./scripts/release.sh 0.2.6
#   ./scripts/release.sh v0.2.6 --yes
#   ./scripts/release.sh 0.2.6 --dry-run
#
# Requires: git, gh (GitHub CLI), and GH_TOKEN or GITHUB_TOKEN (or `gh auth login`).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DRY_RUN=false
ASSUME_YES=false
VERSION_RAW=""

usage() {
  cat <<'EOF'
Usage: ./scripts/release.sh <version> [options]

Arguments:
  version     Semver without leading v (e.g. 0.2.6) or with v prefix

Options:
  -y, --yes     Skip confirmation prompt
  --dry-run     Show planned changes without committing or pushing
  -h, --help    Show this help

Environment:
  GH_TOKEN or GITHUB_TOKEN  Used by gh (or run `gh auth login`)

The tag push triggers .github/workflows/release.yml, which builds installers
and uploads assets to the GitHub release.
EOF
}

log() {
  printf '%s\n' "$*"
}

die() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -y | --yes)
      ASSUME_YES=true
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    -*)
      die "Unknown option: $1"
      ;;
    *)
      if [[ -n "$VERSION_RAW" ]]; then
        die "Unexpected argument: $1"
      fi
      VERSION_RAW="$1"
      shift
      ;;
  esac
done

[[ -n "$VERSION_RAW" ]] || {
  usage
  exit 1
}

normalize_version() {
  local v="${1#v}"
  if [[ ! "$v" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
    die "Invalid version '$1' (expected semver like 0.2.6)"
  fi
  printf '%s' "$v"
}

ensure_tools() {
  command -v git >/dev/null 2>&1 || die "git is required"
  command -v gh >/dev/null 2>&1 || die "GitHub CLI (gh) is required: https://cli.github.com/"
  if command -v jq >/dev/null 2>&1; then
    JSON_TOOL=jq
  elif command -v node >/dev/null 2>&1; then
    JSON_TOOL=node
  else
    die "jq or node is required to update JSON version files"
  fi
}

ensure_gh_auth() {
  if [[ -n "${GH_TOKEN:-}" || -n "${GITHUB_TOKEN:-}" ]]; then
    export GH_TOKEN="${GH_TOKEN:-${GITHUB_TOKEN:-}}"
    log "Using GH_TOKEN / GITHUB_TOKEN from environment."
    return
  fi
  if gh auth status >/dev/null 2>&1; then
    log "Using GitHub CLI credentials from gh auth login."
    return
  fi
  die "Set GH_TOKEN or GITHUB_TOKEN, or run: gh auth login"
}

current_project_version() {
  if [[ "$JSON_TOOL" == jq ]]; then
    jq -r .version package.json
  else
    node -e "console.log(JSON.parse(require('fs').readFileSync('package.json','utf8')).version)"
  fi
}

bump_json_version() {
  local file="$1"
  local version="$2"
  if [[ "$JSON_TOOL" == jq ]]; then
    jq --arg v "$version" '.version = $v' "$file" >"${file}.tmp"
    mv "${file}.tmp" "$file"
  else
    node - "$file" "$version" <<'NODE'
const fs = require("fs");
const [file, version] = process.argv.slice(2);
const data = JSON.parse(fs.readFileSync(file, "utf8"));
data.version = version;
fs.writeFileSync(file, JSON.stringify(data, null, 2) + "\n");
NODE
  fi
}

bump_versions() {
  local version="$1"
  bump_json_version package.json "$version"
  bump_json_version src-tauri/tauri.conf.json "$version"
  if grep -q '^version = ' src-tauri/Cargo.toml; then
    sed -i "s/^version = \".*\"/version = \"${version}\"/" src-tauri/Cargo.toml
  else
    die "Could not find version line in src-tauri/Cargo.toml"
  fi
}

ensure_clean_repo() {
  if [[ -n "$(git status --porcelain)" ]]; then
    die "Working tree is not clean. Commit or stash changes before releasing."
  fi
}

ensure_branch_ready() {
  local branch
  branch="$(git rev-parse --abbrev-ref HEAD)"
  if [[ "$branch" != "main" ]]; then
    die "Release from main (current branch: $branch)"
  fi
  git fetch origin main --quiet
  local behind
  behind="$(git rev-list --count HEAD..origin/main 2>/dev/null || echo 0)"
  if [[ "$behind" != "0" ]]; then
    die "main is $behind commit(s) behind origin/main. Pull first."
  fi
}

tag_exists() {
  local tag="$1"
  git rev-parse "$tag" >/dev/null 2>&1 && return 0
  git ls-remote --tags origin "$tag" | grep -q "$tag"
}

wait_for_release_workflow() {
  local tag="$1"
  local run_id=""
  log "Waiting for Release workflow for $tag..."
  for _ in $(seq 1 30); do
    run_id="$(
      gh run list \
        --workflow release.yml \
        --limit 20 \
        --json databaseId,headBranch,event \
        --jq "map(select(.headBranch == \"${tag}\")) | .[0].databaseId // empty" 2>/dev/null || true
    )"
    [[ -n "$run_id" ]] && break
    sleep 2
  done
  if [[ -z "$run_id" ]]; then
    log "Could not find a Release workflow run for $tag."
    log "Check manually: gh run list --workflow release.yml"
    return 0
  fi
  log "Watching workflow run $run_id ..."
  gh run watch "$run_id" --exit-status
}

print_release_url() {
  local tag="$1"
  local url
  url="$(gh release view "$tag" --json url --jq .url 2>/dev/null || true)"
  if [[ -n "$url" ]]; then
    log "Release: $url"
  else
    log "Release page will appear when the workflow finishes: https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/releases/tag/$tag"
  fi
}

VERSION="$(normalize_version "$VERSION_RAW")"
TAG="v${VERSION}"

ensure_tools
ensure_gh_auth
ensure_clean_repo
ensure_branch_ready

if tag_exists "$TAG"; then
  die "Tag $TAG already exists locally or on origin"
fi

OLD_VERSION="$(current_project_version)"
if [[ "$OLD_VERSION" == "$VERSION" ]]; then
  log "Project version is already $VERSION."
else
  log "Bumping version: $OLD_VERSION -> $VERSION"
fi

if [[ "$DRY_RUN" == true ]]; then
  log "[dry-run] Would update package.json, src-tauri/tauri.conf.json, src-tauri/Cargo.toml"
  log "[dry-run] Would commit, tag $TAG, and push to origin"
  exit 0
fi

bump_versions "$VERSION"

VERSION_FILES=(package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml)
HAS_VERSION_DIFF=false
if ! git diff --quiet -- "${VERSION_FILES[@]}"; then
  HAS_VERSION_DIFF=true
  log "Version files to commit:"
  git diff --stat -- "${VERSION_FILES[@]}"
fi

if [[ "$ASSUME_YES" != true ]]; then
  printf 'Release %s on main and push tag %s? [y/N] ' "$VERSION" "$TAG"
  read -r confirm
  if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
    log "Aborted."
    if [[ "$HAS_VERSION_DIFF" == true ]]; then
      git checkout -- "${VERSION_FILES[@]}"
    fi
    exit 1
  fi
fi

if [[ "$HAS_VERSION_DIFF" == true ]]; then
  git add "${VERSION_FILES[@]}"
  git commit -m "$(cat <<EOF
chore: release ${TAG}

Bump package, Tauri, and Cargo versions so CI bundles match the git tag.
EOF
)"
else
  log "Version files already at $VERSION; creating tag only."
fi

git tag -a "$TAG" -m "answerplz ${TAG}"

log "Pushing main and $TAG ..."
git push origin main
git push origin "$TAG"

log "Tag pushed. GitHub Actions will build installers and publish the release."
wait_for_release_workflow "$TAG"
print_release_url "$TAG"
log "Done."
