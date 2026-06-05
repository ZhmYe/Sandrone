#!/usr/bin/env sh
set -eu

usage() {
  cat <<'USAGE'
usage: scripts/install.sh [--skill-only|--cli-only] [--dest <codex-home>] [--force]

Installs the Sandrone workflow skill, the bundled obsidian-change-trace skill,
CodeGraph when possible, and unless --skill-only is used the sandrone CLI
via cargo install --path .

Options:
  --skill-only        install only the Codex skill
  --cli-only          install only the CLI
  --dest <path>       Codex home directory, defaults to $CODEX_HOME or ~/.codex
  --force             replace an existing installed skill

Environment:
  SANDRONE_SKIP_CODEGRAPH_INSTALL=1   skip CodeGraph installation/config
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

install_skill_dir() {
  skill_name="$1"
  skill_src="$repo_root/skills/$skill_name"
  skill_dest="$dest/skills/$skill_name"

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
  cp -R "$skill_src/." "$skill_dest/"
  echo "Installed skill: $skill_dest"
}

cleanup_legacy_skill_dirs() {
  legacy_skill="$dest/skills/codex-auto-dev-workflow"
  if [ -e "$legacy_skill" ]; then
    rm -rf "$legacy_skill"
    echo "Removed legacy skill: $legacy_skill"
  fi
}

install_skills() {
  cleanup_legacy_skill_dirs
  install_skill_dir sandrone
  install_skill_dir obsidian-change-trace
}

install_codegraph() {
  if [ "${SANDRONE_SKIP_CODEGRAPH_INSTALL:-0}" = "1" ]; then
    echo "Skipping CodeGraph install because SANDRONE_SKIP_CODEGRAPH_INSTALL=1"
    return
  fi

  if ! command -v codegraph >/dev/null 2>&1; then
    if command -v npm >/dev/null 2>&1; then
      echo "Installing CodeGraph with npm..."
      if ! npm install -g @colbymchenry/codegraph; then
        echo "warning: failed to install CodeGraph with npm" >&2
      fi
    else
      echo "warning: npm is unavailable; cannot auto-install CodeGraph" >&2
    fi
  fi

  if command -v codegraph >/dev/null 2>&1; then
    echo "CodeGraph available: $(command -v codegraph)"
    if ! codegraph install -t codex -l global -y >/dev/null 2>&1; then
      echo "warning: failed to auto-configure CodeGraph MCP for Codex" >&2
      echo "         run manually: codegraph install -t codex -l global -y" >&2
    fi
  else
    echo "warning: CodeGraph is still unavailable" >&2
    echo "         install manually: npm install -g @colbymchenry/codegraph" >&2
    echo "         or set SANDRONE_CODEGRAPH_BIN=/absolute/path/to/codegraph" >&2
  fi
}

install_cli() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required to install the CLI" >&2
    exit 1
  fi
  cargo install --path "$repo_root"
  bin_dir="${CARGO_HOME:-"$HOME/.cargo"}/bin"
  for legacy_bin in codex-auto-dev cad; do
    legacy_path="$bin_dir/$legacy_bin"
    if [ -e "$legacy_path" ]; then
      rm -f "$legacy_path"
      echo "Removed legacy CLI entry: $legacy_path"
    fi
  done
}

case "$mode" in
  all)
    install_skills
    install_codegraph
    install_cli
    ;;
  skill)
    install_skills
    install_codegraph
    ;;
  cli)
    install_codegraph
    install_cli
    ;;
esac

echo "Restart Codex to pick up newly installed skills and CodeGraph MCP changes."
