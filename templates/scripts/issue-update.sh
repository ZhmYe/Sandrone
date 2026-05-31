#!/usr/bin/env sh
set -eu

cd dev/repo

# Output TSV lines:
# external_id<TAB>source<TAB>title<TAB>body<TAB>url
#
# Connector contract:
# - stdout MUST contain zero or more TSV records and no header.
# - Column 1 external_id MUST be stable for the same upstream request.
# - Column 2 source SHOULD be a short provider name such as github, jira, linear, or internal.
# - Column 3 title becomes the normalized requirement name.
# - Column 4 body becomes the normalized requirement description; preserve full user-visible detail.
# - Column 5 url MAY be empty when the source has no browser URL.
# - stderr is reserved for diagnostics. Exit non-zero only when update failed.
#
# Replace this script for Jira, Linear, internal workspaces, or other sources.
# The connector should emit a stable external_id so repeated updates do not
# create duplicate requests.

repo="$(gh repo view --json nameWithOwner -q .nameWithOwner)"
gh api --method GET "repos/${repo}/issues" \
  -f state=open \
  --paginate \
  --jq ".[] | select(.pull_request == null) | [\"github:${repo}#\" + (.number|tostring), \"github\", .title, (.body // \"\"), .html_url] | @tsv"
