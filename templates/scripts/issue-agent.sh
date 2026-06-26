#!/usr/bin/env sh
set -eu
trap '' HUP

# Replace this script to use Codex CLI, Codex API provider config, Claude Code,
# an internal agent, or any other implementation backend. The script processes exactly one
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
# - The agent MUST NOT call sandrone approve/reject or edit document gate frontmatter.
# - A historical review summary with gate_unavailable=true is not a reason to block after resume.
#   The agent should fix artifacts and exit 0 so outer advance can create a fresh review attempt.
# - The launcher provides latest review summary/detail paths in the prompt. Agents should not scan
#   the entire reviews/ history unless those latest files explicitly require older evidence.
# - The agent MUST append recovery-oriented notes to SANDRONE_AGENT_JOURNAL.
# - Success means the phase artifact is ready for the outer advance/tick review gate.
#   When successful, mark SANDRONE_AGENT_STATUS_DOC frontmatter as submitted.
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

resolve_agent_backend() {
  phase="$1"
  env_key_backend="SANDRONE_AGENT_BACKEND"
  backend=""

  case "$phase" in
    decomposition)
      env_key_backend="SANDRONE_DECOMPOSITION_AGENT_BACKEND"
      backend="${SANDRONE_DECOMPOSITION_AGENT_BACKEND:-}"
      ;;
    planning)
      env_key_backend="SANDRONE_PLAN_AGENT_BACKEND"
      backend="${SANDRONE_PLAN_AGENT_BACKEND:-}"
      ;;
    implementation)
      env_key_backend="SANDRONE_IMPLEMENTATION_AGENT_BACKEND"
      backend="${SANDRONE_IMPLEMENTATION_AGENT_BACKEND:-}"
      ;;
  esac

  if [ -z "$backend" ]; then
    backend="$(read_dotenv_value "$env_file" "$env_key_backend")"
  fi
  if [ -z "$backend" ]; then
    backend="${SANDRONE_AGENT_BACKEND:-${SANDRONE_BACKEND:-}}"
  fi
  if [ -z "$backend" ]; then
    backend="$(read_dotenv_value "$env_file" "SANDRONE_AGENT_BACKEND")"
  fi
  if [ -z "$backend" ]; then
    backend="$(read_dotenv_value "$env_file" "SANDRONE_BACKEND")"
  fi
  if [ -z "$backend" ]; then
    backend="codex-cli"
  fi
  normalize_backend "$backend"
}

agent_backend="$(resolve_agent_backend "$phase")"

resolve_llm_api_key() {
  key="${LLM_API_KEY:-}"
  if [ -z "$key" ]; then
    key="$(read_dotenv_value "$env_file" "LLM_API_KEY")"
  fi
  printf '%s\n' "$key"
}

resolve_llm_base_url() {
  value="${LLM_BASE_URL:-}"
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

review_stage_for_phase() {
  case "$1" in
    decomposition) printf '%s\n' "decomposition-review" ;;
    planning) printf '%s\n' "plan-review" ;;
    implementation) printf '%s\n' "code-review" ;;
    *) printf '%s\n' "" ;;
  esac
}

latest_review_attempt_prefix() {
  details_dir="$1"
  [ -d "$details_dir" ] || return 0
  find "$details_dir" -type f -name '[0-9][0-9][0-9]-*.json' 2>/dev/null \
    | sed 's#.*/##' \
    | sed 's/-.*//' \
    | sort \
    | tail -n 1
}

review_detail_files_for_prefix() {
  details_dir="$1"
  prefix="$2"
  [ -n "$prefix" ] || return 0
  [ -d "$details_dir" ] || return 0
  find "$details_dir" -type f -name "${prefix}-*.json" 2>/dev/null | sort
}

latest_actionable_review_details() {
  details_dir="$1"
  [ -d "$details_dir" ] || return 0
  for stem in decomposition-reviewer plan-reviewer test-reviewer design-reviewer integration-reviewer; do
    find "$details_dir" -type f -name "[0-9][0-9][0-9]-${stem}.json" 2>/dev/null \
      | sort -r \
      | while IFS= read -r file; do
          if grep -q '"gate_unavailable"[[:space:]]*:[[:space:]]*false' "$file"; then
            printf '%s\n' "$file"
            break
          fi
        done
  done | sort -u
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
  codex_model_catalog_file="$(mktemp "${TMPDIR:-/tmp}/Sandrone-model-catalog.XXXXXX")"
  if "$codex_bin_for_catalog" debug models --bundled > "$codex_model_catalog_file" 2>/dev/null \
    && [ -s "$codex_model_catalog_file" ]; then
    printf '%s\n' "$codex_model_catalog_file"
    return 0
  fi
  rm -f "$codex_model_catalog_file"
  codex_model_catalog_file=""
  printf '%s\n' ""
}

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

if [ ! -f "$prompt" ]; then
  echo "agent prompt does not exist: $prompt" >&2
  exit 1
fi

review_stage="$(review_stage_for_phase "$phase")"
review_summary_path=""
latest_review_details=""
latest_actionable_review_details=""
if [ -n "${SANDRONE_CHANGE_PATH:-}" ] && [ -n "$review_stage" ]; then
  review_summary_path="${SANDRONE_CHANGE_PATH}/reviews/${review_stage}/summary.json"
  review_details_dir="${SANDRONE_CHANGE_PATH}/reviews/${review_stage}/details"
  latest_review_prefix="$(latest_review_attempt_prefix "$review_details_dir")"
  latest_review_details="$(review_detail_files_for_prefix "$review_details_dir" "$latest_review_prefix")"
  latest_actionable_review_details="$(latest_actionable_review_details "$review_details_dir")"
fi
export SANDRONE_LATEST_REVIEW_STAGE="$review_stage"
export SANDRONE_LATEST_REVIEW_SUMMARY="$review_summary_path"
export SANDRONE_LATEST_REVIEW_DETAILS="$latest_review_details"
export SANDRONE_LATEST_ACTIONABLE_REVIEW_DETAILS="$latest_actionable_review_details"

agent_prompt_file="${TMPDIR:-/tmp}/sandrone-agent-${SANDRONE_REQUEST_ID:-unknown}-$$.prompt.md"
codex_model_catalog_file=""
cleanup_agent_tmp() {
  rm -f "$agent_prompt_file"
  [ -n "$codex_model_catalog_file" ] && rm -f "$codex_model_catalog_file"
}
trap cleanup_agent_tmp EXIT

render_agent_prompt() {
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
  printf 'Agent status doc: %s\n' "${SANDRONE_AGENT_STATUS_DOC:-}"
  printf 'Check format tool: %s\n' "${SANDRONE_CHECK_FORMAT_TOOL:-tools/check-format.sh}"
  printf 'CodeGraph context: %s\n' "${SANDRONE_CODEGRAPH_CONTEXT:-}"
  printf 'Obsidian project: %s\n' "${SANDRONE_OBSIDIAN_PROJECT:-}"
  printf 'Obsidian note: %s\n' "${SANDRONE_OBSIDIAN_NOTE:-}"
  printf 'Worktree: %s\n' "${SANDRONE_WORKTREE:-}"
  printf 'Env file: %s\n' "$env_file"
  printf 'Agent backend: %s\n' "$agent_backend"
  printf 'Ignore user Codex config: %s\n' "$(resolve_agent_ignore_user_config)"
  printf 'Resume session id: %s\n' "${SANDRONE_AGENT_RESUME_SESSION_ID:-}"
  printf 'Resolved model: %s\n' "${agent_model:-inherited}"
  printf 'Resolved reasoning effort: %s\n' "${agent_reasoning_effort:-inherited}"
  printf 'Review stage: %s\n' "${review_stage:-none}"
  printf 'Latest review summary: %s\n' "${review_summary_path:-}"
  printf 'Latest review detail files:\n'
  if [ -n "$latest_review_details" ]; then
    printf '%s\n' "$latest_review_details" | sed 's/^/- /'
  else
    printf '%s\n' "- none"
  fi
  printf 'Latest actionable non-unavailable review detail files:\n'
  if [ -n "$latest_actionable_review_details" ]; then
    printf '%s\n' "$latest_actionable_review_details" | sed 's/^/- /'
  else
    printf '%s\n' "- none"
  fi
  printf '\n'
  if [ -f "$shared_prompt" ]; then
    cat "$shared_prompt"
    printf '\n\n'
  fi
  cat "$prompt"
}

append_agent_block_note() {
  reason="$1"
  journal="${SANDRONE_AGENT_JOURNAL:-}"
  if [ -n "$journal" ]; then
    mkdir -p "$(dirname "$journal")"
    {
      printf '\n## Agent Blocked - %s\n\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
      printf -- '- Backend: `%s`\n' "$agent_backend"
      printf -- '- Phase: `%s`\n' "$phase"
      printf -- '- Reason: %s\n' "$reason"
    } >> "$journal"
  fi
}

run_codex_agent() {
  if ! codex_bin="$(resolve_codex_bin)"; then
    echo "replace tools/issue-agent.sh with another agent backend if Codex CLI is not available" >&2
    exit 1
  fi
  codex_llm_api_key=""
  resume_session_id="${SANDRONE_AGENT_RESUME_SESSION_ID:-}"
  if [ -n "$resume_session_id" ]; then
    set -- \
      --skip-git-repo-check \
      -c 'approval_policy="never"' \
      -c 'shell_environment_policy.inherit="all"'
  else
    set -- \
      --cd "$workspace" \
      --skip-git-repo-check \
      -c 'approval_policy="never"' \
      -c 'shell_environment_policy.inherit="all"' \
      --sandbox workspace-write
  fi
  if truthy "$(resolve_agent_ignore_user_config)"; then
    set -- "$@" --ignore-user-config
  fi

  if [ -n "$agent_model" ]; then
    set -- "$@" -c "model=\"${agent_model}\""
  fi
  if [ -n "$agent_reasoning_effort" ]; then
    set -- "$@" -c "model_reasoning_effort=\"${agent_reasoning_effort}\""
  fi
  if [ "$agent_backend" = "codex-cli" ] || [ "$agent_backend" = "codex-api" ]; then
    codex_model_catalog_json="$(resolve_or_generate_codex_model_catalog_json "$codex_bin")"
    if [ -n "$codex_model_catalog_json" ]; then
      escaped_catalog_json="$(codex_toml_escape "$codex_model_catalog_json")"
      set -- "$@" -c "model_catalog_json=\"${escaped_catalog_json}\""
    fi
  fi
  if [ "$agent_backend" = "codex-api" ]; then
    codex_llm_api_key="$(resolve_llm_api_key)"
    codex_llm_base_url="$(resolve_llm_base_url)"
    codex_provider_id="$(resolve_dotenv_or_env "SANDRONE_CODEX_MODEL_PROVIDER")"
    codex_provider_name="$(resolve_dotenv_or_env "SANDRONE_CODEX_PROVIDER_NAME")"
    codex_wire_api="$(resolve_dotenv_or_env "SANDRONE_CODEX_WIRE_API")"
    codex_provider_id="${codex_provider_id:-sandrone-api}"
    codex_provider_name="${codex_provider_name:-Sandrone API}"
    codex_wire_api="${codex_wire_api:-responses}"
    if [ -z "$codex_llm_api_key" ]; then
      append_agent_block_note "codex-api backend requires LLM_API_KEY"
      echo "codex-api backend requires LLM_API_KEY" >&2
      exit 1
    fi
    if [ -z "$codex_llm_base_url" ]; then
      append_agent_block_note "codex-api backend requires LLM_BASE_URL"
      echo "codex-api backend requires LLM_BASE_URL" >&2
      exit 1
    fi
    escaped_provider_name="$(codex_toml_escape "$codex_provider_name")"
    escaped_base_url="$(codex_toml_escape "$codex_llm_base_url")"
    escaped_wire_api="$(codex_toml_escape "$codex_wire_api")"
    set -- "$@" \
      -c "model_provider=\"${codex_provider_id}\"" \
      -c "model_providers.${codex_provider_id}={name=\"${escaped_provider_name}\", base_url=\"${escaped_base_url}\", env_key=\"LLM_API_KEY\", wire_api=\"${escaped_wire_api}\"}"
  fi

  if [ "$agent_backend" = "codex-api" ]; then
    if [ -n "$resume_session_id" ]; then
      LLM_API_KEY="$codex_llm_api_key" nohup "$codex_bin" exec resume "$@" "$resume_session_id" - < "$agent_prompt_file"
    else
      LLM_API_KEY="$codex_llm_api_key" nohup "$codex_bin" exec "$@" - < "$agent_prompt_file"
    fi
  else
    if [ -n "$resume_session_id" ]; then
      nohup "$codex_bin" exec resume "$@" "$resume_session_id" - < "$agent_prompt_file"
    else
      nohup "$codex_bin" exec "$@" - < "$agent_prompt_file"
    fi
  fi
}

render_agent_prompt > "$agent_prompt_file"

case "$agent_backend" in
  codex-cli|codex-api)
    run_codex_agent
    ;;
  claude-code)
    append_agent_block_note "claude-code agent backend is reserved but not implemented by the default connector"
    echo "claude-code agent backend is reserved but not implemented by the default connector" >&2
    exit 1
    ;;
  *)
    append_agent_block_note "unknown agent backend: $agent_backend"
    echo "unknown agent backend: $agent_backend" >&2
    exit 1
    ;;
esac
