#!/usr/bin/env sh
set -eu

usage() {
  cat <<'USAGE'
usage: scripts/install.sh [--skill-only|--cli-only] [--dest <codex-home>] [--force]

Installs the codex-auto-dev workflow skill and, unless --skill-only is used,
the codex-auto-dev CLI via cargo install --path .

Options:
  --skill-only        install only the Codex skill
  --cli-only          install only the CLI
  --dest <path>       Codex home directory, defaults to $CODEX_HOME or ~/.codex
  --force             replace an existing installed skill
USAGE
}

mode="all"
force="0"
dest="${CODEX_HOME:-"$HOME/.codex"}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --skill-only)
      mode="skill"
      shift
      ;;
    --cli-only)
      mode="cli"
      shift
      ;;
    --dest)
      if [ "$#" -lt 2 ]; then
        echo "--dest requires a value" >&2
        exit 1
      fi
      dest="$2"
      shift 2
      ;;
    --force)
      force="1"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
skill_src="$repo_root/skills/codex-auto-dev-workflow"
skill_dest="$dest/skills/codex-auto-dev-workflow"

install_skill() {
  if [ ! -f "$skill_src/SKILL.md" ]; then
    echo "missing skill source: $skill_src/SKILL.md" >&2
    exit 1
  fi

  mkdir -p "$dest/skills"

  if [ -e "$skill_dest" ]; then
    if cmp -s "$skill_src/SKILL.md" "$skill_dest/SKILL.md"; then
      echo "Skill already installed: $skill_dest"
      return
    fi
    if [ "$force" != "1" ]; then
      echo "skill already exists with different content: $skill_dest" >&2
      echo "rerun with --force to replace it" >&2
      exit 1
    fi
    rm -rf "$skill_dest"
  fi

  mkdir -p "$skill_dest"
  cp "$skill_src/SKILL.md" "$skill_dest/SKILL.md"
  echo "Installed skill: $skill_dest"
}

install_cli() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required to install the CLI" >&2
    exit 1
  fi
  cargo install --path "$repo_root"
}

case "$mode" in
  all)
    install_skill
    install_cli
    ;;
  skill)
    install_skill
    ;;
  cli)
    install_cli
    ;;
esac

echo "Restart Codex to pick up newly installed skills."
