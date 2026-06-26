use super::*;

#[derive(Clone)]
struct MergeCandidate {
    request: Request,
    pr_status: PrStatusReport,
    eligible: bool,
    eligibility_detail: String,
}

struct MergeQueueSnapshot {
    plan_id: String,
    run_dir: PathBuf,
    queue_path: PathBuf,
    compat_queue_path: PathBuf,
    plan_md_path: PathBuf,
    plan_json_path: PathBuf,
    compat_plan_json_path: PathBuf,
    history_json_path: PathBuf,
    output_path: PathBuf,
    compat_output_path: PathBuf,
}

struct MergePlanReport {
    queue_decision: String,
    request_id: String,
    reason: String,
    raw: String,
}

pub(crate) struct MergePlanOutcome {
    pub(crate) queue_decision: String,
    pub(crate) request_id: String,
    pub(crate) reason: String,
    pub(crate) queue_path: PathBuf,
    pub(crate) plan_md_path: PathBuf,
}

pub(crate) fn plan_merge_queue_from_tick(
    request_filter: Option<&str>,
    auto_merge_enabled: bool,
) -> Result<Option<MergePlanOutcome>> {
    let requests = load_requests()?;
    let config = load_config()?;
    let candidates = collect_merge_candidates(&requests, request_filter, &config)?;
    if candidates.is_empty() {
        return Ok(None);
    }
    let snapshot = write_merge_queue_snapshot(&candidates, auto_merge_enabled)?;
    let mut plan = run_merge_plan_tool(&snapshot, &candidates, auto_merge_enabled)?;
    if plan.queue_decision == "ready_for_merge"
        && !candidates
            .iter()
            .any(|candidate| candidate.request.request_id == plan.request_id)
    {
        plan.queue_decision = "blocked".to_string();
        plan.reason = format!(
            "planner selected request outside current queue: {}",
            fallback_empty(&plan.request_id, "empty request id")
        );
    }
    write_merge_plan_artifacts(&snapshot, &candidates, &plan, auto_merge_enabled)?;
    Ok(Some(MergePlanOutcome {
        queue_decision: plan.queue_decision,
        request_id: plan.request_id,
        reason: plan.reason,
        queue_path: snapshot.queue_path,
        plan_md_path: snapshot.plan_md_path,
    }))
}

fn collect_merge_candidates(
    requests: &[Request],
    request_filter: Option<&str>,
    config: &Config,
) -> Result<Vec<MergeCandidate>> {
    let mut candidates = Vec::new();
    for request in requests.iter().filter(|request| {
        canonical_status(&request.status) == STATUS_WAIT_FINISH
            && request_filter
                .map(|filter| request.request_id == filter)
                .unwrap_or(true)
    }) {
        let (pr_status, eligible, eligibility_detail) =
            if let Err(error) = ensure_refreshable_request(request) {
                (
                    PrStatusReport {
                        status: "blocked".to_string(),
                        url: String::new(),
                        detail: error.to_string(),
                        raw: format!("blocked\t\t{error}"),
                    },
                    false,
                    "request is not refreshable".to_string(),
                )
            } else if let Err(error) = ensure_gate_approved(request, "change-doc") {
                (
                    PrStatusReport {
                        status: "blocked".to_string(),
                        url: String::new(),
                        detail: error.to_string(),
                        raw: format!("blocked\t\t{error}"),
                    },
                    false,
                    "change-doc gate is not approved".to_string(),
                )
            } else {
                let report = run_pr_status_tool(request, config)?;
                let eligible = matches!(report.status.as_str(), "safe" | "merged");
                let detail = if eligible {
                    "eligible for merge planning".to_string()
                } else {
                    format!("pr-status is {}", report.status)
                };
                (report, eligible, detail)
            };
        candidates.push(MergeCandidate {
            request: request.clone(),
            pr_status,
            eligible,
            eligibility_detail,
        });
    }
    Ok(candidates)
}

fn write_merge_queue_snapshot(
    candidates: &[MergeCandidate],
    auto_merge_enabled: bool,
) -> Result<MergeQueueSnapshot> {
    let plan_id = merge_plan_id("merge-queue");
    let history_id = merge_plan_id("merge-plan");
    let run_dir = create_named_agent_run_state_dir(
        "merge-planner",
        &["merge-plan", &history_id],
        "merge-plan",
        "current",
        "merge-planner",
    )?;
    let artifacts_dir = job_artifacts_dir(&run_dir);
    let scheduler_dir = Path::new(".sandrone").join("state").join("scheduler");
    let obsidian_merge_dir = Path::new("obsidian").join("merge");
    fs::create_dir_all(&scheduler_dir)?;
    fs::create_dir_all(&obsidian_merge_dir)?;

    let snapshot = MergeQueueSnapshot {
        plan_id,
        run_dir,
        queue_path: artifacts_dir.join("merge-queue.tsv"),
        compat_queue_path: scheduler_dir.join("merge-queue.tsv"),
        plan_md_path: obsidian_merge_dir.join("merge-plan.md"),
        plan_json_path: artifacts_dir.join("merge-plan.json"),
        compat_plan_json_path: scheduler_dir.join("merge-plan.json"),
        history_json_path: artifacts_dir.join(format!("{history_id}.json")),
        output_path: artifacts_dir.join("merge-plan-output.tsv"),
        compat_output_path: scheduler_dir.join("merge-plan-output.tsv"),
    };
    write_merge_queue_tsv(
        &snapshot.queue_path,
        Some(&snapshot.compat_queue_path),
        candidates,
    )?;
    let pending = MergePlanReport {
        queue_decision: "pending".to_string(),
        request_id: String::new(),
        reason: "merge planner has not returned a decision yet".to_string(),
        raw: String::new(),
    };
    write_merge_plan_artifacts(&snapshot, candidates, &pending, auto_merge_enabled)?;
    Ok(snapshot)
}

fn write_merge_queue_tsv(
    path: &Path,
    compat_path: Option<&Path>,
    candidates: &[MergeCandidate],
) -> Result<()> {
    let mut content =
        "request_id\ttitle\tbranch\tupdated_at\tpr_status\tpr_url\tpr_detail\tchange_path\n"
            .to_string();
    for candidate in candidates {
        content.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            tsv_cell(&candidate.request.request_id),
            tsv_cell(&candidate.request.title),
            tsv_cell(&candidate.request.branch),
            tsv_cell(&candidate.request.updated_at),
            tsv_cell(&candidate.pr_status.status),
            tsv_cell(&candidate.pr_status.url),
            tsv_cell(&candidate.pr_status.detail),
            tsv_cell(&candidate.request.change_path),
        ));
    }
    write_runtime_text(path, &content, compat_path)?;
    Ok(())
}

fn run_merge_plan_tool(
    snapshot: &MergeQueueSnapshot,
    candidates: &[MergeCandidate],
    auto_merge_enabled: bool,
) -> Result<MergePlanReport> {
    if !Path::new(MERGE_PLAN_TOOL).exists() {
        return Ok(default_merge_plan_report(
            candidates,
            &format!("{MERGE_PLAN_TOOL} missing; using built-in first-safe planner"),
        ));
    }
    let output = Command::new("sh")
        .arg(MERGE_PLAN_TOOL)
        .current_dir(".")
        .env("SANDRONE_MERGE_QUEUE", &snapshot.queue_path)
        .env("SANDRONE_MERGE_PLAN_MD", &snapshot.plan_md_path)
        .env("SANDRONE_MERGE_PLAN_JSON", &snapshot.plan_json_path)
        .env(
            "SANDRONE_AUTO_MERGE_ENABLED",
            if auto_merge_enabled { "true" } else { "false" },
        )
        .env("SANDRONE_SCHEDULER_DECISION_ID", &snapshot.plan_id)
        .envs(proxy_env())
        .output();
    let raw = match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8(output.stdout)?;
            stdout
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("defer\t\tmerge-plan returned no output")
                .to_string()
        }
        Ok(output) => format!(
            "blocked\t\t{}",
            review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr))
        ),
        Err(error) => format!("blocked\t\t{error}"),
    };
    write_runtime_text(
        &snapshot.output_path,
        &ensure_trailing_newline(&raw),
        Some(&snapshot.compat_output_path),
    )?;
    Ok(parse_merge_plan_report(&raw))
}

fn default_merge_plan_report(candidates: &[MergeCandidate], prefix: &str) -> MergePlanReport {
    if let Some(candidate) = candidates.iter().find(|candidate| candidate.eligible) {
        let reason = if candidate.pr_status.status == "merged" {
            format!("{prefix}; PR already merged and should be marked finished")
        } else {
            format!(
                "{prefix}; first safe PR in queue: {}",
                fallback_empty(&candidate.pr_status.detail, "ready")
            )
        };
        return MergePlanReport {
            queue_decision: "ready_for_merge".to_string(),
            request_id: candidate.request.request_id.clone(),
            raw: format!(
                "ready_for_merge\t{}\t{reason}",
                candidate.request.request_id
            ),
            reason,
        };
    }
    MergePlanReport {
        queue_decision: "defer".to_string(),
        request_id: String::new(),
        reason: format!("{prefix}; no safe PR is ready to merge"),
        raw: "defer\t\tno safe PR is ready to merge".to_string(),
    }
}

fn parse_merge_plan_report(line: &str) -> MergePlanReport {
    let fields: Vec<&str> = line.split('\t').collect();
    let mut queue_decision = fields
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "defer".to_string());
    if !matches!(
        queue_decision.as_str(),
        "ready_for_merge" | "defer" | "blocked"
    ) {
        queue_decision = "blocked".to_string();
    }
    MergePlanReport {
        queue_decision,
        request_id: fields
            .get(1)
            .map(|value| value.trim().to_string())
            .unwrap_or_default(),
        reason: fields
            .get(2)
            .map(|value| value.trim().to_string())
            .unwrap_or_default(),
        raw: line.trim().to_string(),
    }
}

fn write_merge_plan_artifacts(
    snapshot: &MergeQueueSnapshot,
    candidates: &[MergeCandidate],
    report: &MergePlanReport,
    auto_merge_enabled: bool,
) -> Result<()> {
    let json = render_merge_plan_json(snapshot, candidates, report, auto_merge_enabled);
    let markdown = render_merge_plan_markdown(snapshot, candidates, report, auto_merge_enabled);
    write_runtime_text(
        &snapshot.plan_json_path,
        &json,
        Some(&snapshot.compat_plan_json_path),
    )?;
    fs::write(&snapshot.history_json_path, &json)?;
    fs::write(&snapshot.plan_md_path, &markdown)?;
    Ok(())
}

fn render_merge_plan_json(
    snapshot: &MergeQueueSnapshot,
    candidates: &[MergeCandidate],
    report: &MergePlanReport,
    auto_merge_enabled: bool,
) -> String {
    let candidates_json = candidates
        .iter()
        .map(|candidate| {
            format!(
                "    {{\"request_id\":\"{}\",\"title\":\"{}\",\"branch\":\"{}\",\"updated_at\":\"{}\",\"request_status\":\"{}\",\"pr_status\":\"{}\",\"pr_url\":\"{}\",\"pr_detail\":\"{}\",\"eligible\":{},\"eligibility_detail\":\"{}\",\"change_path\":\"{}\"}}",
                json_escape(&candidate.request.request_id),
                json_escape(&candidate.request.title),
                json_escape(&candidate.request.branch),
                json_escape(&candidate.request.updated_at),
                json_escape(&candidate.request.status),
                json_escape(&candidate.pr_status.status),
                json_escape(&candidate.pr_status.url),
                json_escape(&candidate.pr_status.detail),
                if candidate.eligible { "true" } else { "false" },
                json_escape(&candidate.eligibility_detail),
                json_escape(&candidate.request.change_path),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema_version\": 1,\n  \"plan_id\": \"{}\",\n  \"auto_merge_enabled\": {},\n  \"queue_decision\": \"{}\",\n  \"selected_request_id\": \"{}\",\n  \"reason\": \"{}\",\n  \"raw\": \"{}\",\n  \"queue_path\": \"{}\",\n  \"plan_markdown\": \"{}\",\n  \"run_dir\": \"{}\",\n  \"updated_at\": \"{}\",\n  \"candidates\": [\n{}\n  ]\n}}\n",
        json_escape(&snapshot.plan_id),
        if auto_merge_enabled { "true" } else { "false" },
        json_escape(&report.queue_decision),
        json_escape(&report.request_id),
        json_escape(&report.reason),
        json_escape(&report.raw),
        json_escape(&snapshot.queue_path.to_string_lossy()),
        json_escape(&snapshot.plan_md_path.to_string_lossy()),
        json_escape(&snapshot.run_dir.to_string_lossy()),
        json_escape(&now_string()),
        candidates_json,
    )
}

fn render_merge_plan_markdown(
    snapshot: &MergeQueueSnapshot,
    candidates: &[MergeCandidate],
    report: &MergePlanReport,
    auto_merge_enabled: bool,
) -> String {
    let mut content = format!(
        "# Merge Plan - {}\n\n- Plan id: `{}`\n- Auto merge: `{}`\n- Decision: `{}`\n- Selected request: `{}`\n- Reason: {}\n- Queue snapshot: `{}`\n- Machine plan: `{}`\n\n",
        now_string(),
        snapshot.plan_id,
        auto_merge_enabled,
        report.queue_decision,
        fallback_empty(&report.request_id, "none"),
        fallback_empty(&report.reason, "n/a"),
        snapshot.queue_path.display(),
        snapshot.plan_json_path.display(),
    );
    content.push_str("## Candidates\n\n");
    content.push_str("| Request | PR status | Branch | Updated | Eligibility | Detail |\n");
    content.push_str("|---|---|---|---|---|---|\n");
    for candidate in candidates {
        content.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | {} |\n",
            markdown_table_escape(&candidate.request.request_id),
            markdown_table_escape(&candidate.pr_status.status),
            markdown_table_escape(&candidate.request.branch),
            markdown_table_escape(&candidate.request.updated_at),
            if candidate.eligible {
                "eligible"
            } else {
                "defer"
            },
            markdown_table_escape(&candidate.eligibility_detail),
        ));
    }
    content.push_str("\n## Scope\n\n");
    content.push_str("- This plan decides merge order only.\n");
    content.push_str("- PR implementation quality is owned by the completed code-review gate.\n");
    content.push_str(
        "- Sandrone re-runs `pr-status` before `pr-merge`; this plan cannot bypass merge safety.\n",
    );
    content
}

fn tsv_cell(value: &str) -> String {
    value.replace(['\t', '\n', '\r'], " ")
}

fn markdown_table_escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn merge_plan_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos}")
}
