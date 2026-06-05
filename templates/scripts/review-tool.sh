#!/usr/bin/env sh
set -eu

# Replace this script to use Claude Code, OpenAI API, an internal reviewer,
# or any other model backend. The script must print exactly one JSON object
# matching tools/schemas/review-result.schema.json to stdout.
#
# Connector contract:
# - Inputs are provided through SANDRONE_* environment variables.
# - stdout MUST be exactly one JSON object matching tools/schemas/review-result.schema.json.
# - Optional gate_unavailable=true means the reviewer backend/gate cannot make a valid decision.
# - stderr is reserved for diagnostics.
# - Any invalid JSON, empty output, or tool failure becomes a blocking review result.
# - The reviewer MUST NOT modify code or documents.
# - Reviewers receive SANDRONE_REVIEW_CONTEXT as an isolated copy of request/plan/change-doc/status and gate records.
# - Reviewers MUST NOT read SANDRONE_REVIEW_FORBIDDEN_PATHS, previous review summaries, or other reviewers' details.

workspace="${SANDRONE_WORKSPACE:-$(pwd)}"
prompt="${SANDRONE_REVIEW_PROMPT:-{{PROMPT_PATH}}}"
schema="${SANDRONE_REVIEW_SCHEMA:-tools/schemas/review-result.schema.json}"
env_file="${SANDRONE_ENV_FILE:-$workspace/.env}"

source_codex_home="${CODEX_HOME:-}"
if [ -z "$source_codex_home" ] && [ -n "${HOME:-}" ]; then
  source_codex_home="${HOME}/.codex"
fi

resolve_config_value() {
  file="$1"
  key="$2"
  [ -f "$file" ] || return 0
  awk -F'"' -v key="$key" '$1 ~ ("^[[:space:]]*" key "[[:space:]]*=" ) { print $2; exit }' "$file"
}

read_dotenv_value() {
  file="$1"
  key="$2"
  [ -f "$file" ] || return 0
  awk -v key="$key" '
    function trim(s) { sub(/^[[:space:]]+/, "", s); sub(/[[:space:]]+$/, "", s); return s }
    {
      line=$0
      gsub(/\r$/, "", line)
      if (line ~ /^[[:space:]]*#/ || trim(line) == "") {
        next
      }
      idx=match(line, "=")
      if (idx <= 0) {
        next
      }
      k = substr(line, 1, idx-1)
      k = trim(k)
      if (k != key) {
        next
      }
      v = substr(line, idx + 1)
      v = trim(v)
      if (v ~ /^".*"$/) {
        sub(/^"/, "", v)
        sub(/"$/, "", v)
      } else if (v ~ /^'\''.*'\''$/) {
        sub(/^\x27/, "", v)
        sub(/\x27$/, "", v)
      } else {
        sub(/[[:space:]]#.*/, "", v)
      }
      print v
      exit
    }
  ' "$file"
}

resolve_reviewer_model() {
  reviewer_name="${1:-{{REVIEWER}}}"
  env_key_model="SANDRONE_REVIEWER_MODEL"
  env_key_reasoning="SANDRONE_REVIEWER_REASONING_EFFORT"

  case "$reviewer_name" in
    PlanReviewer)
      env_key_model="SANDRONE_PLAN_REVIEWER_MODEL"
      env_key_reasoning="SANDRONE_PLAN_REVIEWER_REASONING_EFFORT"
      reviewer_model="${SANDRONE_PLAN_REVIEWER_MODEL:-}"
      reviewer_reasoning_effort="${SANDRONE_PLAN_REVIEWER_REASONING_EFFORT:-}"
      ;;
    TestReviewer)
      env_key_model="SANDRONE_TEST_REVIEWER_MODEL"
      env_key_reasoning="SANDRONE_TEST_REVIEWER_REASONING_EFFORT"
      reviewer_model="${SANDRONE_TEST_REVIEWER_MODEL:-}"
      reviewer_reasoning_effort="${SANDRONE_TEST_REVIEWER_REASONING_EFFORT:-}"
      ;;
    DesignReviewer)
      env_key_model="SANDRONE_DESIGN_REVIEWER_MODEL"
      env_key_reasoning="SANDRONE_DESIGN_REVIEWER_REASONING_EFFORT"
      reviewer_model="${SANDRONE_DESIGN_REVIEWER_MODEL:-}"
      reviewer_reasoning_effort="${SANDRONE_DESIGN_REVIEWER_REASONING_EFFORT:-}"
      ;;
    IntegrationReviewer)
      env_key_model="SANDRONE_INTEGRATION_REVIEWER_MODEL"
      env_key_reasoning="SANDRONE_INTEGRATION_REVIEWER_REASONING_EFFORT"
      reviewer_model="${SANDRONE_INTEGRATION_REVIEWER_MODEL:-}"
      reviewer_reasoning_effort="${SANDRONE_INTEGRATION_REVIEWER_REASONING_EFFORT:-}"
      ;;
    DecompositionReviewer)
      env_key_model="SANDRONE_DECOMPOSITION_REVIEWER_MODEL"
      env_key_reasoning="SANDRONE_DECOMPOSITION_REVIEWER_REASONING_EFFORT"
      reviewer_model="${SANDRONE_DECOMPOSITION_REVIEWER_MODEL:-}"
      reviewer_reasoning_effort="${SANDRONE_DECOMPOSITION_REVIEWER_REASONING_EFFORT:-}"
      ;;
    *)
      env_key_model="SANDRONE_REVIEWER_MODEL"
      env_key_reasoning="SANDRONE_REVIEWER_REASONING_EFFORT"
      reviewer_model="${SANDRONE_REVIEWER_MODEL:-${SANDRONE_MODEL:-}}"
      reviewer_reasoning_effort="${SANDRONE_REVIEWER_REASONING_EFFORT:-${SANDRONE_REASONING_EFFORT:-}}"
      ;;
  esac


  if [ -z "$reviewer_model" ]; then
    reviewer_model="$(read_dotenv_value "$env_file" "$env_key_model")"
  fi
  if [ -z "$reviewer_reasoning_effort" ]; then
    reviewer_reasoning_effort="$(read_dotenv_value "$env_file" "$env_key_reasoning")"
  fi

  if [ -z "$reviewer_model" ]; then
    reviewer_model="$(read_dotenv_value "$env_file" "SANDRONE_REVIEWER_MODEL")"
  fi
  if [ -z "$reviewer_reasoning_effort" ]; then
    reviewer_reasoning_effort="$(read_dotenv_value "$env_file" "SANDRONE_REVIEWER_REASONING_EFFORT")"
  fi
  if [ -z "$reviewer_model" ]; then
    reviewer_model="$(read_dotenv_value "$env_file" "SANDRONE_MODEL")"
  fi
  if [ -z "$reviewer_reasoning_effort" ]; then
    reviewer_reasoning_effort="$(read_dotenv_value "$env_file" "SANDRONE_REASONING_EFFORT")"
  fi

  if [ -z "$reviewer_model" ] && [ -n "$source_codex_home" ] && [ -f "$source_codex_home/config.toml" ]; then
    reviewer_model="$(resolve_config_value "$source_codex_home/config.toml" "model")"
  fi
  if [ -z "$reviewer_reasoning_effort" ] && [ -n "$source_codex_home" ] && [ -f "$source_codex_home/config.toml" ]; then
    reviewer_reasoning_effort="$(resolve_config_value "$source_codex_home/config.toml" "model_reasoning_effort")"
  fi

  printf "%s\n%s\n%s\n%s\n" "$reviewer_model" "$reviewer_reasoning_effort" "$env_key_model" "$env_key_reasoning"
}

resolved="$(resolve_reviewer_model "{{REVIEWER}}")"
reviewer_model="$(printf "%s\n" "$resolved" | sed -n '1p')"
reviewer_reasoning_effort="$(printf "%s\n" "$resolved" | sed -n '2p')"
resolved_reviewer_model_key="$(printf "%s\n" "$resolved" | sed -n '3p')"
resolved_reviewer_reasoning_key="$(printf "%s\n" "$resolved" | sed -n '4p')"

{{CODEX_BIN_RESOLVER}}

if ! codex_bin="$(resolve_codex_bin)"; then
  printf '{"reviewer":"{{REVIEWER}}","approved":false,"gate_unavailable":true,"decision":"rejected","recommended_next_phase":"blocked","summary":"codex CLI is not available","process":["checked reviewer backend"],"critical":[{"title":"missing reviewer backend","evidence":"codex command is unavailable; SANDRONE_CODEX_BIN and SANDRONE_CODEX_APP did not resolve an executable backend","impact":"review gate cannot evaluate the artifact without a reviewer backend","required_fix":"Install Codex CLI, add it to PATH, set SANDRONE_CODEX_BIN, set SANDRONE_CODEX_APP, or replace this reviewer script with another backend.","suggested_change":"Use a wrapper script path in SANDRONE_CODEX_BIN or point SANDRONE_CODEX_APP to the Codex app bundle; do not hardcode a machine-specific application path in this connector.","verification":"Run the same review command and confirm stdout is one valid JSON object with gate_unavailable=false."}],"high":[],"warning":[],"info":[]}\n'
  exit 0
fi

write_minimal_review_config() {
  config_path="$1"
  : > "$config_path"
  printf 'approval_policy = "never"\n' >> "$config_path"
  printf 'sandbox_mode = "{{SANDBOX}}"\n' >> "$config_path"
  printf '\n[features]\n' >> "$config_path"
  printf 'plugin_hooks = false\n' >> "$config_path"
  printf 'goals = false\n' >> "$config_path"
  printf 'js_repl = false\n' >> "$config_path"
}

review_tmp="${TMPDIR:-/tmp}"
cleanup_review_codex_home=""
if [ -n "${SANDRONE_REVIEW_CODEX_HOME:-}" ]; then
  review_codex_home="$SANDRONE_REVIEW_CODEX_HOME"
else
  review_codex_home="$(mktemp -d "$review_tmp/Sandrone-review-home.XXXXXX")"
  cleanup_review_codex_home="$review_codex_home"
  if [ -n "$source_codex_home" ]; then
    [ -f "$source_codex_home/auth.json" ] && cp "$source_codex_home/auth.json" "$review_codex_home/auth.json"
  fi
  write_minimal_review_config "$review_codex_home/config.toml"
  chmod 700 "$review_codex_home"
  [ -f "$review_codex_home/auth.json" ] && chmod 600 "$review_codex_home/auth.json"
  [ -f "$review_codex_home/config.toml" ] && chmod 600 "$review_codex_home/config.toml"
fi

review_output_file="$(mktemp "$review_tmp/Sandrone-review-output.XXXXXX")"

cleanup_review_home() {
  rm -f "$review_output_file"
  if [ -n "$cleanup_review_codex_home" ]; then
    rm -rf "$cleanup_review_codex_home"
  fi
}
trap cleanup_review_home EXIT

{
  printf 'Reviewer: {{REVIEWER}}\n'
  printf 'Workspace: %s\n' "$workspace"
  printf 'Request ID: %s\n' "${SANDRONE_REQUEST_ID:-}"
  printf 'External ID: %s\n' "${SANDRONE_REQUEST_EXTERNAL_ID:-}"
  printf 'Requirement name: %s\n' "${SANDRONE_REQUEST_TITLE:-}"
  printf 'Change path: %s\n' "${SANDRONE_CHANGE_PATH:-}"
  printf 'Review context: %s\n' "${SANDRONE_REVIEW_CONTEXT:-}"
  printf 'Canonical change path: %s\n' "${SANDRONE_CANONICAL_CHANGE_PATH:-}"
  printf 'Forbidden review paths: %s\n' "${SANDRONE_REVIEW_FORBIDDEN_PATHS:-}"
  printf 'Target repo: %s\n' "${SANDRONE_TARGET_REPO:-}"
  printf 'Worktree: %s\n' "${SANDRONE_WORKTREE:-}"
  printf 'Env file: %s\n' "$env_file"
  printf 'Reviewer model key: %s\n' "$resolved_reviewer_model_key"
  printf 'Reviewer reasoning key: %s\n' "$resolved_reviewer_reasoning_key"
  printf 'Resolved reviewer model: %s\n' "${reviewer_model:-inherited}"
  printf 'Resolved reasoning effort: %s\n\n' "${reviewer_reasoning_effort:-inherited}"
  cat "$prompt"
} | {
  set -- \
    --cd "$workspace" \
    --skip-git-repo-check \
    --ephemeral \
    -c 'approval_policy="never"' \
    --sandbox {{SANDBOX}} \
    --output-schema "$schema" \
    --output-last-message "$review_output_file"

  if [ -n "$reviewer_model" ]; then
    set -- "$@" -c "model=\"${reviewer_model}\""
  fi
  if [ -n "$reviewer_reasoning_effort" ]; then
    set -- "$@" -c "model_reasoning_effort=\"${reviewer_reasoning_effort}\""
  fi

  CODEX_HOME="$review_codex_home" "$codex_bin" exec "$@" - 1>&2
}

if [ ! -s "$review_output_file" ]; then
  echo "review backend completed without writing structured output to --output-last-message" >&2
  exit 1
fi

cat "$review_output_file"
