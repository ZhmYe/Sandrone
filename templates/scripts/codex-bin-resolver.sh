resolve_codex_bin() {
  if [ -n "${SANDRONE_CODEX_BIN:-}" ]; then
    if [ -x "$SANDRONE_CODEX_BIN" ]; then
      printf '%s\n' "$SANDRONE_CODEX_BIN"
      return 0
    fi
    if command -v "$SANDRONE_CODEX_BIN" >/dev/null 2>&1; then
      command -v "$SANDRONE_CODEX_BIN"
      return 0
    fi
    echo "SANDRONE_CODEX_BIN is set but is not executable and was not found on PATH: $SANDRONE_CODEX_BIN" >&2
    return 1
  fi

  if command -v codex >/dev/null 2>&1; then
    command -v codex
    return 0
  fi

  if [ -n "${SANDRONE_CODEX_APP:-}" ]; then
    for candidate in \
      "$SANDRONE_CODEX_APP/Contents/Resources/codex" \
      "$SANDRONE_CODEX_APP/Contents/MacOS/codex"
    do
      if [ -x "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    done
    echo "SANDRONE_CODEX_APP is set but no codex binary was found inside it: $SANDRONE_CODEX_APP" >&2
    return 1
  fi

  echo "codex CLI is unavailable; add codex to PATH, set SANDRONE_CODEX_BIN, or set SANDRONE_CODEX_APP to the Codex app bundle" >&2
  return 1
}
