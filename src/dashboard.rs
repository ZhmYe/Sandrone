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
    println!("Codex Auto Dev dashboard running:");
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
    let requests_json = if record.last_status == "ready"
        && Path::new(&record.workspace_path).join(CONFIG_PATH).exists()
    {
        registry::with_current_dir(Path::new(&record.workspace_path), || {
            let requests = load_requests()?;
            let mut rendered = String::new();
            for (index, request) in requests.iter().enumerate() {
                if index > 0 {
                    rendered.push_str(",\n");
                }
                rendered.push_str(&render_dashboard_request_json(request)?);
            }
            Ok(rendered)
        })?
    } else {
        String::new()
    };

    Ok(format!(
        "    {{\n      \"key\": \"{}\",\n      \"repo_name\": \"{}\",\n      \"git_url\": \"{}\",\n      \"workspace_path\": \"{}\",\n      \"target_repo\": \"{}\",\n      \"last_status\": \"{}\",\n      \"request_count\": {},\n      \"status_counts\": {},\n      \"updated_at\": \"{}\",\n      \"requests\": [\n{}\n      ]\n    }}",
        json_escape(&record.key),
        json_escape(&record.repo_name),
        json_escape(&record.git_url),
        json_escape(&record.workspace_path),
        json_escape(&record.target_repo),
        json_escape(&record.last_status),
        record.request_count,
        registry::render_usize_map_json(&record.status_counts),
        json_escape(&record.updated_at),
        requests_json,
    ))
}

fn render_dashboard_request_json(request: &Request) -> Result<String> {
    let stages = [
        render_dashboard_stage_json(
            request,
            "request",
            "Request",
            "需求记录",
            &request_artifact_path(request, "request.md"),
            dashboard_request_content(request),
            "markdown",
        )?,
        render_dashboard_stage_json(
            request,
            "plan",
            "Plan",
            "计划",
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
        {
            let finish_path = finish_artifact_path(request);
            let finish_kind = dashboard_artifact_kind(&finish_path, "markdown");
            render_dashboard_stage_json(
                request,
                "finish-pr",
                "Finish / PR",
                "交付与 PR",
                &finish_path,
                dashboard_file_content(&finish_path),
                &finish_kind,
            )?
        },
    ];
    Ok(format!(
        "        {{\n          \"request_id\": \"{}\",\n          \"external_id\": \"{}\",\n          \"source\": \"{}\",\n          \"title\": \"{}\",\n          \"body\": \"{}\",\n          \"url\": \"{}\",\n          \"status\": \"{}\",\n          \"stage\": \"{}\",\n          \"change_name\": \"{}\",\n          \"change_path\": \"{}\",\n          \"branch\": \"{}\",\n          \"worktree_path\": \"{}\",\n          \"created_at\": \"{}\",\n          \"updated_at\": \"{}\",\n          \"stages\": [\n{}\n          ]\n        }}",
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
    Ok(format!(
        "            {{ \"stage_id\": \"{}\", \"label\": \"{}\", \"title\": \"{}\", \"state\": \"{}\", \"artifact_path\": \"{}\", \"artifact_kind\": \"{}\", \"content\": \"{}\", \"review_attempts\": [] }}",
        json_escape(stage_id),
        json_escape(label),
        json_escape(title),
        json_escape(&dashboard_stage_state(request, stage_id)),
        json_escape(artifact_path),
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
    Ok(format!(
        "            {{ \"stage_id\": \"{}\", \"label\": \"{}\", \"title\": \"{}\", \"state\": \"{}\", \"artifact_path\": \"{}\", \"artifact_kind\": \"review-details\", \"content\": \"\", \"review_attempts\": {} }}",
        json_escape(stage_id),
        json_escape(label),
        json_escape(title),
        json_escape(&dashboard_stage_state(request, stage_id)),
        json_escape(&artifact_path),
        attempts,
    ))
}

fn dashboard_current_stage(status: &str) -> String {
    match status {
        "discovered" | "planning" | "planning-agent-running" | "plan-review-rejected" => {
            "plan".to_string()
        }
        "plan-submitted" => "plan-review".to_string(),
        "plan-approved"
        | "in-progress"
        | "implementation-agent-running"
        | "code-review-rejected" => "implementation".to_string(),
        "change-doc-submitted" => "code-review".to_string(),
        "change-doc-approved" | "waiting-finish" | "finished" => "finish-pr".to_string(),
        "blocked" => "blocked".to_string(),
        _ => "request".to_string(),
    }
}

fn dashboard_stage_state(request: &Request, stage_id: &str) -> String {
    let rank = status_progress_rank(&request.status).unwrap_or(0);
    match stage_id {
        "request" => "done".to_string(),
        "plan" if rank >= 30 => "done".to_string(),
        "plan"
            if matches!(
                request.status.as_str(),
                "planning" | "planning-agent-running" | "plan-review-rejected"
            ) =>
        {
            "active".to_string()
        }
        "plan" => "pending".to_string(),
        "plan-review" if rank >= 40 => "done".to_string(),
        "plan-review" if matches!(request.status.as_str(), "plan-submitted") => {
            "active".to_string()
        }
        "plan-review" => "pending".to_string(),
        "implementation" if rank >= 70 => "done".to_string(),
        "implementation"
            if matches!(
                request.status.as_str(),
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
        "code-review" if matches!(request.status.as_str(), "change-doc-submitted") => {
            "active".to_string()
        }
        "code-review" => "pending".to_string(),
        "finish-pr" if request.status == "finished" => "done".to_string(),
        "finish-pr"
            if matches!(
                request.status.as_str(),
                "change-doc-approved" | "waiting-finish"
            ) =>
        {
            "active".to_string()
        }
        "finish-pr" => "pending".to_string(),
        _ => "pending".to_string(),
    }
}

fn request_artifact_path(request: &Request, file: &str) -> String {
    if request.change_path.is_empty() {
        String::new()
    } else {
        Path::new(&request.change_path)
            .join(file)
            .to_string_lossy()
            .to_string()
    }
}

fn dashboard_artifact_kind(path: &str, default_kind: &str) -> String {
    if path.ends_with(".json") {
        "json".to_string()
    } else {
        default_kind.to_string()
    }
}

fn finish_artifact_path(request: &Request) -> String {
    let pr_body_path = Path::new(".codex-auto-dev")
        .join("state")
        .join(format!("{}-pr-body.md", request.request_id));
    if pr_body_path.exists() {
        return pr_body_path.to_string_lossy().to_string();
    }
    request_artifact_path(request, "status.json")
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
    if !details_dir.exists() {
        return Ok("[]".to_string());
    }
    let mut by_attempt = BTreeMap::<u32, Vec<(String, String, String)>>::new();
    for entry in fs::read_dir(details_dir)? {
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
        let path = entry.path().to_string_lossy().to_string();
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        by_attempt
            .entry(attempt)
            .or_default()
            .push((filename, path, content));
    }

    let mut attempts = String::from("[");
    for (attempt_index, (attempt, mut details)) in by_attempt.into_iter().enumerate() {
        details.sort_by(|left, right| left.0.cmp(&right.0));
        if attempt_index > 0 {
            attempts.push_str(", ");
        }
        let mut reviewers = String::new();
        let mut approved_count = 0usize;
        let detail_count = details.len();
        for (detail_index, (filename, path, content)) in details.into_iter().enumerate() {
            if detail_index > 0 {
                reviewers.push_str(", ");
            }
            if json_bool(&content, "approved").unwrap_or(false)
                && !review_has_blocking_findings(&content)
                && !json_bool(&content, "gate_unavailable").unwrap_or(false)
            {
                approved_count += 1;
            }
            reviewers.push_str(&render_review_detail_json(
                request, stage, &filename, &path, &content,
            ));
        }
        let status = if detail_count > 0 && approved_count == detail_count {
            "approved"
        } else {
            "rejected"
        };
        attempts.push_str(&format!(
            "{{ \"attempt\": {}, \"status\": \"{}\", \"reviewers\": [{}] }}",
            attempt,
            json_escape(status),
            reviewers,
        ));
    }
    attempts.push(']');
    Ok(attempts)
}

fn render_review_detail_json(
    request: &Request,
    stage: &str,
    filename: &str,
    path: &str,
    content: &str,
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
        "{{ \"reviewer\": \"{}\", \"approved\": {}, \"gate_unavailable\": {}, \"decision\": \"{}\", \"recommended_next_phase\": \"{}\", \"summary\": \"{}\", \"detail_path\": \"{}\", \"detail\": \"{}\", \"critical\": {}, \"high\": {}, \"warning\": {}, \"info\": {} }}",
        json_escape(&reviewer),
        json_bool_literal(approved),
        json_bool_literal(gate_unavailable),
        json_escape(&decision),
        json_escape(&recommended_next_phase),
        json_escape(&summary),
        json_escape(&dashboard_review_detail_relative_path(request, path)),
        json_escape(&truncate_dashboard_content(content)),
        render_review_findings_json(content, "critical"),
        render_review_findings_json(content, "high"),
        render_review_findings_json(content, "warning"),
        render_review_findings_json(content, "info"),
    )
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
