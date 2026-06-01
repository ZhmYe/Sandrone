use super::*;

pub(crate) fn prepare_workspace_dirs() -> Result<()> {
    fs::create_dir_all(".codex-auto-dev/state")?;
    fs::create_dir_all("dev")?;
    fs::create_dir_all(WORKTREES)?;
    fs::create_dir_all("docs/changes")?;
    fs::create_dir_all("tools")?;
    fs::create_dir_all("skills/codex-auto-dev-workflow")?;
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

pub(crate) fn generate_plan_packet(request: &Request, preflight: &PlanPreflight) -> Result<()> {
    fs::create_dir_all(&request.change_path)?;
    fs::create_dir_all(Path::new(&request.change_path).join("approvals"))?;
    fs::write(
        Path::new(&request.change_path).join("request.md"),
        render_request(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("plan.md"),
        render_plan_template(request, preflight),
    )?;
    fs::write(
        Path::new(&request.change_path).join("change-doc.md"),
        render_change_doc_template(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("agent-journal.md"),
        render_agent_journal_template(request),
    )?;
    write_status_json(request, "planning", "planning", "")?;
    Ok(())
}

pub(crate) fn generate_start_packet(request: &Request) -> Result<()> {
    fs::create_dir_all(&request.change_path)?;
    write_status_json(request, "implementation", "in-progress", "")?;
    Ok(())
}

pub(crate) fn render_request(request: &Request) -> String {
    render_template(
        assets::REQUEST_TEMPLATE,
        &[
            ("title", request.title.clone()),
            ("request_id", request.request_id.clone()),
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
        ],
    )
}

pub(crate) fn render_plan_template(request: &Request, preflight: &PlanPreflight) -> String {
    render_template(
        assets::PLAN_TEMPLATE,
        &[
            ("title", request.title.clone()),
            ("request_id", request.request_id.clone()),
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
            ("preflight_notes", render_preflight_notes(preflight)),
        ],
    )
}

pub(crate) fn render_change_doc_template(request: &Request) -> String {
    render_template(
        assets::CHANGE_DOC_TEMPLATE,
        &[("request_id", request.request_id.clone())],
    )
}

pub(crate) fn render_agent_journal_template(request: &Request) -> String {
    render_template(
        assets::AGENT_JOURNAL_TEMPLATE,
        &[("request_id", request.request_id.clone())],
    )
}

fn render_template(template: &str, values: &[(&str, String)]) -> String {
    let mut rendered = template.to_string();
    for (key, value) in values {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    rendered
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
    write_default_plan_review_tool()?;
    write_default_test_review_tool()?;
    write_default_design_review_tool()?;
    write_default_integration_review_tool()?;
    write_default_review_prompt(PLAN_REVIEW_PROMPT, default_plan_review_prompt())?;
    write_default_review_prompt(TEST_REVIEW_PROMPT, default_test_review_prompt())?;
    write_default_review_prompt(DESIGN_REVIEW_PROMPT, default_design_review_prompt())?;
    write_default_review_prompt(
        INTEGRATION_REVIEW_PROMPT,
        default_integration_review_prompt(),
    )?;
    write_default_review_schema()?;
    Ok(())
}

fn write_default_plan_review_tool() -> Result<()> {
    write_default_review_tool(
        PLAN_REVIEW_TOOL,
        "PlanReviewer",
        PLAN_REVIEW_PROMPT,
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
    default_managed_assets()
        .into_iter()
        .map(|asset| asset.example_path)
        .collect()
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

fn default_reference_examples() -> Vec<ReferenceExample> {
    default_managed_assets()
        .into_iter()
        .map(|asset| ReferenceExample {
            path: asset.example_path,
            content: asset.content,
            executable: asset.executable,
        })
        .collect()
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
        "请先查看刷新的 .example 文件，再手动复制需要替换的文件；如果确定使用全部默认实现，运行 codex-auto-dev upgrade --default。"
    );
}

fn default_issue_agent_prompt() -> &'static str {
    assets::ISSUE_AGENT_PROMPT
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
