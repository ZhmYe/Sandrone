use super::*;
use std::collections::BTreeMap;

const DOC_STATUS_KEYS: &[&str] = &[
    "sandrone_schema",
    "request_id",
    "document_type",
    "agent_phase",
    "agent_status",
    "agent_ready_for_review",
    "format_check_status",
    "format_check_exit_code",
    "format_check_record",
    "gate_name",
    "gate_status",
    "gate_by",
    "gate_source",
    "gate_comment",
    "gate_body_sha256",
    "gate_updated_at",
    "updated_at",
];

const GATE_STATUS_KEYS: &[&str] = &[
    "gate_name",
    "gate_status",
    "gate_by",
    "gate_source",
    "gate_comment",
    "gate_body_sha256",
    "gate_updated_at",
];

pub(crate) fn phase_document_artifact(phase: AgentPhase) -> &'static str {
    match phase {
        AgentPhase::Decomposition => "decomposition.md",
        AgentPhase::Planning => "plan.md",
        AgentPhase::Implementation | AgentPhase::Rebase => "change-doc.md",
    }
}

pub(crate) fn phase_document_type(phase: AgentPhase) -> &'static str {
    match phase {
        AgentPhase::Decomposition => "decomposition",
        AgentPhase::Planning => "plan",
        AgentPhase::Implementation | AgentPhase::Rebase => "change-doc",
    }
}

pub(crate) fn phase_document_path(request: &Request, phase: AgentPhase) -> PathBuf {
    existing_or_preferred_request_artifact_path(request, phase_document_artifact(phase))
}

pub(crate) fn mark_phase_document_submitted(request: &Request, phase: AgentPhase) -> Result<()> {
    let path = phase_document_path(request, phase);
    if path.exists() {
        upsert_document_status(&path, request, phase, "submitted", true, None)?;
    }
    Ok(())
}

pub(crate) fn update_change_doc_format_status(
    request: &Request,
    status: &str,
    exit_code: &str,
) -> Result<()> {
    let path = phase_document_path(request, AgentPhase::Implementation);
    if !path.exists() {
        return Ok(());
    }
    upsert_document_status(
        &path,
        request,
        AgentPhase::Implementation,
        &document_status_value(&path, "agent_status").unwrap_or_else(|| "draft".to_string()),
        document_status_value(&path, "agent_ready_for_review")
            .as_deref()
            .map(parse_frontmatter_bool)
            .unwrap_or(false),
        Some(FormatStatusUpdate {
            status: status.to_string(),
            exit_code: exit_code.to_string(),
        }),
    )
}

pub(crate) fn agent_document_status_is_submitted(
    request: &Request,
    phase: AgentPhase,
) -> Result<Option<PathBuf>> {
    let path = phase_document_path(request, phase);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    let Some(fields) = frontmatter_fields(&content) else {
        return Ok(None);
    };
    if fields.get("request_id").map(String::as_str) != Some(request.request_id.as_str()) {
        return Ok(None);
    }
    if fields.get("agent_phase").map(String::as_str) != Some(phase.as_str()) {
        return Ok(None);
    }
    if fields.get("agent_status").map(String::as_str) != Some("submitted") {
        return Ok(None);
    }
    if !fields
        .get("agent_ready_for_review")
        .map(|value| parse_frontmatter_bool(value))
        .unwrap_or(false)
    {
        return Ok(None);
    }
    Ok(Some(path))
}

pub(crate) fn render_doc_status(request: &Request, phase: AgentPhase) -> Result<String> {
    let path = phase_document_path(request, phase);
    let content = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };
    let fields = frontmatter_fields(&content).unwrap_or_default();
    let value = |key: &str| fields.get(key).cloned().unwrap_or_default();
    Ok(format!(
        "request_id: {}\nartifact: {}\ndocument_type: {}\nagent_phase: {}\nagent_status: {}\nagent_ready_for_review: {}\nformat_check_status: {}\nformat_check_exit_code: {}\ngate_name: {}\ngate_status: {}\ngate_source: {}\ngate_body_sha256: {}\ngate_updated_at: {}\nupdated_at: {}\n",
        request.request_id,
        path.display(),
        value("document_type"),
        value("agent_phase"),
        value("agent_status"),
        value("agent_ready_for_review"),
        value("format_check_status"),
        value("format_check_exit_code"),
        value("gate_name"),
        value("gate_status"),
        value("gate_source"),
        value("gate_body_sha256"),
        value("gate_updated_at"),
        value("updated_at"),
    ))
}

pub(crate) fn write_document_gate_record(
    request: &Request,
    gate: &str,
    status: &str,
    by: &str,
    source: &str,
    comment: &str,
) -> Result<()> {
    let path = approval_artifact_path(request, gate);
    if !path.exists() {
        return Err(format!("gate artifact does not exist: {}", path.display()).into());
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    let current_fields = frontmatter_fields(&content).unwrap_or_default();
    let phase = gate_agent_phase(gate);
    let body_sha256 = markdown_body_sha256(&content)?;
    let updates = document_status_updates(
        request,
        phase,
        current_fields
            .get("agent_status")
            .map(String::as_str)
            .unwrap_or("draft"),
        current_fields
            .get("agent_ready_for_review")
            .map(|value| parse_frontmatter_bool(value))
            .unwrap_or(false),
        &current_fields,
        None,
        Some(GateStatusUpdate {
            gate,
            status,
            by,
            source,
            comment,
            body_sha256: &body_sha256,
        }),
    );
    fs::write(&path, replace_frontmatter(&content, &updates))?;
    Ok(())
}

pub(crate) fn render_document_gate_record(request: &Request, gate: &str) -> Result<Option<String>> {
    let path = approval_artifact_path(request, gate);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    let Some(fields) = frontmatter_fields(&content) else {
        return Ok(None);
    };
    if fields.get("gate_name").map(String::as_str) != Some(gate) {
        return Ok(None);
    }
    let status = fields.get("gate_status").cloned().unwrap_or_default();
    if status.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(format!(
        "{{ \"gate\": \"{}\", \"status\": \"{}\", \"artifact\": \"{}\", \"artifact_body_sha256\": \"{}\", \"by\": \"{}\", \"source\": \"{}\", \"comment\": \"{}\", \"updated_at\": \"{}\" }}",
        json_escape(gate),
        json_escape(&status),
        json_escape(&path.to_string_lossy()),
        json_escape(&fields.get("gate_body_sha256").cloned().unwrap_or_default()),
        json_escape(&fields.get("gate_by").cloned().unwrap_or_default()),
        json_escape(&fields.get("gate_source").cloned().unwrap_or_default()),
        json_escape(&fields.get("gate_comment").cloned().unwrap_or_default()),
        json_escape(&fields.get("gate_updated_at").cloned().unwrap_or_default()),
    )))
}

pub(crate) fn ensure_document_gate_approved(request: &Request, gate: &str) -> Result<Option<()>> {
    let path = approval_artifact_path(request, gate);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    let Some(fields) = frontmatter_fields(&content) else {
        return Ok(None);
    };
    if fields.get("gate_name").map(String::as_str) != Some(gate) {
        return Ok(None);
    }
    let status = fields.get("gate_status").cloned().unwrap_or_default();
    if status != "approved" {
        return Ok(None);
    }
    let approved_hash = fields.get("gate_body_sha256").cloned().unwrap_or_default();
    let current_hash = markdown_body_sha256(&content)?;
    if approved_hash != current_hash {
        return Err(format!(
            "{gate} gate approval is stale: approved document body hash does not match current artifact"
        )
        .into());
    }
    Ok(Some(()))
}

pub(crate) fn migrate_request_document_status(request: &Request, dry_run: bool) -> Result<bool> {
    let mut changed = false;
    for phase in request_document_status_phases(request) {
        let path = phase_document_path(request, phase);
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap_or_default();
        let (agent_status, ready_for_review) =
            inferred_agent_document_status(request, &path, phase);
        let updated = render_document_status_content(
            &content,
            request,
            phase,
            &agent_status,
            ready_for_review,
            inferred_format_status_update(request, phase),
        );
        if content == updated {
            continue;
        }
        changed = true;
        if dry_run {
            println!("Would normalize document status {}", path.display());
        } else {
            fs::write(&path, updated)?;
            println!("Normalized document status {}", path.display());
        }
    }
    if changed && !dry_run {
        refresh_gate_hashes_after_document_status_migration(request)?;
        write_status_json(
            request,
            stage_for_status_json(&request.status),
            &request.status,
            "upgrade refreshed document status frontmatter",
        )?;
    }
    Ok(changed)
}

pub(crate) fn remove_legacy_agent_success_markers(dry_run: bool) -> Result<bool> {
    let agents_dir = Path::new(".sandrone/state/agents");
    if !agents_dir.exists() {
        return Ok(false);
    }
    let mut removed = false;
    for entry in fs::read_dir(agents_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("success") {
            continue;
        }
        removed = true;
        if dry_run {
            println!(
                "Would remove obsolete agent success marker {}",
                path.display()
            );
        } else {
            fs::remove_file(&path)?;
            println!("Removed obsolete agent success marker {}", path.display());
        }
    }
    Ok(removed)
}

pub(crate) fn remove_obsolete_format_check_record(
    request: &Request,
    dry_run: bool,
) -> Result<bool> {
    let record_path = Path::new(&request.change_path).join("checks/format-check.md");
    if !record_path.exists() {
        return Ok(false);
    }
    if dry_run {
        println!(
            "Would remove obsolete format check record {}",
            record_path.display()
        );
    } else {
        fs::remove_file(&record_path)?;
        println!(
            "Removed obsolete format check record {}",
            record_path.display()
        );
        let checks_dir = record_path.parent().unwrap_or_else(|| Path::new(""));
        if checks_dir.exists()
            && fs::read_dir(checks_dir)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(false)
        {
            fs::remove_dir(checks_dir)?;
        }
    }
    Ok(true)
}

#[derive(Clone)]
struct FormatStatusUpdate {
    status: String,
    exit_code: String,
}

struct GateStatusUpdate<'a> {
    gate: &'a str,
    status: &'a str,
    by: &'a str,
    source: &'a str,
    comment: &'a str,
    body_sha256: &'a str,
}

fn upsert_document_status(
    path: &Path,
    request: &Request,
    phase: AgentPhase,
    agent_status: &str,
    ready_for_review: bool,
    format_update: Option<FormatStatusUpdate>,
) -> Result<()> {
    let content = fs::read_to_string(path).unwrap_or_default();
    let updated = render_document_status_content(
        &content,
        request,
        phase,
        agent_status,
        ready_for_review,
        format_update,
    );
    if content != updated {
        fs::write(path, updated)?;
    }
    Ok(())
}

fn render_document_status_content(
    content: &str,
    request: &Request,
    phase: AgentPhase,
    agent_status: &str,
    ready_for_review: bool,
    format_update: Option<FormatStatusUpdate>,
) -> String {
    let current_fields = frontmatter_fields(content).unwrap_or_default();
    let updates = document_status_updates(
        request,
        phase,
        agent_status,
        ready_for_review,
        &current_fields,
        format_update,
        None,
    );
    replace_frontmatter(content, &updates)
}

fn document_status_updates(
    request: &Request,
    phase: AgentPhase,
    agent_status: &str,
    ready_for_review: bool,
    current_fields: &BTreeMap<String, String>,
    format_update: Option<FormatStatusUpdate>,
    gate_update: Option<GateStatusUpdate<'_>>,
) -> Vec<(&'static str, String)> {
    let format_status = format_update
        .as_ref()
        .map(|update| update.status.clone())
        .or_else(|| current_fields.get("format_check_status").cloned())
        .unwrap_or_else(|| default_format_status(phase).to_string());
    let format_exit_code = format_update
        .as_ref()
        .map(|update| update.exit_code.clone())
        .or_else(|| current_fields.get("format_check_exit_code").cloned())
        .unwrap_or_default();
    let mut updates = vec![
        ("sandrone_schema", "1".to_string()),
        ("request_id", request.request_id.clone()),
        ("document_type", phase_document_type(phase).to_string()),
        ("agent_phase", phase.as_str().to_string()),
        ("agent_status", agent_status.to_string()),
        ("agent_ready_for_review", ready_for_review.to_string()),
        ("format_check_status", format_status),
        ("format_check_exit_code", format_exit_code),
    ];
    if let Some(gate_update) = gate_update {
        updates.extend([
            ("gate_name", gate_update.gate.to_string()),
            ("gate_status", gate_update.status.to_string()),
            ("gate_by", gate_update.by.to_string()),
            ("gate_source", gate_update.source.to_string()),
            ("gate_comment", gate_update.comment.to_string()),
            ("gate_body_sha256", gate_update.body_sha256.to_string()),
            ("gate_updated_at", now_string()),
        ]);
    } else {
        for key in GATE_STATUS_KEYS {
            if let Some(value) = current_fields.get(*key) {
                updates.push((key, value.clone()));
            }
        }
    }
    updates.push(("updated_at", now_string()));
    updates
}

fn gate_agent_phase(gate: &str) -> AgentPhase {
    match gate {
        "decomposition" => AgentPhase::Decomposition,
        "plan" => AgentPhase::Planning,
        "change-doc" => AgentPhase::Implementation,
        _ => AgentPhase::Implementation,
    }
}

fn request_document_status_phases(request: &Request) -> Vec<AgentPhase> {
    if is_parent_request(request) {
        vec![AgentPhase::Decomposition]
    } else if matches!(
        request.status.as_str(),
        "rebase-agent-running"
            | "integration-review-submitted"
            | "integration-review-running"
            | "integration-review-rejected"
    ) {
        vec![AgentPhase::Planning, AgentPhase::Rebase]
    } else {
        vec![AgentPhase::Planning, AgentPhase::Implementation]
    }
}

fn inferred_agent_document_status(
    request: &Request,
    path: &Path,
    phase: AgentPhase,
) -> (String, bool) {
    let existing_status = document_status_value(path, "agent_status").unwrap_or_default();
    let existing_ready = document_status_value(path, "agent_ready_for_review")
        .as_deref()
        .map(parse_frontmatter_bool)
        .unwrap_or(false);
    if existing_status == "submitted" && existing_ready {
        return ("submitted".to_string(), true);
    }
    if phase_document_is_currently_submitted_or_approved(request, phase) {
        ("submitted".to_string(), true)
    } else {
        ("draft".to_string(), false)
    }
}

fn phase_document_is_currently_submitted_or_approved(request: &Request, phase: AgentPhase) -> bool {
    match phase {
        AgentPhase::Decomposition => {
            matches!(
                request.status.as_str(),
                "decomposition-submitted" | "decomposition-review-running"
            ) || ensure_gate_approved(request, "decomposition").is_ok()
        }
        AgentPhase::Planning => {
            matches!(
                request.status.as_str(),
                "plan-submitted" | "plan-review-running" | "change-doc-submitted"
            ) || ensure_gate_approved(request, "plan").is_ok()
        }
        AgentPhase::Implementation => {
            matches!(
                canonical_status(&request.status),
                "change-doc-submitted"
                    | "code-review-running"
                    | "change-doc-approved"
                    | STATUS_SLICE_FINISHED
                    | STATUS_WAIT_UPDATE_PR
                    | STATUS_WAIT_FINISH
                    | STATUS_FINISHED
            ) || ensure_gate_approved(request, "change-doc").is_ok()
        }
        AgentPhase::Rebase => {
            matches!(
                canonical_status(&request.status),
                "integration-review-submitted"
                    | "integration-review-running"
                    | STATUS_WAIT_UPDATE_PR
                    | STATUS_WAIT_FINISH
                    | STATUS_FINISHED
            )
        }
    }
}

fn inferred_format_status_update(
    request: &Request,
    phase: AgentPhase,
) -> Option<FormatStatusUpdate> {
    if !matches!(phase, AgentPhase::Implementation | AgentPhase::Rebase) {
        return None;
    }
    let record_path = Path::new(&request.change_path).join("checks/format-check.md");
    if !record_path.exists() {
        return None;
    }
    let content = fs::read_to_string(&record_path).ok()?;
    let status = markdown_backtick_value(&content, "- Status:")
        .or_else(|| markdown_inline_value(&content, "- Status:"))
        .unwrap_or_else(|| "recorded".to_string());
    let exit_code = markdown_backtick_value(&content, "- exit code:")
        .or_else(|| markdown_inline_value(&content, "- exit code:"))
        .unwrap_or_default();
    Some(FormatStatusUpdate { status, exit_code })
}

fn markdown_backtick_value(content: &str, prefix: &str) -> Option<String> {
    let line = content
        .lines()
        .find(|line| line.trim_start().starts_with(prefix))?;
    let (_, rest) = line.split_once('`')?;
    let (value, _) = rest.split_once('`')?;
    Some(value.trim().to_string()).filter(|value| !value.is_empty())
}

fn markdown_inline_value(content: &str, prefix: &str) -> Option<String> {
    content
        .lines()
        .find_map(|line| line.trim_start().strip_prefix(prefix))
        .map(str::trim)
        .map(|value| value.trim_matches('`').to_string())
        .filter(|value| !value.is_empty())
}

fn refresh_gate_hashes_after_document_status_migration(request: &Request) -> Result<()> {
    for gate in ["decomposition", "plan", "change-doc"] {
        let artifact = approval_artifact_path(request, gate);
        if !artifact.exists() {
            continue;
        }
        let record = render_gate_record_json(request, gate)?;
        let status = json_value(&record, "status").unwrap_or_default();
        if status.is_empty() || status == "missing" {
            continue;
        }
        let by = json_value(&record, "by").unwrap_or_default();
        let source = json_value(&record, "source").unwrap_or_default();
        let comment = json_value(&record, "comment").unwrap_or_default();
        write_approval_record(
            request,
            gate,
            &status,
            fallback_empty(&by, "document-status-migration"),
            fallback_empty(&source, "document-status-migration"),
            fallback_empty(
                &comment,
                "refreshed gate hash after document status frontmatter migration",
            ),
        )?;
    }
    Ok(())
}

fn markdown_body_sha256(content: &str) -> Result<String> {
    sha256_bytes(markdown_body_for_hash(content).as_bytes())
}

fn markdown_body_for_hash(content: &str) -> &str {
    split_frontmatter(content)
        .map(|(_, body)| body.trim_start())
        .unwrap_or_else(|| content.trim_start())
}

fn default_format_status(phase: AgentPhase) -> &'static str {
    match phase {
        AgentPhase::Implementation | AgentPhase::Rebase => "pending",
        AgentPhase::Decomposition | AgentPhase::Planning => "not-applicable",
    }
}

fn document_status_value(path: &Path, key: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| frontmatter_fields(&content))
        .and_then(|fields| fields.get(key).cloned())
}

fn frontmatter_fields(content: &str) -> Option<BTreeMap<String, String>> {
    let (frontmatter, _) = split_frontmatter(content)?;
    let mut fields = BTreeMap::new();
    for line in frontmatter.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        fields.insert(key.trim().to_string(), unquote_yaml_value(value.trim()));
    }
    Some(fields)
}

fn replace_frontmatter(content: &str, updates: &[(&str, String)]) -> String {
    let (frontmatter, body) = split_frontmatter(content).unwrap_or(("", content));
    let mut lines = Vec::new();
    for line in frontmatter.lines() {
        let key = line.split_once(':').map(|(key, _)| key.trim());
        if key.is_some_and(|key| DOC_STATUS_KEYS.contains(&key)) {
            continue;
        }
        if !line.trim().is_empty() {
            lines.push(line.to_string());
        }
    }
    for (key, value) in updates {
        lines.push(format!("{key}: {}", render_yaml_value(value)));
    }
    format!("---\n{}\n---\n\n{}", lines.join("\n"), body.trim_start())
}

fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let rest = content.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    let frontmatter = &rest[..end];
    let body = &rest[end + "\n---\n".len()..];
    Some((frontmatter, body))
}

fn render_yaml_value(value: &str) -> String {
    if value.is_empty() {
        "\"\"".to_string()
    } else {
        value.to_string()
    }
}

fn unquote_yaml_value(value: &str) -> String {
    value.trim_matches('"').to_string()
}

fn parse_frontmatter_bool(value: &str) -> bool {
    matches!(value.trim(), "true" | "yes")
}
