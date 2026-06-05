#!/usr/bin/env sh
set -eu
trap '' HUP

# Replace this script to use Claude Code, OpenAI API, an internal agent,
# or any other implementation backend. The script processes exactly one
# request phase: decomposition, planning, or implementation. The outer sandrone advance/tick
# owns submit, reviewer gates, start, finish, commit, push, PR creation,
# and phase transitions.
#
# Connector contract:
# - Inputs are provided through SANDRONE_* environment variables.
# - SANDRONE_AGENT_PHASE is decomposition, planning, or implementation.
# - The agent MUST read SANDRONE_REQUEST, SANDRONE_PLAN, and SANDRONE_AGENT_JOURNAL.
# - decomposition agents MUST write reviewable decomposition.md, decomposition.json, and dag.json, then exit.
# - planning agents MUST write a reviewable plan.md and then exit.
# - implementation agents MUST work only inside SANDRONE_WORKTREE, update change-doc.md, and then exit.
# - The agent MUST NOT call sandrone submit/plan-review/code-review/start/finish.
# - The agent MUST NOT call sandrone approve/reject or edit approval JSON.
# - A historical review summary with gate_unavailable=true is not a reason to block after resume.
#   The agent should fix artifacts and exit 0 so outer advance can create a fresh review attempt.
# - The agent MUST append recovery-oriented notes to SANDRONE_AGENT_JOURNAL.
# - Success means the phase artifact is ready for the outer advance/tick review gate.
# - Failure should exit non-zero with a helpful stderr message.

{{CODEX_BIN_RESOLVER}}

workspace="${SANDRONE_WORKSPACE:-$(pwd)}"
phase="${SANDRONE_AGENT_PHASE:-planning}"
shared_prompt="${SANDRONE_ISSUE_AGENT_SHARED_PROMPT:-tools/prompts/issue-agent.md}"
env_file="${SANDRONE_ENV_FILE:-$workspace/.env}"
case "$phase" in
  decomposition|planning) default_max_attempts=5 ;;
  implementation) default_max_attempts=20 ;;
  *) default_max_attempts=20 ;;
esac

source_codex_home="${CODEX_HOME:-}"
if [ -z "$source_codex_home" ] && [ -n "${HOME:-}" ]; then
  source_codex_home="${HOME}/.codex"
fi

resolve_config_value() {
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
      split(line, kv, "=")
      if (length(kv) < 2) {
        next
      }
      k = trim(kv[1])
      if (k != key) {
        next
      }
      v = substr(line, index(line, "=") + 1)
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

resolve_agent_model() {
  phase="$1"
  env_key_model=""
  env_key_reasoning=""
  env_model=""
  env_reasoning=""

  case "$phase" in
    decomposition)
      env_key_model="SANDRONE_DECOMPOSITION_AGENT_MODEL"
      env_key_reasoning="SANDRONE_DECOMPOSITION_AGENT_REASONING_EFFORT"
      env_model="${SANDRONE_DECOMPOSITION_AGENT_MODEL:-}"
      env_reasoning="${SANDRONE_DECOMPOSITION_AGENT_REASONING_EFFORT:-}"
      ;;
    planning)
      env_key_model="SANDRONE_PLAN_AGENT_MODEL"
      env_key_reasoning="SANDRONE_PLAN_AGENT_REASONING_EFFORT"
      env_model="${SANDRONE_PLAN_AGENT_MODEL:-}"
      env_reasoning="${SANDRONE_PLAN_AGENT_REASONING_EFFORT:-}"
      ;;
    implementation)
      env_key_model="SANDRONE_IMPLEMENTATION_AGENT_MODEL"
      env_key_reasoning="SANDRONE_IMPLEMENTATION_AGENT_REASONING_EFFORT"
      env_model="${SANDRONE_IMPLEMENTATION_AGENT_MODEL:-}"
      env_reasoning="${SANDRONE_IMPLEMENTATION_AGENT_REASONING_EFFORT:-}"
      ;;
    *)
      env_key_model="SANDRONE_AGENT_MODEL"
      env_key_reasoning="SANDRONE_AGENT_REASONING_EFFORT"
      ;;
  esac

  if [ -z "$env_model" ]; then
    env_model="${SANDRONE_AGENT_MODEL:-${SANDRONE_MODEL:-}}"
  fi
  if [ -z "$env_reasoning" ]; then
    env_reasoning="${SANDRONE_AGENT_REASONING_EFFORT:-${SANDRONE_REASONING_EFFORT:-}}"
  fi

  if [ -z "$env_model" ]; then
    env_model="$(read_dotenv_value "$env_file" "$env_key_model")"
  fi
  if [ -z "$env_reasoning" ]; then
    env_reasoning="$(read_dotenv_value "$env_file" "$env_key_reasoning")"
  fi

  if [ -z "$env_model" ]; then
    env_model="$(read_dotenv_value "$env_file" "SANDRONE_AGENT_MODEL")"
  fi
  if [ -z "$env_reasoning" ]; then
    env_reasoning="$(read_dotenv_value "$env_file" "SANDRONE_AGENT_REASONING_EFFORT")"
  fi
  if [ -z "$env_model" ]; then
    env_model="$(read_dotenv_value "$env_file" "SANDRONE_MODEL")"
  fi
  if [ -z "$env_reasoning" ]; then
    env_reasoning="$(read_dotenv_value "$env_file" "SANDRONE_REASONING_EFFORT")"
  fi

  if [ -z "$env_model" ] && [ -n "$source_codex_home" ] && [ -f "$source_codex_home/config.toml" ]; then
    env_model="$(resolve_config_value "$source_codex_home/config.toml" "model")"
  fi
  if [ -z "$env_reasoning" ] && [ -n "$source_codex_home" ] && [ -f "$source_codex_home/config.toml" ]; then
    env_reasoning="$(resolve_config_value "$source_codex_home/config.toml" "model_reasoning_effort")"
  fi

  printf "%s\n%s\n" "$env_model" "$env_reasoning"
}

resolved="$(resolve_agent_model "$phase")"
agent_model="$(printf "%s\n" "$resolved" | sed -n '1p')"
agent_reasoning_effort="$(printf "%s\n" "$resolved" | sed -n '2p')"
case "$phase" in
  decomposition) default_prompt="tools/prompts/decomposition-agent.md" ;;
  planning) default_prompt="tools/prompts/plan-agent.md" ;;
  implementation) default_prompt="tools/prompts/implementation-agent.md" ;;
  *)
    echo "unsupported SANDRONE_AGENT_PHASE: $phase" >&2
    exit 1
    ;;
esac
prompt="${SANDRONE_ISSUE_AGENT_PROMPT:-$default_prompt}"

if ! codex_bin="$(resolve_codex_bin)"; then
  echo "replace tools/issue-agent.sh with another agent backend if Codex CLI is not available" >&2
  exit 1
fi
if [ ! -f "$prompt" ]; then
  echo "agent prompt does not exist: $prompt" >&2
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
  printf 'Decomposition: %s\n' "${SANDRONE_DECOMPOSITION:-}"
  printf 'DAG: %s\n' "${SANDRONE_DAG:-}"
  printf 'Change doc: %s\n' "${SANDRONE_CHANGE_DOC:-}"
  printf 'Agent journal: %s\n' "${SANDRONE_AGENT_JOURNAL:-}"
  printf 'Check format tool: %s\n' "${SANDRONE_CHECK_FORMAT_TOOL:-tools/check-format.sh}"
  printf 'CodeGraph context: %s\n' "${SANDRONE_CODEGRAPH_CONTEXT:-}"
  printf 'Obsidian project: %s\n' "${SANDRONE_OBSIDIAN_PROJECT:-}"
  printf 'Obsidian note: %s\n' "${SANDRONE_OBSIDIAN_NOTE:-}"
  printf 'Worktree: %s\n' "${SANDRONE_WORKTREE:-}"
  printf 'Env file: %s\n' "$env_file"
  printf 'Resolved model: %s\n' "${agent_model:-inherited}"
  printf 'Resolved reasoning effort: %s\n\n' "${agent_reasoning_effort:-inherited}"
  if [ -f "$shared_prompt" ]; then
    cat "$shared_prompt"
    printf '\n\n'
  fi
  cat "$prompt"
} | {
  set -- \
    --cd "$workspace" \
    --skip-git-repo-check \
    -c 'approval_policy="never"' \
    -c 'shell_environment_policy.inherit="all"' \
    --sandbox workspace-write

  if [ -n "$agent_model" ]; then
    set -- "$@" -c "model=\"${agent_model}\""
  fi
  if [ -n "$agent_reasoning_effort" ]; then
    set -- "$@" -c "model_reasoning_effort=\"${agent_reasoning_effort}\""
  fi

  nohup "$codex_bin" exec "$@" -
}
