#!/usr/bin/env sh
set -eu

usage() {
  cat <<'USAGE'
usage: bootstrap.sh [install.sh options]

Fetches Sandrone from GitHub, then runs scripts/install.sh --force.

Environment:
  SANDRONE_REPO_URL      repository URL, default https://github.com/ZhmYe/Sandrone.git
  SANDRONE_REF           branch or tag, default master
  SANDRONE_INSTALL_DIR   local checkout, default ~/.sandrone

Examples:
  curl -fsSL https://raw.githubusercontent.com/ZhmYe/Sandrone/master/scripts/bootstrap.sh | sh
  curl -fsSL https://raw.githubusercontent.com/ZhmYe/Sandrone/master/scripts/bootstrap.sh | sh -s -- --skill-only
USAGE
}

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
esac

repo_url="${SANDRONE_REPO_URL:-https://github.com/ZhmYe/Sandrone.git}"
ref="${SANDRONE_REF:-master}"
install_dir="${SANDRONE_INSTALL_DIR:-"$HOME/.sandrone"}"

if ! command -v git >/dev/null 2>&1; then
  echo "git is required to bootstrap Sandrone" >&2
  exit 1
fi

if [ -d "$install_dir/.git" ]; then
  echo "Updating Sandrone in $install_dir"
  git -C "$install_dir" fetch origin "$ref"
  git -C "$install_dir" checkout "$ref"
  git -C "$install_dir" pull --ff-only origin "$ref" || true
else
  echo "Cloning Sandrone into $install_dir"
  mkdir -p "$(dirname -- "$install_dir")"
  git clone --branch "$ref" "$repo_url" "$install_dir"
fi

sh "$install_dir/scripts/install.sh" --force "$@"
