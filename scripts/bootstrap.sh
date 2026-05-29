#!/usr/bin/env sh
set -eu

usage() {
  cat <<'USAGE'
usage: bootstrap.sh [install.sh options]

Fetches codex-auto-dev-workflow from GitHub, then runs scripts/install.sh --force.

Environment:
  CODEX_AUTO_DEV_REPO_URL      repository URL, default https://github.com/ZhmYe/codex-auto-dev-workflow.git
  CODEX_AUTO_DEV_REF           branch or tag, default master
  CODEX_AUTO_DEV_INSTALL_DIR   local checkout, default ~/.codex-auto-dev-workflow

Examples:
  curl -fsSL https://raw.githubusercontent.com/ZhmYe/codex-auto-dev-workflow/master/scripts/bootstrap.sh | sh
  curl -fsSL https://raw.githubusercontent.com/ZhmYe/codex-auto-dev-workflow/master/scripts/bootstrap.sh | sh -s -- --skill-only
USAGE
}

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
esac

repo_url="${CODEX_AUTO_DEV_REPO_URL:-https://github.com/ZhmYe/codex-auto-dev-workflow.git}"
ref="${CODEX_AUTO_DEV_REF:-master}"
install_dir="${CODEX_AUTO_DEV_INSTALL_DIR:-"$HOME/.codex-auto-dev-workflow"}"

if ! command -v git >/dev/null 2>&1; then
  echo "git is required to bootstrap codex-auto-dev-workflow" >&2
  exit 1
fi

if [ -d "$install_dir/.git" ]; then
  echo "Updating codex-auto-dev-workflow in $install_dir"
  git -C "$install_dir" fetch origin "$ref"
  git -C "$install_dir" checkout "$ref"
  git -C "$install_dir" pull --ff-only origin "$ref" || true
else
  echo "Cloning codex-auto-dev-workflow into $install_dir"
  mkdir -p "$(dirname -- "$install_dir")"
  git clone --branch "$ref" "$repo_url" "$install_dir"
fi

sh "$install_dir/scripts/install.sh" --force "$@"
