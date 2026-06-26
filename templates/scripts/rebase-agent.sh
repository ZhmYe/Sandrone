#!/usr/bin/env sh
set -eu
trap '' HUP

# Replace this script to use Claude Code, OpenAI API, an internal agent,
# or any other rebase/conflict-resolution backend. This connector handles
# exactly one phase: rebase.
#
# Connector contract:
# - Inputs are provided through SANDRONE_* environment variables.
# - SANDRONE_AGENT_PHASE is rebase.
# - The agent MUST work only inside SANDRONE_WORKTREE.
# - The agent MUST preserve both base/master changes and request-branch changes.
# - The agent MUST NOT delete base/master code merely to keep its own branch easy.
# - The agent MUST update SANDRONE_CHANGE_DOC and SANDRONE_AGENT_JOURNAL.
# - The agent MUST NOT call sandrone approve/reject, integration-review, finish, commit, push, or PR commands.
# - Success means the rebase is complete, conflict markers are gone, and the worktree is ready for outer IntegrationReviewer.
#   When successful, mark SANDRONE_AGENT_STATUS_DOC frontmatter as submitted.
# - Failure should exit non-zero with a helpful stderr message.

{{CODEX_BIN_RESOLVER}}

workspace="${SANDRONE_WORKSPACE:-$(pwd)}"
phase="${SANDRONE_AGENT_PHASE:-rebase}"
prompt="${SANDRONE_REBASE_AGENT_PROMPT:-${SANDRONE_AGENT_PROMPT:-tools/prompts/rebase-agent.md}}"
env_file="${SANDRONE_ENV_FILE:-$workspace/.env}"
agent_config_dir="${SANDRONE_AGENT_CONFIG_DIR:-}"
agent_config_path="${SANDRONE_AGENT_CONFIG_PATH:-}"
agent_kind="${SANDRONE_AGENT_KIND:-rebase-agent}"
if [ -z "$agent_config_path" ] && [ -n "$agent_config_dir" ] && [ -n "$agent_kind" ]; then
  agent_config_path="$agent_config_dir/${agent_kind}.json"
fi
default_max_attempts=20

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

resolve_dotenv_or_env() {
  key="$1"
  value="$(eval "printf '%s' \"\${$key:-}\"")"
  if [ -z "$value" ]; then
    value="$(read_dotenv_value "$env_file" "$key")"
  fi
  printf '%s\n' "$value"
}

resolve_agent_ignore_user_config() {
  value="$(resolve_dotenv_or_env "SANDRONE_AGENT_IGNORE_USER_CONFIG")"
  printf '%s\n' "${value:-1}"
}

truthy() {
  value="$1"
  case "$value" in
    1|true|TRUE|yes|YES|on|ON) return 0 ;;
    0|false|FALSE|no|NO|off|OFF) return 1 ;;
    *) return 1 ;;
  esac
}

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

codex_toml_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

resolve_rebase_agent_model() {
  rebase_agent_model="${SANDRONE_REBASE_AGENT_MODEL:-${SANDRONE_AGENT_MODEL:-${SANDRONE_MODEL:-}}}"
  rebase_agent_reasoning_effort="${SANDRONE_REBASE_AGENT_REASONING_EFFORT:-${SANDRONE_AGENT_REASONING_EFFORT:-${SANDRONE_REASONING_EFFORT:-}}}"
  if [ -z "$rebase_agent_model" ]; then
    rebase_agent_model="$(read_json_value "$agent_config_path" "model")"
  fi
  if [ -z "$rebase_agent_reasoning_effort" ]; then
    rebase_agent_reasoning_effort="$(read_json_value "$agent_config_path" "reasoning_effort")"
  fi

  if [ -z "$rebase_agent_model" ]; then
    rebase_agent_model="$(read_dotenv_value "$env_file" "SANDRONE_REBASE_AGENT_MODEL")"
  fi
  if [ -z "$rebase_agent_reasoning_effort" ]; then
    rebase_agent_reasoning_effort="$(read_dotenv_value "$env_file" "SANDRONE_REBASE_AGENT_REASONING_EFFORT")"
  fi
  if [ -z "$rebase_agent_model" ]; then
    rebase_agent_model="$(read_dotenv_value "$env_file" "SANDRONE_AGENT_MODEL")"
  fi
  if [ -z "$rebase_agent_reasoning_effort" ]; then
    rebase_agent_reasoning_effort="$(read_dotenv_value "$env_file" "SANDRONE_AGENT_REASONING_EFFORT")"
  fi
  if [ -z "$rebase_agent_model" ]; then
    rebase_agent_model="$(read_dotenv_value "$env_file" "SANDRONE_MODEL")"
  fi
  if [ -z "$rebase_agent_reasoning_effort" ]; then
    rebase_agent_reasoning_effort="$(read_dotenv_value "$env_file" "SANDRONE_REASONING_EFFORT")"
  fi

  if [ -z "$rebase_agent_model" ] && [ -n "$source_codex_home" ] && [ -f "$source_codex_home/config.toml" ]; then
    rebase_agent_model="$(resolve_config_value "$source_codex_home/config.toml" "model")"
  fi
  if [ -z "$rebase_agent_reasoning_effort" ] && [ -n "$source_codex_home" ] && [ -f "$source_codex_home/config.toml" ]; then
    rebase_agent_reasoning_effort="$(resolve_config_value "$source_codex_home/config.toml" "model_reasoning_effort")"
  fi

  printf '%s\n%s\n' "$rebase_agent_model" "$rebase_agent_reasoning_effort"
}

resolve_rebase_agent_backend() {
  backend="${SANDRONE_REBASE_AGENT_BACKEND:-${SANDRONE_AGENT_BACKEND:-${SANDRONE_BACKEND:-}}}"
  if [ -z "$backend" ]; then
    backend="$(read_json_value "$agent_config_path" "agent_backend")"
  fi
  if [ -z "$backend" ]; then
    backend="$(read_dotenv_value "$env_file" "SANDRONE_REBASE_AGENT_BACKEND")"
  fi
  if [ -z "$backend" ]; then
    backend="$(read_dotenv_value "$env_file" "SANDRONE_AGENT_BACKEND")"
  fi
  if [ -z "$backend" ]; then
    backend="codex-cli"
  fi
  normalize_backend "$backend"
}

resolve_rebase_agent_api_key() {
  value="${LLM_API_KEY:-}"
  if [ -z "$value" ]; then
    value="$(read_json_value "$agent_config_path" "api_key")"
  fi
  if [ -z "$value" ]; then
    value="$(read_dotenv_value "$env_file" "LLM_API_KEY")"
  fi
  printf '%s\n' "$value"
}

resolve_rebase_agent_base_url() {
  value="${LLM_BASE_URL:-}"
  if [ -z "$value" ]; then
    value="$(read_json_value "$agent_config_path" "base_url")"
  fi
  if [ -z "$value" ]; then
    value="$(read_dotenv_value "$env_file" "LLM_BASE_URL")"
  fi
  printf '%s\n' "$value"
}

if [ "$phase" != "rebase" ]; then
  echo "unsupported SANDRONE_AGENT_PHASE for rebase-agent: $phase" >&2
  exit 1
fi

resolved="$(resolve_rebase_agent_model)"
rebase_agent_model="$(printf '%s\n' "$resolved" | sed -n '1p')"
rebase_agent_reasoning_effort="$(printf '%s\n' "$resolved" | sed -n '2p')"
rebase_agent_backend="$(resolve_rebase_agent_backend)"

if ! codex_bin="$(resolve_codex_bin)"; then
  echo "replace tools/rebase-agent.sh with another agent backend if Codex CLI is not available" >&2
  exit 1
fi

if [ ! -f "$prompt" ]; then
  echo "rebase agent prompt does not exist: $prompt" >&2
  exit 1
fi

{
  printf 'Workspace: %s\n' "$workspace"
  printf 'Request ID: %s\n' "${SANDRONE_REQUEST_ID:-}"
  printf 'Agent phase: %s\n' "$phase"
  printf 'External ID: %s\n' "${SANDRONE_REQUEST_EXTERNAL_ID:-}"
  printf 'Source: %s\n' "${SANDRONE_REQUEST_SOURCE:-}"
  printf 'Requirement name: %s\n' "${SANDRONE_REQUEST_TITLE:-}"
  printf 'Max attempts: %s\n' "${SANDRONE_MAX_ATTEMPTS:-$default_max_attempts}"
  printf 'Request document: %s\n' "${SANDRONE_REQUEST:-}"
  printf 'Plan: %s\n' "${SANDRONE_PLAN:-}"
  printf 'Change doc: %s\n' "${SANDRONE_CHANGE_DOC:-}"
  printf 'Agent journal: %s\n' "${SANDRONE_AGENT_JOURNAL:-}"
  printf 'Agent status doc: %s\n' "${SANDRONE_AGENT_STATUS_DOC:-}"
  printf 'Env file: %s\n' "$env_file"
  printf 'Agent backend: %s\n' "$rebase_agent_backend"
  printf 'Ignore user Codex config: %s\n' "$(resolve_agent_ignore_user_config)"
  printf 'Resolved model: %s\n' "${rebase_agent_model:-inherited}"
  printf 'Resolved reasoning effort: %s\n' "${rebase_agent_reasoning_effort:-inherited}"
  printf 'Worktree: %s\n\n' "${SANDRONE_WORKTREE:-}"
  cat "$prompt"
} | {
  set -- \
    --cd "$workspace" \
    --skip-git-repo-check \
    -c 'approval_policy="never"' \
    -c 'shell_environment_policy.inherit="all"' \
    --sandbox workspace-write

  if truthy "$(resolve_agent_ignore_user_config)"; then
    set -- "$@" --ignore-user-config
  fi

  if [ -n "$rebase_agent_model" ]; then
    set -- "$@" -c "model=\"$rebase_agent_model\""
  fi
  if [ -n "$rebase_agent_reasoning_effort" ]; then
    set -- "$@" -c "model_reasoning_effort=\"$rebase_agent_reasoning_effort\""
  fi

  if [ "$rebase_agent_backend" = "codex-api" ]; then
    rebase_llm_api_key="$(resolve_rebase_agent_api_key)"
    rebase_llm_base_url="$(resolve_rebase_agent_base_url)"
    if [ -z "$rebase_llm_api_key" ]; then
      echo "codex-api backend requires LLM_API_KEY" >&2
      exit 1
    fi
    if [ -z "$rebase_llm_base_url" ]; then
      echo "codex-api backend requires LLM_BASE_URL" >&2
      exit 1
    fi
    codex_provider_id="$(resolve_dotenv_or_env "SANDRONE_CODEX_MODEL_PROVIDER")"
    codex_provider_name="$(resolve_dotenv_or_env "SANDRONE_CODEX_PROVIDER_NAME")"
    codex_wire_api="$(resolve_dotenv_or_env "SANDRONE_CODEX_WIRE_API")"
    codex_provider_id="${codex_provider_id:-sandrone-api}"
    codex_provider_name="${codex_provider_name:-Sandrone API}"
    codex_wire_api="${codex_wire_api:-responses}"
    escaped_provider_name="$(codex_toml_escape "$codex_provider_name")"
    escaped_base_url="$(codex_toml_escape "$rebase_llm_base_url")"
    escaped_wire_api="$(codex_toml_escape "$codex_wire_api")"
    set -- "$@" \
      -c "model_provider=\"${codex_provider_id}\"" \
      -c "model_providers.${codex_provider_id}={name=\"${escaped_provider_name}\", base_url=\"${escaped_base_url}\", env_key=\"LLM_API_KEY\", wire_api=\"${escaped_wire_api}\"}"
  fi

  if [ "$rebase_agent_backend" = "codex-api" ]; then
    LLM_API_KEY="$rebase_llm_api_key" nohup "$codex_bin" exec "$@" -
  else
    nohup "$codex_bin" exec "$@" -
  fi
}
