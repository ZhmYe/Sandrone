#!/usr/bin/env sh
set -eu

# Replace this script for GitLab, Gerrit, Bitbucket, internal workspaces,
# or any other code review system.
#
# Input is provided through environment variables:
# SANDRONE_REQUEST_ID
# SANDRONE_REQUEST_EXTERNAL_ID
# SANDRONE_REQUEST_SOURCE
# SANDRONE_REQUEST_TITLE
# SANDRONE_REQUEST_URL
# SANDRONE_CHANGE_PATH
# SANDRONE_WORKTREE
# SANDRONE_PR_BASE
# SANDRONE_PR_HEAD
# SANDRONE_PR_COMPARE_URL
#
# Connector contract:
# - This script observes PR state only. It must not modify code, branches, or PRs.
# - Print exactly one TSV line to stdout:
#   status<TAB>url<TAB>detail
# - Recommended status values: open, missing, merged, closed, unknown.
# - Sandrone marks a request finished only when status is merged.
# - detail may include platform-specific merge/conflict/outdated notes.
# - Exit non-zero only when the platform check itself is unsafe or impossible.

cd "${SANDRONE_WORKTREE}"

if ! command -v gh >/dev/null 2>&1; then
  printf 'unknown\t%s\tgh is not installed\n' "${SANDRONE_PR_COMPARE_URL:-}"
  exit 0
fi

if ! gh repo view >/dev/null 2>&1; then
  printf 'unknown\t%s\tgh cannot access this repository or this is not a GitHub repository\n' "${SANDRONE_PR_COMPARE_URL:-}"
  exit 0
fi

row="$(
  gh pr list \
    --state all \
    --base "${SANDRONE_PR_BASE}" \
    --head "${SANDRONE_PR_HEAD}" \
    --json url,state \
    --jq '.[0] | if . == null then "" else [.state, .url] | @tsv end'
)"

if [ -z "$row" ]; then
  printf 'missing\t%s\tno PR found for base/head\n' "${SANDRONE_PR_COMPARE_URL:-}"
  exit 0
fi

state="$(printf '%s' "$row" | cut -f1 | tr '[:upper:]' '[:lower:]')"
url="$(printf '%s' "$row" | cut -f2)"
printf '%s\t%s\t%s\n' "$state" "$url" "matched base/head"
