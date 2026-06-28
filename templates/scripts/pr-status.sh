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
# - Delivery observation status values: open, missing, merged, closed, unknown.
# - Merge safety probe status values for automatic merge: safe, unsafe, unsupported.
# - Sandrone marks a request finished only when status is merged.
# - Sandrone invokes pr-merge only when status is safe.
# - unsafe should mean integration refresh is likely useful, for example conflicts
#   or a branch behind the base. Use open/unknown for pending human/platform gates.
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
    --json url,state,isDraft,mergeStateStatus,reviewDecision \
    --jq '.[0] | if . == null then "" else [.state, .url, (.isDraft|tostring), (.mergeStateStatus // ""), (.reviewDecision // "")] | @tsv end'
)"

if [ -z "$row" ]; then
  printf 'missing\t%s\tno PR found for base/head\n' "${SANDRONE_PR_COMPARE_URL:-}"
  exit 0
fi

state="$(printf '%s' "$row" | cut -f1 | tr '[:upper:]' '[:lower:]')"
url="$(printf '%s' "$row" | cut -f2)"
is_draft="$(printf '%s' "$row" | cut -f3)"
merge_state="$(printf '%s' "$row" | cut -f4)"
review_decision="$(printf '%s' "$row" | cut -f5)"

case "$state" in
  merged)
    printf 'merged\t%s\tmatched base/head\n' "$url"
    ;;
  closed)
    printf 'closed\t%s\tmatched base/head\n' "$url"
    ;;
  open)
    if [ "$is_draft" = "true" ]; then
      printf 'open\t%s\tPR is draft\n' "$url"
    elif [ "$review_decision" = "CHANGES_REQUESTED" ]; then
      printf 'open\t%s\tGitHub review decision is CHANGES_REQUESTED\n' "$url"
    else
      case "$merge_state" in
        CLEAN|HAS_HOOKS)
          printf 'safe\t%s\tmergeStateStatus=%s reviewDecision=%s\n' "$url" "$merge_state" "${review_decision:-none}"
          ;;
        DIRTY|BEHIND)
          printf 'unsafe\t%s\tmergeStateStatus=%s requires integration refresh\n' "$url" "$merge_state"
          ;;
        *)
          printf 'open\t%s\tmergeStateStatus=%s reviewDecision=%s\n' "$url" "${merge_state:-unknown}" "${review_decision:-none}"
          ;;
      esac
    fi
    ;;
  *)
    printf '%s\t%s\tmatched base/head\n' "$state" "$url"
    ;;
esac
