#!/usr/bin/env sh
set -eu

# Replace this script to use Codex, Claude Code, a policy engine, or an
# internal merge queue planner. This connector decides merge priority only.
# It must not merge, push, modify branches, or change request state.
#
# Input is provided through environment variables:
# SANDRONE_MERGE_QUEUE
# SANDRONE_MERGE_PLAN_MD
# SANDRONE_MERGE_PLAN_JSON
# SANDRONE_AUTO_MERGE_ENABLED
#
# SANDRONE_MERGE_QUEUE is a TSV file with a header:
# request_id<TAB>title<TAB>branch<TAB>updated_at<TAB>pr_status<TAB>pr_url<TAB>pr_detail<TAB>change_path
#
# Connector contract:
# - Read only the queue snapshot and lightweight artifacts needed to reason
#   about merge order. Do not review implementation quality; code-review
#   already owns PR quality.
# - Print exactly one TSV line to stdout:
#   ready_for_merge<TAB>request_id<TAB>reason
#   defer<TAB>request_id-or-empty<TAB>reason
#   blocked<TAB>request_id-or-empty<TAB>reason
# - Select at most one request per run.
# - Only return ready_for_merge for a request whose queue row has pr_status
#   safe or merged. Sandrone will still re-run pr-status before pr-merge.

queue="${SANDRONE_MERGE_QUEUE:-}"
if [ -z "$queue" ] || [ ! -f "$queue" ]; then
  printf 'defer\t\tmerge queue snapshot is missing\n'
  exit 0
fi

if [ "${SANDRONE_AUTO_MERGE_ENABLED:-false}" != "true" ]; then
  printf 'defer\t\tauto merge is disabled\n'
  exit 0
fi

tab="$(printf '\t')"

candidate="$(
  awk -F "$tab" '
    NR == 1 { next }
    $5 == "safe" || $5 == "merged" {
      print
      exit
    }
  ' "$queue"
)"

if [ -n "$candidate" ]; then
  request_id="$(printf '%s' "$candidate" | cut -f1)"
  pr_status="$(printf '%s' "$candidate" | cut -f5)"
  pr_detail="$(printf '%s' "$candidate" | cut -f7)"
  if [ "$pr_status" = "merged" ]; then
    printf 'ready_for_merge\t%s\tPR is already merged; select it so Sandrone can mark finished\n' "$request_id"
  else
    printf 'ready_for_merge\t%s\tfirst safe PR in queue: %s\n' "$request_id" "${pr_detail:-ready}"
  fi
  exit 0
fi

summary="$(
  awk -F "$tab" '
    NR == 1 { next }
    {
      counts[$5] += 1
      total += 1
    }
    END {
      if (total == 0) {
        printf "no wait-finish PR candidates"
        exit
      }
      first = 1
      for (status in counts) {
        if (!first) {
          printf ", "
        }
        printf "%s=%d", status, counts[status]
        first = 0
      }
    }
  ' "$queue"
)"

printf 'defer\t\tno safe PR is ready to merge: %s\n' "$summary"
