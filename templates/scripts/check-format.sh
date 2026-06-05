#!/usr/bin/env sh
set -eu

# Connector contract:
# - --format formats the target worktree when the default Rust toolchain is applicable.
# - --check verifies formatting, compilation, and clippy before code-review.
# - Replace this script for non-Rust projects or stricter internal checks.
# - Exit 0 means the gate passed or was explicitly skipped for a non-Rust target.
# - Non-zero means implementation must fix the reported issue before reviewer gates run.

mode="${1:-}"
case "$mode" in
  --format|--check) ;;
  *)
    echo "usage: tools/check-format.sh (--format|--check)" >&2
    exit 2
    ;;
esac

worktree="${SANDRONE_WORKTREE:-}"
if [ -z "$worktree" ]; then
  worktree="$(pwd)"
fi
if [ ! -d "$worktree" ]; then
  echo "worktree does not exist: $worktree" >&2
  exit 2
fi

cd "$worktree"

if [ ! -f Cargo.toml ]; then
  echo "No Cargo.toml found in $worktree; check-format skipped."
  exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for Rust check-format but was not found in PATH" >&2
  exit 127
fi

case "$mode" in
  --format)
    cargo fmt
    ;;
  --check)
    cargo fmt --check
    cargo check
    cargo clippy --all-targets --all-features -- -D warnings
    ;;
esac
