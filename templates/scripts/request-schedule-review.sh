#!/usr/bin/env sh
set -eu

# RequestScheduleReviewer connector.
#
# This default reviewer is deterministic and lightweight. Replace it with an
# LLM reviewer if you need richer dependency/priority reasoning. It reviews the
# schedule plan only; it must not review implementation quality, change state,
# dispatch agents, create PRs, or merge PRs.
#
# stdout MUST be exactly one JSON object matching
# tools/schemas/review-result.schema.json.

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\r//g' | tr '\n' ' '
}

finding_json() {
  title="$(json_escape "$1")"
  evidence="$(json_escape "$2")"
  impact="$(json_escape "$3")"
  required_fix="$(json_escape "$4")"
  suggested_change="$(json_escape "$5")"
  verification="$(json_escape "$6")"
  printf '{"title":"%s","evidence":"%s","impact":"%s","required_fix":"%s","suggested_change":"%s","verification":"%s"}' \
    "$title" "$evidence" "$impact" "$required_fix" "$suggested_change" "$verification"
}

emit_result() {
  approved="$1"
  decision="$2"
  next_phase="$3"
  summary="$4"
  high_json="$5"
  info_json="$6"
  printf '{\n'
  printf '  "reviewer": "RequestScheduleReviewer",\n'
  printf '  "approved": %s,\n' "$approved"
  printf '  "gate_unavailable": false,\n'
  printf '  "decision": "%s",\n' "$decision"
  printf '  "recommended_next_phase": "%s",\n' "$next_phase"
  printf '  "summary": "%s",\n' "$(json_escape "$summary")"
  printf '  "process": [\n'
  printf '    "read request schedule queue",\n'
  printf '    "read request schedule output",\n'
  printf '    "checked selected ids are in the queue",\n'
  printf '    "checked selected count does not exceed max parallel"\n'
  printf '  ],\n'
  printf '  "critical": [],\n'
  printf '  "high": [%s],\n' "$high_json"
  printf '  "warning": [],\n'
  printf '  "info": [%s]\n' "$info_json"
  printf '}\n'
}

queue="${SANDRONE_REQUEST_SCHEDULE_QUEUE:-}"
output="${SANDRONE_REQUEST_SCHEDULE_OUTPUT:-}"
max_parallel="${SANDRONE_REQUEST_SCHEDULE_MAX_PARALLEL:-1}"

case "$max_parallel" in
  ''|*[!0-9]*) max_parallel=1 ;;
esac
if [ "$max_parallel" -lt 1 ]; then
  max_parallel=1
fi

if [ -z "$queue" ] || [ ! -f "$queue" ]; then
  finding="$(finding_json \
    "request schedule queue is missing" \
    "SANDRONE_REQUEST_SCHEDULE_QUEUE is empty or not a file" \
    "The reviewer cannot verify whether selected requests are schedulable" \
    "Regenerate the queue before reviewing the schedule" \
    "Run sandrone loop run-once again after restoring the queue artifact" \
    "Confirm agents/request-schedule-agent wrote request-schedule-queue.tsv")"
  emit_result false rejected blocked "request schedule queue is missing" "$finding" ""
  exit 0
fi

if [ -z "$output" ] || [ ! -f "$output" ]; then
  finding="$(finding_json \
    "request schedule output is missing" \
    "SANDRONE_REQUEST_SCHEDULE_OUTPUT is empty or not a file" \
    "The reviewer cannot inspect the selected request ids" \
    "Ensure Request Schedule Agent writes output before review" \
    "Check the request-schedule-agent connector stdout contract" \
    "Confirm request-schedule-output.tsv exists")"
  emit_result false rejected blocked "request schedule output is missing" "$finding" ""
  exit 0
fi

tab="$(printf '\t')"
selected_ids="$(
  awk -F "$tab" '$1 == "selected" && $2 != "" { print $2 }' "$output"
)"
selected_count="$(printf '%s\n' "$selected_ids" | awk 'NF { count += 1 } END { print count + 0 }')"
blocked_reason="$(
  awk -F "$tab" '$1 == "blocked" { print $3; exit }' "$output"
)"

if [ -n "$blocked_reason" ]; then
  finding="$(finding_json \
    "request schedule agent blocked" \
    "$blocked_reason" \
    "The loop cannot safely dispatch new work from a blocked schedule" \
    "Fix the request schedule agent output or its connector runtime" \
    "Inspect agents/request-schedule-agent logs and rerun loop run-once" \
    "RequestScheduleReviewer sees no blocked rows and approved=true")"
  emit_result false rejected blocked "request schedule agent blocked" "$finding" ""
  exit 0
fi

if [ "$selected_count" -eq 0 ]; then
  info="$(finding_json \
    "no request selected" \
    "request schedule output contains no selected rows" \
    "The loop pass will not dispatch new agents" \
    "No fix required if there is no schedulable work" \
    "If work was expected, inspect request statuses and dependencies" \
    "Next loop should select requests once they become schedulable")"
  emit_result true approved implementation "request schedule selected no work for this loop pass" "" "$info"
  exit 0
fi

if [ "$selected_count" -gt "$max_parallel" ]; then
  finding="$(finding_json \
    "selected request count exceeds max parallel" \
    "selected_count=$selected_count max_parallel=$max_parallel" \
    "The loop would exceed its configured concurrency limit" \
    "Select at most SANDRONE_REQUEST_SCHEDULE_MAX_PARALLEL requests" \
    "Fix the request schedule agent output contract or lower selected rows" \
    "Rerun request schedule review and confirm selected_count <= max_parallel")"
  emit_result false rejected blocked "request schedule selected too many requests" "$finding" ""
  exit 0
fi

missing=""
for request_id in $selected_ids; do
  if ! awk -F "$tab" -v id="$request_id" 'NR > 1 && $1 == id { found = 1 } END { exit found ? 0 : 1 }' "$queue"; then
    missing="${missing}${missing:+, }$request_id"
  fi
done

if [ -n "$missing" ]; then
  finding="$(finding_json \
    "selected request is not in current queue" \
    "missing selected ids: $missing" \
    "The schedule may be stale or selecting unrelated work" \
    "Regenerate the schedule from the current queue only" \
    "Ensure Request Schedule Agent reads SANDRONE_REQUEST_SCHEDULE_QUEUE and does not invent ids" \
    "Rerun request schedule review and confirm every selected id exists in the queue")"
  emit_result false rejected blocked "request schedule selected ids outside current queue" "$finding" ""
  exit 0
fi

info="$(finding_json \
  "request schedule approved" \
  "selected_count=$selected_count max_parallel=$max_parallel" \
  "The loop may dispatch the selected requests while preserving the concurrency limit" \
  "No blocking fix required" \
  "Keep schedule output limited to selected/defer/blocked TSV rows" \
  "Observe the loop event log and request states after dispatch")"
emit_result true approved implementation "request schedule approved $selected_count request(s)" "" "$info"
