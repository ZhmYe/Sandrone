#!/usr/bin/env sh
set -eu
trap '' HUP

# Replace this script to use Claude Code, OpenAI API, an internal agent,
# or any other implementation backend. The script processes exactly one
# request phase: planning or implementation. The outer codex-auto-dev advance/tick
# owns submit, reviewer gates, start, finish, commit, push, PR creation,
# and phase transitions.
#
# Connector contract:
# - Inputs are provided through CODEX_AUTO_DEV_* environment variables.
# - CODEX_AUTO_DEV_AGENT_PHASE is planning or implementation.
# - The agent MUST read CODEX_AUTO_DEV_REQUEST, CODEX_AUTO_DEV_PLAN, and CODEX_AUTO_DEV_AGENT_JOURNAL.
# - planning agents MUST write a reviewable plan.md and then exit.
# - implementation agents MUST work only inside CODEX_AUTO_DEV_WORKTREE, update change-doc.md, and then exit.
# - The agent MUST NOT call codex-auto-dev submit/plan-review/code-review/start/finish.
# - The agent MUST NOT call codex-auto-dev approve/reject or edit approval JSON.
# - If a review summary has gate_unavailable=true, the agent MUST block instead of retrying or bypassing.
# - The agent MUST append recovery-oriented notes to CODEX_AUTO_DEV_AGENT_JOURNAL.
# - Success means the phase artifact is ready for the outer advance/tick review gate.
# - Failure should exit non-zero with a helpful stderr message.

{{CODEX_BIN_RESOLVER}}

workspace="${CODEX_AUTO_DEV_WORKSPACE:-$(pwd)}"
phase="${CODEX_AUTO_DEV_AGENT_PHASE:-planning}"
shared_prompt="${CODEX_AUTO_DEV_ISSUE_AGENT_SHARED_PROMPT:-tools/prompts/issue-agent.md}"
case "$phase" in
  planning) default_prompt="tools/prompts/plan-agent.md" ;;
  implementation) default_prompt="tools/prompts/implementation-agent.md" ;;
  *)
    echo "unsupported CODEX_AUTO_DEV_AGENT_PHASE: $phase" >&2
    exit 1
    ;;
esac
prompt="${CODEX_AUTO_DEV_ISSUE_AGENT_PROMPT:-$default_prompt}"

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
  printf 'Request ID: %s\n' "${CODEX_AUTO_DEV_REQUEST_ID:-}"
  printf 'Agent phase: %s\n' "$phase"
  printf 'External ID: %s\n' "${CODEX_AUTO_DEV_REQUEST_EXTERNAL_ID:-}"
  printf 'Source: %s\n' "${CODEX_AUTO_DEV_REQUEST_SOURCE:-}"
  printf 'Requirement name: %s\n' "${CODEX_AUTO_DEV_REQUEST_TITLE:-}"
  printf 'Max attempts: %s\n' "${CODEX_AUTO_DEV_MAX_ATTEMPTS:-20}"
  printf 'Request document: %s\n' "${CODEX_AUTO_DEV_REQUEST:-}"
  printf 'Plan: %s\n' "${CODEX_AUTO_DEV_PLAN:-}"
  printf 'Change doc: %s\n' "${CODEX_AUTO_DEV_CHANGE_DOC:-}"
  printf 'Agent journal: %s\n' "${CODEX_AUTO_DEV_AGENT_JOURNAL:-}"
  printf 'Worktree: %s\n\n' "${CODEX_AUTO_DEV_WORKTREE:-}"
  if [ -f "$shared_prompt" ]; then
    cat "$shared_prompt"
    printf '\n\n'
  fi
  cat "$prompt"
} | nohup "$codex_bin" exec \
  --cd "$workspace" \
  --skip-git-repo-check \
  -c 'approval_policy="never"' \
  -c 'shell_environment_policy.inherit="all"' \
  --sandbox workspace-write \
  -
