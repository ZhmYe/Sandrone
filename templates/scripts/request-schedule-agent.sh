#!/usr/bin/env sh
set -eu

# Request Schedule Agent connector.
#
# This default connector uses Codex CLI to choose which request ids may be
# dispatched in the current loop pass. Replace it with another LLM, a policy
# engine, or an internal scheduler if needed.
#
# It must not modify code, write request state, review code quality, create PRs,
# or merge PRs. It only prints TSV schedule decisions to stdout.
#
# Inputs:
# SANDRONE_REQUEST_SCHEDULE_QUEUE
# SANDRONE_REQUEST_SCHEDULE_MD
# SANDRONE_REQUEST_SCHEDULE_JSON
# SANDRONE_REQUEST_SCHEDULE_MAX_PARALLEL
#
# Queue TSV header:
# request_id<TAB>title<TAB>status<TAB>source<TAB>updated_at<TAB>change_path<TAB>branch<TAB>detail
#
# Output contract:
# - Print zero or more TSV lines.
# - Select at most SANDRONE_REQUEST_SCHEDULE_MAX_PARALLEL request ids.
# - selected<TAB>request_id<TAB>reason
# - defer<TAB>request_id-or-empty<TAB>reason
# - blocked<TAB>request_id-or-empty<TAB>reason

{{CODEX_BIN_RESOLVER}}

workspace="${SANDRONE_WORKSPACE:-$(pwd)}"
queue="${SANDRONE_REQUEST_SCHEDULE_QUEUE:-}"
max_parallel="${SANDRONE_REQUEST_SCHEDULE_MAX_PARALLEL:-1}"
env_file="${SANDRONE_ENV_FILE:-$workspace/.env}"
agent_config_path="${SANDRONE_AGENT_CONFIG_PATH:-${SANDRONE_AGENT_CONFIG_DIR:-agents/config}/request-schedule-agent.json}"

case "$max_parallel" in
  ''|*[!0-9]*) max_parallel=1 ;;
esac
if [ "$max_parallel" -lt 1 ]; then
  max_parallel=1
fi

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

read_json_value() {
  file="$1"
  key="$2"
  [ -f "$file" ] || return 0
  awk -v key="$key" '
    BEGIN { pattern = "^[[:space:]]*\"" key "\"[[:space:]]*:[[:space:]]*\"([^\"]*)\"" }
    {
      line=$0
      gsub(/\r/, "", line)
      if (match(line, pattern, m)) { print m[1]; exit }
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
      if (line ~ /^[[:space:]]*#/ || trim(line) == "") { next }
      idx=match(line, "=")
      if (idx <= 0) { next }
      k=trim(substr(line, 1, idx-1))
      if (k != key) { next }
      v=trim(substr(line, idx + 1))
      if (v ~ /^".*"$/) { sub(/^"/, "", v); sub(/"$/, "", v) }
      else if (v ~ /^'\''.*'\''$/) { sub(/^\x27/, "", v); sub(/\x27$/, "", v) }
      else { sub(/[[:space:]]#.*/, "", v) }
      print v
      exit
    }
  ' "$file"
}

resolve_config_value() {
  file="$1"
  key="$2"
  [ -f "$file" ] || return 0
  awk -F'"' -v key="$key" '$1 ~ ("^[[:space:]]*" key "[[:space:]]*=" ) { print $2; exit }' "$file"
}

resolve_dotenv_or_env() {
  key="$1"
  value="$(eval "printf '%s' \"\${$key:-}\"")"
  if [ -z "$value" ]; then
    value="$(read_dotenv_value "$env_file" "$key")"
  fi
  printf '%s\n' "$value"
}

truthy() {
  case "$1" in
    1|true|TRUE|yes|YES|on|ON) return 0 ;;
    *) return 1 ;;
  esac
}

normalize_backend() {
  case "$1" in
    "" ) printf '%s\n' "codex-cli" ;;
    codex) printf '%s\n' "codex-cli" ;;
    codex-api|codex_cli_api|codex-cli-api) printf '%s\n' "codex-api" ;;
    claude|claude-code) printf '%s\n' "claude-code" ;;
    *) printf '%s\n' "$1" ;;
  esac
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
  if [ -n "${CODEX_HOME:-}" ] && [ -f "$CODEX_HOME/models_cache.json" ]; then
    printf '%s\n' "$CODEX_HOME/models_cache.json"
    return 0
  fi
  if [ -n "${HOME:-}" ] && [ -f "$HOME/.codex/models_cache.json" ]; then
    printf '%s\n' "$HOME/.codex/models_cache.json"
    return 0
  fi
  tmp_catalog="$(mktemp "${TMPDIR:-/tmp}/Sandrone-model-catalog.XXXXXX")"
  if "$codex_bin_for_catalog" debug models --bundled > "$tmp_catalog" 2>/dev/null && [ -s "$tmp_catalog" ]; then
    printf '%s\n' "$tmp_catalog"
    return 0
  fi
  rm -f "$tmp_catalog"
  printf '%s\n' ""
}

if [ -z "$queue" ] || [ ! -f "$queue" ]; then
  printf 'blocked\t\trequest schedule queue is missing\n'
  exit 0
fi

if ! codex_bin="$(resolve_codex_bin)"; then
  printf 'blocked\t\tcodex CLI is not available for request scheduling\n'
  exit 0
fi

backend="${SANDRONE_REQUEST_SCHEDULE_AGENT_BACKEND:-${SANDRONE_AGENT_BACKEND:-${SANDRONE_BACKEND:-}}}"
if [ -z "$backend" ]; then
  backend="$(read_json_value "$agent_config_path" "agent_backend")"
fi
if [ -z "$backend" ]; then
  backend="$(read_dotenv_value "$env_file" "SANDRONE_REQUEST_SCHEDULE_AGENT_BACKEND")"
fi
if [ -z "$backend" ]; then
  backend="$(read_dotenv_value "$env_file" "SANDRONE_AGENT_BACKEND")"
fi
backend="$(normalize_backend "$backend")"

model="${SANDRONE_REQUEST_SCHEDULE_AGENT_MODEL:-${SANDRONE_AGENT_MODEL:-${SANDRONE_MODEL:-}}}"
reasoning="${SANDRONE_REQUEST_SCHEDULE_AGENT_REASONING_EFFORT:-${SANDRONE_AGENT_REASONING_EFFORT:-${SANDRONE_REASONING_EFFORT:-}}}"
if [ -z "$model" ]; then
  model="$(read_json_value "$agent_config_path" "model")"
fi
if [ -z "$reasoning" ]; then
  reasoning="$(read_json_value "$agent_config_path" "reasoning_effort")"
fi
if [ -z "$model" ]; then
  model="$(read_dotenv_value "$env_file" "SANDRONE_REQUEST_SCHEDULE_AGENT_MODEL")"
fi
if [ -z "$reasoning" ]; then
  reasoning="$(read_dotenv_value "$env_file" "SANDRONE_REQUEST_SCHEDULE_AGENT_REASONING_EFFORT")"
fi
if [ -z "$model" ]; then
  model="$(read_dotenv_value "$env_file" "SANDRONE_AGENT_MODEL")"
fi
if [ -z "$reasoning" ]; then
  reasoning="$(read_dotenv_value "$env_file" "SANDRONE_AGENT_REASONING_EFFORT")"
fi
if [ -z "$model" ] && [ -n "${CODEX_HOME:-}" ] && [ -f "$CODEX_HOME/config.toml" ]; then
  model="$(resolve_config_value "$CODEX_HOME/config.toml" "model")"
fi
if [ -z "$reasoning" ] && [ -n "${CODEX_HOME:-}" ] && [ -f "$CODEX_HOME/config.toml" ]; then
  reasoning="$(resolve_config_value "$CODEX_HOME/config.toml" "model_reasoning_effort")"
fi

prompt_file="$(mktemp "${TMPDIR:-/tmp}/sandrone-request-schedule.XXXXXX.md")"
output_file="$(mktemp "${TMPDIR:-/tmp}/sandrone-request-schedule-output.XXXXXX")"
cleanup() {
  rm -f "$prompt_file" "$output_file"
}
trap cleanup EXIT

{
  printf '# Sandrone Request Schedule Agent\n\n'
  printf '你是 Sandrone 的 RequestScheduleAgent。你只决定本轮 loop 最多派发哪些 request 进入实现流程；你不写代码、不改状态、不创建 PR、不合并 PR。\n\n'
  printf '## 调度目标\n\n'
  printf -- '- 从当前 queue 中选择 `0..%s` 个 request，作为本轮可并行实现集合。\n' "$max_parallel"
  printf -- '- 不必一定选满 %s 个；如果依赖、冲突域或上下文不清楚，可以少选。\n' "$max_parallel"
  printf -- '- 目标是“大概率独立、可并行推进”，不是数学证明完全不冲突；小范围可接受冲突可以并行，但必须在 reason 说明。\n'
  printf -- '- 优先选择依赖已满足、不会明显互相修改同一核心模块、不会互相改变同一公共接口/数据模型/迁移路径的 request。\n'
  printf -- '- 如果某个 request 明显依赖另一个未完成 request 的输出，先选前置 request。\n'
  printf -- '- 如果两个 request 都很小、影响域不同，允许同轮选择以提高并行度。\n'
  printf -- '- 不得选择 blocked、finished、wait-update-pr、wait-finish、running 或依赖未满足的 request；如果它们出现在 queue 中也要 defer。\n'
  printf -- '- 不得考虑 PR 合并优先级；实现顺序就是后续合入顺序的主要依据。PR 质量由每个 request 的最后一个 slice code-review 负责。\n\n'
  printf '## 输出格式\n\n'
  printf 'stdout 必须只包含 TSV 行，不要 Markdown，不要解释段落，不要代码块。\n\n'
  printf '允许的行格式:\n\n'
  printf 'selected<TAB>request_id<TAB>reason\n'
  printf 'defer<TAB>request_id-or-empty<TAB>reason\n'
  printf 'blocked<TAB>request_id-or-empty<TAB>reason\n\n'
  printf 'reason 必须简洁但可审计，说明依赖、冲突域和为什么适合本轮或为什么暂缓。\n\n'
  printf '## 当前配置\n\n'
  printf -- '- max_parallel: `%s`\n' "$max_parallel"
  printf -- '- queue path: `%s`\n\n' "$queue"
  printf '## 当前候选 Queue\n\n'
  printf '```tsv\n'
  cat "$queue"
  printf '\n```\n'
} > "$prompt_file"

set -- \
  --cd "$workspace" \
  --skip-git-repo-check \
  -c 'approval_policy="never"' \
  -c 'shell_environment_policy.inherit="all"' \
  --sandbox workspace-write

ignore_user_config="$(resolve_dotenv_or_env "SANDRONE_AGENT_IGNORE_USER_CONFIG")"
if truthy "${ignore_user_config:-1}"; then
  set -- "$@" --ignore-user-config
fi
if [ -n "$model" ]; then
  set -- "$@" -c "model=\"${model}\""
fi
if [ -n "$reasoning" ]; then
  set -- "$@" -c "model_reasoning_effort=\"${reasoning}\""
fi
if [ "$backend" = "codex-cli" ] || [ "$backend" = "codex-api" ]; then
  catalog="$(resolve_or_generate_codex_model_catalog_json "$codex_bin")"
  if [ -n "$catalog" ]; then
    set -- "$@" -c "model_catalog_json=\"$(codex_toml_escape "$catalog")\""
  fi
fi
if [ "$backend" = "codex-api" ]; then
  api_key="${LLM_API_KEY:-$(read_json_value "$agent_config_path" "api_key")}"
  base_url="${LLM_BASE_URL:-$(read_json_value "$agent_config_path" "base_url")}"
  if [ -z "$api_key" ]; then
    api_key="$(read_dotenv_value "$env_file" "LLM_API_KEY")"
  fi
  if [ -z "$base_url" ]; then
    base_url="$(read_dotenv_value "$env_file" "LLM_BASE_URL")"
  fi
  if [ -z "$api_key" ] || [ -z "$base_url" ]; then
    printf 'blocked\t\tcodex-api request schedule requires LLM_API_KEY and LLM_BASE_URL\n'
    exit 0
  fi
  provider_id="$(resolve_dotenv_or_env "SANDRONE_CODEX_MODEL_PROVIDER")"
  provider_name="$(resolve_dotenv_or_env "SANDRONE_CODEX_PROVIDER_NAME")"
  wire_api="$(resolve_dotenv_or_env "SANDRONE_CODEX_WIRE_API")"
  provider_id="${provider_id:-sandrone-api}"
  provider_name="${provider_name:-Sandrone API}"
  wire_api="${wire_api:-responses}"
  set -- "$@" \
    -c "model_provider=\"${provider_id}\"" \
    -c "model_providers.${provider_id}={name=\"$(codex_toml_escape "$provider_name")\", base_url=\"$(codex_toml_escape "$base_url")\", env_key=\"LLM_API_KEY\", wire_api=\"$(codex_toml_escape "$wire_api")\"}"
  LLM_API_KEY="$api_key" "$codex_bin" exec "$@" - < "$prompt_file" > "$output_file" 2>&1 || {
    printf 'blocked\t\trequest schedule codex-api failed: %s\n' "$(head -n 20 "$output_file" | tr '\n' ' ' | sed 's/\t/ /g')"
    exit 0
  }
elif [ "$backend" = "codex-cli" ]; then
  "$codex_bin" exec "$@" - < "$prompt_file" > "$output_file" 2>&1 || {
    printf 'blocked\t\trequest schedule codex-cli failed: %s\n' "$(head -n 20 "$output_file" | tr '\n' ' ' | sed 's/\t/ /g')"
    exit 0
  }
elif [ "$backend" = "claude-code" ]; then
  printf 'blocked\t\tclaude-code request schedule backend is reserved but not implemented by the default connector\n'
  exit 0
else
  printf 'blocked\t\tunknown request schedule backend: %s\n' "$backend"
  exit 0
fi

awk -F '\t' '
  $1 == "selected" || $1 == "defer" || $1 == "blocked" { print; matched = 1 }
  END {
    if (!matched) {
      print "blocked\t\trequest schedule LLM returned no valid TSV decision rows"
    }
  }
' "$output_file"
