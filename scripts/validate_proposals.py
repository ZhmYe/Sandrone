#!/usr/bin/env python3
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
INDEX = ROOT / "proposal.json"
CONSTITUTION = ROOT / ".specify" / "memory" / "constitution.md"
REQUIRED_STATUSES = {"draft", "planned", "approved", "implemented", "merged", "blocked"}
REQUIRED_ARTIFACTS = ("spec_md", "plan_md", "tasks_md", "plan_html", "change_doc_md")


def fail(message: str) -> None:
    print(f"proposal validation failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def require_file(path: Path, label: str) -> None:
    if not path.exists():
        fail(f"missing {label}: {path.relative_to(ROOT)}")
    if not path.is_file():
        fail(f"{label} is not a file: {path.relative_to(ROOT)}")


def main() -> None:
    require_file(INDEX, "proposal index")
    require_file(CONSTITUTION, "Spec Kit constitution")
    try:
        data = json.loads(INDEX.read_text())
    except json.JSONDecodeError as exc:
        fail(f"proposal.json is invalid JSON: {exc}")

    if data.get("schema_version") != 1:
        fail("schema_version must be 1")
    if not data.get("updated_at"):
        fail("updated_at is required")

    proposals = data.get("proposals")
    if not isinstance(proposals, list) or not proposals:
        fail("proposals must be a non-empty array")

    seen_ids = set()
    for proposal in proposals:
        proposal_id = proposal.get("id")
        if not proposal_id:
            fail("proposal id is required")
        if proposal_id in seen_ids:
            fail(f"duplicate proposal id: {proposal_id}")
        seen_ids.add(proposal_id)

        status = proposal.get("status")
        if status not in REQUIRED_STATUSES:
            fail(f"{proposal_id} has invalid status: {status}")

        proposal_path = proposal.get("path")
        if not proposal_path:
            fail(f"{proposal_id} path is required")
        full_proposal_path = ROOT / proposal_path
        if not full_proposal_path.exists() or not full_proposal_path.is_dir():
            fail(f"{proposal_id} path does not exist: {proposal_path}")

        artifacts = proposal.get("artifacts")
        if not isinstance(artifacts, dict):
            fail(f"{proposal_id} artifacts must be an object")

        for key in REQUIRED_ARTIFACTS:
            artifact = artifacts.get(key)
            if not artifact:
                fail(f"{proposal_id} missing artifact key: {key}")
            require_file(ROOT / artifact, f"{proposal_id} {key}")

    print(f"validated {len(proposals)} proposal(s)")


if __name__ == "__main__":
    main()
