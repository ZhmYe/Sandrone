resolve_codex_bin() {
  if [ -n "${CODEX_AUTO_DEV_CODEX_BIN:-}" ]; then
    if [ -x "$CODEX_AUTO_DEV_CODEX_BIN" ]; then
      printf '%s\n' "$CODEX_AUTO_DEV_CODEX_BIN"
      return 0
    fi
    if command -v "$CODEX_AUTO_DEV_CODEX_BIN" >/dev/null 2>&1; then
      command -v "$CODEX_AUTO_DEV_CODEX_BIN"
      return 0
    fi
    echo "CODEX_AUTO_DEV_CODEX_BIN is set but is not executable and was not found on PATH: $CODEX_AUTO_DEV_CODEX_BIN" >&2
    return 1
  fi

  if command -v codex >/dev/null 2>&1; then
    command -v codex
    return 0
  fi

  if [ -n "${CODEX_AUTO_DEV_CODEX_APP:-}" ]; then
    for candidate in \
      "$CODEX_AUTO_DEV_CODEX_APP/Contents/Resources/codex" \
      "$CODEX_AUTO_DEV_CODEX_APP/Contents/MacOS/codex"
    do
      if [ -x "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    done
    echo "CODEX_AUTO_DEV_CODEX_APP is set but no codex binary was found inside it: $CODEX_AUTO_DEV_CODEX_APP" >&2
    return 1
  fi

  echo "codex CLI is unavailable; add codex to PATH, set CODEX_AUTO_DEV_CODEX_BIN, or set CODEX_AUTO_DEV_CODEX_APP to the Codex app bundle" >&2
  return 1
}
