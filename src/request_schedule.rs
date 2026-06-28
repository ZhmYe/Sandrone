use super::*;

struct RequestScheduleCandidate {
    request: Request,
    detail: String,
}

struct RequestScheduleSnapshot {
    schedule_id: String,
    run_dir: PathBuf,
    queue_path: PathBuf,
    compat_queue_path: PathBuf,
    output_path: PathBuf,
    compat_output_path: PathBuf,
    plan_md_path: PathBuf,
    plan_json_path: PathBuf,
    compat_plan_json_path: PathBuf,
}

struct RequestScheduleReport {
    selected: Vec<(String, String)>,
    raw: String,
}

pub(crate) fn schedule_tick_requests(
    requests: &[Request],
    candidate_ids: &[String],
    max_parallel: usize,
) -> Result<Vec<String>> {
    if candidate_ids.is_empty() || max_parallel == 0 {
        return Ok(Vec::new());
    }
    let candidates = request_schedule_candidates(requests, candidate_ids)?;
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let snapshot = write_request_schedule_snapshot(&candidates, max_parallel)?;
    let report = run_request_schedule_agent(&snapshot, &candidates, max_parallel)?;
    write_request_schedule_artifacts(&snapshot, &candidates, &report, max_parallel)?;
    let approved = run_request_schedule_review(&snapshot, max_parallel)?;
    if !approved {
        append_event(
            "request_schedule_rejected",
            "",
            "schedule",
            "deferred",
            &format!("plan={}", snapshot.plan_md_path.display()),
        )?;
        return Ok(Vec::new());
    }
    let allowed = candidates
        .iter()
        .map(|candidate| candidate.request.request_id.as_str())
        .collect::<Vec<_>>();
    let mut selected = Vec::new();
    for (request_id, _) in report.selected {
        if selected.len() >= max_parallel {
            break;
        }
        if allowed.iter().any(|allowed| *allowed == request_id) && !selected.contains(&request_id) {
            selected.push(request_id);
        }
    }
    append_event(
        "request_schedule_approved",
        "",
        "schedule",
        "approved",
        &format!(
            "selected={}; plan={}",
            selected.join(","),
            snapshot.plan_md_path.display()
        ),
    )?;
    Ok(selected)
}

fn request_schedule_candidates(
    requests: &[Request],
    candidate_ids: &[String],
) -> Result<Vec<RequestScheduleCandidate>> {
    let mut candidates = Vec::new();
    for request_id in candidate_ids {
        let Some(request) = requests
            .iter()
            .find(|candidate| candidate.request_id == *request_id)
        else {
            continue;
        };
        candidates.push(RequestScheduleCandidate {
            request: request.clone(),
            detail: request_schedule_detail(request, requests)?,
        });
    }
    Ok(candidates)
}

fn request_schedule_detail(request: &Request, requests: &[Request]) -> Result<String> {
    if is_slice_request(request) {
        if slice_dependencies_ready(request, requests)? {
            Ok("slice dependencies ready".to_string())
        } else {
            Ok("slice dependencies not ready".to_string())
        }
    } else if is_parent_request(request) {
        Ok("parent request".to_string())
    } else {
        Ok("standalone request".to_string())
    }
}

fn write_request_schedule_snapshot(
    candidates: &[RequestScheduleCandidate],
    max_parallel: usize,
) -> Result<RequestScheduleSnapshot> {
    let schedule_id = request_schedule_id("request-schedule");
    let run_dir = create_named_agent_run_state_dir(
        "request-schedule-agent",
        &["request-schedule", &schedule_id],
        "schedule",
        "current",
        "request-schedule-agent",
    )?;
    let artifacts_dir = job_artifacts_dir(&run_dir);
    let scheduler_dir = Path::new(".sandrone").join("state").join("scheduler");
    let obsidian_schedule_dir = Path::new("obsidian").join("schedule");
    fs::create_dir_all(&scheduler_dir)?;
    fs::create_dir_all(&obsidian_schedule_dir)?;
    let snapshot = RequestScheduleSnapshot {
        schedule_id,
        run_dir,
        queue_path: artifacts_dir.join("request-schedule-queue.tsv"),
        compat_queue_path: scheduler_dir.join("request-schedule-queue.tsv"),
        output_path: artifacts_dir.join("request-schedule-output.tsv"),
        compat_output_path: scheduler_dir.join("request-schedule-output.tsv"),
        plan_md_path: obsidian_schedule_dir.join("request-schedule.md"),
        plan_json_path: artifacts_dir.join("request-schedule.json"),
        compat_plan_json_path: scheduler_dir.join("request-schedule.json"),
    };
    write_request_schedule_queue(&snapshot, candidates, max_parallel)?;
    Ok(snapshot)
}

fn write_request_schedule_queue(
    snapshot: &RequestScheduleSnapshot,
    candidates: &[RequestScheduleCandidate],
    max_parallel: usize,
) -> Result<()> {
    let mut content =
        "request_id\ttitle\tstatus\tsource\tupdated_at\tchange_path\tbranch\tdetail\n".to_string();
    for candidate in candidates {
        content.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            schedule_tsv_cell(&candidate.request.request_id),
            schedule_tsv_cell(&candidate.request.title),
            schedule_tsv_cell(&candidate.request.status),
            schedule_tsv_cell(&candidate.request.source),
            schedule_tsv_cell(&candidate.request.updated_at),
            schedule_tsv_cell(&candidate.request.change_path),
            schedule_tsv_cell(&candidate.request.branch),
            schedule_tsv_cell(&candidate.detail),
        ));
    }
    write_runtime_text(
        &snapshot.queue_path,
        &content,
        Some(&snapshot.compat_queue_path),
    )?;
    let pending = RequestScheduleReport {
        selected: Vec::new(),
        raw: format!("defer\t\trequest schedule pending; max_parallel={max_parallel}"),
    };
    write_request_schedule_artifacts(snapshot, candidates, &pending, max_parallel)?;
    Ok(())
}

fn run_request_schedule_agent(
    snapshot: &RequestScheduleSnapshot,
    candidates: &[RequestScheduleCandidate],
    max_parallel: usize,
) -> Result<RequestScheduleReport> {
    let raw = if Path::new(REQUEST_SCHEDULE_AGENT_TOOL).exists() {
        let output = Command::new("sh")
            .arg(REQUEST_SCHEDULE_AGENT_TOOL)
            .current_dir(".")
            .env("SANDRONE_REQUEST_SCHEDULE_QUEUE", &snapshot.queue_path)
            .env("SANDRONE_REQUEST_SCHEDULE_MD", &snapshot.plan_md_path)
            .env("SANDRONE_REQUEST_SCHEDULE_JSON", &snapshot.plan_json_path)
            .env(
                "SANDRONE_REQUEST_SCHEDULE_MAX_PARALLEL",
                max_parallel.to_string(),
            )
            .env("SANDRONE_SCHEDULER_DECISION_ID", &snapshot.schedule_id)
            .envs(proxy_env())
            .output();
        match output {
            Ok(output) if output.status.success() => String::from_utf8(output.stdout)?,
            Ok(output) => format!(
                "blocked\t\t{}",
                review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr))
            ),
            Err(error) => format!("blocked\t\t{error}"),
        }
    } else {
        default_request_schedule_output(candidates, max_parallel)
    };
    write_runtime_text(
        &snapshot.output_path,
        &ensure_trailing_newline(&raw),
        Some(&snapshot.compat_output_path),
    )?;
    Ok(parse_request_schedule_report(&raw))
}

fn default_request_schedule_output(
    candidates: &[RequestScheduleCandidate],
    max_parallel: usize,
) -> String {
    let mut lines = Vec::new();
    for (index, candidate) in candidates.iter().take(max_parallel).enumerate() {
        lines.push(format!(
            "selected\t{}\tbuilt-in first-ready scheduler slot {}/{}",
            candidate.request.request_id,
            index + 1,
            max_parallel
        ));
    }
    if lines.is_empty() {
        "defer\t\tno schedulable request".to_string()
    } else {
        lines.join("\n")
    }
}

fn parse_request_schedule_report(raw: &str) -> RequestScheduleReport {
    let mut selected = Vec::new();
    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        let decision = fields.first().copied().unwrap_or("").trim();
        if decision == "selected" {
            let request_id = fields.get(1).copied().unwrap_or("").trim();
            if !request_id.is_empty() {
                selected.push((
                    request_id.to_string(),
                    fields.get(2).copied().unwrap_or("").trim().to_string(),
                ));
            }
        }
    }
    RequestScheduleReport {
        selected,
        raw: raw.trim().to_string(),
    }
}

fn write_request_schedule_artifacts(
    snapshot: &RequestScheduleSnapshot,
    candidates: &[RequestScheduleCandidate],
    report: &RequestScheduleReport,
    max_parallel: usize,
) -> Result<()> {
    let json = render_request_schedule_json(snapshot, candidates, report, max_parallel);
    let markdown = render_request_schedule_markdown(snapshot, candidates, report, max_parallel);
    write_runtime_text(
        &snapshot.plan_json_path,
        &json,
        Some(&snapshot.compat_plan_json_path),
    )?;
    fs::write(&snapshot.plan_md_path, markdown)?;
    Ok(())
}

fn run_request_schedule_review(
    snapshot: &RequestScheduleSnapshot,
    max_parallel: usize,
) -> Result<bool> {
    let run_id = request_schedule_id("request-schedule-review");
    let run_dir = create_named_agent_run_state_dir(
        "request-schedule-reviewer",
        &["request-schedule-review", &run_id],
        "schedule",
        "current",
        "request-schedule-reviewer",
    )?;
    let artifacts_dir = job_artifacts_dir(&run_dir);
    let logs_dir = job_logs_dir(&run_dir);
    let stdout_path = logs_dir.join("stdout.log");
    let stderr_path = logs_dir.join("stderr.log");
    let detail_path = artifacts_dir.join("request-schedule-review.json");
    let content = if Path::new(REQUEST_SCHEDULE_REVIEW_TOOL).exists() {
        let output = Command::new("sh")
            .arg(REQUEST_SCHEDULE_REVIEW_TOOL)
            .current_dir(".")
            .env("SANDRONE_REQUEST_SCHEDULE_QUEUE", &snapshot.queue_path)
            .env("SANDRONE_REQUEST_SCHEDULE_OUTPUT", &snapshot.output_path)
            .env("SANDRONE_REQUEST_SCHEDULE_MD", &snapshot.plan_md_path)
            .env("SANDRONE_REQUEST_SCHEDULE_JSON", &snapshot.plan_json_path)
            .env(
                "SANDRONE_REQUEST_SCHEDULE_MAX_PARALLEL",
                max_parallel.to_string(),
            )
            .env("SANDRONE_REVIEW_SCHEMA", REVIEW_SCHEMA)
            .envs(proxy_env())
            .output();
        match output {
            Ok(output) => {
                fs::write(&stdout_path, &output.stdout)?;
                fs::write(&stderr_path, &output.stderr)?;
                if output.status.success() {
                    String::from_utf8(output.stdout)?
                } else {
                    rejected_request_schedule_review_json(&format!(
                        "request schedule review tool failed: {}",
                        review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr))
                    ))
                }
            }
            Err(error) => {
                fs::write(&stdout_path, "")?;
                fs::write(&stderr_path, error.to_string())?;
                rejected_request_schedule_review_json(&format!(
                    "request schedule review tool could not run: {error}"
                ))
            }
        }
    } else {
        built_in_request_schedule_review(&snapshot.output_path, &snapshot.queue_path, max_parallel)?
    };
    let normalized = if content.trim_start().starts_with('{')
        && json_bool(&content, "approved").is_some()
        && json_bool(&content, "gate_unavailable").is_some()
    {
        content.trim().to_string()
    } else {
        rejected_request_schedule_review_json("request schedule review returned invalid JSON")
    };
    fs::write(&detail_path, ensure_trailing_newline(&normalized))?;
    write_runtime_text(
        Path::new(".sandrone")
            .join("state")
            .join("scheduler")
            .join("request-schedule-review.json"),
        &ensure_trailing_newline(&normalized),
        None,
    )?;
    let approved = json_bool(&normalized, "approved").unwrap_or(false);
    let gate_unavailable = json_bool(&normalized, "gate_unavailable").unwrap_or(false);
    Ok(approved
        && !gate_unavailable
        && review_array_empty(&normalized, "critical")
        && review_array_empty(&normalized, "high"))
}

fn built_in_request_schedule_review(
    output_path: &Path,
    queue_path: &Path,
    max_parallel: usize,
) -> Result<String> {
    let output = fs::read_to_string(output_path)?;
    let queue = fs::read_to_string(queue_path)?;
    let selected = parse_request_schedule_report(&output).selected;
    let queue_ids = queue
        .lines()
        .skip(1)
        .filter_map(|line| line.split('\t').next())
        .collect::<Vec<_>>();
    if selected.len() > max_parallel {
        return Ok(rejected_request_schedule_review_json(
            "request schedule selected more requests than max_parallel",
        ));
    }
    for (request_id, _) in &selected {
        if !queue_ids.iter().any(|id| id == request_id) {
            return Ok(rejected_request_schedule_review_json(&format!(
                "request schedule selected id outside queue: {request_id}"
            )));
        }
    }
    Ok(format!(
        "{{\n  \"reviewer\": \"RequestScheduleReviewer\",\n  \"approved\": true,\n  \"gate_unavailable\": false,\n  \"decision\": \"approved\",\n  \"recommended_next_phase\": \"implementation\",\n  \"summary\": \"request schedule approved {} request(s)\",\n  \"process\": [\"checked selected ids\", \"checked max parallel\"],\n  \"critical\": [],\n  \"high\": [],\n  \"warning\": [],\n  \"info\": []\n}}\n",
        selected.len()
    ))
}

fn render_request_schedule_json(
    snapshot: &RequestScheduleSnapshot,
    candidates: &[RequestScheduleCandidate],
    report: &RequestScheduleReport,
    max_parallel: usize,
) -> String {
    let selected_json = report
        .selected
        .iter()
        .map(|(request_id, reason)| {
            format!(
                "    {{\"request_id\":\"{}\",\"reason\":\"{}\"}}",
                json_escape(request_id),
                json_escape(reason)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let candidates_json = candidates
        .iter()
        .map(|candidate| {
            format!(
                "    {{\"request_id\":\"{}\",\"title\":\"{}\",\"status\":\"{}\",\"detail\":\"{}\"}}",
                json_escape(&candidate.request.request_id),
                json_escape(&candidate.request.title),
                json_escape(&candidate.request.status),
                json_escape(&candidate.detail),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema_version\": 1,\n  \"schedule_id\": \"{}\",\n  \"max_parallel\": {},\n  \"queue_path\": \"{}\",\n  \"output_path\": \"{}\",\n  \"plan_markdown\": \"{}\",\n  \"run_dir\": \"{}\",\n  \"raw\": \"{}\",\n  \"selected\": [\n{}\n  ],\n  \"candidates\": [\n{}\n  ],\n  \"updated_at\": \"{}\"\n}}\n",
        json_escape(&snapshot.schedule_id),
        max_parallel,
        json_escape(&snapshot.queue_path.to_string_lossy()),
        json_escape(&snapshot.output_path.to_string_lossy()),
        json_escape(&snapshot.plan_md_path.to_string_lossy()),
        json_escape(&snapshot.run_dir.to_string_lossy()),
        json_escape(&report.raw),
        selected_json,
        candidates_json,
        json_escape(&now_string()),
    )
}

fn render_request_schedule_markdown(
    snapshot: &RequestScheduleSnapshot,
    candidates: &[RequestScheduleCandidate],
    report: &RequestScheduleReport,
    max_parallel: usize,
) -> String {
    let mut content = format!(
        "# Request Schedule - {}\n\n- Schedule id: `{}`\n- Max parallel: `{}`\n- Queue snapshot: `{}`\n- Machine plan: `{}`\n\n",
        now_string(),
        snapshot.schedule_id,
        max_parallel,
        snapshot.queue_path.display(),
        snapshot.plan_json_path.display(),
    );
    content.push_str("## Selected\n\n");
    if report.selected.is_empty() {
        content.push_str("No request selected for this loop pass.\n\n");
    } else {
        content.push_str("| Request | Reason |\n|---|---|\n");
        for (request_id, reason) in &report.selected {
            content.push_str(&format!(
                "| `{}` | {} |\n",
                markdown_table_escape_local(request_id),
                markdown_table_escape_local(reason),
            ));
        }
        content.push('\n');
    }
    content.push_str("## Candidates\n\n");
    content.push_str("| Request | Status | Updated | Detail | Title |\n|---|---|---|---|---|\n");
    for candidate in candidates {
        content.push_str(&format!(
            "| `{}` | `{}` | `{}` | {} | {} |\n",
            markdown_table_escape_local(&candidate.request.request_id),
            markdown_table_escape_local(&candidate.request.status),
            markdown_table_escape_local(&candidate.request.updated_at),
            markdown_table_escape_local(&candidate.detail),
            markdown_table_escape_local(&candidate.request.title),
        ));
    }
    content.push_str("\n## Scope\n\n");
    content.push_str(
        "- This schedule only chooses which requests may be dispatched in the current loop pass.\n",
    );
    content.push_str(
        "- Each request still must pass decomposition, plan, implementation, and review gates.\n",
    );
    content.push_str(
        "- PR delivery and automatic merge remain serial and happen after each request passes its own final code-review gate.\n",
    );
    content
}

fn rejected_request_schedule_review_json(reason: &str) -> String {
    format!(
        "{{\n  \"reviewer\": \"RequestScheduleReviewer\",\n  \"approved\": false,\n  \"gate_unavailable\": true,\n  \"decision\": \"rejected\",\n  \"recommended_next_phase\": \"blocked\",\n  \"summary\": \"{}\",\n  \"process\": [\"request schedule review failed closed\"],\n  \"critical\": [{{\"title\":\"request schedule review failed\",\"evidence\":\"{}\",\"impact\":\"loop cannot safely dispatch new work\",\"required_fix\":\"Fix the request schedule output or reviewer connector.\",\"suggested_change\":\"Inspect agents/request-schedule-* logs and rerun sandrone loop start.\",\"verification\":\"RequestScheduleReviewer returns approved=true with no critical/high findings.\"}}],\n  \"high\": [],\n  \"warning\": [],\n  \"info\": []\n}}\n",
        json_escape(reason),
        json_escape(reason),
    )
}

fn review_array_empty(content: &str, key: &str) -> bool {
    let needle = format!("\"{key}\"");
    let Some(key_start) = content.find(&needle) else {
        return false;
    };
    let Some(array_start_offset) = content[key_start..].find('[') else {
        return false;
    };
    let mut depth = 0i32;
    let mut body = String::new();
    for ch in content[key_start + array_start_offset..].chars() {
        if ch == '[' {
            depth += 1;
            if depth == 1 {
                continue;
            }
        } else if ch == ']' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        if depth >= 1 {
            body.push(ch);
        }
    }
    body.trim().is_empty()
}

fn schedule_tsv_cell(value: &str) -> String {
    value.replace(['\t', '\n', '\r'], " ")
}

fn markdown_table_escape_local(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn request_schedule_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos}")
}
