---
name: obsidian-change-trace
description: Use when maintaining Obsidian change-trace notes inside sandrone workspaces, especially request index notes, slice notes, DAG links, small requirement coverage summaries, review summaries, PR delivery notes, and recovery navigation under obsidian/changes. Do not use for general personal Obsidian research notes.
metadata:
  short-description: Maintain Sandrone Obsidian change traces
---

# Obsidian Change Trace

Use this skill only for `sandrone` managed workspaces. It is the project-specific Obsidian tracing skill for request, slice, review, PR, and recovery navigation. It is not a general Obsidian research-note skill.

## Scope

The workspace remains split into two kinds of state:

- `.sandrone/`: machine index, events, locks, sessions, agent logs, registry.
- `obsidian/project.md`: project root note that groups parent requests by date and links only to each parent `<REQ> index.md`.
- `obsidian/derived/*.json`: generated lightweight indexes for AI sessions; read these before scanning many historical notes.
- `obsidian/views/*.base` and `obsidian/project.canvas`: generated human views derived from request/status/DAG data.
- `obsidian/changes/<change-name>/`: human/AI-readable change package and trace graph.

Do not edit `.obsidian/` except for vault configuration created by the framework. Do not hand-edit generated Base, Canvas, or derived JSON files to represent workflow state. Regenerate them with `sandrone obsidian-refresh`. Do not overwrite user-maintained sections in Obsidian notes. Prefer updating bounded generated sections or appending dated changelog entries.

## Required Structure

Each request has a top-level package:

```text
obsidian/project.md
obsidian/relations.md
obsidian/derived/
  requests.json
  slices.json
obsidian/views/
  requests.base
  slices.base
obsidian/project.canvas
obsidian/changes/<change-name>/
  REQ-0001 index.md
  REQ-0001 request.md
  REQ-0001 decomposition.md
  REQ-0001 pr-doc.md
  REQ-0001 agent-journal.md
  decomposition.json
  dag.json
  slices/
    S01/
      REQ-0001-S01 index.md
      REQ-0001-S01 plan.md
      REQ-0001-S01 change-doc.md
      REQ-0001-S01 agent-journal.md
      status.json
      reviews/
  reviews/
  status.json
  pr-conflicts/
```

Small requests still use the same structure. Their decomposition may contain exactly one slice, usually `S01`.

## Generated Indexes

Use generated indexes to save tokens:

- Read `obsidian/project.md` first for project-level navigation and date-grouped parent request links.
- Read `obsidian/derived/requests.json` when you need request IDs, status, note path, branch, worktree, and timestamps without opening every note.
- Read `obsidian/derived/slices.json`, `decomposition.json`, and `dag.json` when you need slice dependency order.
- Use `obsidian/project.canvas` and `obsidian/views/*.base` as human-friendly views. They are derived from the same source data and should not be treated as the only source of truth.

If generated indexes are missing or stale, run:

```bash
sandrone obsidian-refresh
```

## Request Index

`<REQ> index.md` is the request map of content. Keep it concise and navigable:

- YAML properties: `title`, `type`, `request_id`, `status`, `source`, `external_id`, `branch`, `worktree`, `updated`, `tags`.
- Link to `<REQ> agent-journal.md`, status, review folders, PR body/status, and recovery docs when present. Stage documents must include the request id in their filenames. Parent request indexes point to request/decomposition/pr-doc stage docs; slice indexes point to plan/change-doc stage docs. Slice has no separate request.md because its plan is also the slice request; use env/status-provided paths instead of hand-building old short names.
- Maintain a clean graph hierarchy: `project.md -> parent request index -> slice index -> stage documents`. `project.md` must not directly wikilink slice indexes, stage documents, Base views, Canvas, derived JSON, or CodeGraph context. Only parent request indexes may wikilink back to `project.md`; slice indexes should point to their parent request index, and stage documents should point to the current request/slice index or stage-local documents instead of linking back to project. Keep auxiliary project-level paths as plain paths when they need to be mentioned.
- A Mermaid DAG showing slice dependency direction.
- A short current summary. Do not paste full plans, full change docs, long diffs, or full reviewer JSON.
- Next action: one concrete command or human action, such as `sandrone tick --request_id REQ-0001`, `finish`, `pr-refresh`, or manual recovery.

## Slice Notes

Each slice note should be graph-friendly:

```yaml
---
title: "REQ-0005 S01 content-catalog"
type: slice
request_id: REQ-0005
slice_id: S01
status: plan-approved
depends_on: []
blocks: ["[[REQ-0005-S02 world-model]]"]
branch: codex/req-0005-s01-content-catalog
worktree: dev/worktrees/REQ-0005-S01
updated: 2026-06-03
tags:
  - Sandrone/slice
---
```

The body should include:

- Scope: what this slice does and explicitly does not do.
- Inputs and outputs.
- Dependency reasons, not only a list of IDs.
- Links to `<REQ-SNN> plan.md`, `<REQ-SNN> change-doc.md`, review detail folders, target project docs, and CodeGraph context. The slice plan is both the slice request and the approved plan artifact.
- Verification summary and current blocker/next action.

Use wikilinks for relationships and normal Markdown links for local files. The authoritative DAG remains `dag.json`; Obsidian links make the DAG readable.

## Requirement Coverage

Do not create a standalone coverage matrix. Keep requirement coverage as a small table inside `<REQ> decomposition.md`:

```text
original requirement or acceptance point -> covering slice -> verification direction
```

Every original acceptance point must be covered by at least one slice. Detailed plan, code/test evidence, review evidence, and PR evidence live in their own stage documents and are connected through links, not copied into the coverage table. If a requirement is intentionally deferred, record owner, reason, risk, and follow-up trigger outside unchecked checklists.

## Reviews

Review notes or summaries must stay short:

- Link to immutable `reviews/<stage>/details/*.json`.
- Summarize critical/high/warning/info counts.
- Copy only finding titles and required fixes when useful.
- Never let one reviewer summary replace another reviewer source. TestReviewer, DesignReviewer, PlanReviewer, DecompositionReviewer, and IntegrationReviewer remain independent evidence.

## PR And Recovery

When PR delivery or refresh happens, update navigation with:

- branch and PR URL/status if known;
- `wait-update-pr`, `wait-finish`, or `finished` meaning;
- conflict attempt links under `pr-conflicts/attempts/`;
- IntegrationReviewer result links;
- the exact next command or manual action.

When blocked, ensure `<REQ> index.md` points to `<REQ> recovery.md`, latest review summary, latest agent logs, and the safest resume command.

## Quality Bar

A good change trace lets a future Codex session resume without chat history:

- What is the request?
- What slices exist and why?
- Which slice is runnable next?
- What already passed review?
- What branch/worktree contains the work?
- Which tests/docs/reviews prove completion?
- What is the next command or human decision?
