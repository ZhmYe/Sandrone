use super::*;

pub(crate) fn load_config() -> Result<Config> {
    ensure_initialized()?;
    let content = fs::read_to_string(CONFIG_PATH)?;
    let mut schema_version = 1;
    let mut repo_name = String::new();
    let mut git_url = String::new();
    let mut base_branch = "main".to_string();
    let mut parallel_limit = 1;

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
            _ => {}
        }
    }

    Ok(Config {
        schema_version,
        repo_name,
        git_url,
        base_branch,
        parallel_limit,
    })
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
    fs::create_dir_all(".codex-auto-dev/state")?;
    let mut content = String::from("# codex-auto-dev requests v2\n");
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
    fs::create_dir_all(".codex-auto-dev")?;
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
    let phase = if gate == "plan" {
        "planning"
    } else {
        "implementation"
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
    ensure_change_packet(request)?;
    let review_cycle = review_cycle_for_status(request).unwrap_or(0);
    fs::write(
        Path::new(&request.change_path).join("status.json"),
        format!(
            "{{\n  \"schema_version\": 1,\n  \"request_id\": \"{}\",\n  \"stage\": \"{}\",\n  \"current_phase\": \"{}\",\n  \"status\": \"{}\",\n  \"reason\": \"{}\",\n  \"return_to_phase_reason\": \"{}\",\n  \"review_cycle\": {},\n  \"handoff_artifacts\": {{\n    \"request\": \"{}/request.md\",\n    \"plan\": \"{}/plan.md\",\n    \"change_doc\": \"{}/change-doc.md\",\n    \"agent_journal\": \"{}/agent-journal.md\"\n  }},\n  \"branch\": \"{}\",\n  \"worktree\": \"{}\",\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(&request.request_id),
            json_escape(stage),
            json_escape(status),
            json_escape(status),
            json_escape(reason),
            json_escape(reason),
            review_cycle,
            json_escape(&request.change_path),
            json_escape(&request.change_path),
            json_escape(&request.change_path),
            json_escape(&request.change_path),
            json_escape(&request.branch),
            json_escape(&request.worktree_path),
            json_escape(&now_string()),
        ),
    )?;
    Ok(())
}

pub(crate) fn review_cycle_for_status(request: &Request) -> Result<u32> {
    let plan_attempts = review_attempt_count(request, "plan-review")?;
    let code_attempts = review_attempt_count(request, "code-review")?;
    let integration_attempts = review_attempt_count(request, "integration-review")?;
    Ok(plan_attempts.max(code_attempts).max(integration_attempts))
}

pub(crate) fn append_event(
    event: &str,
    request_id: &str,
    phase: &str,
    status: &str,
    detail: &str,
) -> Result<()> {
    fs::create_dir_all(".codex-auto-dev/state")?;
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
    let phase = if stage == "planning" {
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
    let plan_summary = Path::new(&request.change_path)
        .join("reviews/plan-review/summary.json")
        .display()
        .to_string();
    let code_summary = Path::new(&request.change_path)
        .join("reviews/code-review/summary.json")
        .display()
        .to_string();
    fs::write(
        Path::new(&request.change_path).join("recovery.md"),
        format!(
            "# 恢复指南: {request_id}\n\n## 当前状态\n\n- Stage: `{stage}`\n- Status: `blocked`\n- Reason: {reason}\n\n## 关键路径\n\n- Request: `{change_path}/request.md`\n- Plan: `{change_path}/plan.md`\n- Change doc: `{change_path}/change-doc.md`\n- Agent journal: `{change_path}/agent-journal.md`\n- Status: `{change_path}/status.json`\n- Plan review summary: `{plan_summary}`\n- Code review summary: `{code_summary}`\n- Worktree: `{worktree}`\n- Branch: `{branch}`\n\n## 推荐恢复步骤\n\n1. 阅读 `request.md`、`plan.md`、`change-doc.md`、`agent-journal.md` 和本文件。\n2. 查看最后一轮 review summary 和 details，优先处理 critical/high。\n3. 如果需要继续自动修复，运行 `codex-auto-dev tick --request_id {request_id}`。\n4. 如果 reviewer 明显误判，人工审批必须写明 comment 和来源。\n",
            request_id = request.request_id,
            stage = stage,
            reason = reason,
            change_path = request.change_path,
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
        return Err(format!("approval artifact does not exist: {}", artifact.display()).into());
    }
    let approval_path = approval_file_path(request, gate);
    fs::create_dir_all(
        approval_path
            .parent()
            .ok_or("approval path has no parent directory")?,
    )?;
    let now = now_string();
    let artifact_string = artifact.to_string_lossy();
    let artifact_sha256 = file_sha256(&artifact)?;
    let decisions = if by.is_empty() {
        String::from("")
    } else {
        format!(
            "\n    {{ \"decision\": \"{}\", \"by\": \"{}\", \"source\": \"{}\", \"comment\": \"{}\", \"decided_at\": \"{}\" }}\n  ",
            json_escape(status),
            json_escape(by),
            json_escape(source),
            json_escape(comment),
            json_escape(&now),
        )
    };
    fs::write(
        approval_path,
        format!(
            "{{\n  \"schema_version\": 1,\n  \"request_id\": \"{}\",\n  \"gate\": \"{}\",\n  \"status\": \"{}\",\n  \"artifact\": \"{}\",\n  \"artifact_sha256\": \"{}\",\n  \"required_approvals\": 1,\n  \"decisions\": [{}],\n  \"submitted_at\": \"{}\",\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(&request.request_id),
            json_escape(gate),
            json_escape(status),
            json_escape(&artifact_string),
            json_escape(&artifact_sha256),
            decisions,
            json_escape(&now),
            json_escape(&now),
        ),
    )?;
    Ok(())
}

pub(crate) fn ensure_gate_approved(request: &Request, gate: &str) -> Result<()> {
    validate_gate(gate)?;
    ensure_change_packet(request)?;
    let approval_path = approval_file_path(request, gate);
    if !approval_path.exists() {
        return Err(format!(
            "{gate} approval required. Run: codex-auto-dev submit --request_id {} --gate {gate}; then codex-auto-dev approve --request_id {} --gate {gate} --by <actor>",
            request.request_id, request.request_id
        )
        .into());
    }
    let content = fs::read_to_string(&approval_path)?;
    let status = json_value(&content, "status").unwrap_or_default();
    if status != "approved" {
        return Err(format!(
            "{gate} approval required. Current approval status is `{}`.",
            fallback_empty(&status, "missing")
        )
        .into());
    }
    let approved_hash = json_value(&content, "artifact_sha256").unwrap_or_default();
    let current_hash = file_sha256(&approval_artifact_path(request, gate))?;
    if approved_hash != current_hash {
        return Err(format!(
            "{gate} approval is stale: approved artifact hash does not match current artifact"
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
    Path::new(&request.change_path).join(approval_artifact_name(gate).unwrap_or("plan.md"))
}

pub(crate) fn approval_artifact_name(gate: &str) -> Result<&'static str> {
    match gate {
        "plan" => Ok("plan.md"),
        "change-doc" => Ok("change-doc.md"),
        _ => Err(format!("unsupported approval gate: {gate}").into()),
    }
}

pub(crate) fn gate_status_prefix(gate: &str) -> &str {
    match gate {
        "plan" => "plan",
        "change-doc" => "change-doc",
        _ => "approval",
    }
}

pub(crate) fn ensure_change_packet(request: &Request) -> Result<()> {
    if request.change_path.is_empty() {
        return Err(format!(
            "{} has no change packet. Run codex-auto-dev plan first.",
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
