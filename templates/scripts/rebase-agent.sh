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

resolve_rebase_agent_model() {
  rebase_agent_model="${SANDRONE_REBASE_AGENT_MODEL:-${SANDRONE_AGENT_MODEL:-${SANDRONE_MODEL:-}}}"
  rebase_agent_reasoning_effort="${SANDRONE_REBASE_AGENT_REASONING_EFFORT:-${SANDRONE_AGENT_REASONING_EFFORT:-${SANDRONE_REASONING_EFFORT:-}}}"

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

if [ "$phase" != "rebase" ]; then
  echo "unsupported SANDRONE_AGENT_PHASE for rebase-agent: $phase" >&2
  exit 1
fi

resolved="$(resolve_rebase_agent_model)"
rebase_agent_model="$(printf '%s\n' "$resolved" | sed -n '1p')"
rebase_agent_reasoning_effort="$(printf '%s\n' "$resolved" | sed -n '2p')"

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

  nohup "$codex_bin" exec "$@" -
}
