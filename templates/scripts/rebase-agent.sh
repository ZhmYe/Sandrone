#!/usr/bin/env sh
set -eu
trap '' HUP

# Replace this script to use Claude Code, OpenAI API, an internal agent,
# or any other rebase/conflict-resolution backend. This connector handles
# exactly one phase: rebase.
#
# Connector contract:
# - Inputs are provided through CODEX_AUTO_DEV_* environment variables.
# - CODEX_AUTO_DEV_AGENT_PHASE is rebase.
# - The agent MUST work only inside CODEX_AUTO_DEV_WORKTREE.
# - The agent MUST preserve both base/master changes and request-branch changes.
# - The agent MUST NOT delete base/master code merely to keep its own branch easy.
# - The agent MUST update CODEX_AUTO_DEV_CHANGE_DOC and CODEX_AUTO_DEV_AGENT_JOURNAL.
# - The agent MUST NOT call codex-auto-dev approve/reject, integration-review, finish, commit, push, or PR commands.
# - Success means the rebase is complete, conflict markers are gone, and the worktree is ready for outer IntegrationReviewer.
# - Failure should exit non-zero with a helpful stderr message.

{{CODEX_BIN_RESOLVER}}

workspace="${CODEX_AUTO_DEV_WORKSPACE:-$(pwd)}"
phase="${CODEX_AUTO_DEV_AGENT_PHASE:-rebase}"
prompt="${CODEX_AUTO_DEV_REBASE_AGENT_PROMPT:-${CODEX_AUTO_DEV_AGENT_PROMPT:-tools/prompts/rebase-agent.md}}"

if [ "$phase" != "rebase" ]; then
  echo "unsupported CODEX_AUTO_DEV_AGENT_PHASE for rebase-agent: $phase" >&2
  exit 1
fi

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
  cat "$prompt"
} | nohup "$codex_bin" exec \
  --cd "$workspace" \
  --skip-git-repo-check \
  -c 'approval_policy="never"' \
  -c 'shell_environment_policy.inherit="all"' \
  --sandbox workspace-write \
  -
