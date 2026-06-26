use super::*;

pub(crate) fn deliver_finished_request(
    request: &Request,
    commit_message: &str,
) -> Result<DeliveryResult> {
    if request.worktree_path.trim().is_empty() {
        return Err(format!(
            "{} has no worktree. Run sandrone start first.",
            request.request_id
        )
        .into());
    }
    if request.branch.trim().is_empty() {
        return Err(format!(
            "{} has no branch. Run sandrone start first.",
            request.request_id
        )
        .into());
    }
    let worktree = Path::new(&request.worktree_path);
    if !worktree.exists() {
        return Err(format!("worktree does not exist: {}", worktree.display()).into());
    }
    let changes = git_output(&request.worktree_path, &["status", "--porcelain"])?;
    let committed = !changes.trim().is_empty();
    if committed {
        run_command(
            Command::new("git")
                .args(["add", "-A"])
                .current_dir(worktree),
        )?;
        run_command(
            Command::new("git")
                .args(["commit", "-m", commit_message])
                .current_dir(worktree),
        )?;
    }

    if !remote_exists(&request.worktree_path) {
        return Err("git remote origin is required before finish can push".into());
    }
    let pushed_with_force_lease = push_delivery_branch(worktree, &request.branch)?;

    let config = load_config()?;
    let body_file = write_pr_body(request)?;
    let compare_url = github_compare_url(&config.git_url, &config.base_branch, &request.branch);
    let (pr_url, pr_status, pr_error) = run_pr_tool(
        request,
        &config.base_branch,
        &request.branch,
        commit_message,
        &body_file,
        compare_url.as_deref().unwrap_or(""),
    )?;

    Ok(DeliveryResult {
        commit_message: commit_message.to_string(),
        branch: request.branch.clone(),
        committed,
        pushed_with_force_lease,
        pr_url,
        pr_status,
        compare_url,
        pr_error,
    })
}

pub(crate) fn pr_merge_request(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(
        args,
        &[
            "--request_id",
            "--request-id",
            "--queue-decision",
            "--auto-merge",
        ],
    )?;
    let request_id = required_request_id(args)?;
    let queue_decision =
        flag_value(args, "--queue-decision")?.unwrap_or_else(|| "ready_for_merge".to_string());
    let auto_merge_enabled = flag_present(args, "--auto-merge");
    let outcome = run_pr_merge_gate(&request_id, &queue_decision, auto_merge_enabled)?;
    print_pr_merge_outcome(&outcome);
    Ok(())
}

pub(crate) fn run_pr_merge_scheduler_from_tick(
    request_filter: Option<&str>,
    auto_merge_enabled: bool,
) -> Result<bool> {
    if !auto_merge_enabled {
        return Ok(false);
    }
    let Some(plan) = plan_merge_queue_from_tick(request_filter, auto_merge_enabled)? else {
        return Ok(false);
    };
    println!("Tick merge planner: {}", plan.queue_decision);
    println!("  selected: {}", fallback_empty(&plan.request_id, "none"));
    println!("  reason: {}", plan.reason);
    println!("  plan: {}", plan.plan_md_path.display());
    println!("  queue: {}", plan.queue_path.display());

    if plan.queue_decision != "ready_for_merge" {
        append_event(
            "merge_plan_deferred",
            fallback_empty(&plan.request_id, ""),
            "delivery",
            &plan.queue_decision,
            &format!(
                "plan={}; reason={}",
                plan.plan_md_path.display(),
                plan.reason
            ),
        )?;
        return Ok(true);
    }
    let Some(_lock) = RequestLock::acquire(&plan.request_id)? else {
        println!(
            "Tick merge scheduler skipped for {}: request lock is already held.",
            plan.request_id
        );
        return Ok(false);
    };
    let outcome = run_pr_merge_gate(&plan.request_id, &plan.queue_decision, true)?;
    println!(
        "Tick merge scheduler checked {}: {}",
        outcome.request_id, outcome.action
    );
    println!("  reason: {}", outcome.reason);
    println!("  pr-status: {}", outcome.pr_status_raw);
    if let Some(merge_raw) = &outcome.merge_raw {
        println!("  pr-merge: {merge_raw}");
    }
    println!("  decision: {}", outcome.decision_path);
    println!("  request status: {}", outcome.request_status);
    Ok(true)
}

struct PrMergeOutcome {
    request_id: String,
    action: String,
    reason: String,
    pr_status_raw: String,
    merge_raw: Option<String>,
    decision_path: String,
    request_status: String,
}

fn run_pr_merge_gate(
    request_id: &str,
    queue_decision: &str,
    auto_merge_enabled: bool,
) -> Result<PrMergeOutcome> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_refreshable_request(&request)?;
    ensure_gate_approved(&request, "change-doc")?;

    let config = load_config()?;
    let pr_status = run_pr_status_tool(&request, &config)?;
    let decision_id = scheduler_merge_decision_id(&request.request_id);
    let mut merge_report = None;
    let (action, reason) = if pr_status.status == "merged" {
        mark_request_finished_after_merge(
            &mut requests,
            index,
            &mut request,
            &format!(
                "PR already merged; confirmed by {PR_STATUS_TOOL}: {}",
                pr_status.raw
            ),
        )?;
        ("finished".to_string(), "PR already merged".to_string())
    } else if !auto_merge_enabled {
        (
            "skipped".to_string(),
            "auto merge disabled; pass --auto-merge to allow connector execution".to_string(),
        )
    } else if queue_decision != "ready_for_merge" {
        (
            "skipped".to_string(),
            format!("queue decision is {queue_decision}; expected ready_for_merge"),
        )
    } else if pr_status.status != "safe" {
        (
            "skipped".to_string(),
            format!(
                "pr-status returned {}; expected safe before merge",
                pr_status.status
            ),
        )
    } else {
        let report = run_pr_merge_tool(
            &request,
            &config,
            &pr_status,
            queue_decision,
            auto_merge_enabled,
            &decision_id,
        )?;
        let action_reason = if report.status == "merged" {
            mark_request_finished_after_merge(
                &mut requests,
                index,
                &mut request,
                &format!("PR merge confirmed by {PR_MERGE_TOOL}: {}", report.raw),
            )?;
            (
                "merged".to_string(),
                format!("pr-merge returned {}", report.status),
            )
        } else {
            (
                report.status.clone(),
                format!("pr-merge returned {}: {}", report.status, report.detail),
            )
        };
        merge_report = Some(report);
        action_reason
    };
    let decision_path = write_merge_decision_record(MergeDecisionRecord {
        request: &request,
        decision_id: &decision_id,
        queue_decision,
        auto_merge_enabled,
        pr_status: &pr_status,
        action: &action,
        reason: &reason,
        merge_report: merge_report.as_ref(),
    })?;
    append_event(
        "pr_merge_checked",
        &request.request_id,
        "delivery",
        &request.status,
        &format!("decision={decision_path}; action={action}; reason={reason}"),
    )?;

    Ok(PrMergeOutcome {
        request_id: request.request_id,
        action,
        reason,
        pr_status_raw: pr_status.raw,
        merge_raw: merge_report.map(|report| report.raw),
        decision_path,
        request_status: request.status,
    })
}

fn print_pr_merge_outcome(outcome: &PrMergeOutcome) {
    println!(
        "PR merge check for {}: {}",
        outcome.request_id, outcome.action
    );
    println!("  reason: {}", outcome.reason);
    println!("  pr-status: {}", outcome.pr_status_raw);
    if let Some(merge_raw) = &outcome.merge_raw {
        println!("  pr-merge: {merge_raw}");
    }
    println!("  decision: {}", outcome.decision_path);
    println!("  request status: {}", outcome.request_status);
}

#[derive(Clone, Debug)]
pub(crate) struct PrMergeReport {
    pub(crate) status: String,
    pub(crate) url: String,
    pub(crate) detail: String,
    pub(crate) raw: String,
}

fn scheduler_merge_decision_id(request_id: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}-merge-{nanos}", request_id.to_ascii_lowercase())
}

fn mark_request_finished_after_merge(
    requests: &mut [Request],
    index: usize,
    request: &mut Request,
    reason: &str,
) -> Result<()> {
    request.status = STATUS_FINISHED.to_string();
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(requests)?;
    write_status_json(request, "delivery", STATUS_FINISHED, reason)?;
    append_event(
        "pr_merge_finished",
        &request.request_id,
        "delivery",
        STATUS_FINISHED,
        reason,
    )?;
    upsert_session_for_request(request, "implementation", STATUS_FINISHED)?;
    Ok(())
}

pub(crate) fn run_pr_merge_tool(
    request: &Request,
    config: &Config,
    pr_status: &PrStatusReport,
    queue_decision: &str,
    auto_merge_enabled: bool,
    decision_id: &str,
) -> Result<PrMergeReport> {
    if !Path::new(PR_MERGE_TOOL).exists() {
        let raw = format!("blocked\t{}\t{PR_MERGE_TOOL} missing", pr_status.url);
        return Ok(parse_pr_merge_report(&raw));
    }
    let compare_url = github_compare_url(&config.git_url, &config.base_branch, &request.branch)
        .unwrap_or_default();
    let output = Command::new("sh")
        .arg(PR_MERGE_TOOL)
        .current_dir(".")
        .env("SANDRONE_REQUEST_ID", &request.request_id)
        .env("SANDRONE_REQUEST_EXTERNAL_ID", &request.external_id)
        .env("SANDRONE_REQUEST_SOURCE", &request.source)
        .env("SANDRONE_REQUEST_TITLE", &request.title)
        .env("SANDRONE_REQUEST_URL", &request.url)
        .env("SANDRONE_CHANGE_PATH", &request.change_path)
        .env("SANDRONE_WORKTREE", &request.worktree_path)
        .env("SANDRONE_PR_BASE", &config.base_branch)
        .env("SANDRONE_PR_HEAD", &request.branch)
        .env("SANDRONE_PR_COMPARE_URL", compare_url)
        .env("SANDRONE_PR_STATUS", &pr_status.status)
        .env("SANDRONE_PR_STATUS_URL", &pr_status.url)
        .env("SANDRONE_PR_STATUS_DETAIL", &pr_status.detail)
        .env("SANDRONE_PR_STATUS_RAW", &pr_status.raw)
        .env("SANDRONE_QUEUE_DECISION", queue_decision)
        .env(
            "SANDRONE_AUTO_MERGE_ENABLED",
            if auto_merge_enabled { "true" } else { "false" },
        )
        .env("SANDRONE_SCHEDULER_DECISION_ID", decision_id)
        .envs(proxy_env())
        .output();
    let raw = match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8(output.stdout)?;
            stdout
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("blocked\t\tpr-merge returned no output")
                .to_string()
        }
        Ok(output) => format!(
            "failed\t{}\t{}",
            pr_status.url,
            review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr))
        ),
        Err(error) => format!("failed\t{}\t{error}", pr_status.url),
    };
    let merge_path = Path::new(".sandrone")
        .join("state")
        .join(format!("{}-pr-merge.tsv", request.request_id));
    fs::write(merge_path, ensure_trailing_newline(&raw))?;
    Ok(parse_pr_merge_report(&raw))
}

struct MergeDecisionRecord<'a> {
    request: &'a Request,
    decision_id: &'a str,
    queue_decision: &'a str,
    auto_merge_enabled: bool,
    pr_status: &'a PrStatusReport,
    action: &'a str,
    reason: &'a str,
    merge_report: Option<&'a PrMergeReport>,
}

fn write_merge_decision_record(record: MergeDecisionRecord<'_>) -> Result<String> {
    let decisions_dir = Path::new(".sandrone")
        .join("state")
        .join("scheduler")
        .join("decisions");
    fs::create_dir_all(&decisions_dir)?;
    let path = decisions_dir.join(format!("{}.json", record.decision_id));
    let merge_status = record
        .merge_report
        .map(|report| report.status.as_str())
        .unwrap_or("not-run");
    let merge_url = record
        .merge_report
        .map(|report| report.url.as_str())
        .unwrap_or("");
    let merge_detail = record
        .merge_report
        .map(|report| report.detail.as_str())
        .unwrap_or("");
    let merge_raw = record
        .merge_report
        .map(|report| report.raw.as_str())
        .unwrap_or("");
    fs::write(
        &path,
        format!(
            "{{\n  \"schema_version\": 1,\n  \"decision_id\": \"{}\",\n  \"request_id\": \"{}\",\n  \"queue_decision\": \"{}\",\n  \"auto_merge_enabled\": {},\n  \"pr_status\": \"{}\",\n  \"pr_status_url\": \"{}\",\n  \"pr_status_detail\": \"{}\",\n  \"pr_status_raw\": \"{}\",\n  \"action\": \"{}\",\n  \"reason\": \"{}\",\n  \"merge_status\": \"{}\",\n  \"merge_url\": \"{}\",\n  \"merge_detail\": \"{}\",\n  \"merge_raw\": \"{}\",\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(record.decision_id),
            json_escape(&record.request.request_id),
            json_escape(record.queue_decision),
            if record.auto_merge_enabled {
                "true"
            } else {
                "false"
            },
            json_escape(&record.pr_status.status),
            json_escape(&record.pr_status.url),
            json_escape(&record.pr_status.detail),
            json_escape(&record.pr_status.raw),
            json_escape(record.action),
            json_escape(record.reason),
            json_escape(merge_status),
            json_escape(merge_url),
            json_escape(merge_detail),
            json_escape(merge_raw),
            json_escape(&now_string()),
        ),
    )?;
    Ok(path.to_string_lossy().to_string())
}

fn parse_pr_merge_report(line: &str) -> PrMergeReport {
    let fields: Vec<&str> = line.split('\t').collect();
    PrMergeReport {
        status: fields
            .first()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "blocked".to_string()),
        url: fields
            .get(1)
            .map(|value| value.trim().to_string())
            .unwrap_or_default(),
        detail: fields
            .get(2)
            .map(|value| value.trim().to_string())
            .unwrap_or_default(),
        raw: line.trim().to_string(),
    }
}

fn write_pr_body(request: &Request) -> Result<String> {
    let body_path = Path::new(".sandrone")
        .join("state")
        .join(format!("{}-pr-body.md", request.request_id));
    let change_doc = pr_change_doc_content(request)?;
    let request_path = review_context_artifact_source(request, "request.md");
    let request_doc = fs::read_to_string(&request_path)?;
    let issue_reference = render_pr_issue_reference(request);
    let review_findings = render_pr_review_findings(request);
    let body = format!(
        "# 关联需求\n\n{issue_reference}\n\n---\n\n{review_findings}\n\n---\n\n# Request\n\n{request_doc}\n\n---\n\n# Change Doc\n\n{change_doc}\n",
    );
    fs::write(&body_path, body)?;
    Ok(absolute_path_string(&body_path))
}

fn pr_change_doc_content(request: &Request) -> Result<String> {
    let change_doc_path = existing_or_preferred_request_artifact_path(request, "change-doc.md");
    if change_doc_path.exists() {
        return Ok(fs::read_to_string(change_doc_path)?);
    }
    if is_slice_request(request) {
        return Err(format!(
            "{} has no change-doc artifact: {}",
            request.request_id,
            change_doc_path.display()
        )
        .into());
    }
    let requests = load_requests()?;
    let mut sections = Vec::new();
    for slice in requests
        .iter()
        .filter(|candidate| slice_parent_id(candidate).as_deref() == Some(&request.request_id))
    {
        let slice_change_doc = existing_or_preferred_request_artifact_path(slice, "change-doc.md");
        if !slice_change_doc.exists() {
            continue;
        }
        let content = fs::read_to_string(&slice_change_doc)?;
        sections.push(format!(
            "## {} {}\n\n> 来源: `{}`\n\n{}",
            slice.request_id,
            slice.title,
            slice_change_doc.display(),
            content.trim_end()
        ));
    }
    if sections.is_empty() {
        return Err(format!(
            "{} has no parent change-doc and no slice change-docs to aggregate",
            request.request_id
        )
        .into());
    }
    Ok(format!(
        "# Slice Change Doc 汇总: {} {}\n\n{}",
        request.request_id,
        request.title,
        sections.join("\n\n---\n\n")
    ))
}

fn render_pr_review_findings(request: &Request) -> String {
    let mut lines = vec![
        "# 自动评审意见".to_string(),
        String::new(),
        "本节由 `sandrone finish` 从最终 review detail JSON 生成，方便人类在 PR 页面直接查看 reviewer 的 warning/info 以及必要的上下文。".to_string(),
        String::new(),
    ];
    let mut rendered_stage = false;

    for stage in ["plan-review", "code-review", "integration-review"] {
        let summary_path = Path::new(&request.change_path)
            .join("reviews")
            .join(stage)
            .join("summary.json");
        if !summary_path.exists() {
            continue;
        }
        rendered_stage = true;
        let summary = fs::read_to_string(&summary_path).unwrap_or_default();
        let approved = json_bool(&summary, "approved").unwrap_or(false);
        let attempt = json_number(&summary, "attempt").unwrap_or(0);
        lines.push(format!(
            "## {}",
            if stage == "plan-review" {
                "Plan Review"
            } else if stage == "integration-review" {
                "Integration Review"
            } else {
                "Code Review"
            }
        ));
        lines.push(String::new());
        lines.push(format!(
            "- Gate: {}",
            if approved { "approved" } else { "rejected" }
        ));
        lines.push(format!("- Attempt: {attempt}"));
        lines.push(format!("- Summary: `reviews/{stage}/summary.json`"));
        lines.push(String::new());

        let mut rendered_reviewer = false;
        for (reviewer, file_stem) in review_detail_file_stems(stage) {
            let detail_path = Path::new(&request.change_path)
                .join("reviews")
                .join(stage)
                .join("details")
                .join(format!("{attempt:03}-{file_stem}.json"));
            if !detail_path.exists() {
                continue;
            }
            rendered_reviewer = true;
            let detail = fs::read_to_string(&detail_path).unwrap_or_default();
            render_pr_reviewer_detail(request, stage, reviewer, &detail_path, &detail, &mut lines);
        }
        if !rendered_reviewer {
            lines.push("本轮没有找到 reviewer detail JSON。".to_string());
            lines.push(String::new());
        }
    }

    if !rendered_stage {
        lines.push("未找到自动评审结果；如果本次是人工审批，请在 PR 评审时重点核对 change doc 和验证证据。".to_string());
        lines.push(String::new());
    }

    lines.join("\n")
}

fn review_detail_file_stems(stage: &str) -> Vec<(&'static str, &'static str)> {
    if stage == "plan-review" {
        vec![("PlanReviewer", "plan-reviewer")]
    } else if stage == "integration-review" {
        vec![("IntegrationReviewer", "integration-reviewer")]
    } else {
        vec![
            ("TestReviewer", "test-reviewer"),
            ("DesignReviewer", "design-reviewer"),
        ]
    }
}

fn render_pr_reviewer_detail(
    request: &Request,
    stage: &str,
    reviewer: &str,
    detail_path: &Path,
    detail: &str,
    lines: &mut Vec<String>,
) {
    let declared_reviewer = json_value(detail, "reviewer").unwrap_or_else(|| reviewer.to_string());
    let approved = json_bool(detail, "approved").unwrap_or(false);
    let decision = json_value(detail, "decision").unwrap_or_else(|| {
        if approved {
            "approved".to_string()
        } else {
            "rejected".to_string()
        }
    });
    let recommended_next_phase =
        json_value(detail, "recommended_next_phase").unwrap_or_else(|| "unknown".to_string());
    let summary = json_value(detail, "summary").unwrap_or_else(|| "no summary".to_string());
    lines.push(format!("### {declared_reviewer}"));
    lines.push(String::new());
    lines.push(format!("- Decision: {decision}"));
    lines.push(format!(
        "- Recommended next phase: {recommended_next_phase}"
    ));
    lines.push(format!("- Summary: {}", markdown_inline(&summary)));
    lines.push(format!(
        "- Detail: `{}`",
        review_detail_relative_path(request, stage, detail_path)
    ));
    lines.push(String::new());

    let mut finding_count = 0usize;
    for severity in ["critical", "high", "warning", "info"] {
        let findings = review_findings(detail, severity);
        if findings.is_empty() {
            continue;
        }
        finding_count += findings.len();
        lines.push(format!("#### {severity}"));
        lines.push(String::new());
        for (index, finding) in findings.iter().enumerate() {
            lines.push(format!(
                "{}. **{}**",
                index + 1,
                markdown_inline(&finding.title)
            ));
            lines.push(format!(
                "   - Evidence: {}",
                markdown_inline(&finding.evidence)
            ));
            lines.push(format!("   - Impact: {}", markdown_inline(&finding.impact)));
            lines.push(format!(
                "   - Required fix: {}",
                markdown_inline(&finding.required_fix)
            ));
            lines.push(format!(
                "   - Suggested change: {}",
                markdown_inline(&finding.suggested_change)
            ));
            lines.push(format!(
                "   - Verification: {}",
                markdown_inline(&finding.verification)
            ));
        }
        lines.push(String::new());
    }

    if finding_count == 0 {
        lines.push("- Findings: 无。".to_string());
        lines.push(String::new());
    }
}

fn review_detail_relative_path(request: &Request, stage: &str, detail_path: &Path) -> String {
    let fallback = detail_path.to_string_lossy().to_string();
    let Ok(relative_to_change) = detail_path.strip_prefix(&request.change_path) else {
        return fallback;
    };
    let relative = relative_to_change.to_string_lossy();
    if relative.is_empty() {
        format!("reviews/{stage}/details")
    } else {
        relative.to_string()
    }
}

fn run_pr_tool(
    request: &Request,
    base_branch: &str,
    head_branch: &str,
    title: &str,
    body_file: &str,
    compare_url: &str,
) -> Result<(Option<String>, String, String)> {
    if !Path::new(PR_TOOL).exists() {
        return Ok((
            None,
            "skipped".to_string(),
            format!("{PR_TOOL} does not exist"),
        ));
    }
    let output = Command::new("sh")
        .arg(PR_TOOL)
        .current_dir(".")
        .env("SANDRONE_REQUEST_ID", &request.request_id)
        .env("SANDRONE_REQUEST_EXTERNAL_ID", &request.external_id)
        .env("SANDRONE_REQUEST_SOURCE", &request.source)
        .env("SANDRONE_REQUEST_TITLE", &request.title)
        .env("SANDRONE_REQUEST_URL", &request.url)
        .env("SANDRONE_CHANGE_PATH", &request.change_path)
        .env(
            "SANDRONE_CHANGE_DOC",
            request_handoff_artifact_path_string(request, "change-doc.md"),
        )
        .env(
            "SANDRONE_REQUEST",
            request_handoff_artifact_path_string(request, "request.md"),
        )
        .env("SANDRONE_WORKTREE", &request.worktree_path)
        .env("SANDRONE_PR_TITLE", title)
        .env("SANDRONE_PR_BODY_FILE", body_file)
        .env("SANDRONE_PR_BASE", base_branch)
        .env("SANDRONE_PR_HEAD", head_branch)
        .env("SANDRONE_PR_COMPARE_URL", compare_url)
        .envs(proxy_env())
        .output();
    let Ok(output) = output else {
        return Ok((
            None,
            "failed".to_string(),
            format!("{PR_TOOL} could not be executed"),
        ));
    };
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        match parse_pr_tool_success(&stdout) {
            Ok((status, url)) => Ok((Some(url), status, String::new())),
            Err(error) => Ok((None, "failed".to_string(), error.to_string())),
        }
    } else {
        Ok((
            None,
            "failed".to_string(),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

fn parse_pr_tool_success(stdout: &str) -> Result<(String, String)> {
    let line = stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| format!("{PR_TOOL} succeeded without returning a PR URL"))?;
    if let Some((status, url)) = line.split_once('\t') {
        let status = status.trim();
        let url = url.trim();
        if !matches!(status, "created" | "existing") {
            return Err(format!(
                "{PR_TOOL} returned unknown PR status: {status}. Expected created<TAB>url or existing<TAB>url"
            )
            .into());
        }
        if url.is_empty() {
            return Err(format!("{PR_TOOL} returned {status} without a PR URL").into());
        }
        return Ok((status.to_string(), url.to_string()));
    }
    Ok(("created".to_string(), line.to_string()))
}

fn push_delivery_branch(worktree: &Path, branch: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["push", "-u", "origin", branch])
        .current_dir(worktree)
        .envs(proxy_env())
        .output()?;
    if output.status.success() {
        return Ok(false);
    }

    let detail = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let non_fast_forward = detail.contains("non-fast-forward")
        || detail.contains("fetch first")
        || detail.contains("stale info")
        || detail.contains("failed to push some refs");
    if !non_fast_forward {
        return Err(review_diagnostic_excerpt(&detail).into());
    }

    run_command(
        Command::new("git")
            .args(["push", "--force-with-lease", "-u", "origin", branch])
            .current_dir(worktree)
            .envs(proxy_env()),
    )?;
    Ok(true)
}
