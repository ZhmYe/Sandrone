use super::*;

pub(crate) fn load_config() -> Result<Config> {
    ensure_initialized()?;
    let content = fs::read_to_string(CONFIG_PATH)?;
    let mut schema_version = 1;
    let mut repo_name = String::new();
    let mut git_url = String::new();
    let mut base_branch = "main".to_string();
    let mut parallel_limit = 1;
    let mut auto_merge = false;

    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"');
        match key {
            "schema_version" => schema_version = value.parse().unwrap_or(1),
            "repo_name" => repo_name = value.to_string(),
            "git_url" => git_url = value.to_string(),
            "base_branch" => base_branch = value.to_string(),
            "parallel_limit" => {
                if let Some(parsed) = value.parse::<usize>().ok().filter(|parsed| *parsed > 0) {
                    parallel_limit = parsed;
                }
            }
            "auto_merge" => auto_merge = parse_config_bool(value).unwrap_or(false),
            _ => {}
        }
    }

    Ok(Config {
        schema_version,
        repo_name,
        git_url,
        base_branch,
        parallel_limit,
        auto_merge,
    })
}

fn parse_config_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub(crate) fn load_requests() -> Result<Vec<Request>> {
    if !Path::new(STATE_PATH).exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(STATE_PATH)?;
    let mut requests = Vec::new();
    for line in content.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<String> = line.split('\t').map(unescape_field).collect();
        if fields.len() < 13 {
            continue;
        }
        requests.push(Request {
            request_id: fields[0].clone(),
            external_id: fields[1].clone(),
            source: fields[2].clone(),
            title: fields[3].clone(),
            body: fields[4].clone(),
            url: fields[5].clone(),
            status: canonical_status(&fields[6]).to_string(),
            change_name: fields[7].clone(),
            change_path: fields[8].clone(),
            branch: fields[9].clone(),
            worktree_path: fields[10].clone(),
            created_at: fields[11].clone(),
            updated_at: fields[12].clone(),
        });
    }
    Ok(requests)
}

pub(crate) fn save_requests(requests: &[Request]) -> Result<()> {
    fs::create_dir_all(".sandrone/state")?;
    let mut content = String::from("# Sandrone requests v2\n");
    for request in requests {
        content.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            escape_field(&request.request_id),
            escape_field(&request.external_id),
            escape_field(&request.source),
            escape_field(&request.title),
            escape_field(&request.body),
            escape_field(&request.url),
            escape_field(&request.status),
            escape_field(&request.change_name),
            escape_field(&request.change_path),
            escape_field(&request.branch),
            escape_field(&request.worktree_path),
            escape_field(&request.created_at),
            escape_field(&request.updated_at),
        ));
    }
    fs::write(STATE_PATH, content)?;
    sync_obsidian_project_note(requests)?;
    Ok(())
}

pub(crate) fn load_sessions() -> Result<Vec<SessionRecord>> {
    if !Path::new(SESSIONS_PATH).exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(SESSIONS_PATH)?;
    let mut sessions = Vec::new();
    for line in content.lines() {
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with('{') || !line.contains("\"request_id\"") {
            continue;
        }
        sessions.push(SessionRecord {
            request_id: json_value(line, "request_id").unwrap_or_default(),
            phase: json_value(line, "phase").unwrap_or_default(),
            status: json_value(line, "status").unwrap_or_default(),
            thread_id: json_value(line, "thread_id").unwrap_or_default(),
            thread_url: json_value(line, "thread_url").unwrap_or_default(),
            workspace: json_value(line, "workspace").unwrap_or_default(),
            target_repo: json_value(line, "target_repo").unwrap_or_default(),
            worktree: json_value(line, "worktree").unwrap_or_default(),
            change_path: json_value(line, "change_path").unwrap_or_default(),
            started_at: json_value(line, "started_at").unwrap_or_default(),
            updated_at: json_value(line, "updated_at").unwrap_or_default(),
        });
    }
    Ok(sessions)
}

pub(crate) fn save_sessions(sessions: &[SessionRecord]) -> Result<()> {
    fs::create_dir_all(".sandrone")?;
    let mut content = String::from("{\n  \"schema_version\": 1,\n  \"sessions\": [\n");
    for (index, session) in sessions.iter().enumerate() {
        if index > 0 {
            content.push_str(",\n");
        }
        content.push_str(&format!(
            "    {{ \"request_id\": \"{}\", \"phase\": \"{}\", \"status\": \"{}\", \"thread_id\": \"{}\", \"thread_url\": \"{}\", \"workspace\": \"{}\", \"target_repo\": \"{}\", \"worktree\": \"{}\", \"change_path\": \"{}\", \"started_at\": \"{}\", \"updated_at\": \"{}\" }}",
            json_escape(&session.request_id),
            json_escape(&session.phase),
            json_escape(&session.status),
            json_escape(&session.thread_id),
            json_escape(&session.thread_url),
            json_escape(&session.workspace),
            json_escape(&session.target_repo),
            json_escape(&session.worktree),
            json_escape(&session.change_path),
            json_escape(&session.started_at),
            json_escape(&session.updated_at),
        ));
    }
    content.push_str("\n  ]\n}\n");
    fs::write(SESSIONS_PATH, content)?;
    Ok(())
}

pub(crate) fn upsert_session(session: SessionRecord) -> Result<()> {
    let mut sessions = load_sessions()?;
    if let Some(existing) = sessions.iter_mut().find(|existing| {
        existing.request_id == session.request_id && existing.phase == session.phase
    }) {
        let thread_id = if session.thread_id.is_empty() {
            existing.thread_id.clone()
        } else {
            session.thread_id.clone()
        };
        let thread_url = if session.thread_url.is_empty() {
            existing.thread_url.clone()
        } else {
            session.thread_url.clone()
        };
        let started_at = if existing.started_at.is_empty() {
            session.started_at.clone()
        } else {
            existing.started_at.clone()
        };
        *existing = SessionRecord {
            thread_id,
            thread_url,
            started_at,
            ..session
        };
    } else {
        sessions.push(session);
    }
    save_sessions(&sessions)
}

pub(crate) fn upsert_session_for_request(
    request: &Request,
    phase: &str,
    status: &str,
) -> Result<()> {
    upsert_session(session_from_request(request, phase, status)?)
}

pub(crate) fn update_gate_session(request: &Request, gate: &str, status: &str) -> Result<()> {
    let phase = match gate {
        "decomposition" => "decomposition",
        "plan" => "planning",
        _ => "implementation",
    };
    upsert_session_for_request(request, phase, status)
}

pub(crate) fn session_from_request(
    request: &Request,
    phase: &str,
    status: &str,
) -> Result<SessionRecord> {
    validate_session_phase(phase)?;
    let now = now_string();
    Ok(SessionRecord {
        request_id: request.request_id.clone(),
        phase: phase.to_string(),
        status: status.to_string(),
        thread_id: String::new(),
        thread_url: String::new(),
        workspace: absolute_path_string("."),
        target_repo: absolute_path_string(DEV_REPO),
        worktree: if request.worktree_path.is_empty() {
            String::new()
        } else {
            absolute_path_string(request.worktree_path.as_str())
        },
        change_path: request.change_path.clone(),
        started_at: now.clone(),
        updated_at: now,
    })
}

pub(crate) fn write_status_json(
    request: &Request,
    stage: &str,
    status: &str,
    reason: &str,
) -> Result<()> {
    write_status_json_with_gate_update(request, stage, status, reason, None)
}

fn write_status_json_with_gate_update(
    request: &Request,
    stage: &str,
    status: &str,
    reason: &str,
    _gate_update: Option<(String, String)>,
) -> Result<()> {
    ensure_change_packet(request)?;
    let review_cycle = review_cycle_for_status(request).unwrap_or(0);
    let request_artifact = request_handoff_artifact_path_string(request, "request.md");
    let decomposition_artifact = request_handoff_artifact_path_string(request, "decomposition.md");
    let dag_artifact = request_handoff_artifact_path_string(request, "dag.json");
    let plan_artifact = request_handoff_artifact_path_string(request, "plan.md");
    let change_doc_artifact = request_handoff_artifact_path_string(request, "change-doc.md");
    let pr_doc_artifact = request_handoff_artifact_path_string(request, "pr-doc.md");
    let agent_journal_artifact = request_handoff_artifact_path_string(request, "agent-journal.md");
    fs::write(
        status_json_path(request),
        format!(
            "{{\n  \"schema_version\": 1,\n  \"request_id\": \"{}\",\n  \"stage\": \"{}\",\n  \"current_phase\": \"{}\",\n  \"status\": \"{}\",\n  \"reason\": \"{}\",\n  \"return_to_phase_reason\": \"{}\",\n  \"review_cycle\": {},\n  \"handoff_artifacts\": {{\n    \"request\": \"{}\",\n    \"decomposition\": \"{}\",\n    \"dag\": \"{}\",\n    \"plan\": \"{}\",\n    \"change_doc\": \"{}\",\n    \"pr_doc\": \"{}\",\n    \"agent_journal\": \"{}\",\n    \"codegraph_context\": \"obsidian/codegraph/context.md\",\n    \"obsidian_project\": \"obsidian/project.md\",\n    \"obsidian_note\": \"{}\"\n  }},\n  \"branch\": \"{}\",\n  \"worktree\": \"{}\",\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(&request.request_id),
            json_escape(stage),
            json_escape(status),
            json_escape(status),
            json_escape(reason),
            json_escape(reason),
            review_cycle,
            json_escape(&request_artifact),
            json_escape(&decomposition_artifact),
            json_escape(&dag_artifact),
            json_escape(&plan_artifact),
            json_escape(&change_doc_artifact),
            json_escape(&pr_doc_artifact),
            json_escape(&agent_journal_artifact),
            json_escape(&obsidian_request_note_path(request).display().to_string()),
            json_escape(&request.branch),
            json_escape(&request.worktree_path),
            json_escape(&now_string()),
        ),
    )?;
    sync_obsidian_request_note(request)?;
    Ok(())
}

fn status_json_path(request: &Request) -> PathBuf {
    Path::new(&request.change_path).join("status.json")
}

fn status_gate_record_from_content(content: &str, gate: &str) -> Option<String> {
    let needle = format!("\"gate\": \"{gate}\"");
    content
        .lines()
        .find(|line| line.contains(&needle))
        .map(|line| line.trim().trim_end_matches(',').to_string())
}

pub(crate) fn review_cycle_for_status(request: &Request) -> Result<u32> {
    let decomposition_attempts = review_attempt_count(request, "decomposition-review")?;
    let plan_attempts = review_attempt_count(request, "plan-review")?;
    let code_attempts = review_attempt_count(request, "code-review")?;
    let integration_attempts = review_attempt_count(request, "integration-review")?;
    Ok(decomposition_attempts
        .max(plan_attempts)
        .max(code_attempts)
        .max(integration_attempts))
}

pub(crate) fn append_event(
    event: &str,
    request_id: &str,
    phase: &str,
    status: &str,
    detail: &str,
) -> Result<()> {
    fs::create_dir_all(".sandrone/state")?;
    let line = format!(
        "{{\"time\": \"{}\", \"event\": \"{}\", \"request_id\": \"{}\", \"phase\": \"{}\", \"status\": \"{}\", \"detail\": \"{}\"}}\n",
        json_escape(&now_string()),
        json_escape(event),
        json_escape(request_id),
        json_escape(phase),
        json_escape(status),
        json_escape(detail),
    );
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(EVENTS_PATH)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

pub(crate) fn mark_blocked(
    requests: &mut [Request],
    index: usize,
    request: &mut Request,
    stage: &str,
    reason: &str,
) -> Result<()> {
    request.status = "blocked".to_string();
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(requests)?;
    write_status_json(request, stage, "blocked", reason)?;
    write_recovery_doc(request, stage, reason)?;
    let phase = if stage == "decomposition" {
        "decomposition"
    } else if stage == "planning" {
        "planning"
    } else if stage == "rebase" {
        "rebase"
    } else {
        "implementation"
    };
    append_event("blocked", &request.request_id, phase, "blocked", reason)?;
    upsert_session_for_request(request, phase, "blocked")
}

pub(crate) fn write_recovery_doc(request: &Request, stage: &str, reason: &str) -> Result<()> {
    let request_artifact = request_handoff_artifact_path_string(request, "request.md");
    let plan_artifact = request_handoff_artifact_path_string(request, "plan.md");
    let change_doc_artifact = request_handoff_artifact_path_string(request, "change-doc.md");
    let agent_journal_artifact = request_handoff_artifact_path_string(request, "agent-journal.md");
    let plan_summary = Path::new(&request.change_path)
        .join("reviews/plan-review/summary.json")
        .display()
        .to_string();
    let code_summary = Path::new(&request.change_path)
        .join("reviews/code-review/summary.json")
        .display()
        .to_string();
    fs::write(
        request_artifact_path_buf(request, "recovery.md"),
        format!(
            "# 恢复指南: {request_id}\n\n## 当前状态\n\n- Stage: `{stage}`\n- Status: `blocked`\n- Reason: {reason}\n\n## 关键路径\n\n- Request: `{request_artifact}`\n- Plan: `{plan_artifact}`\n- Change doc: `{change_doc_artifact}`\n- Agent journal: `{agent_journal_artifact}`\n- Status: `{change_path}/status.json`\n- Plan review summary: `{plan_summary}`\n- Code review summary: `{code_summary}`\n- Worktree: `{worktree}`\n- Branch: `{branch}`\n\n## 推荐恢复步骤\n\n1. 阅读 request、plan、change-doc、agent-journal 和本文件。\n2. 查看最后一轮 review summary 和 details，优先处理 critical/high。\n3. 如果需要继续自动修复，运行 `sandrone tick --request_id {request_id}`。\n4. 如果 reviewer 明显误判，人工审批必须写明 comment 和来源。\n",
            request_id = request.request_id,
            stage = stage,
            reason = reason,
            change_path = request.change_path,
            request_artifact = request_artifact,
            plan_artifact = plan_artifact,
            change_doc_artifact = change_doc_artifact,
            agent_journal_artifact = agent_journal_artifact,
            plan_summary = plan_summary,
            code_summary = code_summary,
            worktree = fallback_empty(&request.worktree_path, "not started"),
            branch = fallback_empty(&request.branch, "not started"),
        ),
    )?;
    Ok(())
}

pub(crate) fn write_approval_record(
    request: &Request,
    gate: &str,
    status: &str,
    by: &str,
    source: &str,
    comment: &str,
) -> Result<()> {
    validate_gate(gate)?;
    let artifact = approval_artifact_path(request, gate);
    if !artifact.exists() {
        return Err(format!("gate artifact does not exist: {}", artifact.display()).into());
    }
    write_document_gate_record(request, gate, status, by, source, comment)?;
    write_status_json(
        request,
        gate_stage(gate),
        &request.status,
        fallback_empty(comment, "gate state updated in document frontmatter"),
    )
}

fn gate_stage(gate: &str) -> &'static str {
    match gate {
        "decomposition" => "decomposition",
        "plan" => "planning",
        "change-doc" => "implementation",
        _ => "gate",
    }
}

fn status_gate_record(request: &Request, gate: &str) -> Option<String> {
    render_document_gate_record(request, gate)
        .ok()
        .flatten()
        .or_else(|| legacy_status_gate_record(request, gate))
}

fn legacy_status_gate_record(request: &Request, gate: &str) -> Option<String> {
    fs::read_to_string(status_json_path(request))
        .ok()
        .and_then(|content| status_gate_record_from_content(&content, gate))
        .or_else(|| legacy_approval_record(request, gate))
}

fn legacy_approval_record(request: &Request, gate: &str) -> Option<String> {
    let content = fs::read_to_string(approval_file_path(request, gate)).ok()?;
    Some(format!(
        "{{ \"gate\": \"{}\", \"status\": \"{}\", \"artifact\": \"{}\", \"artifact_sha256\": \"{}\", \"by\": \"legacy-gate-migration\", \"source\": \"legacy-gate-migration\", \"comment\": \"migrated from legacy gate directory\", \"updated_at\": \"{}\" }}",
        json_escape(gate),
        json_escape(&json_value(&content, "status").unwrap_or_default()),
        json_escape(&json_value(&content, "artifact").unwrap_or_default()),
        json_escape(&json_value(&content, "artifact_sha256").unwrap_or_default()),
        json_escape(&json_value(&content, "updated_at").unwrap_or_default()),
    ))
}

pub(crate) fn migrate_legacy_approval_records(request: &Request) -> Result<()> {
    for gate in ["decomposition", "plan", "change-doc"] {
        let Some(record) = legacy_approval_record(request, gate) else {
            continue;
        };
        let status = json_value(&record, "status").unwrap_or_default();
        if approval_artifact_path(request, gate).exists() {
            write_document_gate_record(
                request,
                gate,
                &status,
                "legacy-gate-migration",
                "legacy-gate-migration",
                "migrated from legacy approval directory",
            )?;
        }
    }
    Ok(())
}

pub(crate) fn normalize_legacy_gate_records(request: &Request, dry_run: bool) -> Result<()> {
    let Ok(content) = fs::read_to_string(status_json_path(request)) else {
        return Ok(());
    };
    let stage = fallback_empty(
        &json_value(&content, "stage").unwrap_or_default(),
        "unknown",
    )
    .to_string();
    let status = fallback_empty(
        &json_value(&content, "status").unwrap_or_default(),
        &request.status,
    )
    .to_string();
    let reason = fallback_empty(
        &json_value(&content, "reason").unwrap_or_default(),
        "normalized legacy gate records",
    )
    .to_string();
    for gate in ["decomposition", "plan", "change-doc"] {
        let Some(record) = status_gate_record_from_content(&content, gate) else {
            continue;
        };
        let gate_status = json_value(&record, "status").unwrap_or_default();
        let artifact_path = approval_artifact_path(request, gate);
        if !artifact_path.exists() {
            continue;
        }
        if dry_run {
            println!(
                "Would migrate legacy status gate record into document frontmatter for {} {}",
                request.request_id, gate
            );
        } else {
            write_document_gate_record(
                request,
                gate,
                &gate_status,
                "legacy-gate-migration",
                "legacy-gate-migration",
                "migrated from status.json.gates during upgrade",
            )?;
            write_status_json(request, &stage, &status, &reason)?;
        }
    }
    Ok(())
}

pub(crate) fn remove_legacy_approvals_dir(request: &Request, dry_run: bool) -> Result<()> {
    let approvals_dir = Path::new(&request.change_path).join("approvals");
    if !approvals_dir.exists() {
        return Ok(());
    }
    if dry_run {
        println!(
            "Would remove {} (legacy approval records will be moved into document frontmatter)",
            approvals_dir.display()
        );
    } else {
        fs::remove_dir_all(&approvals_dir)?;
        println!(
            "Removed {} (legacy approval records moved into document frontmatter)",
            approvals_dir.display()
        );
    }
    Ok(())
}

pub(crate) fn render_gate_record_json(request: &Request, gate: &str) -> Result<String> {
    validate_gate(gate)?;
    Ok(status_gate_record(request, gate).unwrap_or_else(|| {
        format!(
            "{{ \"gate\": \"{}\", \"status\": \"missing\" }}",
            json_escape(gate),
        )
    }))
}

pub(crate) fn ensure_gate_approved(request: &Request, gate: &str) -> Result<()> {
    validate_gate(gate)?;
    ensure_change_packet(request)?;
    if let Some(record) = render_document_gate_record(request, gate)? {
        let status = json_value(&record, "status").unwrap_or_default();
        if status != "approved" {
            return Err(format!(
                "{gate} gate approval required. Current gate status is `{}`.",
                fallback_empty(&status, "missing")
            )
            .into());
        }
        ensure_document_gate_approved(request, gate)?;
        return Ok(());
    }
    let Some(record) = legacy_status_gate_record(request, gate) else {
        return Err(
            format!("{gate} gate approval required. Current gate status is `missing`.").into(),
        );
    };
    let status = json_value(&record, "status").unwrap_or_default();
    if status != "approved" {
        return Err(format!(
            "{gate} gate approval required. Current gate status is `{}`.",
            fallback_empty(&status, "missing")
        )
        .into());
    }
    let approved_hash = json_value(&record, "artifact_sha256").unwrap_or_default();
    let current_hash = file_sha256(&approval_artifact_path(request, gate))?;
    if approved_hash != current_hash {
        return Err(format!(
            "{gate} gate approval is stale: approved artifact hash does not match current artifact"
        )
        .into());
    }
    Ok(())
}

pub(crate) fn approval_file_path(request: &Request, gate: &str) -> std::path::PathBuf {
    Path::new(&request.change_path)
        .join("approvals")
        .join(format!("{gate}.approval.json"))
}

pub(crate) fn approval_artifact_path(request: &Request, gate: &str) -> std::path::PathBuf {
    existing_or_preferred_request_artifact_path(
        request,
        approval_artifact_name(gate).unwrap_or("plan.md"),
    )
}

pub(crate) fn approval_artifact_name(gate: &str) -> Result<&'static str> {
    match gate {
        "decomposition" => Ok("decomposition.md"),
        "plan" => Ok("plan.md"),
        "change-doc" => Ok("change-doc.md"),
        _ => Err(format!("unsupported gate: {gate}").into()),
    }
}

pub(crate) fn gate_status_prefix(gate: &str) -> &str {
    match gate {
        "decomposition" => "decomposition",
        "plan" => "plan",
        "change-doc" => "change-doc",
        _ => "approval",
    }
}

pub(crate) fn ensure_change_packet(request: &Request) -> Result<()> {
    if request.change_path.is_empty() {
        return Err(format!(
            "{} has no change packet. Run sandrone plan first.",
            request.request_id
        )
        .into());
    }
    Ok(())
}

pub(crate) fn next_request_id(requests: &[Request]) -> String {
    let next = requests
        .iter()
        .filter_map(|request| request.request_id.strip_prefix("REQ-"))
        .filter_map(|value| value.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    format!("REQ-{next:04}")
}
