use super::*;

const WORKSPACE_ENV_FILE: &str = ".env";
const WORKSPACE_ENV_EXAMPLE: &str = ".env.example";

pub(crate) fn prepare_workspace_dirs() -> Result<()> {
    fs::create_dir_all(".sandrone/state")?;
    ensure_obsidian_vault_dirs()?;
    fs::create_dir_all("dev")?;
    fs::create_dir_all(WORKTREES)?;
    fs::create_dir_all("docs/guides")?;
    fs::create_dir_all("docs/playbooks")?;
    fs::create_dir_all("docs/reference")?;
    fs::create_dir_all("tools")?;
    fs::create_dir_all("skills/sandrone")?;
    Ok(())
}

pub(crate) fn write_config(repo_name: &str, git_url: &str, base_branch: &str) -> Result<()> {
    if Path::new(CONFIG_PATH).exists() {
        return Ok(());
    }
    let config = Config {
        schema_version: FRAMEWORK_SCHEMA_VERSION,
        repo_name: repo_name.to_string(),
        git_url: git_url.to_string(),
        base_branch: base_branch.to_string(),
        parallel_limit: 1,
    };
    rewrite_config(&config)
}

pub(crate) fn rewrite_config(config: &Config) -> Result<()> {
    fs::write(
        CONFIG_PATH,
        format!(
            "schema_version = {}\nrepo_name = \"{}\"\ngit_url = \"{}\"\nbase_branch = \"{}\"\nparallel_limit = {}\n",
            FRAMEWORK_SCHEMA_VERSION,
            toml_escape(&config.repo_name),
            toml_escape(&config.git_url),
            toml_escape(&config.base_branch),
            config.parallel_limit
        ),
    )?;
    Ok(())
}

pub(crate) fn ensure_state_file() -> Result<()> {
    if !Path::new(STATE_PATH).exists() {
        save_requests(&[])?;
    }
    Ok(())
}

pub(crate) fn ensure_sessions_file() -> Result<()> {
    if !Path::new(SESSIONS_PATH).exists() {
        save_sessions(&[])?;
    }
    Ok(())
}

pub(crate) fn write_default_env_files() -> Result<()> {
    if !Path::new(WORKSPACE_ENV_EXAMPLE).exists() {
        fs::write(WORKSPACE_ENV_EXAMPLE, default_workspace_env_example())?;
    }
    if !Path::new(WORKSPACE_ENV_FILE).exists() {
        fs::copy(WORKSPACE_ENV_EXAMPLE, WORKSPACE_ENV_FILE)?;
    }
    Ok(())
}

pub(crate) fn generate_plan_packet(request: &Request, preflight: &PlanPreflight) -> Result<()> {
    fs::create_dir_all(&request.change_path)?;
    write_plan_packet_artifacts(request, preflight, true)?;
    write_status_json(request, "planning", "planning", "")?;
    Ok(())
}

pub(crate) fn generate_decomposition_packet(
    request: &Request,
    preflight: &PlanPreflight,
) -> Result<()> {
    fs::create_dir_all(&request.change_path)?;
    write_parent_decomposition_packet_artifacts(request, preflight, true)?;
    write_decomposition_artifacts(request, true)?;
    Ok(())
}

pub(crate) fn ensure_decomposition_artifacts(request: &Request) -> Result<()> {
    fs::create_dir_all(&request.change_path)?;
    let preflight = PlanPreflight { notes: Vec::new() };
    write_parent_decomposition_packet_artifacts(request, &preflight, false)?;
    write_decomposition_artifacts(request, false)
}

fn write_plan_packet_artifacts(
    request: &Request,
    preflight: &PlanPreflight,
    overwrite: bool,
) -> Result<()> {
    let artifacts = [
        ("request.md", render_request(request)),
        ("plan.md", render_plan_template(request, preflight)),
        ("change-doc.md", render_change_doc_template(request)),
        ("pr-doc.md", render_pr_doc_template(request)),
        ("agent-journal.md", render_agent_journal_template(request)),
    ];
    for (name, content) in artifacts {
        if is_slice_request(request) && !request_generates_markdown_artifact(request, name) {
            continue;
        }
        let path = request_artifact_path_buf(request, name);
        if overwrite || !path.exists() {
            fs::write(path, content)?;
        }
    }
    Ok(())
}

fn write_parent_decomposition_packet_artifacts(
    request: &Request,
    preflight: &PlanPreflight,
    overwrite: bool,
) -> Result<()> {
    let artifacts = [
        ("request.md", render_request(request)),
        ("plan.md", render_plan_template(request, preflight)),
        ("change-doc.md", render_change_doc_template(request)),
        ("pr-doc.md", render_pr_doc_template(request)),
        ("agent-journal.md", render_agent_journal_template(request)),
    ];
    write_runtime_markdown_artifacts(request, &artifacts, overwrite)
}

fn write_runtime_markdown_artifacts(
    request: &Request,
    artifacts: &[(&str, String)],
    overwrite: bool,
) -> Result<()> {
    for (name, content) in artifacts {
        if !request_generates_markdown_artifact(request, name) {
            continue;
        }
        let path = request_artifact_path_buf(request, name);
        if overwrite || !path.exists() {
            fs::write(path, content)?;
        }
    }
    Ok(())
}

fn write_decomposition_artifacts(request: &Request, overwrite: bool) -> Result<()> {
    let artifacts = [
        ("decomposition.md", render_decomposition_template(request)),
        (
            "decomposition.json",
            render_decomposition_json_template(request),
        ),
        ("dag.json", render_dag_json_template(request)),
    ];
    for (name, content) in artifacts {
        let path = request_artifact_path_buf(request, name);
        if overwrite || !path.exists() {
            fs::write(path, content)?;
        }
    }
    fs::write(
        Path::new(&request.change_path).join(".decomposition-kind"),
        "request-slices\n",
    )?;
    sync_obsidian_request_note(request)?;
    Ok(())
}

pub(crate) fn generate_start_packet(request: &Request) -> Result<()> {
    fs::create_dir_all(&request.change_path)?;
    write_status_json(request, "implementation", "in-progress", "")?;
    Ok(())
}

pub(crate) fn render_request(request: &Request) -> String {
    render_template(assets::REQUEST_TEMPLATE, &request_template_values(request))
}

pub(crate) fn render_plan_template(request: &Request, preflight: &PlanPreflight) -> String {
    let mut values = request_template_values(request);
    values.push(("preflight_notes", render_preflight_notes(preflight)));
    render_template(assets::PLAN_TEMPLATE, &values)
}

pub(crate) fn render_change_doc_template(request: &Request) -> String {
    render_template(
        assets::CHANGE_DOC_TEMPLATE,
        &request_template_values(request),
    )
}

pub(crate) fn render_pr_doc_template(request: &Request) -> String {
    let mut values = request_template_values(request);
    values.extend([
        (
            "pr_title",
            format!("{} {} PR", request.request_id, request.title),
        ),
        (
            "branch",
            fallback_empty(&request.branch, "not-started").to_string(),
        ),
        ("status", request.status.clone()),
        ("delivered_at", now_string()),
        ("pr_tool_output", "pending".to_string()),
        ("pr_url", "n/a".to_string()),
        ("pr_status_raw", "n/a".to_string()),
        ("pr_status", "not-started".to_string()),
        ("change_doc_approved", "pending".to_string()),
        ("integration_review_status", "pending".to_string()),
    ]);
    render_template(assets::PR_DOC_TEMPLATE, &values)
}

pub(crate) fn render_decomposition_template(request: &Request) -> String {
    render_template(
        assets::DECOMPOSITION_TEMPLATE,
        &request_template_values(request),
    )
}

pub(crate) fn render_decomposition_json_template(request: &Request) -> String {
    render_template(
        assets::DECOMPOSITION_JSON_TEMPLATE,
        &request_template_values(request),
    )
}

pub(crate) fn render_dag_json_template(request: &Request) -> String {
    render_template(assets::DAG_JSON_TEMPLATE, &request_template_values(request))
}

pub(crate) fn render_agent_journal_template(request: &Request) -> String {
    render_template(
        assets::AGENT_JOURNAL_TEMPLATE,
        &request_template_values(request),
    )
}

fn render_template(template: &str, values: &[(&str, String)]) -> String {
    let mut rendered = template.to_string();
    for (key, value) in values {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    rendered
}

fn request_template_values(request: &Request) -> Vec<(&'static str, String)> {
    let request_link_artifact = if is_slice_request(request) {
        "plan.md"
    } else {
        "request.md"
    };
    let decomposition_link = if is_slice_request(request) {
        parent_artifact_wikilink_for_slice(request, "decomposition.md", "decomposition.md")
            .unwrap_or_else(|| "父需求 decomposition.md".to_string())
    } else {
        request_artifact_markdown_link(request, "decomposition.md", "decomposition.md")
    };
    let decomposition_wikilink = if is_slice_request(request) {
        parent_artifact_wikilink_for_slice(request, "decomposition.md", "decomposition.md")
            .unwrap_or_else(|| "父需求 decomposition.md".to_string())
    } else {
        request_artifact_wikilink(request, "decomposition.md", "decomposition.md")
    };
    vec![
        ("title", request.title.clone()),
        ("request_id", request.request_id.clone()),
        ("request_id_lower", request.request_id.to_lowercase()),
        ("external_id", request.external_id.clone()),
        ("source", request.source.clone()),
        ("url", fallback_empty(&request.url, "n/a").to_string()),
        (
            "body",
            fallback_empty(
                &request.body,
                "Codex 必须从用户对话或外部需求来源补充完整需求。",
            )
            .to_string(),
        ),
        ("updated_at", now_string()),
        (
            "request_file",
            request_artifact_file_name(request, "request.md"),
        ),
        (
            "decomposition_file",
            request_artifact_file_name(request, "decomposition.md"),
        ),
        ("plan_file", request_artifact_file_name(request, "plan.md")),
        (
            "change_doc_file",
            request_artifact_file_name(request, "change-doc.md"),
        ),
        (
            "agent_journal_file",
            request_artifact_file_name(request, "agent-journal.md"),
        ),
        (
            "pr_doc_file",
            request_artifact_file_name(request, "pr-doc.md"),
        ),
        (
            "request_link",
            request_artifact_markdown_link(request, request_link_artifact, "request.md"),
        ),
        ("decomposition_link", decomposition_link),
        (
            "plan_link",
            request_artifact_markdown_link(request, "plan.md", "plan.md"),
        ),
        (
            "change_doc_link",
            request_artifact_markdown_link(request, "change-doc.md", "change-doc.md"),
        ),
        (
            "agent_journal_link",
            request_artifact_markdown_link(request, "agent-journal.md", "agent-journal.md"),
        ),
        (
            "pr_doc_link",
            request_artifact_markdown_link(request, "pr-doc.md", "pr-doc.md"),
        ),
        (
            "request_wikilink",
            request_artifact_wikilink(request, request_link_artifact, "request.md"),
        ),
        ("decomposition_wikilink", decomposition_wikilink),
        (
            "plan_wikilink",
            request_artifact_wikilink(request, "plan.md", "plan.md"),
        ),
        (
            "change_doc_wikilink",
            request_artifact_wikilink(request, "change-doc.md", "change-doc.md"),
        ),
        (
            "agent_journal_wikilink",
            request_artifact_wikilink(request, "agent-journal.md", "agent-journal.md"),
        ),
        (
            "pr_doc_wikilink",
            request_artifact_wikilink(request, "pr-doc.md", "pr-doc.md"),
        ),
    ]
}

fn write_executable_file(path: &str, content: impl AsRef<[u8]>) -> Result<()> {
    fs::write(path, content)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

pub(crate) fn write_default_issue_tool() -> Result<()> {
    if Path::new(ISSUE_TOOL).exists() {
        return Ok(());
    }
    write_executable_file(ISSUE_TOOL, default_issue_tool_content())
}

fn default_issue_tool_content() -> &'static str {
    assets::ISSUE_UPDATE_SCRIPT
}

pub(crate) fn write_default_issue_agent_tool() -> Result<()> {
    fs::create_dir_all("tools/prompts")?;
    if !Path::new(ISSUE_AGENT_TOOL).exists() {
        write_executable_file(ISSUE_AGENT_TOOL, default_issue_agent_tool_content())?;
    }
    if !Path::new(REBASE_AGENT_TOOL).exists() {
        write_executable_file(REBASE_AGENT_TOOL, default_rebase_agent_tool_content())?;
    }
    if !Path::new(ISSUE_AGENT_PROMPT).exists() {
        fs::write(ISSUE_AGENT_PROMPT, default_issue_agent_prompt())?;
    }
    if !Path::new(DECOMPOSITION_AGENT_PROMPT).exists() {
        fs::write(
            DECOMPOSITION_AGENT_PROMPT,
            default_decomposition_agent_prompt(),
        )?;
    }
    if !Path::new(PLAN_AGENT_PROMPT).exists() {
        fs::write(PLAN_AGENT_PROMPT, default_plan_agent_prompt())?;
    }
    if !Path::new(IMPLEMENTATION_AGENT_PROMPT).exists() {
        fs::write(
            IMPLEMENTATION_AGENT_PROMPT,
            default_implementation_agent_prompt(),
        )?;
    }
    if !Path::new(REBASE_AGENT_PROMPT).exists() {
        fs::write(REBASE_AGENT_PROMPT, default_rebase_agent_prompt())?;
    }
    Ok(())
}

fn codex_bin_resolver_shell() -> &'static str {
    assets::CODEX_BIN_RESOLVER_SCRIPT
}

fn default_issue_agent_tool_content() -> String {
    assets::ISSUE_AGENT_SCRIPT.replace("{{CODEX_BIN_RESOLVER}}", codex_bin_resolver_shell())
}

fn default_rebase_agent_tool_content() -> String {
    assets::REBASE_AGENT_SCRIPT.replace("{{CODEX_BIN_RESOLVER}}", codex_bin_resolver_shell())
}

pub(crate) fn write_default_pr_tool() -> Result<()> {
    if Path::new(PR_TOOL).exists() {
        return Ok(());
    }
    write_executable_file(PR_TOOL, default_pr_tool_content())
}

pub(crate) fn write_default_pr_status_tool() -> Result<()> {
    if Path::new(PR_STATUS_TOOL).exists() {
        return Ok(());
    }
    write_executable_file(PR_STATUS_TOOL, default_pr_status_tool_content())
}

fn default_pr_tool_content() -> &'static str {
    assets::PR_CREATE_SCRIPT
}

fn default_pr_status_tool_content() -> &'static str {
    assets::PR_STATUS_SCRIPT
}

pub(crate) fn write_default_review_tools() -> Result<()> {
    fs::create_dir_all("tools/prompts")?;
    fs::create_dir_all("tools/schemas")?;
    write_default_check_format_tool()?;
    write_default_plan_review_tool()?;
    write_default_decomposition_review_tool()?;
    write_default_test_review_tool()?;
    write_default_design_review_tool()?;
    write_default_integration_review_tool()?;
    write_default_review_prompt(PLAN_REVIEW_PROMPT, default_plan_review_prompt())?;
    write_default_review_prompt(
        DECOMPOSITION_REVIEW_PROMPT,
        default_decomposition_review_prompt(),
    )?;
    write_default_review_prompt(TEST_REVIEW_PROMPT, default_test_review_prompt())?;
    write_default_review_prompt(DESIGN_REVIEW_PROMPT, default_design_review_prompt())?;
    write_default_review_prompt(
        INTEGRATION_REVIEW_PROMPT,
        default_integration_review_prompt(),
    )?;
    write_default_review_schema()?;
    Ok(())
}

fn write_default_check_format_tool() -> Result<()> {
    if Path::new(CHECK_FORMAT_TOOL).exists() {
        return Ok(());
    }
    write_executable_file(CHECK_FORMAT_TOOL, default_check_format_tool_content())
}

fn default_check_format_tool_content() -> &'static str {
    assets::CHECK_FORMAT_SCRIPT
}

fn write_default_plan_review_tool() -> Result<()> {
    write_default_review_tool(
        PLAN_REVIEW_TOOL,
        "PlanReviewer",
        PLAN_REVIEW_PROMPT,
        "workspace-write",
    )
}

fn write_default_decomposition_review_tool() -> Result<()> {
    write_default_review_tool(
        DECOMPOSITION_REVIEW_TOOL,
        "DecompositionReviewer",
        DECOMPOSITION_REVIEW_PROMPT,
        "workspace-write",
    )
}

fn write_default_test_review_tool() -> Result<()> {
    write_default_review_tool(
        TEST_REVIEW_TOOL,
        "TestReviewer",
        TEST_REVIEW_PROMPT,
        "workspace-write",
    )
}

fn write_default_design_review_tool() -> Result<()> {
    write_default_review_tool(
        DESIGN_REVIEW_TOOL,
        "DesignReviewer",
        DESIGN_REVIEW_PROMPT,
        "workspace-write",
    )
}

fn write_default_integration_review_tool() -> Result<()> {
    write_default_review_tool(
        INTEGRATION_REVIEW_TOOL,
        "IntegrationReviewer",
        INTEGRATION_REVIEW_PROMPT,
        "workspace-write",
    )
}

fn write_default_review_tool(
    path: &str,
    reviewer: &str,
    prompt_path: &str,
    sandbox: &str,
) -> Result<()> {
    if Path::new(path).exists() {
        return Ok(());
    }
    write_executable_file(
        path,
        default_review_tool_content(reviewer, prompt_path, sandbox),
    )
}

fn default_review_tool_content(reviewer: &str, prompt_path: &str, sandbox: &str) -> String {
    assets::REVIEW_TOOL_SCRIPT
        .replace("{{REVIEWER}}", reviewer)
        .replace("{{PROMPT_PATH}}", prompt_path)
        .replace("{{SANDBOX}}", sandbox)
        .replace("{{CODEX_BIN_RESOLVER}}", codex_bin_resolver_shell())
}

fn write_default_review_prompt(path: &str, content: &str) -> Result<()> {
    if Path::new(path).exists() {
        return Ok(());
    }
    fs::write(path, content)?;
    Ok(())
}

fn write_default_review_schema() -> Result<()> {
    if Path::new(REVIEW_SCHEMA).exists() {
        return Ok(());
    }
    fs::write(REVIEW_SCHEMA, default_review_schema_content())?;
    Ok(())
}

fn default_review_schema_content() -> &'static str {
    assets::REVIEW_RESULT_SCHEMA
}

pub(crate) struct DefaultManagedAsset {
    pub(crate) path: &'static str,
    pub(crate) example_path: &'static str,
    content: String,
    executable: bool,
}

struct ReferenceExample {
    path: &'static str,
    content: String,
    executable: bool,
}

pub(crate) fn default_reference_example_paths() -> Vec<&'static str> {
    let mut paths = default_managed_assets()
        .into_iter()
        .map(|asset| asset.example_path)
        .collect::<Vec<_>>();
    paths.push(WORKSPACE_ENV_EXAMPLE);
    paths
}

fn default_reference_examples() -> Vec<ReferenceExample> {
    let mut examples = vec![ReferenceExample {
        path: WORKSPACE_ENV_EXAMPLE,
        content: default_workspace_env_example().to_string(),
        executable: false,
    }];
    examples.extend(
        default_managed_assets()
            .into_iter()
            .map(|asset| ReferenceExample {
                path: asset.example_path,
                content: asset.content,
                executable: asset.executable,
            }),
    );
    examples
}

fn default_workspace_env_example() -> &'static str {
    assets::WORKSPACE_ENV_EXAMPLE
}

pub(crate) fn default_managed_assets() -> Vec<DefaultManagedAsset> {
    vec![
        DefaultManagedAsset {
            path: ISSUE_TOOL,
            example_path: ISSUE_TOOL_EXAMPLE,
            content: default_issue_tool_content().to_string(),
            executable: true,
        },
        DefaultManagedAsset {
            path: ISSUE_AGENT_TOOL,
            example_path: ISSUE_AGENT_TOOL_EXAMPLE,
            content: default_issue_agent_tool_content().to_string(),
            executable: true,
        },
        DefaultManagedAsset {
            path: REBASE_AGENT_TOOL,
            example_path: REBASE_AGENT_TOOL_EXAMPLE,
            content: default_rebase_agent_tool_content().to_string(),
            executable: true,
        },
        DefaultManagedAsset {
            path: PR_TOOL,
            example_path: PR_TOOL_EXAMPLE,
            content: default_pr_tool_content().to_string(),
            executable: true,
        },
        DefaultManagedAsset {
            path: PR_STATUS_TOOL,
            example_path: PR_STATUS_TOOL_EXAMPLE,
            content: default_pr_status_tool_content().to_string(),
            executable: true,
        },
        DefaultManagedAsset {
            path: CHECK_FORMAT_TOOL,
            example_path: CHECK_FORMAT_TOOL_EXAMPLE,
            content: default_check_format_tool_content().to_string(),
            executable: true,
        },
        DefaultManagedAsset {
            path: PLAN_REVIEW_TOOL,
            example_path: PLAN_REVIEW_TOOL_EXAMPLE,
            content: default_review_tool_content(
                "PlanReviewer",
                PLAN_REVIEW_PROMPT,
                "workspace-write",
            ),
            executable: true,
        },
        DefaultManagedAsset {
            path: DECOMPOSITION_REVIEW_TOOL,
            example_path: DECOMPOSITION_REVIEW_TOOL_EXAMPLE,
            content: default_review_tool_content(
                "DecompositionReviewer",
                DECOMPOSITION_REVIEW_PROMPT,
                "workspace-write",
            ),
            executable: true,
        },
        DefaultManagedAsset {
            path: TEST_REVIEW_TOOL,
            example_path: TEST_REVIEW_TOOL_EXAMPLE,
            content: default_review_tool_content(
                "TestReviewer",
                TEST_REVIEW_PROMPT,
                "workspace-write",
            ),
            executable: true,
        },
        DefaultManagedAsset {
            path: DESIGN_REVIEW_TOOL,
            example_path: DESIGN_REVIEW_TOOL_EXAMPLE,
            content: default_review_tool_content(
                "DesignReviewer",
                DESIGN_REVIEW_PROMPT,
                "workspace-write",
            ),
            executable: true,
        },
        DefaultManagedAsset {
            path: INTEGRATION_REVIEW_TOOL,
            example_path: INTEGRATION_REVIEW_TOOL_EXAMPLE,
            content: default_review_tool_content(
                "IntegrationReviewer",
                INTEGRATION_REVIEW_PROMPT,
                "workspace-write",
            ),
            executable: true,
        },
        DefaultManagedAsset {
            path: ISSUE_AGENT_PROMPT,
            example_path: ISSUE_AGENT_PROMPT_EXAMPLE,
            content: default_issue_agent_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: DECOMPOSITION_AGENT_PROMPT,
            example_path: DECOMPOSITION_AGENT_PROMPT_EXAMPLE,
            content: default_decomposition_agent_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: PLAN_AGENT_PROMPT,
            example_path: PLAN_AGENT_PROMPT_EXAMPLE,
            content: default_plan_agent_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: IMPLEMENTATION_AGENT_PROMPT,
            example_path: IMPLEMENTATION_AGENT_PROMPT_EXAMPLE,
            content: default_implementation_agent_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: REBASE_AGENT_PROMPT,
            example_path: REBASE_AGENT_PROMPT_EXAMPLE,
            content: default_rebase_agent_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: PLAN_REVIEW_PROMPT,
            example_path: PLAN_REVIEW_PROMPT_EXAMPLE,
            content: default_plan_review_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: DECOMPOSITION_REVIEW_PROMPT,
            example_path: DECOMPOSITION_REVIEW_PROMPT_EXAMPLE,
            content: default_decomposition_review_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: TEST_REVIEW_PROMPT,
            example_path: TEST_REVIEW_PROMPT_EXAMPLE,
            content: default_test_review_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: DESIGN_REVIEW_PROMPT,
            example_path: DESIGN_REVIEW_PROMPT_EXAMPLE,
            content: default_design_review_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: INTEGRATION_REVIEW_PROMPT,
            example_path: INTEGRATION_REVIEW_PROMPT_EXAMPLE,
            content: default_integration_review_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: REVIEW_SCHEMA,
            example_path: REVIEW_SCHEMA_EXAMPLE,
            content: default_review_schema_content().to_string(),
            executable: false,
        },
    ]
}

pub(crate) fn refresh_default_reference_examples() -> Result<()> {
    for example in default_reference_examples() {
        if let Some(parent) = Path::new(example.path).parent() {
            fs::create_dir_all(parent)?;
        }
        if example.executable {
            write_executable_file(example.path, example.content)?;
        } else {
            fs::write(example.path, example.content)?;
        }
    }
    Ok(())
}

pub(crate) fn replace_default_runtime_assets_from_examples() -> Result<()> {
    for asset in default_managed_assets() {
        if let Some(parent) = Path::new(asset.path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(asset.example_path, asset.path)?;
        if asset.executable {
            let mut permissions = fs::metadata(asset.path)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(asset.path, permissions)?;
        }
    }
    Ok(())
}

pub(crate) fn print_upgrade_default_asset_guidance() {
    println!("普通 upgrade 不会替换正式 connector、prompt 或 review schema。");
    println!(
        "请先查看刷新的 .example 文件，再手动复制需要替换的文件；如果确定使用全部默认实现，运行 sandrone upgrade --default。"
    );
}

fn default_issue_agent_prompt() -> &'static str {
    assets::ISSUE_AGENT_PROMPT
}

fn default_decomposition_agent_prompt() -> &'static str {
    assets::DECOMPOSITION_AGENT_PROMPT
}

fn default_plan_agent_prompt() -> &'static str {
    assets::PLAN_AGENT_PROMPT
}

fn default_implementation_agent_prompt() -> &'static str {
    assets::IMPLEMENTATION_AGENT_PROMPT
}

fn default_rebase_agent_prompt() -> &'static str {
    assets::REBASE_AGENT_PROMPT
}

fn default_plan_review_prompt() -> &'static str {
    assets::PLAN_REVIEWER_PROMPT
}

fn default_decomposition_review_prompt() -> &'static str {
    assets::DECOMPOSITION_REVIEWER_PROMPT
}

fn default_test_review_prompt() -> &'static str {
    assets::TEST_REVIEWER_PROMPT
}

fn default_design_review_prompt() -> &'static str {
    assets::DESIGN_REVIEWER_PROMPT
}

fn default_integration_review_prompt() -> &'static str {
    assets::INTEGRATION_REVIEWER_PROMPT
}

pub(crate) fn write_default_workflow_skill() -> Result<()> {
    if Path::new(WORKFLOW_SKILL).exists() {
        return Ok(());
    }
    fs::write(WORKFLOW_SKILL, WORKFLOW_SKILL_CONTENT)?;
    Ok(())
}
