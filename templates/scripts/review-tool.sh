#!/usr/bin/env sh
set -eu

# Replace this script to use Claude Code, OpenAI API, an internal reviewer,
# or any other model backend. The script must print exactly one JSON object
# matching tools/schemas/review-result.schema.json to stdout.
#
# Connector contract:
# - Inputs are provided through CODEX_AUTO_DEV_* environment variables.
# - stdout MUST be exactly one JSON object matching tools/schemas/review-result.schema.json.
# - Optional gate_unavailable=true means the reviewer backend/gate cannot make a valid decision.
# - stderr is reserved for diagnostics.
# - Any invalid JSON, empty output, or tool failure becomes a blocking review result.
# - The reviewer MUST NOT modify code or documents.
# - Reviewers receive CODEX_AUTO_DEV_REVIEW_CONTEXT as an isolated copy of request/plan/change-doc/status/approvals.
# - Reviewers MUST NOT read CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS, previous review summaries, or other reviewers' details.

workspace="${CODEX_AUTO_DEV_WORKSPACE:-$(pwd)}"
prompt="${CODEX_AUTO_DEV_REVIEW_PROMPT:-{{PROMPT_PATH}}}"
schema="${CODEX_AUTO_DEV_REVIEW_SCHEMA:-tools/schemas/review-result.schema.json}"

{{CODEX_BIN_RESOLVER}}

if ! codex_bin="$(resolve_codex_bin)"; then
  printf '{"reviewer":"{{REVIEWER}}","approved":false,"gate_unavailable":true,"decision":"rejected","recommended_next_phase":"blocked","summary":"codex CLI is not available","process":["checked reviewer backend"],"critical":[{"title":"missing reviewer backend","evidence":"codex command is unavailable; CODEX_AUTO_DEV_CODEX_BIN and CODEX_AUTO_DEV_CODEX_APP did not resolve an executable backend","impact":"review gate cannot evaluate the artifact without a reviewer backend","required_fix":"Install Codex CLI, add it to PATH, set CODEX_AUTO_DEV_CODEX_BIN, set CODEX_AUTO_DEV_CODEX_APP, or replace this reviewer script with another backend.","suggested_change":"Use a wrapper script path in CODEX_AUTO_DEV_CODEX_BIN or point CODEX_AUTO_DEV_CODEX_APP to the Codex app bundle; do not hardcode a machine-specific application path in this connector.","verification":"Run the same review command and confirm stdout is one valid JSON object with gate_unavailable=false."}],"high":[],"warning":[],"info":[]}\n'
  exit 0
fi

source_codex_home="${CODEX_HOME:-}"
if [ -z "$source_codex_home" ] && [ -n "${HOME:-}" ]; then
  source_codex_home="${HOME}/.codex"
fi

cleanup_review_codex_home=""
if [ -n "${CODEX_AUTO_DEV_REVIEW_CODEX_HOME:-}" ]; then
  review_codex_home="$CODEX_AUTO_DEV_REVIEW_CODEX_HOME"
else
  review_tmp="${TMPDIR:-/tmp}"
  review_codex_home="$(mktemp -d "$review_tmp/codex-auto-dev-review-home.XXXXXX")"
  cleanup_review_codex_home="$review_codex_home"
  if [ -n "$source_codex_home" ]; then
    [ -f "$source_codex_home/auth.json" ] && cp "$source_codex_home/auth.json" "$review_codex_home/auth.json"
    [ -f "$source_codex_home/config.toml" ] && cp "$source_codex_home/config.toml" "$review_codex_home/config.toml"
  fi
  chmod 700 "$review_codex_home"
  [ -f "$review_codex_home/auth.json" ] && chmod 600 "$review_codex_home/auth.json"
  [ -f "$review_codex_home/config.toml" ] && chmod 600 "$review_codex_home/config.toml"
fi

cleanup_review_home() {
  if [ -n "$cleanup_review_codex_home" ]; then
    rm -rf "$cleanup_review_codex_home"
  fi
}
trap cleanup_review_home EXIT

{
  printf 'Reviewer: {{REVIEWER}}\n'
  printf 'Workspace: %s\n' "$workspace"
  printf 'Request ID: %s\n' "${CODEX_AUTO_DEV_REQUEST_ID:-}"
  printf 'External ID: %s\n' "${CODEX_AUTO_DEV_REQUEST_EXTERNAL_ID:-}"
  printf 'Requirement name: %s\n' "${CODEX_AUTO_DEV_REQUEST_TITLE:-}"
  printf 'Change path: %s\n' "${CODEX_AUTO_DEV_CHANGE_PATH:-}"
  printf 'Review context: %s\n' "${CODEX_AUTO_DEV_REVIEW_CONTEXT:-}"
  printf 'Canonical change path: %s\n' "${CODEX_AUTO_DEV_CANONICAL_CHANGE_PATH:-}"
  printf 'Forbidden review paths: %s\n' "${CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS:-}"
  printf 'Target repo: %s\n' "${CODEX_AUTO_DEV_TARGET_REPO:-}"
  printf 'Worktree: %s\n\n' "${CODEX_AUTO_DEV_WORKTREE:-}"
  cat "$prompt"
} | CODEX_HOME="$review_codex_home" "$codex_bin" exec \
  --cd "$workspace" \
  --skip-git-repo-check \
  --ephemeral \
  -c 'approval_policy="never"' \
  --sandbox {{SANDBOX}} \
  --output-schema "$schema" \
  -
