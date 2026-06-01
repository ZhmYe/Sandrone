use super::*;

pub(crate) fn plan_review(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;

    let reviewers = [ReviewDefinition {
        name: "PlanReviewer",
        tool: PLAN_REVIEW_TOOL,
        file_stem: "plan-reviewer",
    }];
    let results = run_review_stage(&request, "plan-review", &reviewers)?;
    if reviews_approved(&results) {
        approve_gate_from_review(
            &mut requests,
            index,
            &mut request,
            "plan",
            "PlanReviewer",
            "plan-review",
            "PlanReviewer approved the plan review gate",
        )?;
        println!("Plan review approved for {request_id}");
        println!(
            "  review summary: {}/reviews/plan-review/summary.json",
            request.change_path
        );
        println!(
            "  approval: {}",
            approval_file_path(&request, "plan").display()
        );
        Ok(())
    } else {
        if review_gate_unavailable(&results) {
            let reason = review_gate_unavailable_reason("plan-review", &results);
            mark_blocked(&mut requests, index, &mut request, "planning", &reason)?;
            return Err(format!(
                "{} review gate unavailable: {reason}",
                rejected_reviewers(&results).join(", ")
            )
            .into());
        }
        mark_review_rejected(
            &mut requests,
            index,
            &mut request,
            "planning",
            "plan-review",
            "plan-review rejected; return to planning",
        )?;
        let rejected = rejected_reviewers(&results);
        Err(format!("{} rejected plan review", rejected.join(", ")).into())
    }
}

pub(crate) fn code_review(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;
    ensure_gate_approved(&request, "plan")?;
    if request.worktree_path.trim().is_empty() {
        return Err(
            format!("{request_id} has no worktree. Run codex-auto-dev start first.").into(),
        );
    }

    let reviewers = [
        ReviewDefinition {
            name: "TestReviewer",
            tool: TEST_REVIEW_TOOL,
            file_stem: "test-reviewer",
        },
        ReviewDefinition {
            name: "DesignReviewer",
            tool: DESIGN_REVIEW_TOOL,
            file_stem: "design-reviewer",
        },
    ];
    let results = run_review_stage(&request, "code-review", &reviewers)?;
    if reviews_approved(&results) {
        approve_gate_from_review(
            &mut requests,
            index,
            &mut request,
            "change-doc",
            "code-review",
            "code-review",
            "TestReviewer and DesignReviewer approved the code review gate",
        )?;
        mark_wait_update_pr_by_id(
            &request_id,
            "code-review approved; waiting for PR creation or update",
        )?;
        println!("Code review approved for {request_id}");
        println!(
            "  review summary: {}/reviews/code-review/summary.json",
            request.change_path
        );
        println!(
            "  approval: {}",
            approval_file_path(&request, "change-doc").display()
        );
        Ok(())
    } else {
        if review_gate_unavailable(&results) {
            let reason = review_gate_unavailable_reason("code-review", &results);
            mark_blocked(
                &mut requests,
                index,
                &mut request,
                "implementation",
                &reason,
            )?;
            return Err(format!(
                "{} review gate unavailable: {reason}",
                rejected_reviewers(&results).join(", ")
            )
            .into());
        }
        match recommended_next_phase(&results, "implementation").as_str() {
            "planning" => {
                mark_review_rejected(
                    &mut requests,
                    index,
                    &mut request,
                    "planning",
                    "plan-review",
                    "code-review requested planning; reviewer findings require plan revision",
                )?;
            }
            "blocked" => {
                mark_blocked(
                    &mut requests,
                    index,
                    &mut request,
                    "implementation",
                    "code-review recommended blocking; manual recovery is required",
                )?;
            }
            _ => {
                mark_review_rejected(
                    &mut requests,
                    index,
                    &mut request,
                    "implementation",
                    "code-review",
                    "code-review rejected; return to implementation",
                )?;
            }
        }
        let rejected = rejected_reviewers(&results);
        Err(format!("{} rejected code review", rejected.join(", ")).into())
    }
}

pub(crate) fn integration_review(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;
    ensure_gate_approved(&request, "plan")?;
    if request.worktree_path.trim().is_empty() {
        return Err(
            format!("{request_id} has no worktree. Run codex-auto-dev start first.").into(),
        );
    }

    let reviewers = [ReviewDefinition {
        name: "IntegrationReviewer",
        tool: INTEGRATION_REVIEW_TOOL,
        file_stem: "integration-reviewer",
    }];
    let results = run_review_stage(&request, "integration-review", &reviewers)?;
    if reviews_approved(&results) {
        approve_gate_from_review(
            &mut requests,
            index,
            &mut request,
            "change-doc",
            "IntegrationReviewer",
            "integration-review",
            "IntegrationReviewer approved the rebase integration gate",
        )?;
        mark_wait_update_pr_by_id(
            &request_id,
            "integration-review approved; waiting for PR branch update",
        )?;
        println!("Integration review approved for {request_id}");
        println!(
            "  review summary: {}/reviews/integration-review/summary.json",
            request.change_path
        );
        println!(
            "  approval: {}",
            approval_file_path(&request, "change-doc").display()
        );
        Ok(())
    } else {
        if review_gate_unavailable(&results) {
            let reason = review_gate_unavailable_reason("integration-review", &results);
            mark_blocked(&mut requests, index, &mut request, "rebase", &reason)?;
            return Err(format!(
                "{} review gate unavailable: {reason}",
                rejected_reviewers(&results).join(", ")
            )
            .into());
        }
        match recommended_next_phase(&results, "implementation").as_str() {
            "blocked" => {
                mark_blocked(
                    &mut requests,
                    index,
                    &mut request,
                    "rebase",
                    "integration-review recommended blocking; manual recovery is required",
                )?;
            }
            _ => {
                mark_review_rejected(
                    &mut requests,
                    index,
                    &mut request,
                    "rebase",
                    "integration-review",
                    "integration-review rejected; return to RebaseAgent",
                )?;
            }
        }
        let rejected = rejected_reviewers(&results);
        Err(format!("{} rejected integration review", rejected.join(", ")).into())
    }
}

fn run_review_stage(
    request: &Request,
    stage: &str,
    reviewers: &[ReviewDefinition],
) -> Result<Vec<ReviewResult>> {
    let review_dir = Path::new(&request.change_path).join("reviews").join(stage);
    let details_dir = review_dir.join("details");
    fs::create_dir_all(&review_dir)?;
    fs::create_dir_all(&details_dir)?;
    let attempt = next_review_attempt(&details_dir)?;
    let mut results = Vec::new();
    for reviewer in reviewers {
        results.push(run_single_reviewer(
            request,
            stage,
            reviewer,
            &details_dir,
            attempt,
        )?);
    }
    write_review_summary(request, stage, attempt, &results)?;
    update_change_doc_review_section(request)?;
    Ok(results)
}

fn run_single_reviewer(
    request: &Request,
    stage: &str,
    reviewer: &ReviewDefinition,
    details_dir: &Path,
    attempt: u32,
) -> Result<ReviewResult> {
    let output_path = details_dir.join(format!("{attempt:03}-{}.json", reviewer.file_stem));
    let review_context = prepare_review_context(request, stage, reviewer, attempt)?;
    let review_context_string = absolute_path_string(&review_context);
    let forbidden_review_paths = format!(
        "{};{}",
        absolute_path_string(Path::new(&request.change_path).join("reviews")),
        absolute_path_string(Path::new(&request.change_path).join("reviews").join(stage)),
    );
    let (content, tool_unavailable, diagnostic) = if !Path::new(reviewer.tool).exists() {
        let diagnostic = format!("{} does not exist", reviewer.tool);
        (
            rejected_review_json(reviewer.name, "review tool missing", &diagnostic),
            true,
            diagnostic,
        )
    } else {
        let output = Command::new("sh")
            .arg(reviewer.tool)
            .current_dir(".")
            .env("CODEX_AUTO_DEV_REVIEW_STAGE", stage)
            .env("CODEX_AUTO_DEV_REVIEWER", reviewer.name)
            .env("CODEX_AUTO_DEV_WORKSPACE", absolute_path_string("."))
            .env("CODEX_AUTO_DEV_TARGET_REPO", absolute_path_string(DEV_REPO))
            .env("CODEX_AUTO_DEV_REQUEST_ID", &request.request_id)
            .env("CODEX_AUTO_DEV_REQUEST_EXTERNAL_ID", &request.external_id)
            .env("CODEX_AUTO_DEV_REQUEST_SOURCE", &request.source)
            .env("CODEX_AUTO_DEV_REQUEST_TITLE", &request.title)
            .env("CODEX_AUTO_DEV_REQUEST_BODY", &request.body)
            .env("CODEX_AUTO_DEV_REQUEST_URL", &request.url)
            .env("CODEX_AUTO_DEV_CHANGE_PATH", &review_context_string)
            .env("CODEX_AUTO_DEV_REVIEW_CONTEXT", &review_context_string)
            .env(
                "CODEX_AUTO_DEV_CANONICAL_CHANGE_PATH",
                absolute_path_string(request.change_path.as_str()),
            )
            .env(
                "CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS",
                forbidden_review_paths,
            )
            .env(
                "CODEX_AUTO_DEV_REQUEST",
                absolute_path_string(review_context.join("request.md")),
            )
            .env(
                "CODEX_AUTO_DEV_ISSUE",
                absolute_path_string(review_context.join("request.md")),
            )
            .env(
                "CODEX_AUTO_DEV_SPEC",
                absolute_path_string(review_context.join("plan.md")),
            )
            .env(
                "CODEX_AUTO_DEV_PLAN",
                absolute_path_string(review_context.join("plan.md")),
            )
            .env(
                "CODEX_AUTO_DEV_TASKS",
                absolute_path_string(review_context.join("plan.md")),
            )
            .env(
                "CODEX_AUTO_DEV_CHANGE_DOC",
                absolute_path_string(review_context.join("change-doc.md")),
            )
            .env(
                "CODEX_AUTO_DEV_WORKTREE",
                absolute_path_string(request.worktree_path.as_str()),
            )
            .env(
                "CODEX_AUTO_DEV_REVIEW_SCHEMA",
                absolute_path_string(REVIEW_SCHEMA),
            )
            .output();
        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8(output.stdout)?;
                if stdout.trim().is_empty() {
                    let diagnostic = "review tool succeeded without JSON output".to_string();
                    (
                        rejected_review_json(reviewer.name, "empty review output", &diagnostic),
                        true,
                        diagnostic,
                    )
                } else {
                    (stdout, false, String::new())
                }
            }
            Ok(output) => {
                let diagnostic =
                    review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr));
                (
                    rejected_review_json(reviewer.name, "review tool failed", &diagnostic),
                    true,
                    diagnostic,
                )
            }
            Err(error) => {
                let diagnostic = error.to_string();
                (
                    rejected_review_json(reviewer.name, "review tool could not run", &diagnostic),
                    true,
                    diagnostic,
                )
            }
        }
    };

    let (normalized, invalid_json) = normalize_review_json(reviewer.name, &content);
    fs::write(&output_path, ensure_trailing_newline(&normalized))?;
    let approved = json_bool(&normalized, "approved").unwrap_or(false);
    let has_blocking_findings = review_has_blocking_findings(&normalized);
    let reviewer_declared_unavailable = json_bool(&normalized, "gate_unavailable").unwrap_or(false);
    let recommended_next_phase = normalize_recommended_next_phase(
        &json_value(&normalized, "recommended_next_phase").unwrap_or_else(|| {
            default_recommended_next_phase(reviewer.name, approved, reviewer_declared_unavailable)
                .to_string()
        }),
        reviewer.name,
        approved,
        reviewer_declared_unavailable,
    );
    let summary = json_value(&normalized, "summary").unwrap_or_else(|| "no summary".to_string());
    let diagnostic = if invalid_json && diagnostic.is_empty() {
        review_diagnostic_excerpt(&content)
    } else {
        diagnostic
    };
    Ok(ReviewResult {
        reviewer: reviewer.name.to_string(),
        approved,
        has_blocking_findings,
        gate_unavailable: tool_unavailable || invalid_json || reviewer_declared_unavailable,
        recommended_next_phase,
        summary,
        diagnostic,
        path: output_path.to_string_lossy().to_string(),
    })
}

fn prepare_review_context(
    request: &Request,
    stage: &str,
    reviewer: &ReviewDefinition,
    attempt: u32,
) -> Result<PathBuf> {
    let context = Path::new(".codex-auto-dev/state/review-contexts")
        .join(&request.request_id)
        .join(stage)
        .join(format!("{attempt:03}"))
        .join(slugify(reviewer.name));
    if context.exists() {
        fs::remove_dir_all(&context)?;
    }
    fs::create_dir_all(&context)?;
    for artifact in ["request.md", "plan.md", "change-doc.md", "status.json"] {
        let source = Path::new(&request.change_path).join(artifact);
        if source.exists() {
            fs::copy(&source, context.join(artifact))?;
        }
    }
    let approvals_source = Path::new(&request.change_path).join("approvals");
    if approvals_source.exists() {
        let approvals_target = context.join("approvals");
        fs::create_dir_all(&approvals_target)?;
        for entry in fs::read_dir(approvals_source)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::copy(entry.path(), approvals_target.join(entry.file_name()))?;
            }
        }
    }
    Ok(context)
}

fn normalize_review_json(reviewer: &str, content: &str) -> (String, bool) {
    let trimmed = content.trim();
    if trimmed.starts_with('{')
        && trimmed.ends_with('}')
        && json_bool(trimmed, "approved").is_some()
        && json_bool(trimmed, "gate_unavailable").is_some()
        && json_value(trimmed, "reviewer").is_some()
        && json_value(trimmed, "decision").is_some()
        && json_value(trimmed, "recommended_next_phase").is_some()
        && json_value(trimmed, "summary").is_some()
        && review_json_has_required_arrays(trimmed)
        && review_json_findings_have_required_fields(trimmed)
    {
        (trimmed.to_string(), false)
    } else {
        (
            rejected_review_json(
                reviewer,
                "invalid review JSON",
                "review tool must return one JSON object matching review-result.schema.json",
            ),
            true,
        )
    }
}

fn review_json_has_required_arrays(content: &str) -> bool {
    ["process", "critical", "high", "warning", "info"]
        .iter()
        .all(|key| content.contains(&format!("\"{key}\"")))
}

fn review_json_findings_have_required_fields(content: &str) -> bool {
    if !content.contains("\"title\"") {
        return true;
    }
    [
        "\"evidence\"",
        "\"impact\"",
        "\"required_fix\"",
        "\"suggested_change\"",
        "\"verification\"",
    ]
    .iter()
    .all(|needle| content.contains(needle))
}

fn default_recommended_next_phase(
    reviewer: &str,
    approved: bool,
    gate_unavailable: bool,
) -> &'static str {
    if gate_unavailable {
        "blocked"
    } else if approved {
        "implementation"
    } else if reviewer == "PlanReviewer" {
        "planning"
    } else {
        "implementation"
    }
}

fn normalize_recommended_next_phase(
    value: &str,
    reviewer: &str,
    approved: bool,
    gate_unavailable: bool,
) -> String {
    match value.trim() {
        "planning" | "implementation" | "blocked" => value.trim().to_string(),
        _ => default_recommended_next_phase(reviewer, approved, gate_unavailable).to_string(),
    }
}

pub(crate) fn review_diagnostic_excerpt(detail: &str) -> String {
    let collapsed = detail
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut excerpt = String::new();
    for ch in collapsed.chars().take(500) {
        excerpt.push(ch);
    }
    if excerpt.is_empty() {
        "review tool failed without stderr diagnostics".to_string()
    } else {
        excerpt
    }
}

fn rejected_review_json(reviewer: &str, title: &str, detail: &str) -> String {
    format!(
        "{{\n  \"reviewer\": \"{}\",\n  \"approved\": false,\n  \"gate_unavailable\": true,\n  \"decision\": \"rejected\",\n  \"recommended_next_phase\": \"blocked\",\n  \"summary\": \"{}\",\n  \"process\": [\"review tool failure was converted into a blocking finding\"],\n  \"critical\": [{{ \"title\": \"{}\", \"evidence\": \"{}\", \"impact\": \"review gate cannot make a reliable approval decision\", \"required_fix\": \"Fix the reviewer tool or return valid structured review JSON.\", \"suggested_change\": \"Inspect the reviewer script stderr/stdout, restore the configured model backend, and make stdout exactly one JSON object matching tools/schemas/review-result.schema.json.\", \"verification\": \"Rerun the same codex-auto-dev review command and confirm the detail JSON validates and gate_unavailable is false.\" }}],\n  \"high\": [],\n  \"warning\": [],\n  \"info\": []\n}}",
        json_escape(reviewer),
        json_escape(title),
        json_escape(title),
        json_escape(detail),
    )
}

fn write_review_summary(
    request: &Request,
    stage: &str,
    attempt: u32,
    results: &[ReviewResult],
) -> Result<()> {
    let summary_path = Path::new(&request.change_path)
        .join("reviews")
        .join(stage)
        .join("summary.json");
    let approved = reviews_approved(results);
    let mut reviewers = String::new();
    for (index, result) in results.iter().enumerate() {
        if index > 0 {
            reviewers.push_str(",\n");
        }
        reviewers.push_str(&format!(
            "    {{ \"reviewer\": \"{}\", \"approved\": {}, \"has_blocking_findings\": {}, \"gate_unavailable\": {}, \"recommended_next_phase\": \"{}\", \"summary\": \"{}\", \"diagnostic\": \"{}\", \"path\": \"{}\" }}",
            json_escape(&result.reviewer),
            json_bool_literal(result.approved),
            json_bool_literal(result.has_blocking_findings),
            json_bool_literal(result.gate_unavailable),
            json_escape(&result.recommended_next_phase),
            json_escape(&result.summary),
            json_escape(&result.diagnostic),
            json_escape(&result.path),
        ));
    }
    fs::write(
        summary_path,
        format!(
            "{{\n  \"schema_version\": 1,\n  \"request_id\": \"{}\",\n  \"stage\": \"{}\",\n  \"attempt\": {},\n  \"approved\": {},\n  \"reviewers\": [\n{}\n  ],\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(&request.request_id),
            json_escape(stage),
            attempt,
            json_bool_literal(approved),
            reviewers,
            json_escape(&now_string()),
        ),
    )?;
    Ok(())
}

fn next_review_attempt(details_dir: &Path) -> Result<u32> {
    let mut max_attempt = 0;
    if details_dir.exists() {
        for entry in fs::read_dir(details_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let Some((prefix, _)) = name.split_once('-') else {
                continue;
            };
            if let Ok(value) = prefix.parse::<u32>() {
                max_attempt = max_attempt.max(value);
            }
        }
    }
    Ok(max_attempt + 1)
}

fn update_change_doc_review_section(request: &Request) -> Result<()> {
    let path = Path::new(&request.change_path).join("change-doc.md");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&path)?;
    let section = render_review_results_section(request);
    fs::write(
        path,
        replace_markdown_section(&content, "## Review 结果", &section),
    )?;
    Ok(())
}

fn render_review_results_section(request: &Request) -> String {
    let mut lines = vec!["## Review 结果".to_string(), String::new()];
    for stage in ["plan-review", "code-review", "integration-review"] {
        let summary_path = Path::new(&request.change_path)
            .join("reviews")
            .join(stage)
            .join("summary.json");
        if !summary_path.exists() {
            continue;
        }
        let content = fs::read_to_string(&summary_path).unwrap_or_default();
        let approved = json_bool(&content, "approved").unwrap_or(false);
        let attempt = json_number(&content, "attempt").unwrap_or(0);
        lines.push(format!(
            "### {}",
            match stage {
                "plan-review" => "Plan Review",
                "code-review" => "Code Review",
                "integration-review" => "Integration Review",
                _ => stage,
            }
        ));
        lines.push(String::new());
        lines.push(format!(
            "- 最终状态: {}",
            if approved { "approved" } else { "rejected" }
        ));
        lines.push(format!("- 尝试次数: {attempt}"));
        lines.push(format!("- 详情: `reviews/{stage}/summary.json`"));
        for reviewer in [
            "PlanReviewer",
            "TestReviewer",
            "DesignReviewer",
            "IntegrationReviewer",
        ] {
            if content.contains(&format!("\"reviewer\": \"{reviewer}\"")) {
                lines.push(format!("- {reviewer}: 已记录"));
            }
        }
        lines.push(String::new());
    }
    if lines.len() == 2 {
        lines.push("尚未产生 review 结果。".to_string());
        lines.push(String::new());
    }
    lines.join("\n")
}

fn replace_markdown_section(content: &str, heading: &str, replacement: &str) -> String {
    let Some(start) = content.find(heading) else {
        return format!("{}\n\n{}\n", content.trim_end(), replacement.trim_end());
    };
    let after_heading = start + heading.len();
    let next_heading = content[after_heading..]
        .find("\n## ")
        .map(|offset| after_heading + offset);
    let end = next_heading.unwrap_or(content.len());
    format!(
        "{}{}{}",
        &content[..start],
        replacement.trim_end(),
        &content[end..]
    )
}

fn reviews_approved(results: &[ReviewResult]) -> bool {
    !results.is_empty()
        && results.iter().all(|result| {
            result.approved && !result.has_blocking_findings && !result.gate_unavailable
        })
}

fn review_gate_unavailable(results: &[ReviewResult]) -> bool {
    results.iter().any(|result| result.gate_unavailable)
}

fn review_gate_unavailable_reason(stage: &str, results: &[ReviewResult]) -> String {
    let diagnostics = results
        .iter()
        .filter(|result| result.gate_unavailable)
        .map(|result| {
            let diagnostic = fallback_empty(&result.diagnostic, "no diagnostic available");
            format!(
                "{}: {} ({diagnostic}); details: {}",
                result.reviewer, result.summary, result.path
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("{stage} gate unavailable; {diagnostics}")
}

fn rejected_reviewers(results: &[ReviewResult]) -> Vec<String> {
    results
        .iter()
        .filter(|result| !result.approved || result.has_blocking_findings)
        .map(|result| result.reviewer.clone())
        .collect()
}

fn recommended_next_phase(results: &[ReviewResult], default_phase: &str) -> String {
    if results
        .iter()
        .any(|result| result.gate_unavailable || result.recommended_next_phase == "blocked")
    {
        return "blocked".to_string();
    }
    if results
        .iter()
        .any(|result| result.recommended_next_phase == "planning")
    {
        return "planning".to_string();
    }
    if results
        .iter()
        .any(|result| result.recommended_next_phase == "implementation")
    {
        return "implementation".to_string();
    }
    default_phase.to_string()
}

fn approve_gate_from_review(
    requests: &mut [Request],
    index: usize,
    request: &mut Request,
    gate: &str,
    by: &str,
    source: &str,
    comment: &str,
) -> Result<()> {
    write_approval_record(request, gate, "approved", by, source, comment)?;
    request.status = format!("{}-approved", gate_status_prefix(gate));
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(requests)?;
    let stage = if gate == "plan" {
        "planning"
    } else {
        "implementation"
    };
    write_status_json(request, stage, &request.status, comment)?;
    append_event(
        "gate_approved",
        &request.request_id,
        stage,
        &request.status,
        &format!("gate={gate}; source={source}; by={by}"),
    )?;
    update_gate_session(request, gate, "approved")
}

fn mark_review_rejected(
    requests: &mut [Request],
    index: usize,
    request: &mut Request,
    phase: &str,
    stage: &str,
    reason: &str,
) -> Result<()> {
    request.status = format!("{stage}-rejected");
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(requests)?;
    write_status_json(request, phase, &request.status, reason)?;
    append_event(
        "review_rejected",
        &request.request_id,
        phase,
        &request.status,
        reason,
    )?;
    upsert_session_for_request(request, phase, "review-rejected")
}
