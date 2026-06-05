use super::*;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

pub(crate) const OBSIDIAN_CHANGE_ROOT: &str = "obsidian/changes";
pub(crate) const OBSIDIAN_PROJECT_NOTE: &str = "obsidian/project.md";
pub(crate) const OBSIDIAN_RELATIONS_NOTE: &str = "obsidian/relations.md";
pub(crate) const OBSIDIAN_PROJECT_CANVAS: &str = "obsidian/project.canvas";
pub(crate) const OBSIDIAN_DERIVED_ROOT: &str = "obsidian/derived";
const REQUEST_NAMED_MARKDOWN_ARTIFACTS: &[&str] = &[
    "request.md",
    "decomposition.md",
    "plan.md",
    "change-doc.md",
    "agent-journal.md",
    "pr-doc.md",
    "recovery.md",
];
const PARENT_DECOMPOSITION_OBSOLETE_ARTIFACTS: &[&str] = &["plan.md", "change-doc.md"];
const SLICE_OBSOLETE_ARTIFACTS: &[&str] = &["request.md", "decomposition.md", "pr-doc.md"];

pub(crate) fn change_artifact_path(change_name: &str) -> String {
    Path::new(OBSIDIAN_CHANGE_ROOT)
        .join(change_name)
        .to_string_lossy()
        .to_string()
}

pub(crate) fn request_artifact_file_name(request: &Request, legacy_file: &str) -> String {
    let Some(file_name) = Path::new(legacy_file)
        .file_name()
        .and_then(|name| name.to_str())
    else {
        return legacy_file.to_string();
    };
    if !REQUEST_NAMED_MARKDOWN_ARTIFACTS.contains(&file_name) {
        return legacy_file.to_string();
    }
    let parent = Path::new(legacy_file).parent().and_then(|path| {
        if path.as_os_str().is_empty() {
            None
        } else {
            Some(path)
        }
    });
    let named_file = format!("{} {}", request.request_id, file_name);
    parent
        .map(|path| path.join(&named_file).to_string_lossy().to_string())
        .unwrap_or(named_file)
}

pub(crate) fn request_artifact_file_stem(request: &Request, legacy_file: &str) -> String {
    request_artifact_file_name(request, legacy_file)
        .trim_end_matches(".md")
        .to_string()
}

pub(crate) fn request_artifact_path_buf(request: &Request, legacy_file: &str) -> PathBuf {
    if request.change_path.trim().is_empty() {
        return PathBuf::new();
    }
    Path::new(&request.change_path).join(request_artifact_file_name(request, legacy_file))
}

pub(crate) fn existing_or_preferred_request_artifact_path(
    request: &Request,
    legacy_file: &str,
) -> PathBuf {
    if request.change_path.trim().is_empty() {
        return PathBuf::new();
    }
    let preferred = request_artifact_path_buf(request, legacy_file);
    if preferred.exists() {
        return preferred;
    }
    let legacy = Path::new(&request.change_path).join(legacy_file);
    if legacy.exists() { legacy } else { preferred }
}

pub(crate) fn request_artifact_path_string(request: &Request, legacy_file: &str) -> String {
    if request.change_path.trim().is_empty() {
        String::new()
    } else {
        existing_or_preferred_request_artifact_path(request, legacy_file)
            .to_string_lossy()
            .to_string()
    }
}

pub(crate) fn request_generates_markdown_artifact(request: &Request, legacy_file: &str) -> bool {
    match legacy_file {
        "request.md" | "decomposition.md" => is_parent_request(request),
        "plan.md" | "change-doc.md" => is_slice_request(request),
        "pr-doc.md" => is_parent_request(request),
        "agent-journal.md" | "recovery.md" => true,
        _ => true,
    }
}

pub(crate) fn request_required_runtime_artifacts(request: &Request) -> Vec<&'static str> {
    if is_slice_request(request) {
        vec![
            "plan.md",
            "change-doc.md",
            "agent-journal.md",
            "status.json",
        ]
    } else {
        vec![
            "request.md",
            "decomposition.md",
            "agent-journal.md",
            "pr-doc.md",
            "status.json",
        ]
    }
}

pub(crate) fn request_handoff_artifact_path_string(request: &Request, legacy_file: &str) -> String {
    match legacy_file {
        "request.md" if is_slice_request(request) => {
            request_artifact_path_string(request, "plan.md")
        }
        "decomposition.md" | "decomposition.json" | "dag.json" if is_slice_request(request) => {
            parent_artifact_path_for_slice(request, legacy_file)
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_default()
        }
        "plan.md" | "change-doc.md" if is_parent_request(request) => {
            let existing = existing_or_preferred_request_artifact_path(request, legacy_file);
            if existing.exists() {
                existing.to_string_lossy().to_string()
            } else {
                String::new()
            }
        }
        "pr-doc.md" if is_slice_request(request) => String::new(),
        "decomposition.json" | "dag.json" => {
            if request.change_path.trim().is_empty() {
                String::new()
            } else {
                Path::new(&request.change_path)
                    .join(legacy_file)
                    .to_string_lossy()
                    .to_string()
            }
        }
        _ => request_artifact_path_string(request, legacy_file),
    }
}

pub(crate) fn review_context_artifact_source(request: &Request, artifact: &str) -> PathBuf {
    if is_slice_request(request) {
        match artifact {
            "request.md" => {
                return existing_or_preferred_request_artifact_path(request, "plan.md");
            }
            "decomposition.md" | "decomposition.json" | "dag.json" => {
                return parent_artifact_path_for_slice(request, artifact).unwrap_or_default();
            }
            "pr-doc.md" => return PathBuf::new(),
            _ => {}
        }
    } else if matches!(artifact, "plan.md" | "change-doc.md") {
        let source = existing_or_preferred_request_artifact_path(request, artifact);
        if !source.exists() {
            return PathBuf::new();
        }
        return source;
    }
    existing_or_preferred_request_artifact_path(request, artifact)
}

pub(crate) fn parent_artifact_path_for_slice(
    request: &Request,
    legacy_file: &str,
) -> Option<PathBuf> {
    if !is_slice_request(request) || request.change_path.trim().is_empty() {
        return None;
    }
    let slice_meta_path = Path::new(&request.change_path).join("slice.json");
    let content = fs::read_to_string(slice_meta_path).ok()?;
    let parent_change_path = json_value(&content, "parent_change_path")?;
    if parent_change_path.trim().is_empty() {
        return None;
    }
    if matches!(legacy_file, "decomposition.json" | "dag.json") {
        return Some(Path::new(&parent_change_path).join(legacy_file));
    }
    let parent_request_id = json_value(&content, "parent_request_id")?;
    let preferred =
        Path::new(&parent_change_path).join(format!("{parent_request_id} {legacy_file}"));
    if preferred.exists() {
        return Some(preferred);
    }
    Some(Path::new(&parent_change_path).join(legacy_file))
}

pub(crate) fn parent_artifact_wikilink_for_slice(
    request: &Request,
    legacy_file: &str,
    label: &str,
) -> Option<String> {
    if !is_slice_request(request) || request.change_path.trim().is_empty() {
        return None;
    }
    let slice_meta_path = Path::new(&request.change_path).join("slice.json");
    let content = fs::read_to_string(slice_meta_path).ok()?;
    let parent_request_id = json_value(&content, "parent_request_id")?;
    if parent_request_id.trim().is_empty() {
        return None;
    }
    let file_stem = Path::new(legacy_file)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.trim_end_matches(".md"))
        .unwrap_or(legacy_file);
    Some(format!(
        "[[{} {}|{}]]",
        parent_request_id,
        file_stem,
        markdown_link_label(label)
    ))
}

pub(crate) fn request_artifact_wikilink(
    request: &Request,
    legacy_file: &str,
    label: &str,
) -> String {
    format!(
        "[[{}|{}]]",
        request_artifact_file_stem(request, legacy_file),
        markdown_link_label(label)
    )
}

pub(crate) fn request_artifact_markdown_link(
    request: &Request,
    legacy_file: &str,
    label: &str,
) -> String {
    let file = request_artifact_file_name(request, legacy_file);
    format!(
        "[{}]({})",
        markdown_link_label(label),
        file.replace(' ', "%20")
    )
}

pub(crate) fn ensure_prefixed_change_artifact_names(
    request: &Request,
    dry_run: bool,
) -> Result<()> {
    if request.change_path.trim().is_empty() {
        return Ok(());
    }
    for legacy_file in REQUEST_NAMED_MARKDOWN_ARTIFACTS {
        let legacy_path = Path::new(&request.change_path).join(legacy_file);
        let preferred_path = request_artifact_path_buf(request, legacy_file);
        if legacy_path == preferred_path || !legacy_path.exists() {
            continue;
        }
        if preferred_path.exists() {
            remove_change_artifact(
                request,
                &legacy_path,
                dry_run,
                "duplicate legacy short-name artifact",
            )?;
            continue;
        }
        if dry_run {
            println!(
                "Would rename {} -> {}",
                legacy_path.display(),
                preferred_path.display()
            );
        } else {
            fs::rename(&legacy_path, &preferred_path)?;
            println!(
                "Renamed {} -> {}",
                legacy_path.display(),
                preferred_path.display()
            );
        }
    }
    Ok(())
}

pub(crate) fn remove_obsolete_change_artifacts(request: &Request, dry_run: bool) -> Result<()> {
    if request.change_path.trim().is_empty() {
        return Ok(());
    }
    remove_legacy_archive_entries(request, dry_run)?;
    let mut seen = BTreeSet::new();
    for legacy_file in obsolete_markdown_artifacts(request) {
        for candidate in [
            request_artifact_path_buf(request, legacy_file),
            Path::new(&request.change_path).join(legacy_file),
        ] {
            if seen.insert(candidate.clone()) && candidate.exists() {
                remove_change_artifact(
                    request,
                    &candidate,
                    dry_run,
                    "artifact is obsolete for the current request/slice model",
                )?;
            }
        }
    }
    Ok(())
}

fn remove_legacy_archive_entries(request: &Request, dry_run: bool) -> Result<()> {
    let change_path = Path::new(&request.change_path);
    if !change_path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(change_path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with("archived-") {
            remove_change_artifact(
                request,
                &entry.path(),
                dry_run,
                "legacy archive entry is no longer kept",
            )?;
        }
    }
    Ok(())
}

fn obsolete_markdown_artifacts(request: &Request) -> &'static [&'static str] {
    if is_slice_request(request) {
        SLICE_OBSOLETE_ARTIFACTS
    } else if parent_uses_decomposition_model(request) {
        PARENT_DECOMPOSITION_OBSOLETE_ARTIFACTS
    } else {
        &[]
    }
}

fn parent_uses_decomposition_model(request: &Request) -> bool {
    if !is_parent_request(request) || request.change_path.trim().is_empty() {
        return false;
    }
    let change_path = Path::new(&request.change_path);
    change_path.join(".decomposition-kind").exists()
        || change_path.join("decomposition.json").exists()
        || existing_or_preferred_request_artifact_path(request, "decomposition.md").exists()
}

fn remove_change_artifact(
    request: &Request,
    source_path: &Path,
    dry_run: bool,
    reason: &str,
) -> Result<()> {
    if !source_path.starts_with(&request.change_path) {
        return Ok(());
    }
    if dry_run {
        println!("Would remove {} ({})", source_path.display(), reason);
    } else if source_path.is_dir() {
        fs::remove_dir_all(source_path)?;
        println!("Removed {} ({})", source_path.display(), reason);
    } else {
        fs::remove_file(source_path)?;
        println!("Removed {} ({})", source_path.display(), reason);
    }
    Ok(())
}

pub(crate) fn ensure_obsidian_vault_dirs() -> Result<()> {
    fs::create_dir_all(".obsidian")?;
    fs::create_dir_all(OBSIDIAN_CHANGE_ROOT)?;
    fs::create_dir_all("obsidian/codegraph")?;
    fs::create_dir_all("obsidian/projects")?;
    fs::create_dir_all("obsidian/views")?;
    fs::create_dir_all(OBSIDIAN_DERIVED_ROOT)?;
    ensure_obsidian_vault_metadata()
}

pub(crate) fn sync_obsidian_request_note(request: &Request) -> Result<()> {
    if request.change_path.trim().is_empty() {
        return Ok(());
    }
    ensure_prefixed_change_artifact_names(request, false)?;
    fs::create_dir_all("obsidian/changes")?;
    let kind = if is_slice_request(request) {
        "slice"
    } else {
        "request"
    };
    let note_path = obsidian_request_note_path(request);
    if let Some(parent) = note_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut rendered = render_obsidian_request_note(request, kind);
    if let Ok(existing) = fs::read_to_string(&note_path) {
        rendered = preserve_obsidian_note_body(&rendered, &existing);
    }
    fs::write(note_path, rendered)?;
    Ok(())
}

pub(crate) fn sync_obsidian_project_note(requests: &[Request]) -> Result<()> {
    ensure_obsidian_vault_dirs()?;
    fs::write(
        OBSIDIAN_PROJECT_NOTE,
        render_obsidian_project_note(requests),
    )?;
    ensure_obsidian_relations_note()?;
    write_obsidian_base_views()?;
    write_obsidian_derived_json(requests)?;
    fs::write(
        OBSIDIAN_PROJECT_CANVAS,
        render_obsidian_project_canvas(requests),
    )?;
    Ok(())
}

pub(crate) fn refresh_obsidian_artifacts(requests: &[Request]) -> Result<()> {
    ensure_obsidian_vault_dirs()?;
    for request in requests {
        if !request.change_path.trim().is_empty() {
            ensure_prefixed_change_artifact_names(request, false)?;
            sync_obsidian_request_note(request)?;
        }
    }
    sync_obsidian_project_note(requests)?;
    migrate_obsidian_legacy_navigation_links()
}

pub(crate) fn obsidian_request_note_path(request: &Request) -> PathBuf {
    let change_path = request.change_path.trim();
    if !change_path.is_empty() && change_path.starts_with(OBSIDIAN_CHANGE_ROOT) {
        Path::new(change_path).join(format!("{} index.md", request.request_id))
    } else {
        Path::new(OBSIDIAN_CHANGE_ROOT).join(format!("{}.md", request.request_id))
    }
}

fn render_obsidian_project_note(requests: &[Request]) -> String {
    let metadata = project_metadata();
    render_obsidian_template(
        assets::OBSIDIAN_PROJECT_TEMPLATE,
        &[
            (
                "project_name",
                fallback_empty(&metadata.repo_name, "Sandrone Project").to_string(),
            ),
            (
                "repo_name",
                fallback_empty(&metadata.repo_name, "unknown").to_string(),
            ),
            (
                "git_url",
                fallback_empty(&metadata.git_url, "unknown").to_string(),
            ),
            (
                "base_branch",
                fallback_empty(&metadata.base_branch, "unknown").to_string(),
            ),
            ("status_summary", render_project_status_summary(requests)),
            ("request_index", render_project_request_index(requests)),
            ("updated_at", now_string()),
        ],
    )
}

#[derive(Default)]
struct ProjectMetadata {
    repo_name: String,
    git_url: String,
    base_branch: String,
}

fn project_metadata() -> ProjectMetadata {
    let mut metadata = ProjectMetadata {
        base_branch: "unknown".to_string(),
        ..ProjectMetadata::default()
    };
    let Ok(content) = fs::read_to_string(CONFIG_PATH) else {
        return metadata;
    };
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_string();
        match key.trim() {
            "repo_name" => metadata.repo_name = value,
            "git_url" => metadata.git_url = value,
            "base_branch" => metadata.base_branch = value,
            _ => {}
        }
    }
    metadata
}

fn render_project_status_summary(requests: &[Request]) -> String {
    let parent_requests = requests
        .iter()
        .filter(|request| is_parent_request(request))
        .collect::<Vec<_>>();
    if parent_requests.is_empty() {
        return "- 暂无需求。".to_string();
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for request in parent_requests {
        *counts.entry(request.status.as_str()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(status, count)| format!("- `{status}`: {count}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_project_request_index(requests: &[Request]) -> String {
    let parent_requests = requests
        .iter()
        .filter(|request| is_parent_request(request))
        .collect::<Vec<_>>();
    if parent_requests.is_empty() {
        return "暂无需求。".to_string();
    }
    let mut grouped: BTreeMap<String, Vec<&Request>> = BTreeMap::new();
    for request in parent_requests {
        grouped
            .entry(request_group_date(request))
            .or_default()
            .push(request);
    }
    let mut rendered = Vec::new();
    for (date, mut group) in grouped.into_iter().rev() {
        group.sort_by(|left, right| left.request_id.cmp(&right.request_id));
        rendered.push(format!("### {date}"));
        for request in group {
            rendered.push(format!(
                "- {} — `{}` — source `{}` — updated `{}`",
                project_request_link(request),
                request.status,
                fallback_empty(&request.source, "unknown"),
                fallback_empty(&request.updated_at, "unknown"),
            ));
        }
        rendered.push(String::new());
    }
    rendered.join("\n").trim_end().to_string()
}

fn request_group_date(request: &Request) -> String {
    first_valid_date(&request.created_at)
        .or_else(|| first_valid_date(&request.change_name))
        .or_else(|| first_valid_date(&request.updated_at))
        .unwrap_or_else(|| "unknown-date".to_string())
}

fn first_valid_date(value: &str) -> Option<String> {
    let candidate = value.get(..10)?;
    let bytes = candidate.as_bytes();
    let valid = bytes.len() == 10
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && candidate
            .chars()
            .enumerate()
            .all(|(index, ch)| index == 4 || index == 7 || ch.is_ascii_digit());
    valid.then(|| candidate.to_string())
}

fn project_request_link(request: &Request) -> String {
    let label = markdown_link_label(&format!("{} {}", request.request_id, request.title));
    if request.change_path.trim().is_empty() {
        return format!(
            "`{}` {}",
            request.request_id,
            markdown_inline(&request.title)
        );
    }
    let note_path = obsidian_request_note_path(request);
    if let Ok(relative) = note_path.strip_prefix("obsidian") {
        return format!("[[{}|{}]]", relative.with_extension("").display(), label);
    }
    format!(
        "[[{}/{} index|{}]]",
        request.change_path.trim_start_matches("obsidian/"),
        request.request_id,
        label
    )
}

fn markdown_link_label(value: &str) -> String {
    markdown_inline(value)
        .replace('|', r"\|")
        .replace(']', r"\]")
}

fn ensure_obsidian_relations_note() -> Result<()> {
    if Path::new(OBSIDIAN_RELATIONS_NOTE).exists() {
        let content = fs::read_to_string(OBSIDIAN_RELATIONS_NOTE)?;
        let migrated = content.replace(
            "1. 先读 [[project|project.md]] 和本文件。",
            "1. 先读 `obsidian/project.md` 和本文件。",
        );
        if migrated != content {
            fs::write(OBSIDIAN_RELATIONS_NOTE, migrated)?;
        }
        return Ok(());
    }
    fs::write(
        OBSIDIAN_RELATIONS_NOTE,
        render_obsidian_template(
            assets::OBSIDIAN_RELATIONS_TEMPLATE,
            &[("updated_at", now_string())],
        ),
    )?;
    Ok(())
}

fn migrate_obsidian_legacy_navigation_links() -> Result<()> {
    migrate_obsidian_markdown_links_in_dir(Path::new(OBSIDIAN_CHANGE_ROOT))
}

fn migrate_obsidian_markdown_links_in_dir(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            migrate_obsidian_markdown_links_in_dir(&path)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
            migrate_obsidian_markdown_file(&path)?;
        }
    }
    Ok(())
}

fn migrate_obsidian_markdown_file(path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let migrated = migrate_obsidian_legacy_navigation_content(&content);
    if migrated != content {
        fs::write(path, migrated)?;
    }
    Ok(())
}

fn migrate_obsidian_legacy_navigation_content(content: &str) -> String {
    content
        .replace(
            "- Project root: [[project|project.md]]",
            "- 上级索引: 请从当前 request/slice index 进入本文档，保持 Obsidian 主链路清晰。",
        )
        .replace(
            "- Relations: [[relations|relations.md]]",
            "- Relations: `obsidian/relations.md`",
        )
        .replace(
            "- Project Relations: [[relations|relations.md]]",
            "- Project Relations: `obsidian/relations.md`",
        )
}

fn write_obsidian_base_views() -> Result<()> {
    fs::write(
        "obsidian/views/requests.base",
        assets::OBSIDIAN_REQUESTS_BASE_TEMPLATE,
    )?;
    fs::write(
        "obsidian/views/slices.base",
        assets::OBSIDIAN_SLICES_BASE_TEMPLATE,
    )?;
    Ok(())
}

fn write_obsidian_derived_json(requests: &[Request]) -> Result<()> {
    fs::write(
        "obsidian/derived/requests.json",
        render_derived_requests_json(requests),
    )?;
    fs::write(
        "obsidian/derived/slices.json",
        render_derived_slices_json(requests),
    )?;
    Ok(())
}

fn render_derived_requests_json(requests: &[Request]) -> String {
    let items = requests
        .iter()
        .filter(|request| is_parent_request(request))
        .map(|request| {
            format!(
                "    {{\"request_id\":\"{}\",\"status\":\"{}\",\"title\":\"{}\",\"source\":\"{}\",\"external_id\":\"{}\",\"change_path\":\"{}\",\"note\":\"{}\",\"branch\":\"{}\",\"worktree\":\"{}\",\"created_at\":\"{}\",\"updated_at\":\"{}\"}}",
                json_escape(&request.request_id),
                json_escape(&request.status),
                json_escape(&request.title),
                json_escape(&request.source),
                json_escape(&request.external_id),
                json_escape(&request.change_path),
                json_escape(&obsidian_canvas_file_path(request)),
                json_escape(&request.branch),
                json_escape(&request.worktree_path),
                json_escape(&request.created_at),
                json_escape(&request.updated_at),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema_version\": 1,\n  \"updated_at\": \"{}\",\n  \"requests\": [\n{}\n  ]\n}}\n",
        json_escape(&now_string()),
        items
    )
}

fn render_derived_slices_json(requests: &[Request]) -> String {
    let mut items = Vec::new();
    for parent in requests.iter().filter(|request| is_parent_request(request)) {
        if !Path::new(&parent.change_path)
            .join("decomposition.json")
            .exists()
        {
            continue;
        }
        let definitions = load_slice_definitions(parent).unwrap_or_default();
        for definition in definitions {
            let request_id = slice_request_id(&parent.request_id, &definition.id);
            let materialized = requests
                .iter()
                .find(|request| request.request_id == request_id);
            let (status, change_path, note, branch, worktree) = if let Some(slice) = materialized {
                (
                    slice.status.as_str(),
                    slice.change_path.as_str(),
                    obsidian_canvas_file_path(slice),
                    slice.branch.as_str(),
                    slice.worktree_path.as_str(),
                )
            } else {
                ("planned", "", String::new(), "", "")
            };
            items.push(format!(
                "    {{\"parent_request_id\":\"{}\",\"slice_request_id\":\"{}\",\"slice_id\":\"{}\",\"name\":\"{}\",\"summary\":\"{}\",\"depends_on\":[{}],\"status\":\"{}\",\"change_path\":\"{}\",\"note\":\"{}\",\"branch\":\"{}\",\"worktree\":\"{}\"}}",
                json_escape(&parent.request_id),
                json_escape(&request_id),
                json_escape(&definition.id),
                json_escape(&definition.name),
                json_escape(&short_json_summary(&definition.summary)),
                definition
                    .depends_on
                    .iter()
                    .map(|dependency| format!("\"{}\"", json_escape(dependency)))
                    .collect::<Vec<_>>()
                    .join(","),
                json_escape(status),
                json_escape(change_path),
                json_escape(&note),
                json_escape(branch),
                json_escape(worktree),
            ));
        }
    }
    format!(
        "{{\n  \"schema_version\": 1,\n  \"updated_at\": \"{}\",\n  \"slices\": [\n{}\n  ]\n}}\n",
        json_escape(&now_string()),
        items.join(",\n")
    )
}

fn render_obsidian_project_canvas(requests: &[Request]) -> String {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let project_id = stable_canvas_id("project");
    nodes.push(format!(
        "{{\"id\":\"{}\",\"type\":\"file\",\"file\":\"project.md\",\"x\":0,\"y\":0,\"width\":360,\"height\":180,\"color\":\"5\"}}",
        project_id
    ));
    for (index, request) in requests
        .iter()
        .filter(|request| !request.change_path.trim().is_empty() && is_parent_request(request))
        .enumerate()
    {
        let node_id = stable_canvas_id(&request.request_id);
        let file_path = obsidian_canvas_file_path(request);
        let y = (index as i32) * 220 - 120;
        nodes.push(format!(
            "{{\"id\":\"{}\",\"type\":\"file\",\"file\":\"{}\",\"x\":520,\"y\":{},\"width\":460,\"height\":180,\"color\":\"{}\"}}",
            node_id,
            json_escape(&file_path),
            y,
            canvas_color_for_status(&request.status)
        ));
        edges.push(format!(
            "{{\"id\":\"{}\",\"fromNode\":\"{}\",\"fromSide\":\"right\",\"toNode\":\"{}\",\"toSide\":\"left\",\"toEnd\":\"arrow\",\"label\":\"{}\"}}",
            stable_canvas_id(&format!("project->{}", request.request_id)),
            project_id,
            node_id,
            json_escape(&request.status)
        ));
        if Path::new(&request.change_path)
            .join("decomposition.json")
            .exists()
        {
            add_slice_canvas_nodes(request, requests, index, &mut nodes, &mut edges);
        }
    }
    format!(
        "{{\n  \"nodes\": [\n    {}\n  ],\n  \"edges\": [\n    {}\n  ]\n}}\n",
        nodes.join(",\n    "),
        edges.join(",\n    ")
    )
}

fn add_slice_canvas_nodes(
    parent: &Request,
    requests: &[Request],
    parent_index: usize,
    nodes: &mut Vec<String>,
    edges: &mut Vec<String>,
) {
    let Ok(definitions) = load_slice_definitions(parent) else {
        return;
    };
    let definition_ids = definitions
        .iter()
        .map(|definition| definition.id.as_str())
        .collect::<BTreeSet<_>>();
    let parent_id = stable_canvas_id(&parent.request_id);
    let base_y = (parent_index as i32) * 220 - 120;
    for (slice_index, definition) in definitions.iter().enumerate() {
        let request_id = slice_request_id(&parent.request_id, &definition.id);
        let node_id = stable_canvas_id(&request_id);
        let materialized = requests
            .iter()
            .find(|request| request.request_id == request_id);
        let y = base_y + (slice_index as i32) * 150;
        if let Some(slice) = materialized {
            nodes.push(format!(
                "{{\"id\":\"{}\",\"type\":\"file\",\"file\":\"{}\",\"x\":1100,\"y\":{},\"width\":420,\"height\":130,\"color\":\"{}\"}}",
                node_id,
                json_escape(&obsidian_canvas_file_path(slice)),
                y,
                canvas_color_for_status(&slice.status)
            ));
        } else {
            nodes.push(format!(
                "{{\"id\":\"{}\",\"type\":\"text\",\"text\":\"{}\",\"x\":1100,\"y\":{},\"width\":420,\"height\":130,\"color\":\"6\"}}",
                node_id,
                json_escape(&format!(
                    "# {} {}\n\n{}",
                    definition.id,
                    definition.name,
                    short_json_summary(&definition.summary)
                )),
                y
            ));
        }
        edges.push(format!(
            "{{\"id\":\"{}\",\"fromNode\":\"{}\",\"fromSide\":\"right\",\"toNode\":\"{}\",\"toSide\":\"left\",\"toEnd\":\"arrow\",\"label\":\"slice\"}}",
            stable_canvas_id(&format!("{}->{}", parent.request_id, request_id)),
            parent_id,
            node_id
        ));
        for dependency in &definition.depends_on {
            if !definition_ids.contains(dependency.as_str()) {
                continue;
            }
            let dependency_id = stable_canvas_id(&slice_request_id(&parent.request_id, dependency));
            edges.push(format!(
                "{{\"id\":\"{}\",\"fromNode\":\"{}\",\"fromSide\":\"right\",\"toNode\":\"{}\",\"toSide\":\"left\",\"toEnd\":\"arrow\",\"label\":\"depends-on\"}}",
                stable_canvas_id(&format!("{}->{}", dependency_id, node_id)),
                dependency_id,
                node_id
            ));
        }
    }
}

fn obsidian_canvas_file_path(request: &Request) -> String {
    obsidian_request_note_path(request)
        .strip_prefix("obsidian")
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| {
            obsidian_request_note_path(request)
                .to_string_lossy()
                .to_string()
        })
}

fn stable_canvas_id(input: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn canvas_color_for_status(status: &str) -> &'static str {
    match canonical_status(status) {
        "blocked" => "1",
        "finished" => "4",
        "wait-finish" | "wait-update-pr" => "3",
        "decomposition-agent-running"
        | "planning-agent-running"
        | "implementation-agent-running" => "5",
        _ => "6",
    }
}

fn short_json_summary(value: &str) -> String {
    let summary = markdown_inline(value);
    let mut shortened = summary.chars().take(240).collect::<String>();
    if summary.chars().count() > 240 {
        shortened.push_str("...");
    }
    shortened
}

fn ensure_obsidian_vault_metadata() -> Result<()> {
    let metadata_path = Path::new(".obsidian/Sandrone.json");
    if metadata_path.exists() {
        return Ok(());
    }
    fs::write(
        metadata_path,
        "{\n  \"schema_version\": 1,\n  \"managed_by\": \"Sandrone\",\n  \"notes_root\": \"obsidian\"\n}\n",
    )?;
    Ok(())
}

fn render_obsidian_request_note(request: &Request, kind: &str) -> String {
    let note_path = obsidian_request_note_path(request);
    render_obsidian_template(
        assets::OBSIDIAN_CHANGE_TEMPLATE,
        &[
            ("request_id", request.request_id.clone()),
            ("title", request.title.clone()),
            ("kind", kind.to_string()),
            ("status", request.status.clone()),
            ("external_id", request.external_id.clone()),
            ("source", request.source.clone()),
            ("url", fallback_empty(&request.url, "n/a").to_string()),
            ("change_path", request.change_path.clone()),
            (
                "branch",
                fallback_empty(&request.branch, "not-started").to_string(),
            ),
            (
                "worktree",
                fallback_empty(&request.worktree_path, "not-started").to_string(),
            ),
            ("artifact_prefix", artifact_link_prefix(request, &note_path)),
            ("project_link", project_link_from(&note_path)),
            (
                "request_link",
                request_artifact_wikilink(request, "request.md", "request.md"),
            ),
            (
                "decomposition_link",
                request_artifact_wikilink(request, "decomposition.md", "decomposition.md"),
            ),
            (
                "plan_link",
                request_artifact_wikilink(request, "plan.md", "plan.md"),
            ),
            (
                "change_doc_link",
                request_artifact_wikilink(request, "change-doc.md", "change-doc.md"),
            ),
            (
                "pr_doc_link",
                request_artifact_wikilink(request, "pr-doc.md", "pr-doc.md"),
            ),
            (
                "agent_journal_link",
                request_artifact_wikilink(request, "agent-journal.md", "agent-journal.md"),
            ),
            ("upstream_index_link", upstream_index_link(request)),
            (
                "stage_document_links",
                render_obsidian_stage_document_links(request),
            ),
            (
                "slice_index_links",
                render_obsidian_slice_index_links(request),
            ),
            (
                "workflow_mermaid_edges",
                render_obsidian_workflow_mermaid_edges(request),
            ),
            ("updated_at", now_string()),
        ],
    )
}

fn upstream_index_link(request: &Request) -> String {
    if is_slice_request(request) {
        if let Some(parent_id) = slice_parent_id_from_meta(request) {
            return format!(
                "[[{} index|{} index.md]]",
                markdown_link_label(&parent_id),
                markdown_link_label(&parent_id)
            );
        }
        "父 request index（slice.json 不可读）".to_string()
    } else {
        "[[project|project.md]]".to_string()
    }
}

fn slice_parent_id_from_meta(request: &Request) -> Option<String> {
    if !is_slice_request(request) || request.change_path.trim().is_empty() {
        return None;
    }
    let slice_meta_path = Path::new(&request.change_path).join("slice.json");
    let content = fs::read_to_string(slice_meta_path).ok()?;
    json_value(&content, "parent_request_id").filter(|value| !value.trim().is_empty())
}

fn render_obsidian_stage_document_links(request: &Request) -> String {
    let mut links = Vec::new();
    if is_slice_request(request) {
        links.push(format!(
            "  - Slice 需求与计划: {}",
            request_artifact_wikilink(request, "plan.md", "plan.md")
        ));
        links.push(format!(
            "  - 实现与变更: {}",
            request_artifact_wikilink(request, "change-doc.md", "change-doc.md")
        ));
    } else {
        links.push(format!(
            "  - 需求记录: {}",
            request_artifact_wikilink(request, "request.md", "request.md")
        ));
        links.push(format!(
            "  - 需求拆解: {}",
            request_artifact_wikilink(request, "decomposition.md", "decomposition.md")
        ));
        links.push(format!(
            "  - 交付 PR 文档: {}",
            request_artifact_wikilink(request, "pr-doc.md", "pr-doc.md")
        ));
    }
    links.join("\n")
}

fn render_obsidian_slice_index_links(request: &Request) -> String {
    if is_slice_request(request) {
        if let Some(parent_id) = slice_parent_id_from_meta(request) {
            return format!(
                "  - 父 request: [[{} index|{} index.md]]",
                markdown_link_label(&parent_id),
                markdown_link_label(&parent_id)
            );
        }
        return "  - 当前笔记是 slice index；父 request 信息暂不可读。".to_string();
    }
    if request.change_path.trim().is_empty()
        || !Path::new(&request.change_path)
            .join("decomposition.json")
            .exists()
    {
        return "  - 暂无 slice；decomposition 通过后生成。".to_string();
    }
    let definitions = load_slice_definitions(request).unwrap_or_default();
    if definitions.is_empty() {
        return "  - 暂无 slice。".to_string();
    }
    definitions
        .into_iter()
        .map(|definition| {
            let request_id = slice_request_id(&request.request_id, &definition.id);
            let label = format!("{} {}", request_id, definition.name);
            format!(
                "  - [[slices/{}/{} index|{}]]",
                markdown_link_label(&definition.id),
                markdown_link_label(&request_id),
                markdown_link_label(&label)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_obsidian_workflow_mermaid_edges(request: &Request) -> String {
    if is_slice_request(request) {
        [
            r#"  IDX --> P["Slice Request / Plan"]"#,
            r#"  IDX --> C["Change Doc"]"#,
        ]
        .join("\n")
    } else {
        [
            r#"  IDX --> R["Request"]"#,
            r#"  IDX --> D["Decomposition"]"#,
            r#"  IDX --> X["PR Doc"]"#,
        ]
        .join("\n")
    }
}

fn project_link_from(note_path: &Path) -> String {
    let Some(parent) = note_path.parent() else {
        return "project.md".to_string();
    };
    let Ok(relative_parent) = parent.strip_prefix("obsidian") else {
        return "obsidian/project.md".to_string();
    };
    let depth = relative_parent.components().count();
    if depth == 0 {
        return "project.md".to_string();
    }
    format!("{}project.md", "../".repeat(depth))
}

fn artifact_link_prefix(request: &Request, note_path: &Path) -> String {
    let change_path = request.change_path.trim();
    if change_path.is_empty() {
        return String::new();
    }
    if let Some(parent) = note_path.parent()
        && parent == Path::new(change_path)
    {
        return String::new();
    }
    format!("../../{}/", change_path.trim_end_matches('/'))
}

fn render_obsidian_template(template: &str, values: &[(&str, String)]) -> String {
    let mut rendered = template.to_string();
    for (key, value) in values {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    rendered
}

fn preserve_obsidian_note_body(rendered: &str, existing: &str) -> String {
    const PRESERVE_FROM: &str = "## 需求关系";
    let Some(rendered_index) = markdown_heading_offset(rendered, PRESERVE_FROM) else {
        return rendered.to_string();
    };
    let Some(existing_index) = markdown_heading_offset(existing, PRESERVE_FROM) else {
        return rendered.to_string();
    };
    format!(
        "{}\n\n{}",
        rendered[..rendered_index].trim_end(),
        existing[existing_index..].trim_start()
    )
}

fn markdown_heading_offset(content: &str, heading: &str) -> Option<usize> {
    let mut offset = 0usize;
    for line in content.split_inclusive('\n') {
        if line.trim_end() == heading {
            return Some(offset);
        }
        offset += line.len();
    }
    if content.lines().last().map(str::trim_end) == Some(heading) {
        return content.rfind(heading);
    }
    None
}
