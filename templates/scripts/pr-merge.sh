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
# SANDRONE_PR_STATUS
# SANDRONE_PR_STATUS_URL
# SANDRONE_PR_STATUS_DETAIL
# SANDRONE_PR_STATUS_RAW
# SANDRONE_SCHEDULER_DECISION_ID
#
# Connector contract:
# - This script is invoked only after pr-status reported a safe merge state.
# - This script must not decide request priority or implementation order.
# - This script must re-check that the target PR still matches base/head.
# - If the platform cannot guarantee a safe merge, print blocked<TAB>url<TAB>detail
#   and exit 0.
# - Print exactly one TSV line to stdout:
#   merged<TAB>url<TAB>detail
#   blocked<TAB>url<TAB>detail
# - Exit non-zero only when the connector itself failed unexpectedly.

cd "${SANDRONE_WORKTREE}"

if [ "${SANDRONE_PR_STATUS:-}" != "safe" ]; then
  printf 'blocked\t%s\tpr-status did not report safe\n' "${SANDRONE_PR_STATUS_URL:-${SANDRONE_PR_COMPARE_URL:-}}"
  exit 0
fi

if ! command -v gh >/dev/null 2>&1; then
  printf 'blocked\t%s\tgh is not installed\n' "${SANDRONE_PR_COMPARE_URL:-}"
  exit 0
fi

if ! gh repo view >/dev/null 2>&1; then
  printf 'blocked\t%s\tgh cannot access this repository or this is not a GitHub repository\n' "${SANDRONE_PR_COMPARE_URL:-}"
  exit 0
fi

row="$(
  gh pr list \
    --state open \
    --base "${SANDRONE_PR_BASE}" \
    --head "${SANDRONE_PR_HEAD}" \
    --json number,url \
    --jq '.[0] | if . == null then "" else [(.number|tostring), .url] | @tsv end'
)"

if [ -z "$row" ]; then
  printf 'blocked\t%s\tno open PR found for base/head\n' "${SANDRONE_PR_COMPARE_URL:-}"
  exit 0
fi

number="$(printf '%s' "$row" | cut -f1)"
url="$(printf '%s' "$row" | cut -f2)"

gh pr merge "$number" --merge

printf 'merged\t%s\tgh pr merge completed\n' "$url"
