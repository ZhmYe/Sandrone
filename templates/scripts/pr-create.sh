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
# SANDRONE_CHANGE_DOC
# SANDRONE_REQUEST
# SANDRONE_WORKTREE
# SANDRONE_PR_TITLE
# SANDRONE_PR_BODY_FILE
# SANDRONE_PR_BASE
# SANDRONE_PR_HEAD
# SANDRONE_PR_COMPARE_URL
#
# Connector contract:
# - The worktree has already been committed and pushed before this script runs.
# - Before creating anything, determine whether this platform/repository can create PRs.
# - Before creating anything, check whether a PR for base/head already exists.
# - Print exactly one TSV line to stdout on success:
#   created<TAB>url
#   existing<TAB>url
# - Exit non-zero with a helpful stderr message when the platform cannot create a PR
#   or when an existing PR check cannot be performed safely.
# - Do not merge.

cd "${SANDRONE_WORKTREE}"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh is not installed; create the PR manually: ${SANDRONE_PR_COMPARE_URL}" >&2
  exit 1
fi

if ! gh repo view >/dev/null 2>&1; then
  echo "gh cannot access this repository or this is not a GitHub repository; create the PR manually: ${SANDRONE_PR_COMPARE_URL}" >&2
  exit 1
fi

existing_url="$(
  gh pr list \
    --state all \
    --base "${SANDRONE_PR_BASE}" \
    --head "${SANDRONE_PR_HEAD}" \
    --json url \
    --jq '.[0].url // ""'
)"
if [ -n "$existing_url" ]; then
  printf 'existing\t%s\n' "$existing_url"
  exit 0
fi

created_url="$(gh pr create \
  --base "${SANDRONE_PR_BASE}" \
  --head "${SANDRONE_PR_HEAD}" \
  --title "${SANDRONE_PR_TITLE}" \
  --body-file "${SANDRONE_PR_BODY_FILE}")"

if [ -z "$created_url" ]; then
  echo "gh pr create succeeded without returning a PR URL" >&2
  exit 1
fi

printf 'created\t%s\n' "$created_url"
