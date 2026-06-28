#!/usr/bin/env sh
set -eu

# Replace this script to use Codex CLI, Codex API provider config, Claude Code,
# or another reviewer backend. The script must print exactly one JSON object
# matching tools/schemas/review-result.schema.json to stdout.
#
# Connector contract:
# - Inputs are provided through SANDRONE_* environment variables.
# - stdout MUST be exactly one JSON object matching tools/schemas/review-result.schema.json.
# - Optional gate_unavailable=true means the reviewer backend/gate cannot make a valid decision.
# - stderr is reserved for diagnostics.
# - Any invalid JSON, empty output, or tool failure becomes a blocking review result.
# - The reviewer MUST NOT modify code or documents.
# - Reviewers receive SANDRONE_REVIEW_CONTEXT as a lightweight isolated directory.
#   They must read artifact-index.md first; it points to the authoritative
#   request/plan/change-doc/status/worktree paths and generated summaries.
# - Reviewers MUST NOT read SANDRONE_REVIEW_FORBIDDEN_PATHS, previous review summaries, or other reviewers' details.

workspace="${SANDRONE_WORKSPACE:-$(pwd)}"
prompt="${SANDRONE_REVIEW_PROMPT:-{{PROMPT_PATH}}}"
schema="${SANDRONE_REVIEW_SCHEMA:-tools/schemas/review-result.schema.json}"
env_file="${SANDRONE_ENV_FILE:-$workspace/.env}"
reviewer_config_dir="${SANDRONE_REVIEWER_CONFIG_DIR:-}"
reviewer_config_path="${SANDRONE_REVIEWER_CONFIG_PATH:-}"
reviewer_kind="${SANDRONE_REVIEWER_KIND:-}"
if [ -z "$reviewer_config_path" ] && [ -n "$reviewer_config_dir" ] && [ -n "$reviewer_kind" ]; then
  reviewer_config_path="$reviewer_config_dir/${reviewer_kind}.json"
fi

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

read_json_value() {
  file="$1"
  key="$2"
  [ -f "$file" ] || return 0
  awk -v key="$key" '
    BEGIN {
      pattern = "^[[:space:]]*\"" key "\"[[:space:]]*:[[:space:]]*\"([^\"]*)\""
    }
    {
      line=$0
      gsub(/\r/, "", line)
      if (match(line, pattern, m)) {
        print m[1]
        exit
      }
    }
  ' "$file"
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
    RequestScheduleReviewer)
      env_key_model="SANDRONE_REQUEST_SCHEDULE_REVIEWER_MODEL"
      env_key_reasoning="SANDRONE_REQUEST_SCHEDULE_REVIEWER_REASONING_EFFORT"
      reviewer_model="${SANDRONE_REQUEST_SCHEDULE_REVIEWER_MODEL:-}"
      reviewer_reasoning_effort="${SANDRONE_REQUEST_SCHEDULE_REVIEWER_REASONING_EFFORT:-}"
      ;;
    *)
      env_key_model="SANDRONE_REVIEWER_MODEL"
      env_key_reasoning="SANDRONE_REVIEWER_REASONING_EFFORT"
      reviewer_model="${SANDRONE_REVIEWER_MODEL:-${SANDRONE_MODEL:-}}"
      reviewer_reasoning_effort="${SANDRONE_REVIEWER_REASONING_EFFORT:-${SANDRONE_REASONING_EFFORT:-}}"
      ;;
  esac


  if [ -z "$reviewer_model" ]; then
    reviewer_model="$(read_json_value "$reviewer_config_path" "model")"
  fi
  if [ -z "$reviewer_reasoning_effort" ]; then
    reviewer_reasoning_effort="$(read_json_value "$reviewer_config_path" "reasoning_effort")"
  fi
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

normalize_backend() {
  backend="$1"
  case "$backend" in
    "" ) backend="" ;;
    codex) backend="codex-cli" ;;
    codex-api|codex_cli_api|codex-cli-api) backend="codex-api" ;;
    claude) backend="claude-code" ;;
  esac
  printf '%s\n' "$backend"
}

resolve_review_backend() {
  reviewer_name="${1:-{{REVIEWER}}}"
  env_key_backend="SANDRONE_REVIEW_BACKEND"
  backend=""
  config_backend="$(read_json_value "$reviewer_config_path" "agent_backend")"
  if [ -z "$config_backend" ]; then
    config_backend="$(read_json_value "$reviewer_config_path" "review_backend")"
  fi

  case "$reviewer_name" in
    PlanReviewer)
      env_key_backend="SANDRONE_PLAN_REVIEWER_BACKEND"
      backend="${SANDRONE_PLAN_REVIEWER_BACKEND:-}"
      ;;
    TestReviewer)
      env_key_backend="SANDRONE_TEST_REVIEWER_BACKEND"
      backend="${SANDRONE_TEST_REVIEWER_BACKEND:-}"
      ;;
    DesignReviewer)
      env_key_backend="SANDRONE_DESIGN_REVIEWER_BACKEND"
      backend="${SANDRONE_DESIGN_REVIEWER_BACKEND:-}"
      ;;
    IntegrationReviewer)
      env_key_backend="SANDRONE_INTEGRATION_REVIEWER_BACKEND"
      backend="${SANDRONE_INTEGRATION_REVIEWER_BACKEND:-}"
      ;;
    DecompositionReviewer)
      env_key_backend="SANDRONE_DECOMPOSITION_REVIEWER_BACKEND"
      backend="${SANDRONE_DECOMPOSITION_REVIEWER_BACKEND:-}"
      ;;
    RequestScheduleReviewer)
      env_key_backend="SANDRONE_REQUEST_SCHEDULE_REVIEWER_BACKEND"
      backend="${SANDRONE_REQUEST_SCHEDULE_REVIEWER_BACKEND:-}"
      ;;
  esac

  if [ -z "$backend" ]; then
    backend="${SANDRONE_REVIEWER_BACKEND:-${SANDRONE_REVIEW_BACKEND:-}}"
  fi
  if [ -z "$backend" ]; then
    backend="$config_backend"
  fi
  if [ -z "$backend" ]; then
    backend="$(read_dotenv_value "$env_file" "$env_key_backend")"
  fi
  if [ -z "$backend" ]; then
    backend="$(read_dotenv_value "$env_file" "SANDRONE_REVIEWER_BACKEND")"
  fi
  if [ -z "$backend" ]; then
    backend="$(read_dotenv_value "$env_file" "SANDRONE_REVIEW_BACKEND")"
  fi
  if [ -z "$backend" ]; then
    backend="codex-cli"
  fi
  normalize_backend "$backend"
}

resolve_llm_api_key() {
  key="${LLM_API_KEY:-}"
  if [ -z "$key" ]; then
    key="$(read_json_value "$reviewer_config_path" "api_key")"
  fi
  if [ -z "$key" ]; then
    key="$(read_dotenv_value "$env_file" "LLM_API_KEY")"
  fi
  printf '%s\n' "$key"
}

resolve_llm_base_url() {
  value="${LLM_BASE_URL:-}"
  if [ -z "$value" ]; then
    value="$(read_json_value "$reviewer_config_path" "base_url")"
  fi
  if [ -z "$value" ]; then
    value="$(read_dotenv_value "$env_file" "LLM_BASE_URL")"
  fi
  printf '%s\n' "$value"
}

resolve_dotenv_or_env() {
  key="$1"
  value="$(eval "printf '%s' \"\${$key:-}\"")"
  if [ -z "$value" ]; then
    value="$(read_dotenv_value "$env_file" "$key")"
  fi
  printf '%s\n' "$value"
}

codex_toml_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

resolve_or_generate_codex_model_catalog_json() {
  codex_bin_for_catalog="$1"
  configured="$(resolve_dotenv_or_env "SANDRONE_CODEX_MODEL_CATALOG_JSON")"
  if [ -n "$configured" ] && [ -f "$configured" ]; then
    printf '%s\n' "$configured"
    return 0
  fi
  if [ -n "$source_codex_home" ] && [ -f "$source_codex_home/models_cache.json" ]; then
    printf '%s\n' "$source_codex_home/models_cache.json"
    return 0
  fi
  if [ -n "${HOME:-}" ] && [ -f "$HOME/.codex/models_cache.json" ]; then
    printf '%s\n' "$HOME/.codex/models_cache.json"
    return 0
  fi
  review_model_catalog_file="$(mktemp "$review_tmp/Sandrone-model-catalog.XXXXXX")"
  if "$codex_bin_for_catalog" debug models --bundled > "$review_model_catalog_file" 2>/dev/null \
    && [ -s "$review_model_catalog_file" ]; then
    printf '%s\n' "$review_model_catalog_file"
    return 0
  fi
  rm -f "$review_model_catalog_file"
  review_model_catalog_file=""
  printf '%s\n' ""
}

review_backend="$(resolve_review_backend "{{REVIEWER}}")"
llm_api_key="$(resolve_llm_api_key)"
llm_base_url="$(resolve_llm_base_url)"

{{CODEX_BIN_RESOLVER}}

if [ "$review_backend" = "codex-cli" ] && ! codex_bin="$(resolve_codex_bin)"; then
  printf '{"reviewer":"{{REVIEWER}}","approved":false,"gate_unavailable":true,"decision":"rejected","recommended_next_phase":"blocked","summary":"codex CLI is not available","process":["checked reviewer backend"],"critical":[{"title":"missing reviewer backend","evidence":"codex command is unavailable; SANDRONE_CODEX_BIN and SANDRONE_CODEX_APP did not resolve an executable backend","impact":"review gate cannot evaluate the artifact without a reviewer backend","required_fix":"Install Codex CLI, add it to PATH, set SANDRONE_CODEX_BIN, set SANDRONE_CODEX_APP, or replace this reviewer script with another backend.","suggested_change":"Use a wrapper script path in SANDRONE_CODEX_BIN or point SANDRONE_CODEX_APP to the Codex app bundle; do not hardcode a machine-specific application path in this connector.","verification":"Run the same review command and confirm stdout is one valid JSON object with gate_unavailable=false."}],"high":[],"warning":[],"info":[]}\n'
  exit 0
fi
if [ "$review_backend" = "codex-api" ] && ! codex_bin="$(resolve_codex_bin)"; then
  printf '{"reviewer":"{{REVIEWER}}","approved":false,"gate_unavailable":true,"decision":"rejected","recommended_next_phase":"blocked","summary":"codex CLI is not available","process":["checked reviewer backend"],"critical":[{"title":"missing reviewer backend","evidence":"codex command is unavailable; SANDRONE_CODEX_BIN and SANDRONE_CODEX_APP did not resolve an executable backend for codex-api","impact":"review gate cannot evaluate the artifact without Codex CLI","required_fix":"Install Codex CLI, add it to PATH, set SANDRONE_CODEX_BIN, or set SANDRONE_CODEX_APP.","suggested_change":"Use codex-api when you want Codex CLI to use LLM_API_KEY, LLM_BASE_URL and SANDRONE_*_MODEL; use codex-cli when you want the normal Codex login.","verification":"Run the same review command and confirm stdout is one valid JSON object with gate_unavailable=false."}],"high":[],"warning":[],"info":[]}\n'
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
review_prompt_file="$(mktemp "$review_tmp/Sandrone-review-prompt.XXXXXX")"
review_model_catalog_file=""

cleanup_review_home() {
  rm -f "$review_output_file"
  rm -f "$review_prompt_file"
  [ -n "$review_model_catalog_file" ] && rm -f "$review_model_catalog_file"
  if [ -n "$cleanup_review_codex_home" ]; then
    rm -rf "$cleanup_review_codex_home"
  fi
}
trap cleanup_review_home EXIT

render_review_prompt() {
  printf 'Reviewer: {{REVIEWER}}\n'
  printf 'Request ID: %s\n' "${SANDRONE_REQUEST_ID:-}"
  printf 'Review context: %s\n' "${SANDRONE_REVIEW_CONTEXT:-}"
  printf 'Reviewer backend: %s\n' "$review_backend"
  printf 'Resolved reviewer model: %s\n' "${reviewer_model:-inherited}"
  printf 'Resolved reasoning effort: %s\n' "${reviewer_reasoning_effort:-inherited}"
  printf '\nRead %s/artifact-index.md first. It contains the authoritative paths, reading order, generated summaries, and forbidden paths for this review. Do not infer paths by scanning the workspace before reading that index.\n\n' "${SANDRONE_REVIEW_CONTEXT:-}"
  cat "$prompt"
}

run_codex_review() {
  codex_llm_api_key=""
  set -- \
    --cd "$workspace" \
    --skip-git-repo-check \
    --ephemeral \
    --ignore-user-config \
    -c 'approval_policy="never"' \
    -c 'features.plugin_hooks=false' \
    -c 'features.goals=false' \
    -c 'features.js_repl=false' \
    --sandbox {{SANDBOX}} \
    --output-schema "$schema" \
    --output-last-message "$review_output_file"

  if [ -n "$reviewer_model" ]; then
    set -- "$@" -c "model=\"${reviewer_model}\""
  fi
  if [ -n "$reviewer_reasoning_effort" ]; then
    set -- "$@" -c "model_reasoning_effort=\"${reviewer_reasoning_effort}\""
  fi
  if [ "$review_backend" = "codex-cli" ] || [ "$review_backend" = "codex-api" ]; then
    codex_model_catalog_json="$(resolve_or_generate_codex_model_catalog_json "$codex_bin")"
    if [ -n "$codex_model_catalog_json" ]; then
      escaped_catalog_json="$(codex_toml_escape "$codex_model_catalog_json")"
      set -- "$@" -c "model_catalog_json=\"${escaped_catalog_json}\""
    fi
  fi
  if [ "$review_backend" = "codex-api" ]; then
    codex_llm_api_key="$llm_api_key"
    codex_llm_base_url="$llm_base_url"
    codex_provider_id="$(resolve_dotenv_or_env "SANDRONE_CODEX_MODEL_PROVIDER")"
    codex_provider_name="$(resolve_dotenv_or_env "SANDRONE_CODEX_PROVIDER_NAME")"
    codex_wire_api="$(resolve_dotenv_or_env "SANDRONE_CODEX_WIRE_API")"
    codex_provider_id="${codex_provider_id:-sandrone-api}"
    codex_provider_name="${codex_provider_name:-Sandrone API}"
    codex_wire_api="${codex_wire_api:-responses}"
    if [ -z "$codex_llm_api_key" ]; then
      printf '{"reviewer":"{{REVIEWER}}","approved":false,"gate_unavailable":true,"decision":"rejected","recommended_next_phase":"blocked","summary":"LLM API key is not configured for codex-api","process":["checked reviewer backend","checked LLM_API_KEY"],"critical":[{"title":"missing LLM API key","evidence":"SANDRONE_REVIEW_BACKEND=codex-api requires LLM_API_KEY from the environment or .env","impact":"Codex CLI cannot authenticate to the configured provider","required_fix":"Set LLM_API_KEY in the workspace .env or shell environment, or choose codex-cli to use the normal Codex login.","suggested_change":"Store the key only in the local untracked .env or shell environment; never commit API keys or paste them into review documents.","verification":"Rerun the review command and confirm the detail JSON has gate_unavailable=false."}],"high":[],"warning":[],"info":[]}\n' > "$review_output_file"
      return 0
    fi
    if [ -z "$codex_llm_base_url" ]; then
      printf '{"reviewer":"{{REVIEWER}}","approved":false,"gate_unavailable":true,"decision":"rejected","recommended_next_phase":"blocked","summary":"LLM base URL is not configured for codex-api","process":["checked reviewer backend","checked LLM_BASE_URL"],"critical":[{"title":"missing LLM base URL","evidence":"SANDRONE_REVIEW_BACKEND=codex-api requires LLM_BASE_URL from the environment or .env","impact":"Codex CLI cannot route to the configured provider","required_fix":"Set LLM_BASE_URL in the workspace .env or shell environment, or choose codex-cli to use the normal Codex login.","suggested_change":"Use the provider base URL ending at the API root, for example https://api.openai.com/v1 or an OpenAI-compatible /v1 endpoint.","verification":"Rerun the review command and confirm Codex doctor recognizes the configured model provider."}],"high":[],"warning":[],"info":[]}\n' > "$review_output_file"
      return 0
    fi
    escaped_provider_name="$(codex_toml_escape "$codex_provider_name")"
    escaped_base_url="$(codex_toml_escape "$codex_llm_base_url")"
    escaped_wire_api="$(codex_toml_escape "$codex_wire_api")"
    set -- "$@" \
      -c "model_provider=\"${codex_provider_id}\"" \
      -c "model_providers.${codex_provider_id}={name=\"${escaped_provider_name}\", base_url=\"${escaped_base_url}\", env_key=\"LLM_API_KEY\", wire_api=\"${escaped_wire_api}\"}"
  fi

  if [ "$review_backend" = "codex-api" ]; then
    LLM_API_KEY="$codex_llm_api_key" CODEX_HOME="$review_codex_home" "$codex_bin" exec "$@" - < "$review_prompt_file" 1>&2
  else
    CODEX_HOME="$review_codex_home" "$codex_bin" exec "$@" - < "$review_prompt_file" 1>&2
  fi
}

render_review_prompt > "$review_prompt_file"

case "$review_backend" in
  codex-cli|codex-api)
    run_codex_review
    ;;
  claude-code)
    printf '{"reviewer":"{{REVIEWER}}","approved":false,"gate_unavailable":true,"decision":"rejected","recommended_next_phase":"blocked","summary":"claude-code reviewer backend is not implemented yet","process":["checked SANDRONE_REVIEW_BACKEND"],"critical":[{"title":"claude-code backend is reserved but unavailable","evidence":"SANDRONE_REVIEW_BACKEND=claude-code is configured, but this connector intentionally does not implement Claude Code yet","impact":"review gate cannot run until a supported backend is configured","required_fix":"Use codex-cli or codex-api, or replace the reviewer script with a Claude Code implementation.","suggested_change":"Set SANDRONE_REVIEW_BACKEND=codex-cli for normal Codex CLI login, or codex-api when Codex CLI should use LLM_API_KEY and LLM_BASE_URL.","verification":"Rerun the review command and confirm the selected backend returns gate_unavailable=false."}],"high":[],"warning":[],"info":[]}\n' > "$review_output_file"
    ;;
  *)
    printf '{"reviewer":"{{REVIEWER}}","approved":false,"gate_unavailable":true,"decision":"rejected","recommended_next_phase":"blocked","summary":"unknown reviewer backend","process":["checked SANDRONE_REVIEW_BACKEND"],"critical":[{"title":"unknown reviewer backend","evidence":"SANDRONE_REVIEW_BACKEND must be codex-cli, codex-api, or claude-code","impact":"review gate cannot choose a reviewer backend","required_fix":"Set SANDRONE_REVIEW_BACKEND to codex-cli, codex-api, or claude-code.","suggested_change":"Use codex-cli for normal Codex CLI login, or codex-api for Codex CLI with LLM_API_KEY, LLM_BASE_URL and provider config. claude-code is currently reserved but not implemented.","verification":"Rerun the review command and confirm the selected backend is valid."}],"high":[],"warning":[],"info":[]}\n' > "$review_output_file"
    ;;
esac

if [ ! -s "$review_output_file" ]; then
  echo "review backend completed without writing structured output to --output-last-message" >&2
  exit 1
fi

cat "$review_output_file"
