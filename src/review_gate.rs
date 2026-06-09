use super::*;
use std::io::{Read, Write};

pub(crate) fn plan_review(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;

    let dispatch = dispatch_review_stage(&mut requests, index, &mut request, "plan-review")?;
    print_review_dispatch("Plan review", &request, &dispatch);
    Ok(())
}

pub(crate) fn decomposition_review(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;
    let decomposition_path =
        existing_or_preferred_request_artifact_path(&request, "decomposition.md");
    if !decomposition_path.exists() {
        return Err(format!(
            "{} has no decomposition artifact. Run: sandrone decompose --request_id {}",
            request.request_id, request.request_id
        )
        .into());
    }

    let dispatch =
        dispatch_review_stage(&mut requests, index, &mut request, "decomposition-review")?;
    print_review_dispatch("Decomposition review", &request, &dispatch);
    Ok(())
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
        return Err(format!("{request_id} has no worktree. Run sandrone start first.").into());
    }
    if !Path::new(CHECK_FORMAT_TOOL).exists() {
        let reason = format!(
            "{CHECK_FORMAT_TOOL} does not exist; run sandrone upgrade or provide a replacement check connector"
        );
        mark_blocked(
            &mut requests,
            index,
            &mut request,
            "implementation",
            &reason,
        )?;
        return Err(reason.into());
    }
    if let Some(reason) = run_check_format_gate(&request)? {
        mark_review_rejected(
            &mut requests,
            index,
            &mut request,
            "implementation",
            "code-review",
            &reason,
        )?;
        return Err(format!("format check failed before code-review: {reason}").into());
    }

    let dispatch = dispatch_review_stage(&mut requests, index, &mut request, "code-review")?;
    print_review_dispatch("Code review", &request, &dispatch);
    Ok(())
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
        return Err(format!("{request_id} has no worktree. Run sandrone start first.").into());
    }

    let dispatch = dispatch_review_stage(&mut requests, index, &mut request, "integration-review")?;
    print_review_dispatch("Integration review", &request, &dispatch);
    Ok(())
}

fn run_check_format_gate(request: &Request) -> Result<Option<String>> {
    let output = Command::new("sh")
        .arg(CHECK_FORMAT_TOOL)
        .arg("--check")
        .current_dir(".")
        .env("SANDRONE_REQUEST_ID", &request.request_id)
        .env("SANDRONE_REQUEST_EXTERNAL_ID", &request.external_id)
        .env("SANDRONE_REQUEST_SOURCE", &request.source)
        .env("SANDRONE_REQUEST_TITLE", &request.title)
        .env("SANDRONE_REQUEST_URL", &request.url)
        .env("SANDRONE_BRANCH", &request.branch)
        .env(
            "SANDRONE_WORKTREE",
            absolute_path_string(&request.worktree_path),
        )
        .env(
            "SANDRONE_CHANGE_PATH",
            absolute_path_string(&request.change_path),
        )
        .envs(proxy_env())
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status = if output.status.success() {
        "passed"
    } else {
        "failed"
    };
    let exit_code = output
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "signal".to_string());
    update_change_doc_format_status(request, status, &exit_code)?;
    if output.status.success() {
        append_event(
            "format_check_passed",
            &request.request_id,
            "code-review",
            "format-check-passed",
            &format!("status={status}; exit_code={exit_code}"),
        )?;
        Ok(None)
    } else {
        let diagnostic = review_diagnostic_excerpt(&format!("{stdout}\n{stderr}"));
        let reason = format!(
            "format check failed before code-review; return to implementation. exit_code={exit_code}; diagnostic: {}",
            fallback_empty(&diagnostic, "check-format exited non-zero")
        );
        append_event(
            "format_check_failed",
            &request.request_id,
            "code-review",
            "code-review-rejected",
            &reason,
        )?;
        Ok(Some(reason))
    }
}

struct ReviewDispatch {
    attempt: u32,
    jobs: Vec<ReviewJob>,
    already_running: bool,
}

struct ReviewJob {
    reviewer: &'static str,
    pid: Option<u32>,
    stdout: PathBuf,
    stderr: PathBuf,
}

fn dispatch_review_stage(
    requests: &mut [Request],
    index: usize,
    request: &mut Request,
    stage: &str,
) -> Result<ReviewDispatch> {
    let review_dir = Path::new(&request.change_path).join("reviews").join(stage);
    let details_dir = review_dir.join("details");
    fs::create_dir_all(&review_dir)?;
    fs::create_dir_all(&details_dir)?;
    if request.status == review_running_status(stage) {
        return Ok(existing_review_dispatch(request, stage));
    }

    let attempt = next_review_attempt(&details_dir)?;
    let attempt_dir = review_attempt_state_dir(&request.request_id, stage, attempt);
    if attempt_dir.exists() {
        fs::remove_dir_all(&attempt_dir)?;
    }
    fs::create_dir_all(&attempt_dir)?;

    let mut jobs = Vec::new();
    for reviewer in review_definitions_for_stage(stage)? {
        let pid = spawn_reviewer_worker(request, stage, &reviewer, attempt)?;
        jobs.push(ReviewJob {
            reviewer: reviewer.name,
            pid: Some(pid),
            stdout: review_job_stdout_path(&request.request_id, stage, attempt, reviewer.file_stem),
            stderr: review_job_stderr_path(&request.request_id, stage, attempt, reviewer.file_stem),
        });
    }

    request.status = review_running_status(stage).to_string();
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(requests)?;
    write_status_json(
        request,
        review_phase_for_stage(stage),
        review_running_status(stage),
        "",
    )?;
    append_event(
        "review_dispatched",
        &request.request_id,
        stage,
        review_running_status(stage),
        &format!("attempt={attempt}; reviewers={}", jobs.len()),
    )?;
    update_gate_session(request, review_gate_for_stage(stage), "review-running")?;
    Ok(ReviewDispatch {
        attempt,
        jobs,
        already_running: false,
    })
}

fn existing_review_dispatch(request: &Request, stage: &str) -> ReviewDispatch {
    let attempt = latest_review_job_attempt(&request.request_id, stage).unwrap_or(0);
    let jobs = review_definitions_for_stage(stage)
        .unwrap_or_default()
        .into_iter()
        .map(|reviewer| ReviewJob {
            reviewer: reviewer.name,
            pid: read_review_job_pid(&request.request_id, stage, attempt, reviewer.file_stem)
                .ok()
                .flatten(),
            stdout: review_job_stdout_path(&request.request_id, stage, attempt, reviewer.file_stem),
            stderr: review_job_stderr_path(&request.request_id, stage, attempt, reviewer.file_stem),
        })
        .collect();
    ReviewDispatch {
        attempt,
        jobs,
        already_running: true,
    }
}

fn print_review_dispatch(label: &str, request: &Request, dispatch: &ReviewDispatch) {
    if dispatch.already_running {
        println!(
            "{label} already running for {} attempt {}.",
            request.request_id, dispatch.attempt
        );
    } else {
        println!(
            "{label} dispatched for {} attempt {}.",
            request.request_id, dispatch.attempt
        );
    }
    println!("  change path: {}", request.change_path);
    println!(
        "  summary: {}/reviews/{}/summary.json",
        request.change_path,
        review_stage_label(label)
    );
    for job in &dispatch.jobs {
        let pid = job
            .pid
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!("  {reviewer}: pid {pid}", reviewer = job.reviewer);
        println!(
            "    logs: {} | {}",
            job.stdout.display(),
            job.stderr.display()
        );
    }
}

fn review_stage_label(label: &str) -> &'static str {
    match label {
        "Decomposition review" => "decomposition-review",
        "Plan review" => "plan-review",
        "Code review" => "code-review",
        "Integration review" => "integration-review",
        _ => "review",
    }
}

pub(crate) fn refresh_review_stage(request_id: &str, stage: &str) -> Result<bool> {
    let requests = load_requests()?;
    let request = requests
        .iter()
        .find(|request| request.request_id == request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?
        .clone();
    let Some(attempt) = latest_review_job_attempt(request_id, stage) else {
        let reason = format!("{stage} is marked running but no reviewer attempt state exists");
        block_request_by_id(request_id, review_phase_for_stage(stage), &reason)?;
        return Ok(true);
    };
    let details_dir = Path::new(&request.change_path)
        .join("reviews")
        .join(stage)
        .join("details");
    fs::create_dir_all(&details_dir)?;

    let mut results = Vec::new();
    for reviewer in review_definitions_for_stage(stage)? {
        let detail_path = details_dir.join(format!("{attempt:03}-{}.json", reviewer.file_stem));
        let exit_path =
            existing_review_job_exit_path(request_id, stage, attempt, reviewer.file_stem);
        if !exit_path.exists() {
            match read_review_job_pid(request_id, stage, attempt, reviewer.file_stem)? {
                Some(pid) if process_is_running(pid) => return Ok(false),
                Some(pid) => {
                    let diagnostic = format!(
                        "{} reviewer pid {pid} is no longer running and no exit code was written. See {} and {}",
                        reviewer.name,
                        review_job_stdout_path(request_id, stage, attempt, reviewer.file_stem)
                            .display(),
                        review_job_stderr_path(request_id, stage, attempt, reviewer.file_stem)
                            .display(),
                    );
                    write_missing_review_detail(&detail_path, reviewer.name, &diagnostic)?;
                }
                None => {
                    let diagnostic = format!(
                        "{} reviewer is marked running but no pid was recorded. See {} and {}",
                        reviewer.name,
                        review_job_stdout_path(request_id, stage, attempt, reviewer.file_stem)
                            .display(),
                        review_job_stderr_path(request_id, stage, attempt, reviewer.file_stem)
                            .display(),
                    );
                    write_missing_review_detail(&detail_path, reviewer.name, &diagnostic)?;
                }
            }
        }
        if !detail_path.exists() {
            if exit_path.exists() {
                let exit_code = fs::read_to_string(&exit_path)
                    .map(|content| content.trim().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                let diagnostic = format!(
                    "{} reviewer worker exited with code {exit_code} before writing detail JSON. See {} and {}",
                    reviewer.name,
                    review_job_stdout_path(request_id, stage, attempt, reviewer.file_stem)
                        .display(),
                    review_job_stderr_path(request_id, stage, attempt, reviewer.file_stem)
                        .display(),
                );
                write_missing_review_detail(&detail_path, reviewer.name, &diagnostic)?;
            } else {
                return Ok(false);
            }
        }
        results.push(read_review_result_from_detail(&detail_path, reviewer.name)?);
    }
    write_review_summary(&request, stage, attempt, &results)?;
    update_change_doc_review_section(&request)?;
    match apply_review_results(request_id, stage, &results) {
        Ok(()) => {}
        Err(error) if is_review_terminal_error(&error.to_string()) => {}
        Err(error) => return Err(error),
    }
    Ok(true)
}

pub(crate) fn review_worker(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(
        args,
        &[
            "--request_id",
            "--request-id",
            "--stage",
            "--reviewer",
            "--attempt",
        ],
    )?;
    let request_id = required_request_id(args)?;
    let stage = required_flag(args, "--stage")?;
    let reviewer_name = required_flag(args, "--reviewer")?;
    let attempt = required_flag(args, "--attempt")?
        .parse::<u32>()
        .map_err(|_| "--attempt must be a positive integer")?;
    let requests = load_requests()?;
    let request = requests
        .iter()
        .find(|request| request.request_id == request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?
        .clone();
    ensure_change_packet(&request)?;
    let reviewer = review_definitions_for_stage(&stage)?
        .into_iter()
        .find(|definition| definition.name == reviewer_name)
        .ok_or_else(|| format!("unknown reviewer {reviewer_name} for stage {stage}"))?;
    let details_dir = Path::new(&request.change_path)
        .join("reviews")
        .join(&stage)
        .join("details");
    fs::create_dir_all(&details_dir)?;
    append_current_job_event(
        "worker-started",
        &format!("stage={stage}; reviewer={reviewer_name}; attempt={attempt:03}"),
    );
    match run_single_reviewer(&request, &stage, &reviewer, &details_dir, attempt) {
        Ok(result) => {
            append_current_job_event(
                "worker-completed",
                &format!(
                    "approved={}; gate_unavailable={}; detail={}",
                    result.approved, result.gate_unavailable, result.path
                ),
            );
            println!(
                "{} reviewer completed: approved={} gate_unavailable={} detail={}",
                result.reviewer, result.approved, result.gate_unavailable, result.path
            );
            Ok(())
        }
        Err(error) => {
            append_current_job_event("worker-failed", &error.to_string());
            Err(error)
        }
    }
}

fn spawn_reviewer_worker(
    request: &Request,
    stage: &str,
    reviewer: &ReviewDefinition,
    attempt: u32,
) -> Result<u32> {
    let state_dir = review_job_state_dir(&request.request_id, stage, attempt, reviewer.file_stem);
    let legacy_state_dir =
        legacy_review_job_state_dir(&request.request_id, stage, attempt, reviewer.file_stem);
    fs::create_dir_all(&state_dir)?;
    let stdout = create_truncated_runtime_file(
        review_job_stdout_path(&request.request_id, stage, attempt, reviewer.file_stem),
        Some(&legacy_review_job_stdout_path(
            &request.request_id,
            stage,
            attempt,
            reviewer.file_stem,
        )),
    )?;
    let stderr = create_truncated_runtime_file(
        review_job_stderr_path(&request.request_id, stage, attempt, reviewer.file_stem),
        Some(&legacy_review_job_stderr_path(
            &request.request_id,
            stage,
            attempt,
            reviewer.file_stem,
        )),
    )?;
    let exit_path = review_job_exit_path(&request.request_id, stage, attempt, reviewer.file_stem);
    let legacy_exit_path =
        legacy_review_job_exit_path(&request.request_id, stage, attempt, reviewer.file_stem);
    let hook_log_path =
        review_job_hook_log_path(&request.request_id, stage, attempt, reviewer.file_stem);
    let legacy_hook_log_path =
        legacy_review_job_hook_log_path(&request.request_id, stage, attempt, reviewer.file_stem);
    let events_log_path = job_events_log_path(&state_dir);
    drop(create_truncated_runtime_file(
        &hook_log_path,
        Some(&legacy_hook_log_path),
    )?);
    drop(create_truncated_runtime_file(&events_log_path, None)?);
    remove_runtime_file(&exit_path, Some(&legacy_exit_path))?;
    if legacy_state_dir.exists() {
        fs::create_dir_all(&legacy_state_dir)?;
    }
    let current_exe = env::current_exe()?;
    let wrapper_script = r#"bin=$1
exit_path=$2
legacy_exit_path=$3
hook_log=$4
runtime_log=$5
request_id=$6
shift 6

write_runtime_event() {
  event=$1
  detail=${2:-}
  printf '%s\t%s\t%s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$event" "$detail" >> "$runtime_log" 2>/dev/null || true
}

run_hook() {
  if [ -n "$request_id" ]; then
    "$bin" advance --request_id "$request_id" >> "$hook_log" 2>&1 || true
  fi
}

write_exit() {
  code=$1
  printf '%s\n' "$code" > "$exit_path"
  [ -n "$legacy_exit_path" ] && printf '%s\n' "$code" > "$legacy_exit_path"
  write_runtime_event wrapper-exited "exit=$code"
  run_hook "$code"
  exit "$code"
}

trap 'write_exit 129' HUP
trap 'write_exit 130' INT
trap 'write_exit 143' TERM
write_runtime_event wrapper-started "request_id=$request_id"
"$bin" "$@"
code=$?
write_runtime_event worker-exited "exit=$code"
write_exit "$code"
"#;
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(wrapper_script)
        .arg("Sandrone-review-wrapper")
        .arg(&current_exe)
        .arg(&exit_path)
        .arg(&legacy_exit_path)
        .arg(&hook_log_path)
        .arg(&events_log_path)
        .arg(&request.request_id)
        .arg("__review-worker")
        .arg("--request_id")
        .arg(&request.request_id)
        .arg("--stage")
        .arg(stage)
        .arg("--reviewer")
        .arg(reviewer.name)
        .arg("--attempt")
        .arg(attempt.to_string())
        .current_dir(".")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .env("SANDRONE_BIN", current_exe.to_string_lossy().to_string())
        .env(
            "SANDRONE_JOB_EVENTS_LOG",
            absolute_path_string(&events_log_path),
        )
        .env(
            "SANDRONE_JOB_RUNTIME",
            absolute_path_string(job_runtime_path(&state_dir)),
        )
        .envs(proxy_env());
    command.process_group(0);
    write_job_runtime(
        job_runtime_path(&state_dir),
        &JobRuntime {
            kind: "reviewer",
            request_id: &request.request_id,
            stage,
            attempt: &format!("{attempt:03}"),
            worker: reviewer.file_stem,
            tool: reviewer.tool,
            pid: None,
            status: "spawning",
        },
    )?;
    append_job_event(
        &events_log_path,
        "dispatched",
        &format!("stage={stage}; reviewer={}", reviewer.name),
    )?;
    let child = command.spawn()?;
    write_runtime_text(
        review_job_pid_path(&request.request_id, stage, attempt, reviewer.file_stem),
        &format!("{}\n", child.id()),
        Some(&legacy_review_job_pid_path(
            &request.request_id,
            stage,
            attempt,
            reviewer.file_stem,
        )),
    )?;
    write_job_runtime(
        job_runtime_path(&state_dir),
        &JobRuntime {
            kind: "reviewer",
            request_id: &request.request_id,
            stage,
            attempt: &format!("{attempt:03}"),
            worker: reviewer.file_stem,
            tool: reviewer.tool,
            pid: Some(child.id()),
            status: "running",
        },
    )?;
    Ok(child.id())
}

fn read_review_result_from_detail(path: &Path, reviewer: &str) -> Result<ReviewResult> {
    let content = fs::read_to_string(path)?;
    let (normalized, invalid_json) = normalize_review_json(reviewer, &content);
    if invalid_json {
        fs::write(path, ensure_trailing_newline(&normalized))?;
    }
    let reviewer_declared_unavailable = json_bool(&normalized, "gate_unavailable").unwrap_or(false);
    Ok(review_result_from_normalized(
        reviewer,
        path,
        &normalized,
        invalid_json,
        if invalid_json || reviewer_declared_unavailable {
            review_diagnostic_excerpt(&content)
        } else {
            String::new()
        },
    ))
}

fn write_missing_review_detail(path: &Path, reviewer: &str, diagnostic: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        ensure_trailing_newline(&rejected_review_json(
            reviewer,
            "review worker failed",
            diagnostic,
        )),
    )?;
    Ok(())
}

fn review_result_from_normalized(
    reviewer: &str,
    path: &Path,
    normalized: &str,
    tool_unavailable: bool,
    diagnostic: String,
) -> ReviewResult {
    let approved = json_bool(normalized, "approved").unwrap_or(false);
    let has_blocking_findings = review_has_blocking_findings(normalized);
    let reviewer_declared_unavailable = json_bool(normalized, "gate_unavailable").unwrap_or(false);
    let recommended_next_phase = normalize_recommended_next_phase(
        &json_value(normalized, "recommended_next_phase").unwrap_or_else(|| {
            default_recommended_next_phase(reviewer, approved, reviewer_declared_unavailable)
                .to_string()
        }),
        reviewer,
        approved,
        reviewer_declared_unavailable,
    );
    let summary = json_value(normalized, "summary").unwrap_or_else(|| "no summary".to_string());
    ReviewResult {
        reviewer: reviewer.to_string(),
        approved,
        has_blocking_findings,
        gate_unavailable: tool_unavailable || reviewer_declared_unavailable,
        recommended_next_phase,
        summary,
        diagnostic,
        path: path.to_string_lossy().to_string(),
    }
}

fn apply_review_results(request_id: &str, stage: &str, results: &[ReviewResult]) -> Result<()> {
    match stage {
        "decomposition-review" => apply_decomposition_review_results(request_id, results),
        "plan-review" => apply_plan_review_results(request_id, results),
        "code-review" => apply_code_review_results(request_id, results),
        "integration-review" => apply_integration_review_results(request_id, results),
        _ => Err(format!("unknown review stage: {stage}").into()),
    }
}

fn apply_plan_review_results(request_id: &str, results: &[ReviewResult]) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    if reviews_approved(results) {
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
        return Ok(());
    }
    if review_gate_unavailable(results) {
        let reason = review_gate_unavailable_reason("plan-review", results);
        mark_blocked(&mut requests, index, &mut request, "planning", &reason)?;
        return Err(format!(
            "{} review gate unavailable: {reason}",
            rejected_reviewers(results).join(", ")
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
    Err(format!(
        "{} rejected plan review",
        rejected_reviewers(results).join(", ")
    )
    .into())
}

fn apply_decomposition_review_results(request_id: &str, results: &[ReviewResult]) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    if reviews_approved(results) {
        approve_gate_from_review(
            &mut requests,
            index,
            &mut request,
            "decomposition",
            "DecompositionReviewer",
            "decomposition-review",
            "DecompositionReviewer approved the request decomposition gate",
        )?;
        let mut refreshed_requests = load_requests()?;
        if let Some(parent_index) = find_request_index(&refreshed_requests, request_id) {
            let preflight = assess_repository_before_planning()?;
            if materialize_slices_for_parent(&mut refreshed_requests, parent_index, &preflight)? {
                save_requests(&refreshed_requests)?;
            }
        }
        println!("Decomposition review approved for {request_id}");
        return Ok(());
    }
    if review_gate_unavailable(results) {
        let reason = review_gate_unavailable_reason("decomposition-review", results);
        mark_blocked(&mut requests, index, &mut request, "decomposition", &reason)?;
        return Err(format!(
            "{} review gate unavailable: {reason}",
            rejected_reviewers(results).join(", ")
        )
        .into());
    }
    mark_review_rejected(
        &mut requests,
        index,
        &mut request,
        "decomposition",
        "decomposition-review",
        "decomposition-review rejected; return to decomposition",
    )?;
    Err(format!(
        "{} rejected decomposition review",
        rejected_reviewers(results).join(", ")
    )
    .into())
}

fn apply_code_review_results(request_id: &str, results: &[ReviewResult]) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    if reviews_approved(results) {
        approve_gate_from_review(
            &mut requests,
            index,
            &mut request,
            "change-doc",
            "code-review",
            "code-review",
            "TestReviewer and DesignReviewer approved the code review gate",
        )?;
        if is_slice_request(&request) {
            mark_slice_finished_by_id(request_id, "code-review approved; slice finished")?;
        } else {
            mark_wait_update_pr_by_id(
                request_id,
                "code-review approved; waiting for PR creation or update",
            )?;
        }
        println!("Code review approved for {request_id}");
        return Ok(());
    }
    if review_gate_unavailable(results) {
        let reason = review_gate_unavailable_reason("code-review", results);
        mark_blocked(
            &mut requests,
            index,
            &mut request,
            "implementation",
            &reason,
        )?;
        return Err(format!(
            "{} review gate unavailable: {reason}",
            rejected_reviewers(results).join(", ")
        )
        .into());
    }
    match recommended_next_phase(results, "implementation").as_str() {
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
    Err(format!(
        "{} rejected code review",
        rejected_reviewers(results).join(", ")
    )
    .into())
}

fn apply_integration_review_results(request_id: &str, results: &[ReviewResult]) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    if reviews_approved(results) {
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
            request_id,
            "integration-review approved; waiting for PR branch update",
        )?;
        println!("Integration review approved for {request_id}");
        return Ok(());
    }
    if review_gate_unavailable(results) {
        let reason = review_gate_unavailable_reason("integration-review", results);
        mark_blocked(&mut requests, index, &mut request, "rebase", &reason)?;
        return Err(format!(
            "{} review gate unavailable: {reason}",
            rejected_reviewers(results).join(", ")
        )
        .into());
    }
    match recommended_next_phase(results, "implementation").as_str() {
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
    Err(format!(
        "{} rejected integration review",
        rejected_reviewers(results).join(", ")
    )
    .into())
}

fn review_definitions_for_stage(stage: &str) -> Result<Vec<ReviewDefinition>> {
    match stage {
        "decomposition-review" => Ok(vec![ReviewDefinition {
            name: "DecompositionReviewer",
            tool: DECOMPOSITION_REVIEW_TOOL,
            file_stem: "decomposition-reviewer",
        }]),
        "plan-review" => Ok(vec![ReviewDefinition {
            name: "PlanReviewer",
            tool: PLAN_REVIEW_TOOL,
            file_stem: "plan-reviewer",
        }]),
        "code-review" => Ok(vec![
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
        ]),
        "integration-review" => Ok(vec![ReviewDefinition {
            name: "IntegrationReviewer",
            tool: INTEGRATION_REVIEW_TOOL,
            file_stem: "integration-reviewer",
        }]),
        _ => Err(format!("unknown review stage: {stage}").into()),
    }
}

fn review_running_status(stage: &str) -> &'static str {
    match stage {
        "decomposition-review" => "decomposition-review-running",
        "plan-review" => "plan-review-running",
        "code-review" => "code-review-running",
        "integration-review" => "integration-review-running",
        _ => "review-running",
    }
}

fn review_phase_for_stage(stage: &str) -> &'static str {
    match stage {
        "decomposition-review" => "decomposition",
        "plan-review" => "planning",
        "code-review" => "implementation",
        "integration-review" => "integration-review",
        _ => "review",
    }
}

fn review_gate_for_stage(stage: &str) -> &'static str {
    match stage {
        "decomposition-review" => "decomposition",
        "plan-review" => "plan",
        "code-review" | "integration-review" => "change-doc",
        _ => "review",
    }
}

fn review_stage_state_dir(request_id: &str, stage: &str) -> PathBuf {
    Path::new(".sandrone/state/jobs")
        .join(request_id)
        .join(stage)
}

fn review_attempt_state_dir(request_id: &str, stage: &str, attempt: u32) -> PathBuf {
    review_stage_state_dir(request_id, stage).join(format!("{attempt:03}"))
}

fn review_job_state_dir(request_id: &str, stage: &str, attempt: u32, file_stem: &str) -> PathBuf {
    runtime_review_job_state_dir(request_id, stage, attempt, file_stem)
}

fn legacy_review_job_pid_path(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    legacy_review_job_state_dir(request_id, stage, attempt, file_stem).join("pid")
}

fn legacy_review_job_exit_path(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    legacy_review_job_state_dir(request_id, stage, attempt, file_stem).join("exit")
}

fn legacy_review_job_stdout_path(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    legacy_review_job_state_dir(request_id, stage, attempt, file_stem).join("stdout.log")
}

fn legacy_review_job_stderr_path(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    legacy_review_job_state_dir(request_id, stage, attempt, file_stem).join("stderr.log")
}

fn legacy_review_job_hook_log_path(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    legacy_review_job_state_dir(request_id, stage, attempt, file_stem).join("hook.log")
}

fn review_job_pid_path(request_id: &str, stage: &str, attempt: u32, file_stem: &str) -> PathBuf {
    review_job_state_dir(request_id, stage, attempt, file_stem).join("pid")
}

fn review_job_exit_path(request_id: &str, stage: &str, attempt: u32, file_stem: &str) -> PathBuf {
    review_job_state_dir(request_id, stage, attempt, file_stem).join("exit")
}

fn existing_review_job_exit_path(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    existing_runtime_path(
        review_job_exit_path(request_id, stage, attempt, file_stem),
        legacy_review_job_exit_path(request_id, stage, attempt, file_stem),
    )
}

fn review_job_stdout_path(request_id: &str, stage: &str, attempt: u32, file_stem: &str) -> PathBuf {
    review_job_state_dir(request_id, stage, attempt, file_stem).join("stdout.log")
}

fn review_job_stderr_path(request_id: &str, stage: &str, attempt: u32, file_stem: &str) -> PathBuf {
    review_job_state_dir(request_id, stage, attempt, file_stem).join("stderr.log")
}

fn review_job_hook_log_path(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    review_job_state_dir(request_id, stage, attempt, file_stem).join("hook.log")
}

fn read_review_job_pid(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> Result<Option<u32>> {
    let content = read_runtime_text(
        review_job_pid_path(request_id, stage, attempt, file_stem),
        Some(&legacy_review_job_pid_path(
            request_id, stage, attempt, file_stem,
        )),
    )?;
    if content.trim().is_empty() {
        return Ok(None);
    }
    Ok(content.trim().parse::<u32>().ok())
}

fn latest_review_job_attempt(request_id: &str, stage: &str) -> Option<u32> {
    let mut latest = 0;
    for dir in [
        review_stage_state_dir(request_id, stage),
        legacy_review_stage_state_dir(request_id, stage),
    ] {
        latest = latest.max(latest_review_job_attempt_in_dir(&dir));
    }
    (latest > 0).then_some(latest)
}

fn latest_review_job_attempt_in_dir(dir: &Path) -> u32 {
    let mut latest = 0;
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    for entry in entries.flatten() {
        if !entry
            .file_type()
            .ok()
            .is_some_and(|file_type| file_type.is_dir())
        {
            continue;
        }
        if let Ok(attempt) = entry.file_name().to_string_lossy().parse::<u32>() {
            latest = latest.max(attempt);
        }
    }
    latest
}

struct ReviewerProcessOutput {
    status_success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

fn reviewer_timeout_duration() -> Duration {
    let seconds = env::var("SANDRONE_REVIEW_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .or_else(|| {
            dotenv_value("SANDRONE_REVIEW_TIMEOUT_SECONDS")
                .and_then(|value| value.trim().parse::<u64>().ok())
        })
        .unwrap_or(1800)
        .max(1);
    Duration::from_secs(seconds)
}

fn dotenv_value(key: &str) -> Option<String> {
    let content = fs::read_to_string(".env").ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        if raw_key.trim() != key {
            continue;
        }
        let mut value = raw_value.trim().to_string();
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = value[1..value.len() - 1].to_string();
        } else if let Some((before_comment, _)) = value.split_once(" #") {
            value = before_comment.trim().to_string();
        }
        return Some(value);
    }
    None
}

fn run_reviewer_command_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> std::io::Result<ReviewerProcessOutput> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    command.process_group(0);
    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("reviewer stdout pipe was not available"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| std::io::Error::other("reviewer stderr pipe was not available"))?;
    let stdout_reader = spawn_reviewer_pipe_reader(stdout, ReviewerPipe::Stdout);
    let stderr_reader = spawn_reviewer_pipe_reader(stderr, ReviewerPipe::Stderr);
    let started_at = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let stdout = join_reviewer_pipe_reader(stdout_reader)?;
            let stderr = join_reviewer_pipe_reader(stderr_reader)?;
            return Ok(ReviewerProcessOutput {
                status_success: status.success(),
                stdout,
                stderr,
                timed_out: false,
            });
        }
        if started_at.elapsed() >= timeout {
            terminate_reviewer_process_group(child.id());
            let _ = child.kill();
            let _ = child.wait();
            let stdout = join_reviewer_pipe_reader(stdout_reader)?;
            let stderr = join_reviewer_pipe_reader(stderr_reader)?;
            return Ok(ReviewerProcessOutput {
                status_success: false,
                stdout,
                stderr,
                timed_out: true,
            });
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

enum ReviewerPipe {
    Stdout,
    Stderr,
}

fn spawn_reviewer_pipe_reader<R>(
    mut reader: R,
    pipe: ReviewerPipe,
) -> std::thread::JoinHandle<std::io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let mut captured = Vec::new();
        let mut buffer = [0u8; 8192];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            captured.extend_from_slice(&buffer[..read]);
            match pipe {
                ReviewerPipe::Stdout => {
                    std::io::stdout().write_all(&buffer[..read])?;
                    std::io::stdout().flush()?;
                }
                ReviewerPipe::Stderr => {
                    std::io::stderr().write_all(&buffer[..read])?;
                    std::io::stderr().flush()?;
                }
            }
        }
        Ok(captured)
    })
}

fn join_reviewer_pipe_reader(
    reader: std::thread::JoinHandle<std::io::Result<Vec<u8>>>,
) -> std::io::Result<Vec<u8>> {
    reader
        .join()
        .map_err(|_| std::io::Error::other("reviewer output reader thread panicked"))?
}

fn terminate_reviewer_process_group(child_id: u32) {
    let process_group = format!("-{child_id}");
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(&process_group)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    std::thread::sleep(Duration::from_millis(500));
    let _ = Command::new("kill")
        .arg("-KILL")
        .arg(&process_group)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn append_current_job_event(event: &str, detail: &str) {
    if let Ok(path) = env::var("SANDRONE_JOB_EVENTS_LOG") {
        let _ = append_job_event(Path::new(&path), event, detail);
    }
}

fn write_current_job_result(content: &str) {
    let Ok(path) = env::var("SANDRONE_JOB_RUNTIME") else {
        return;
    };
    let runtime_path = Path::new(&path);
    let Some(parent) = runtime_path.parent() else {
        return;
    };
    let _ = fs::write(parent.join("result.json"), ensure_trailing_newline(content));
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
    let request_artifact = review_context_artifact_source(request, "request.md");
    let plan_artifact = review_context_artifact_source(request, "plan.md");
    let request_or_plan_artifact = if request_artifact.exists() {
        request_artifact.clone()
    } else {
        plan_artifact.clone()
    };
    let decomposition_artifact = review_context_artifact_source(request, "decomposition.md");
    let dag_artifact = review_context_artifact_source(request, "dag.json");
    let change_doc_artifact = review_context_artifact_source(request, "change-doc.md");
    let codegraph_context_artifact = Path::new("obsidian/codegraph/context.md");
    let obsidian_note_artifact = obsidian_request_note_path(request);
    let obsidian_project_artifact = Path::new(OBSIDIAN_PROJECT_NOTE);
    let forbidden_review_paths = format!(
        "{};{}",
        absolute_path_string(Path::new(&request.change_path).join("reviews")),
        absolute_path_string(Path::new(&request.change_path).join("reviews").join(stage)),
    );
    let (content, tool_unavailable, diagnostic) = if !Path::new(reviewer.tool).exists() {
        let diagnostic = format!("{} does not exist", reviewer.tool);
        append_current_job_event("review-tool-missing", &diagnostic);
        (
            rejected_review_json(reviewer.name, "review tool missing", &diagnostic),
            true,
            diagnostic,
        )
    } else {
        let timeout = reviewer_timeout_duration();
        let mut command = Command::new("sh");
        command
            .arg(reviewer.tool)
            .current_dir(".")
            .env("SANDRONE_REVIEW_STAGE", stage)
            .env("SANDRONE_REVIEWER", reviewer.name)
            .env("SANDRONE_WORKSPACE", absolute_path_string("."))
            .env("SANDRONE_ENV_FILE", absolute_path_string(".env"))
            .env("SANDRONE_TARGET_REPO", absolute_path_string(DEV_REPO))
            .env("SANDRONE_REQUEST_ID", &request.request_id)
            .env("SANDRONE_REQUEST_EXTERNAL_ID", &request.external_id)
            .env("SANDRONE_REQUEST_SOURCE", &request.source)
            .env("SANDRONE_REQUEST_TITLE", &request.title)
            .env("SANDRONE_REQUEST_BODY", &request.body)
            .env("SANDRONE_REQUEST_URL", &request.url)
            .env("SANDRONE_CHANGE_PATH", &review_context_string)
            .env("SANDRONE_REVIEW_CONTEXT", &review_context_string)
            .env(
                "SANDRONE_CANONICAL_CHANGE_PATH",
                absolute_path_string(request.change_path.as_str()),
            )
            .env("SANDRONE_REVIEW_FORBIDDEN_PATHS", forbidden_review_paths)
            .env(
                "SANDRONE_REQUEST",
                absolute_path_string_or_empty(&request_or_plan_artifact),
            )
            .env(
                "SANDRONE_ISSUE",
                absolute_path_string_or_empty(&request_or_plan_artifact),
            )
            .env(
                "SANDRONE_SPEC",
                absolute_path_string_or_empty(&plan_artifact),
            )
            .env(
                "SANDRONE_PLAN",
                absolute_path_string_or_empty(&plan_artifact),
            )
            .env(
                "SANDRONE_DECOMPOSITION",
                absolute_path_string_or_empty(&decomposition_artifact),
            )
            .env("SANDRONE_DAG", absolute_path_string_or_empty(&dag_artifact))
            .env(
                "SANDRONE_CODEGRAPH_CONTEXT",
                absolute_path_string_or_empty(codegraph_context_artifact),
            )
            .env(
                "SANDRONE_OBSIDIAN_NOTE",
                absolute_path_string_or_empty(&obsidian_note_artifact),
            )
            .env(
                "SANDRONE_OBSIDIAN_PROJECT",
                absolute_path_string_or_empty(obsidian_project_artifact),
            )
            .env(
                "SANDRONE_TASKS",
                absolute_path_string_or_empty(&plan_artifact),
            )
            .env(
                "SANDRONE_CHANGE_DOC",
                absolute_path_string_or_empty(&change_doc_artifact),
            )
            .env(
                "SANDRONE_WORKTREE",
                absolute_path_string(request.worktree_path.as_str()),
            )
            .env(
                "SANDRONE_REVIEW_SCHEMA",
                absolute_path_string(REVIEW_SCHEMA),
            )
            .envs(proxy_env());
        append_current_job_event(
            "review-tool-started",
            &format!("tool={}; timeout={}s", reviewer.tool, timeout.as_secs()),
        );
        let output = run_reviewer_command_with_timeout(&mut command, timeout);
        match output {
            Ok(output) if output.timed_out => {
                let timeout_seconds = timeout.as_secs();
                let diagnostic = format!(
                    "review tool timed out after {timeout_seconds}s: {}",
                    review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr))
                );
                append_current_job_event("review-tool-timed-out", &diagnostic);
                (
                    rejected_review_json(reviewer.name, "review tool timed out", &diagnostic),
                    true,
                    diagnostic,
                )
            }
            Ok(output) if output.status_success => {
                append_current_job_event("review-tool-exited", "exit=0");
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
                append_current_job_event(
                    "review-tool-exited",
                    &format!("exit=nonzero; {diagnostic}"),
                );
                (
                    rejected_review_json(reviewer.name, "review tool failed", &diagnostic),
                    true,
                    diagnostic,
                )
            }
            Err(error) => {
                let diagnostic = error.to_string();
                append_current_job_event("review-tool-spawn-failed", &diagnostic);
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
    write_current_job_result(&normalized);
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
    let context = Path::new(".sandrone/state/review-contexts")
        .join(&request.request_id)
        .join(stage)
        .join(format!("{attempt:03}"))
        .join(slugify(reviewer.name));
    if context.exists() {
        fs::remove_dir_all(&context)?;
    }
    fs::create_dir_all(&context)?;
    write_review_context_changed_files(&context, request)?;
    write_review_context_diff_stat(&context, request)?;
    write_review_context_test_summary(&context, request)?;
    write_review_context_artifact_index(&context, request, stage, reviewer, attempt)?;
    Ok(context)
}

fn write_review_context_artifact_index(
    context: &Path,
    request: &Request,
    stage: &str,
    reviewer: &ReviewDefinition,
    attempt: u32,
) -> Result<()> {
    let request_path = review_context_artifact_source(request, "request.md");
    let plan_path = review_context_artifact_source(request, "plan.md");
    let request_note = if is_slice_request(request) {
        "本 slice 没有独立 request.md；权威需求与 approved plan 都在 Plan 路径。"
    } else {
        "父 request 的权威需求见 Request 路径。"
    };
    let mut content = String::new();
    content.push_str("# Review Artifact Index\n\n");
    content.push_str("## 运行对象\n\n");
    content.push_str(&format!("- Reviewer: `{}`\n", reviewer.name));
    content.push_str(&format!("- Request ID: `{}`\n", request.request_id));
    content.push_str(&format!("- External ID: `{}`\n", request.external_id));
    content.push_str(&format!("- Stage: `{stage}`\n"));
    content.push_str(&format!("- Attempt: `{attempt:03}`\n"));
    content.push_str(&format!("- Title: {}\n", request.title));
    content.push_str(&format!("- Note: {request_note}\n\n"));
    content.push_str("## 必读路径\n\n");
    append_review_context_path(&mut content, "Review context", context);
    append_review_context_path(&mut content, "Request", &request_path);
    append_review_context_path(&mut content, "Plan", &plan_path);
    append_review_context_path(
        &mut content,
        "Decomposition",
        &review_context_artifact_source(request, "decomposition.md"),
    );
    append_review_context_path(
        &mut content,
        "DAG",
        &review_context_artifact_source(request, "dag.json"),
    );
    append_review_context_path(
        &mut content,
        "Change doc",
        &review_context_artifact_source(request, "change-doc.md"),
    );
    append_review_context_path(
        &mut content,
        "Status",
        &review_context_artifact_source(request, "status.json"),
    );
    append_review_context_path(&mut content, "Worktree", Path::new(&request.worktree_path));
    append_review_context_path(&mut content, "Target repo", Path::new(DEV_REPO));
    append_review_context_path(
        &mut content,
        "CodeGraph context",
        Path::new("obsidian/codegraph/context.md"),
    );
    append_review_context_path(
        &mut content,
        "Obsidian note",
        &obsidian_request_note_path(request),
    );
    append_review_context_path(
        &mut content,
        "Obsidian project",
        Path::new(OBSIDIAN_PROJECT_NOTE),
    );
    content.push('\n');
    content.push_str("## 自动摘要文件\n\n");
    append_review_context_path(
        &mut content,
        "Changed files",
        &context.join("changed-files.txt"),
    );
    append_review_context_path(&mut content, "Diff stat", &context.join("diff-stat.txt"));
    append_review_context_path(
        &mut content,
        "Test summary",
        &context.join("test-summary.txt"),
    );
    content.push('\n');
    content.push_str("## 推荐读取顺序\n\n");
    content.push_str("1. 先读本文件，确认边界、原始文档路径和禁止路径。\n");
    content.push_str("2. 读 `changed-files.txt`、`diff-stat.txt` 和 `test-summary.txt`。\n");
    content.push_str("3. 只打开 Plan/Change doc 中与当前 reviewer 判断直接相关的章节。\n");
    content.push_str("4. 按 changed files 精读少量源码或测试；不要默认扫描完整 worktree。\n");
    content.push_str("5. 如果自动摘要不足，再按本索引列出的原始路径补充读取。\n\n");
    content.push_str("## 禁止读取\n\n");
    content.push_str(&format!(
        "- `{}`\n",
        absolute_path_string(Path::new(&request.change_path).join("reviews"))
    ));
    content.push_str(&format!(
        "- `{}`\n",
        absolute_path_string(Path::new(&request.change_path).join("reviews").join(stage))
    ));
    content.push_str("- 历史 reviewer detail、summary、其他 reviewer 输出和 agent journal，除非 prompt 明确允许。\n");
    fs::write(context.join("artifact-index.md"), content)?;
    Ok(())
}

fn append_review_context_path(content: &mut String, label: &str, path: &Path) {
    let display = if path.as_os_str().is_empty() {
        "(not available)".to_string()
    } else {
        absolute_path_string(path)
    };
    content.push_str(&format!("- {label}: `{display}`\n"));
}

fn write_review_context_changed_files(context: &Path, request: &Request) -> Result<()> {
    let mut content = String::new();
    content.push_str("# Changed Files\n\n");
    append_git_section(
        &mut content,
        "git status --short",
        Path::new(&request.worktree_path),
        &["status", "--short"],
    );
    append_git_section(
        &mut content,
        "git diff --name-status HEAD",
        Path::new(&request.worktree_path),
        &["diff", "--name-status", "HEAD"],
    );
    append_git_section(
        &mut content,
        "git diff --cached --name-status HEAD",
        Path::new(&request.worktree_path),
        &["diff", "--cached", "--name-status", "HEAD"],
    );
    fs::write(context.join("changed-files.txt"), content)?;
    Ok(())
}

fn write_review_context_diff_stat(context: &Path, request: &Request) -> Result<()> {
    let mut content = String::new();
    content.push_str("# Diff Stat\n\n");
    append_git_section(
        &mut content,
        "git diff --stat HEAD",
        Path::new(&request.worktree_path),
        &["diff", "--stat", "HEAD"],
    );
    append_git_section(
        &mut content,
        "git diff --cached --stat HEAD",
        Path::new(&request.worktree_path),
        &["diff", "--cached", "--stat", "HEAD"],
    );
    fs::write(context.join("diff-stat.txt"), content)?;
    Ok(())
}

fn append_git_section(content: &mut String, title: &str, cwd: &Path, args: &[&str]) {
    content.push_str(&format!("## {title}\n\n```text\n"));
    let output = if cwd.as_os_str().is_empty() || !cwd.exists() {
        format!("skipped: {} does not exist", cwd.display())
    } else {
        match Command::new("git").args(args).current_dir(cwd).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut section = String::new();
                if !output.status.success() {
                    section.push_str(&format!("exit: {:?}\n", output.status.code()));
                }
                if stdout.trim().is_empty() && stderr.trim().is_empty() {
                    section.push_str("(empty)\n");
                } else {
                    section.push_str(stdout.trim_end());
                    if !stderr.trim().is_empty() {
                        if !section.ends_with('\n') {
                            section.push('\n');
                        }
                        section.push_str("stderr:\n");
                        section.push_str(stderr.trim_end());
                    }
                }
                section
            }
            Err(error) => format!("failed to run git: {error}"),
        }
    };
    content.push_str(&output);
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str("```\n\n");
}

fn write_review_context_test_summary(context: &Path, request: &Request) -> Result<()> {
    let change_doc = review_context_artifact_source(request, "change-doc.md");
    let plan = review_context_artifact_source(request, "plan.md");
    let mut content = String::new();
    content.push_str("# Test Summary\n\n");
    append_review_context_path(&mut content, "Plan", &plan);
    append_review_context_path(&mut content, "Change doc", &change_doc);
    content.push('\n');
    if change_doc.exists() {
        content.push_str("## Change Doc 验证相关摘录\n\n");
        content.push_str(&review_summary_excerpt(&fs::read_to_string(&change_doc)?));
    } else {
        content.push_str("Change doc is not available.\n");
    }
    fs::write(context.join("test-summary.txt"), content)?;
    Ok(())
}

fn review_summary_excerpt(content: &str) -> String {
    let keywords = [
        "验证",
        "测试",
        "test",
        "cargo",
        "clippy",
        "fmt",
        "format",
        "check",
        "pre-commit",
        "simulate",
        "通过",
        "失败",
    ];
    let mut lines = Vec::new();
    for line in content.lines() {
        let lower = line.to_ascii_lowercase();
        if keywords
            .iter()
            .any(|keyword| lower.contains(&keyword.to_ascii_lowercase()))
        {
            lines.push(line.trim_end().to_string());
        }
        if lines.len() >= 80 {
            break;
        }
    }
    if lines.is_empty() {
        "未从 change-doc 中提取到明显验证摘要；请按 artifact-index.md 中的 Change doc 路径按需读取验证章节。\n".to_string()
    } else {
        format!("{}\n", lines.join("\n"))
    }
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
    } else if approved && reviewer == "DecompositionReviewer" {
        "planning"
    } else if approved {
        "implementation"
    } else if matches!(reviewer, "PlanReviewer" | "DecompositionReviewer") {
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
        "{{\n  \"reviewer\": \"{}\",\n  \"approved\": false,\n  \"gate_unavailable\": true,\n  \"decision\": \"rejected\",\n  \"recommended_next_phase\": \"blocked\",\n  \"summary\": \"{}\",\n  \"process\": [\"review tool failure was converted into a blocking finding\"],\n  \"critical\": [{{ \"title\": \"{}\", \"evidence\": \"{}\", \"impact\": \"review gate cannot make a reliable approval decision\", \"required_fix\": \"Fix the reviewer tool or return valid structured review JSON.\", \"suggested_change\": \"Inspect the reviewer script stderr/stdout, restore the configured model backend, and make stdout exactly one JSON object matching tools/schemas/review-result.schema.json.\", \"verification\": \"Rerun the same Sandrone review command and confirm the detail JSON validates and gate_unavailable is false.\" }}],\n  \"high\": [],\n  \"warning\": [],\n  \"info\": []\n}}",
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
    let path = existing_or_preferred_request_artifact_path(request, "change-doc.md");
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
    for stage in [
        "decomposition-review",
        "plan-review",
        "code-review",
        "integration-review",
    ] {
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
                "decomposition-review" => "Decomposition Review",
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
            "DecompositionReviewer",
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
    request.status = format!("{}-approved", gate_status_prefix(gate));
    request.updated_at = now_string();
    write_approval_record(request, gate, "approved", by, source, comment)?;
    requests[index] = request.clone();
    save_requests(requests)?;
    let stage = match gate {
        "decomposition" => "decomposition",
        "plan" => "planning",
        _ => "implementation",
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
