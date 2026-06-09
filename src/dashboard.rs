use super::*;
use crate::registry;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

pub(crate) fn dashboard(args: &[String]) -> Result<()> {
    ensure_allowed_flags(args, &["--json", "--host", "--port"])?;
    if flag_present(args, "--json") {
        println!("{}", render_dashboard_json_from_registry()?);
        return Ok(());
    }

    let host = flag_value(args, "--host")?.unwrap_or_else(|| DEFAULT_DASHBOARD_HOST.to_string());
    let port = parse_dashboard_port(flag_value(args, "--port")?)?;
    let listener = TcpListener::bind(format!("{host}:{port}"))?;
    let address = listener.local_addr()?;
    println!("sandrone dashboard running:");
    println!("  http://{address}/");
    println!(
        "  registry: {}",
        registry::global_workspaces_path().display()
    );
    println!("Press Ctrl-C to stop.");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = handle_dashboard_stream(&mut stream) {
                    eprintln!("dashboard request failed: {error}");
                }
            }
            Err(error) => eprintln!("dashboard connection failed: {error}"),
        }
    }
    Ok(())
}

fn parse_dashboard_port(value: Option<String>) -> Result<u16> {
    let Some(value) = value else {
        return Ok(DEFAULT_DASHBOARD_PORT);
    };
    value
        .parse::<u16>()
        .map_err(|_| format!("--port must be a valid TCP port: {value}").into())
}

fn handle_dashboard_stream(stream: &mut TcpStream) -> Result<()> {
    let mut buffer = [0u8; 4096];
    let bytes_read = stream.read(&mut buffer)?;
    if bytes_read == 0 {
        return Ok(());
    }
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut parts = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or("/");
    if method != "GET" {
        write_http_response(
            stream,
            "405 Method Not Allowed",
            "application/json; charset=utf-8",
            "{\"error\":\"method not allowed\"}\n",
        )?;
        return Ok(());
    }

    let clean_path = path.split('?').next().unwrap_or(path);
    match clean_path {
        "/" | "/index.html" => write_http_response(
            stream,
            "200 OK",
            "text/html; charset=utf-8",
            dashboard_html(),
        )?,
        "/api/dashboard" => {
            let body = render_dashboard_json_from_registry()?;
            write_http_response(stream, "200 OK", "application/json; charset=utf-8", &body)?;
        }
        "/api/health" => write_http_response(
            stream,
            "200 OK",
            "application/json; charset=utf-8",
            "{\"status\":\"ok\"}\n",
        )?,
        _ => write_http_response(
            stream,
            "404 Not Found",
            "application/json; charset=utf-8",
            "{\"error\":\"not found\"}\n",
        )?,
    }
    Ok(())
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn render_dashboard_json_from_registry() -> Result<String> {
    let records = registry::refresh_registered_workspaces()?;
    let mut projects = String::new();
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            projects.push_str(",\n");
        }
        projects.push_str(&render_dashboard_project_json(record)?);
    }
    Ok(format!(
        "{{\n  \"schema_version\": 1,\n  \"generated_at\": \"{}\",\n  \"registry_path\": \"{}\",\n  \"projects\": [\n{}\n  ]\n}}",
        json_escape(&now_string()),
        json_escape(&registry::global_workspaces_path().to_string_lossy()),
        projects,
    ))
}

fn render_dashboard_project_json(record: &WorkspaceRecord) -> Result<String> {
    let (requests_json, request_count, status_counts_json) = if record.last_status == "ready"
        && Path::new(&record.workspace_path).join(CONFIG_PATH).exists()
    {
        registry::with_current_dir(Path::new(&record.workspace_path), || {
            let requests = load_requests()?;
            let parent_requests = requests
                .iter()
                .filter(|request| is_parent_request(request))
                .collect::<Vec<_>>();
            let status_counts = dashboard_status_counts(&parent_requests);
            let mut rendered = String::new();
            for (index, request) in parent_requests.iter().enumerate() {
                if index > 0 {
                    rendered.push_str(",\n");
                }
                rendered.push_str(&render_dashboard_parent_request_json(request, &requests)?);
            }
            Ok((
                rendered,
                parent_requests.len(),
                registry::render_usize_map_json(&status_counts),
            ))
        })?
    } else {
        (
            String::new(),
            record.request_count,
            registry::render_usize_map_json(&record.status_counts),
        )
    };

    Ok(format!(
        "    {{\n      \"key\": \"{}\",\n      \"repo_name\": \"{}\",\n      \"git_url\": \"{}\",\n      \"workspace_path\": \"{}\",\n      \"target_repo\": \"{}\",\n      \"last_status\": \"{}\",\n      \"request_count\": {},\n      \"status_counts\": {},\n      \"updated_at\": \"{}\",\n      \"requests\": [\n{}\n      ]\n    }}",
        json_escape(&record.key),
        json_escape(&record.repo_name),
        json_escape(&record.git_url),
        json_escape(&record.workspace_path),
        json_escape(&record.target_repo),
        json_escape(&record.last_status),
        request_count,
        status_counts_json,
        json_escape(&record.updated_at),
        requests_json,
    ))
}

fn dashboard_status_counts(requests: &[&Request]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for request in requests {
        *counts.entry(request.status.clone()).or_insert(0) += 1;
    }
    counts
}

fn render_dashboard_parent_request_json(
    request: &Request,
    all_requests: &[Request],
) -> Result<String> {
    let decomposition_stages = render_dashboard_decomposition_stages(request)?;
    let slices = all_requests
        .iter()
        .filter(|candidate| slice_parent_id(candidate).as_deref() == Some(&request.request_id))
        .collect::<Vec<_>>();
    let mut slices_json = String::new();
    for (index, slice) in slices.iter().enumerate() {
        if index > 0 {
            slices_json.push_str(",\n");
        }
        slices_json.push_str(&render_dashboard_slice_request_json(slice)?);
    }
    let pr_stages = render_dashboard_pr_stages(request)?;
    render_dashboard_request_object_json(
        request,
        &decomposition_stages,
        &format!(
            ",\n          \"decomposition\": {{ \"label\": \"需求分析\", \"stages\": [\n{}\n          ] }},\n          \"slices\": [\n{}\n          ],\n          \"pr\": {{ \"label\": \"PR\", \"stages\": [\n{}\n          ] }}",
            decomposition_stages.join(",\n"),
            slices_json,
            pr_stages.join(",\n"),
        ),
    )
}

fn render_dashboard_decomposition_stages(request: &Request) -> Result<Vec<String>> {
    let mut stages = vec![render_dashboard_stage_json(
        request,
        "request",
        "Request",
        "需求记录",
        &request_artifact_path(request, "request.md"),
        dashboard_request_content(request),
        "markdown",
    )?];
    if dashboard_has_decomposition(request) {
        stages.push(render_dashboard_stage_json(
            request,
            "decomposition",
            "Decomposition",
            "需求拆解",
            &request_artifact_path(request, "decomposition.md"),
            dashboard_file_content(&request_artifact_path(request, "decomposition.md")),
            "markdown",
        )?);
        stages.push(render_dashboard_review_stage_json(
            request,
            "decomposition-review",
            "Decomposition Review",
            "拆解评审",
        )?);
    }
    Ok(stages)
}

fn render_dashboard_slice_request_json(request: &Request) -> Result<String> {
    let stages = vec![
        render_dashboard_stage_json(
            request,
            "plan",
            "Plan",
            "计划文档",
            &request_artifact_path(request, "plan.md"),
            dashboard_file_content(&request_artifact_path(request, "plan.md")),
            "markdown",
        )?,
        render_dashboard_review_stage_json(request, "plan-review", "Plan Review", "计划评审")?,
        render_dashboard_stage_json(
            request,
            "implementation",
            "Implementation",
            "实现与变更文档",
            &request_artifact_path(request, "change-doc.md"),
            dashboard_file_content(&request_artifact_path(request, "change-doc.md")),
            "markdown",
        )?,
        render_dashboard_review_stage_json(request, "code-review", "Code Review", "代码评审")?,
    ];
    render_dashboard_request_object_json(request, &stages, "")
}

fn render_dashboard_pr_stages(request: &Request) -> Result<Vec<String>> {
    let pr_refresh_artifact = dashboard_pr_refresh_artifact_path(request);
    let mut stages = vec![render_dashboard_stage_json(
        request,
        "finish-pr",
        "PR",
        "PR 交付",
        &request_handoff_artifact_path_string(request, "pr-doc.md"),
        dashboard_file_content(&request_handoff_artifact_path_string(request, "pr-doc.md")),
        "markdown",
    )?];
    if dashboard_has_pr_refresh(request) {
        stages.push(render_dashboard_stage_json(
            request,
            "pr-refresh",
            "PR Refresh",
            "PR 冲突刷新",
            &pr_refresh_artifact,
            dashboard_file_content(&pr_refresh_artifact),
            "markdown",
        )?);
        stages.push(render_dashboard_review_stage_json(
            request,
            "integration-review",
            "Integration Review",
            "集成评审",
        )?);
    }
    Ok(stages)
}

fn render_dashboard_request_object_json(
    request: &Request,
    stages: &[String],
    extra_fields: &str,
) -> Result<String> {
    Ok(format!(
        "        {{\n          \"request_id\": \"{}\",\n          \"external_id\": \"{}\",\n          \"source\": \"{}\",\n          \"title\": \"{}\",\n          \"body\": \"{}\",\n          \"url\": \"{}\",\n          \"status\": \"{}\",\n          \"stage\": \"{}\",\n          \"change_name\": \"{}\",\n          \"change_path\": \"{}\",\n          \"branch\": \"{}\",\n          \"worktree_path\": \"{}\",\n          \"created_at\": \"{}\",\n          \"updated_at\": \"{}\",\n          \"stages\": [\n{}\n          ]{}\n        }}",
        json_escape(&request.request_id),
        json_escape(&request.external_id),
        json_escape(&request.source),
        json_escape(&request.title),
        json_escape(&request.body),
        json_escape(&request.url),
        json_escape(&request.status),
        json_escape(&dashboard_current_stage(&request.status)),
        json_escape(&request.change_name),
        json_escape(&request.change_path),
        json_escape(&request.branch),
        json_escape(&request.worktree_path),
        json_escape(&request.created_at),
        json_escape(&request.updated_at),
        stages.join(",\n"),
        extra_fields,
    ))
}

fn render_dashboard_stage_json(
    request: &Request,
    stage_id: &str,
    label: &str,
    title: &str,
    artifact_path: &str,
    content: String,
    artifact_kind: &str,
) -> Result<String> {
    let artifact_name = dashboard_artifact_name(artifact_path, title);
    Ok(format!(
        "            {{ \"stage_id\": \"{}\", \"label\": \"{}\", \"title\": \"{}\", \"state\": \"{}\", \"artifact_path\": \"{}\", \"artifact_name\": \"{}\", \"artifact_kind\": \"{}\", \"content\": \"{}\", \"review_attempts\": [] }}",
        json_escape(stage_id),
        json_escape(label),
        json_escape(title),
        json_escape(&dashboard_stage_state(request, stage_id)),
        json_escape(artifact_path),
        json_escape(&artifact_name),
        json_escape(artifact_kind),
        json_escape(&content),
    ))
}

fn render_dashboard_review_stage_json(
    request: &Request,
    stage_id: &str,
    label: &str,
    title: &str,
) -> Result<String> {
    let attempts = render_review_attempts_json(request, stage_id)?;
    let artifact_path = if request.change_path.is_empty() {
        String::new()
    } else {
        format!("{}/reviews/{stage_id}/details", request.change_path)
    };
    let artifact_name = if artifact_path.is_empty() {
        title.to_string()
    } else {
        format!("reviews/{stage_id}/details")
    };
    Ok(format!(
        "            {{ \"stage_id\": \"{}\", \"label\": \"{}\", \"title\": \"{}\", \"state\": \"{}\", \"artifact_path\": \"{}\", \"artifact_name\": \"{}\", \"artifact_kind\": \"review-details\", \"content\": \"\", \"review_attempts\": {} }}",
        json_escape(stage_id),
        json_escape(label),
        json_escape(title),
        json_escape(&dashboard_stage_state(request, stage_id)),
        json_escape(&artifact_path),
        json_escape(&artifact_name),
        attempts,
    ))
}

fn dashboard_artifact_name(artifact_path: &str, fallback: &str) -> String {
    Path::new(artifact_path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn dashboard_current_stage(status: &str) -> String {
    match canonical_status(status) {
        "decomposition" | "decomposition-agent-running" | "decomposition-review-rejected" => {
            "decomposition".to_string()
        }
        "decomposition-submitted" | "decomposition-review-running" => {
            "decomposition-review".to_string()
        }
        "discovered" | "planning" | "planning-agent-running" | "plan-review-rejected" => {
            "plan".to_string()
        }
        "plan-submitted" | "plan-review-running" => "plan-review".to_string(),
        "plan-approved"
        | "in-progress"
        | "implementation-agent-running"
        | "code-review-rejected" => "implementation".to_string(),
        "change-doc-submitted" | "code-review-running" => "code-review".to_string(),
        "change-doc-approved" | "wait-update-pr" | "wait-finish" | "finished" => {
            "finish-pr".to_string()
        }
        STATUS_SLICE_FINISHED => "finish-pr".to_string(),
        "rebase-agent-running" | "integration-review-rejected" => "pr-refresh".to_string(),
        "integration-review-submitted" | "integration-review-running" => {
            "integration-review".to_string()
        }
        "blocked" => "blocked".to_string(),
        _ => "request".to_string(),
    }
}

fn dashboard_stage_state(request: &Request, stage_id: &str) -> String {
    let rank = status_progress_rank(&request.status).unwrap_or(0);
    let has_pr_refresh = dashboard_has_pr_refresh(request);
    let status = canonical_status(&request.status);
    match stage_id {
        "request" => "done".to_string(),
        "decomposition" if rank >= 10 => "done".to_string(),
        "decomposition"
            if matches!(
                status,
                "decomposition" | "decomposition-agent-running" | "decomposition-review-rejected"
            ) =>
        {
            "active".to_string()
        }
        "decomposition" => "pending".to_string(),
        "decomposition-review" if rank >= 15 => "done".to_string(),
        "decomposition-review"
            if matches!(
                status,
                "decomposition-submitted" | "decomposition-review-running"
            ) =>
        {
            "active".to_string()
        }
        "decomposition-review" => "pending".to_string(),
        "plan" if rank >= 40 => "done".to_string(),
        "plan"
            if matches!(
                status,
                "planning" | "planning-agent-running" | "plan-review-rejected"
            ) =>
        {
            "active".to_string()
        }
        "plan" => "pending".to_string(),
        "plan-review" if rank >= 50 => "done".to_string(),
        "plan-review" if matches!(status, "plan-submitted" | "plan-review-running") => {
            "active".to_string()
        }
        "plan-review" => "pending".to_string(),
        "implementation" if rank >= 70 => "done".to_string(),
        "implementation"
            if matches!(
                status,
                "plan-approved"
                    | "in-progress"
                    | "implementation-agent-running"
                    | "code-review-rejected"
            ) =>
        {
            "active".to_string()
        }
        "implementation" => "pending".to_string(),
        "code-review" if rank >= 80 => "done".to_string(),
        "code-review" if matches!(status, "change-doc-submitted" | "code-review-running") => {
            "active".to_string()
        }
        "code-review" => "pending".to_string(),
        "finish-pr" if status == STATUS_FINISHED => "done".to_string(),
        "finish-pr" if status == STATUS_SLICE_FINISHED => "done".to_string(),
        "finish-pr" if matches!(status, "change-doc-approved" | STATUS_WAIT_UPDATE_PR) => {
            "active".to_string()
        }
        "finish-pr" => "pending".to_string(),
        "pr-refresh"
            if has_pr_refresh
                && matches!(
                    status,
                    "integration-review-submitted"
                        | "integration-review-running"
                        | STATUS_WAIT_UPDATE_PR
                        | STATUS_WAIT_FINISH
                        | STATUS_FINISHED
                ) =>
        {
            "done".to_string()
        }
        "pr-refresh"
            if matches!(
                status,
                "rebase-agent-running" | "integration-review-rejected"
            ) =>
        {
            "active".to_string()
        }
        "pr-refresh" => "pending".to_string(),
        "integration-review"
            if has_pr_refresh
                && matches!(
                    status,
                    STATUS_WAIT_UPDATE_PR | STATUS_WAIT_FINISH | STATUS_FINISHED
                ) =>
        {
            "done".to_string()
        }
        "integration-review"
            if matches!(
                status,
                "integration-review-submitted" | "integration-review-running"
            ) =>
        {
            "active".to_string()
        }
        "integration-review" if status == "integration-review-rejected" => "done".to_string(),
        "integration-review" => "pending".to_string(),
        _ => "pending".to_string(),
    }
}

fn dashboard_has_pr_refresh(request: &Request) -> bool {
    if matches!(
        canonical_status(&request.status),
        "rebase-agent-running"
            | "integration-review-submitted"
            | "integration-review-running"
            | "integration-review-rejected"
    ) {
        return true;
    }
    let integration_details =
        Path::new(&request.change_path).join("reviews/integration-review/details");
    if integration_details.is_dir() {
        return true;
    }
    let conflict_attempts = Path::new(&request.change_path).join("pr-conflicts/attempts");
    if conflict_attempts.is_dir() {
        return true;
    }
    let change_doc_path = existing_or_preferred_request_artifact_path(request, "change-doc.md");
    fs::read_to_string(change_doc_path)
        .map(|content| content.contains("PR 集成刷新记录") || content.contains("PR 冲突记录"))
        .unwrap_or(false)
}

fn dashboard_pr_refresh_artifact_path(request: &Request) -> String {
    let conflict_attempts = Path::new(&request.change_path).join("pr-conflicts/attempts");
    if let Ok(entries) = fs::read_dir(conflict_attempts) {
        let mut attempt_paths = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension == "md")
            {
                attempt_paths.push(path.to_string_lossy().to_string());
            }
        }
        attempt_paths.sort();
        if let Some(path) = attempt_paths.pop() {
            return path;
        }
    }

    let pr_status_path = Path::new(".sandrone")
        .join("state")
        .join(format!("{}-pr-status.tsv", request.request_id));
    if pr_status_path.exists() {
        return pr_status_path.to_string_lossy().to_string();
    }

    request_handoff_artifact_path_string(request, "pr-doc.md")
}

fn dashboard_has_decomposition(request: &Request) -> bool {
    Path::new(&request.change_path)
        .join(".decomposition-kind")
        .exists()
        || Path::new(&request.change_path).join(".epic-kind").exists()
        || existing_or_preferred_request_artifact_path(request, "decomposition.md").exists()
}

fn request_artifact_path(request: &Request, file: &str) -> String {
    request_artifact_path_string(request, file)
}

fn dashboard_request_content(request: &Request) -> String {
    let request_path = request_artifact_path(request, "request.md");
    if !request_path.is_empty() && Path::new(&request_path).exists() {
        return dashboard_file_content(&request_path);
    }
    format!(
        "# {}\n\n- Request ID: `{}`\n- External ID: `{}`\n- Source: `{}`\n- URL: {}\n\n## 需求描述\n\n{}\n",
        fallback_empty(&request.title, "未命名需求"),
        request.request_id,
        request.external_id,
        request.source,
        fallback_empty(&request.url, "n/a"),
        fallback_empty(&request.body, "尚未记录需求描述。"),
    )
}

fn dashboard_file_content(path: &str) -> String {
    if path.trim().is_empty() {
        return String::new();
    }
    match fs::read_to_string(path) {
        Ok(content) => truncate_dashboard_content(&content),
        Err(_) => String::new(),
    }
}

fn truncate_dashboard_content(content: &str) -> String {
    const MAX_CHARS: usize = 80_000;
    let mut out = String::new();
    for ch in content.chars().take(MAX_CHARS) {
        out.push(ch);
    }
    if content.chars().count() > MAX_CHARS {
        out.push_str("\n\n[dashboard truncated oversized artifact]\n");
    }
    out
}

fn render_review_attempts_json(request: &Request, stage: &str) -> Result<String> {
    if request.change_path.is_empty() {
        return Ok("[]".to_string());
    }
    let details_dir = Path::new(&request.change_path)
        .join("reviews")
        .join(stage)
        .join("details");
    let review_state_dir = Path::new(".sandrone")
        .join("state")
        .join("reviews")
        .join(&request.request_id)
        .join(stage);
    let mut attempts = Vec::<u32>::new();
    let mut details_by_attempt = BTreeMap::<u32, Vec<(String, String, String)>>::new();
    if details_dir.exists() {
        for entry in fs::read_dir(&details_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let filename = entry.file_name().to_string_lossy().to_string();
            if !filename.ends_with(".json") {
                continue;
            }
            let Some((attempt_text, _rest)) = filename.split_once('-') else {
                continue;
            };
            let Ok(attempt) = attempt_text.parse::<u32>() else {
                continue;
            };
            if !attempts.contains(&attempt) {
                attempts.push(attempt);
            }
            let path = entry.path().to_string_lossy().to_string();
            let content = fs::read_to_string(entry.path()).unwrap_or_default();
            details_by_attempt
                .entry(attempt)
                .or_default()
                .push((filename, path, content));
        }
    }
    if review_state_dir.exists() {
        for entry in fs::read_dir(review_state_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            if let Ok(attempt) = entry.file_name().to_string_lossy().parse::<u32>()
                && !attempts.contains(&attempt)
            {
                attempts.push(attempt);
            }
        }
    }
    attempts.sort_unstable();
    if attempts.is_empty() {
        return Ok("[]".to_string());
    }

    let reviewers_for_stage = dashboard_reviewers_for_stage(stage);
    let mut attempts_json = String::from("[");
    for (attempt_index, attempt) in attempts.into_iter().enumerate() {
        let mut details = details_by_attempt.remove(&attempt).unwrap_or_default();
        details.sort_by(|left, right| left.0.cmp(&right.0));
        if attempt_index > 0 {
            attempts_json.push_str(", ");
        }
        let mut reviewers = String::new();
        let mut approved_count = 0usize;
        let mut detail_count = 0usize;
        let mut reviewer_count = 0usize;
        let mut waiting_for_detail = false;
        let mut used_filenames = Vec::<String>::new();

        for reviewer in &reviewers_for_stage {
            let filename = format!("{attempt:03}-{}.json", reviewer.file_stem);
            let runtime = dashboard_review_runtime(&request.request_id, stage, attempt, reviewer);
            let detail = details
                .iter()
                .find(|(detail_filename, _, _)| detail_filename == &filename)
                .cloned();
            if reviewer_count > 0 {
                reviewers.push_str(", ");
            }
            reviewer_count += 1;
            used_filenames.push(filename.clone());
            if let Some((filename, path, content)) = detail {
                detail_count += 1;
                if json_bool(&content, "approved").unwrap_or(false)
                    && !review_has_blocking_findings(&content)
                    && !json_bool(&content, "gate_unavailable").unwrap_or(false)
                {
                    approved_count += 1;
                }
                reviewers.push_str(&render_review_detail_json(
                    request, stage, &filename, &path, &content, &runtime,
                ));
            } else {
                if runtime.is_waiting_for_detail() {
                    waiting_for_detail = true;
                }
                reviewers.push_str(&render_pending_review_detail_json(
                    request, stage, attempt, reviewer, &runtime,
                ));
            }
        }

        for (filename, path, content) in details {
            if used_filenames.contains(&filename) {
                continue;
            }
            if reviewer_count > 0 {
                reviewers.push_str(", ");
            }
            reviewer_count += 1;
            detail_count += 1;
            if json_bool(&content, "approved").unwrap_or(false)
                && !review_has_blocking_findings(&content)
                && !json_bool(&content, "gate_unavailable").unwrap_or(false)
            {
                approved_count += 1;
            }
            reviewers.push_str(&render_review_detail_json(
                request,
                stage,
                &filename,
                &path,
                &content,
                &DashboardReviewRuntime::default(),
            ));
        }
        let status = if waiting_for_detail {
            "running"
        } else if detail_count > 0 && approved_count == detail_count {
            "approved"
        } else {
            "rejected"
        };
        attempts_json.push_str(&format!(
            "{{ \"attempt\": {}, \"status\": \"{}\", \"reviewers\": [{}] }}",
            attempt,
            json_escape(status),
            reviewers,
        ));
    }
    attempts_json.push(']');
    Ok(attempts_json)
}

fn render_review_detail_json(
    request: &Request,
    stage: &str,
    filename: &str,
    path: &str,
    content: &str,
    runtime: &DashboardReviewRuntime,
) -> String {
    let reviewer = json_value(content, "reviewer")
        .unwrap_or_else(|| reviewer_name_from_detail_filename(stage, filename));
    let approved = json_bool(content, "approved").unwrap_or(false);
    let gate_unavailable = json_bool(content, "gate_unavailable").unwrap_or(false);
    let decision = json_value(content, "decision").unwrap_or_else(|| {
        if approved {
            "approved".to_string()
        } else {
            "rejected".to_string()
        }
    });
    let recommended_next_phase =
        json_value(content, "recommended_next_phase").unwrap_or_else(|| "unknown".to_string());
    let summary = json_value(content, "summary").unwrap_or_default();
    format!(
        "{{ \"reviewer\": \"{}\", \"approved\": {}, \"gate_unavailable\": {}, \"decision\": \"{}\", \"recommended_next_phase\": \"{}\", \"summary\": \"{}\", \"detail_path\": \"{}\", \"detail\": \"{}\", {}, \"critical\": {}, \"high\": {}, \"warning\": {}, \"info\": {} }}",
        json_escape(&reviewer),
        json_bool_literal(approved),
        json_bool_literal(gate_unavailable),
        json_escape(&decision),
        json_escape(&recommended_next_phase),
        json_escape(&summary),
        json_escape(&dashboard_review_detail_relative_path(request, path)),
        json_escape(&truncate_dashboard_content(content)),
        runtime.render_json(),
        render_review_findings_json(content, "critical"),
        render_review_findings_json(content, "high"),
        render_review_findings_json(content, "warning"),
        render_review_findings_json(content, "info"),
    )
}

fn render_pending_review_detail_json(
    request: &Request,
    stage: &str,
    attempt: u32,
    reviewer: &DashboardReviewerDefinition,
    runtime: &DashboardReviewRuntime,
) -> String {
    let detail_path = Path::new(&request.change_path)
        .join("reviews")
        .join(stage)
        .join("details")
        .join(format!("{attempt:03}-{}.json", reviewer.file_stem));
    let decision = if runtime.runtime_status.is_empty() {
        "pending"
    } else {
        &runtime.runtime_status
    };
    let summary = match runtime.runtime_status.as_str() {
        "running" => "reviewer worker is running; detail JSON has not been written yet",
        "exited" => "reviewer worker exited; detail JSON has not been written yet",
        "stale" => "reviewer worker pid is stale; detail JSON has not been written yet",
        _ => "reviewer detail JSON has not been written yet",
    };
    format!(
        "{{ \"reviewer\": \"{}\", \"approved\": false, \"gate_unavailable\": false, \"decision\": \"{}\", \"recommended_next_phase\": \"unknown\", \"summary\": \"{}\", \"detail_path\": \"{}\", \"detail\": \"\", {}, \"critical\": [], \"high\": [], \"warning\": [], \"info\": [] }}",
        json_escape(reviewer.name),
        json_escape(decision),
        json_escape(summary),
        json_escape(&dashboard_review_detail_relative_path(
            request,
            &detail_path.to_string_lossy()
        )),
        runtime.render_json(),
    )
}

#[derive(Clone, Debug)]
struct DashboardReviewerDefinition {
    name: &'static str,
    file_stem: &'static str,
}

fn dashboard_reviewers_for_stage(stage: &str) -> Vec<DashboardReviewerDefinition> {
    match stage {
        "decomposition-review" => vec![DashboardReviewerDefinition {
            name: "DecompositionReviewer",
            file_stem: "decomposition-reviewer",
        }],
        "plan-review" => vec![DashboardReviewerDefinition {
            name: "PlanReviewer",
            file_stem: "plan-reviewer",
        }],
        "code-review" => vec![
            DashboardReviewerDefinition {
                name: "TestReviewer",
                file_stem: "test-reviewer",
            },
            DashboardReviewerDefinition {
                name: "DesignReviewer",
                file_stem: "design-reviewer",
            },
        ],
        "integration-review" => vec![DashboardReviewerDefinition {
            name: "IntegrationReviewer",
            file_stem: "integration-reviewer",
        }],
        _ => Vec::new(),
    }
}

#[derive(Clone, Debug, Default)]
struct DashboardReviewRuntime {
    runtime_status: String,
    pid: String,
    exit_code: String,
    runtime_path: String,
    events_log_path: String,
    stdout_path: String,
    stderr_path: String,
    hook_log_path: String,
    runtime_tail: String,
    events_log_tail: String,
    stdout_tail: String,
    stderr_tail: String,
    hook_log_tail: String,
}

impl DashboardReviewRuntime {
    fn is_waiting_for_detail(&self) -> bool {
        matches!(
            self.runtime_status.as_str(),
            "running" | "pending" | "stale"
        )
    }

    fn render_json(&self) -> String {
        format!(
            "\"runtime_status\": \"{}\", \"pid\": \"{}\", \"exit_code\": \"{}\", \"runtime_path\": \"{}\", \"events_log_path\": \"{}\", \"stdout_path\": \"{}\", \"stderr_path\": \"{}\", \"hook_log_path\": \"{}\", \"runtime_tail\": \"{}\", \"events_log_tail\": \"{}\", \"stdout_tail\": \"{}\", \"stderr_tail\": \"{}\", \"hook_log_tail\": \"{}\"",
            json_escape(&self.runtime_status),
            json_escape(&self.pid),
            json_escape(&self.exit_code),
            json_escape(&self.runtime_path),
            json_escape(&self.events_log_path),
            json_escape(&self.stdout_path),
            json_escape(&self.stderr_path),
            json_escape(&self.hook_log_path),
            json_escape(&self.runtime_tail),
            json_escape(&self.events_log_tail),
            json_escape(&self.stdout_tail),
            json_escape(&self.stderr_tail),
            json_escape(&self.hook_log_tail),
        )
    }
}

fn dashboard_review_runtime(
    request_id: &str,
    stage: &str,
    attempt: u32,
    reviewer: &DashboardReviewerDefinition,
) -> DashboardReviewRuntime {
    let canonical_job_dir = Path::new(".sandrone")
        .join("state")
        .join("jobs")
        .join(request_id)
        .join(stage)
        .join(format!("{attempt:03}"))
        .join(reviewer.file_stem);
    let legacy_job_dir = Path::new(".sandrone")
        .join("state")
        .join("reviews")
        .join(request_id)
        .join(stage)
        .join(format!("{attempt:03}"))
        .join(reviewer.file_stem);
    let job_dir = if canonical_job_dir.exists() {
        canonical_job_dir.clone()
    } else if legacy_job_dir.exists() {
        legacy_job_dir.clone()
    } else {
        canonical_job_dir.clone()
    };
    let pid_path = existing_runtime_path(job_dir.join("pid"), legacy_job_dir.join("pid"));
    let exit_path = existing_runtime_path(job_dir.join("exit"), legacy_job_dir.join("exit"));
    let runtime_path = canonical_job_dir.join("runtime.json");
    let events_log_path = canonical_job_dir.join("events.log");
    let stdout_path = existing_runtime_path(
        canonical_job_dir.join("stdout.log"),
        legacy_job_dir.join("stdout.log"),
    );
    let stderr_path = existing_runtime_path(
        canonical_job_dir.join("stderr.log"),
        legacy_job_dir.join("stderr.log"),
    );
    let hook_log_path = existing_runtime_path(
        canonical_job_dir.join("hook.log"),
        legacy_job_dir.join("hook.log"),
    );
    let pid = fs::read_to_string(&pid_path)
        .map(|content| content.trim().to_string())
        .unwrap_or_default();
    let exit_code = fs::read_to_string(&exit_path)
        .map(|content| content.trim().to_string())
        .unwrap_or_default();
    let runtime_status = if !exit_code.is_empty() {
        "exited".to_string()
    } else if pid.parse::<u32>().map(process_is_running).unwrap_or(false) {
        "running".to_string()
    } else if !pid.is_empty() {
        "stale".to_string()
    } else if job_dir.exists() {
        "pending".to_string()
    } else {
        String::new()
    };
    DashboardReviewRuntime {
        runtime_status,
        pid,
        exit_code,
        runtime_path: dashboard_relative_path(&runtime_path),
        events_log_path: dashboard_relative_path(&events_log_path),
        stdout_path: dashboard_relative_path(&stdout_path),
        stderr_path: dashboard_relative_path(&stderr_path),
        hook_log_path: dashboard_relative_path(&hook_log_path),
        runtime_tail: dashboard_file_tail(&runtime_path),
        events_log_tail: dashboard_file_tail(&events_log_path),
        stdout_tail: dashboard_file_tail(&stdout_path),
        stderr_tail: dashboard_file_tail(&stderr_path),
        hook_log_tail: dashboard_file_tail(&hook_log_path),
    }
}

fn dashboard_relative_path(path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(env::current_dir().unwrap_or_default()) {
        return relative.to_string_lossy().to_string();
    }
    path.to_string_lossy().to_string()
}

fn dashboard_file_tail(path: &Path) -> String {
    const MAX_CHARS: usize = 12_000;
    let Ok(content) = fs::read_to_string(path) else {
        return String::new();
    };
    let chars = content.chars().collect::<Vec<_>>();
    let start = chars.len().saturating_sub(MAX_CHARS);
    chars[start..].iter().collect()
}

fn dashboard_review_detail_relative_path(request: &Request, path: &str) -> String {
    let detail_path = Path::new(path);
    if let Ok(relative) = detail_path.strip_prefix(env::current_dir().unwrap_or_default()) {
        return relative.to_string_lossy().to_string();
    }
    if !request.change_path.is_empty()
        && let Ok(relative) = detail_path.strip_prefix(&request.change_path)
    {
        return Path::new(&request.change_path)
            .join(relative)
            .to_string_lossy()
            .to_string();
    }
    path.to_string()
}

fn reviewer_name_from_detail_filename(stage: &str, filename: &str) -> String {
    if filename.contains("plan-reviewer") {
        "PlanReviewer".to_string()
    } else if filename.contains("test-reviewer") {
        "TestReviewer".to_string()
    } else if filename.contains("design-reviewer") {
        "DesignReviewer".to_string()
    } else if stage == "plan-review" {
        "PlanReviewer".to_string()
    } else {
        filename.trim_end_matches(".json").to_string()
    }
}

fn render_review_findings_json(content: &str, severity: &str) -> String {
    let findings = review_findings(content, severity);
    let mut rendered = String::from("[");
    for (index, finding) in findings.iter().enumerate() {
        if index > 0 {
            rendered.push_str(", ");
        }
        rendered.push_str(&format!(
            "{{ \"title\": \"{}\", \"evidence\": \"{}\", \"impact\": \"{}\", \"required_fix\": \"{}\", \"suggested_change\": \"{}\", \"verification\": \"{}\" }}",
            json_escape(&finding.title),
            json_escape(&finding.evidence),
            json_escape(&finding.impact),
            json_escape(&finding.required_fix),
            json_escape(&finding.suggested_change),
            json_escape(&finding.verification),
        ));
    }
    rendered.push(']');
    rendered
}
