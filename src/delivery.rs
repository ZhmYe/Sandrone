use super::*;

pub(crate) fn deliver_finished_request(
    request: &Request,
    commit_message: &str,
) -> Result<DeliveryResult> {
    if request.worktree_path.trim().is_empty() {
        return Err(format!(
            "{} has no worktree. Run codex-auto-dev start first.",
            request.request_id
        )
        .into());
    }
    if request.branch.trim().is_empty() {
        return Err(format!(
            "{} has no branch. Run codex-auto-dev start first.",
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

fn write_pr_body(request: &Request) -> Result<String> {
    let body_path = Path::new(".codex-auto-dev")
        .join("state")
        .join(format!("{}-pr-body.md", request.request_id));
    let change_doc_path = Path::new(&request.change_path).join("change-doc.md");
    let request_path = Path::new(&request.change_path).join("request.md");
    let change_doc = fs::read_to_string(&change_doc_path)?;
    let request_doc = fs::read_to_string(&request_path)?;
    let issue_reference = render_pr_issue_reference(request);
    let review_findings = render_pr_review_findings(request);
    let body = format!(
        "# 关联需求\n\n{issue_reference}\n\n---\n\n{review_findings}\n\n---\n\n# Request\n\n{request_doc}\n\n---\n\n# Change Doc\n\n{change_doc}\n",
    );
    fs::write(&body_path, body)?;
    Ok(absolute_path_string(&body_path))
}

fn render_pr_review_findings(request: &Request) -> String {
    let mut lines = vec![
        "# 自动评审意见".to_string(),
        String::new(),
        "本节由 `codex-auto-dev finish` 从最终 review detail JSON 生成，方便人类在 PR 页面直接查看 reviewer 的 warning/info 以及必要的上下文。".to_string(),
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
        .env("CODEX_AUTO_DEV_REQUEST_ID", &request.request_id)
        .env("CODEX_AUTO_DEV_REQUEST_EXTERNAL_ID", &request.external_id)
        .env("CODEX_AUTO_DEV_REQUEST_SOURCE", &request.source)
        .env("CODEX_AUTO_DEV_REQUEST_TITLE", &request.title)
        .env("CODEX_AUTO_DEV_REQUEST_URL", &request.url)
        .env("CODEX_AUTO_DEV_CHANGE_PATH", &request.change_path)
        .env(
            "CODEX_AUTO_DEV_CHANGE_DOC",
            format!("{}/change-doc.md", request.change_path),
        )
        .env(
            "CODEX_AUTO_DEV_REQUEST",
            format!("{}/request.md", request.change_path),
        )
        .env("CODEX_AUTO_DEV_WORKTREE", &request.worktree_path)
        .env("CODEX_AUTO_DEV_PR_TITLE", title)
        .env("CODEX_AUTO_DEV_PR_BODY_FILE", body_file)
        .env("CODEX_AUTO_DEV_PR_BASE", base_branch)
        .env("CODEX_AUTO_DEV_PR_HEAD", head_branch)
        .env("CODEX_AUTO_DEV_PR_COMPARE_URL", compare_url)
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
