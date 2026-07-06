// claurst-commands: Slash command system for Claurst.
//
// This crate implements the /command framework that allows users to type
// commands like /help, /compact, /clear, /model, /config, /cost, etc.
// Each command is a struct implementing the `SlashCommand` trait.

use async_trait::async_trait;
use claurst_core::config::{Config, Settings, Theme};
use claurst_core::cost::CostTracker;
use claurst_core::types::{ContentBlock, Message};
use std::collections::BTreeMap;
use std::sync::Arc;
#[allow(unused_imports)]
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Core trait
// ---------------------------------------------------------------------------

/// Context available to every slash command.
pub struct CommandContext {
    pub config: Config,
    pub cost_tracker: Arc<CostTracker>,
    pub messages: Vec<Message>,
    pub working_dir: std::path::PathBuf,
    pub session_id: String,
    pub session_title: Option<String>,
    /// Remote session URL set when a bridge connection is active.
    pub remote_session_url: Option<String>,
    // Note: config already contains hooks, mcp_servers, etc.
    /// Live MCP manager — present when servers are connected.
    pub mcp_manager: Option<Arc<claurst_mcp::McpManager>>,
    /// Optional callback for starting an MCP OAuth flow in the background.
    pub mcp_auth_runner: Option<Arc<dyn Fn(claurst_mcp::oauth::McpAuthSession) + Send + Sync>>,
}

/// Result of running a slash command.
#[derive(Debug)]
pub enum CommandResult {
    /// Display a message to the user (does NOT go to the model).
    Message(String),
    /// Inject a message into the conversation as though the user typed it.
    UserMessage(String),
    /// Modify the configuration.
    ConfigChange(Config),
    /// Modify the configuration and show a specific status message.
    ConfigChangeMessage(Config, String),
    /// Trigger a background MCP OAuth flow and request runtime reconnect on success.
    McpAuthFlow {
        /// The configured MCP server name.
        server_name: String,
        /// The browser URL shown to the user while the background flow runs.
        auth_url: String,
        /// The local callback URL waiting for the OAuth redirect.
        redirect_uri: String,
    },
    /// Clear the conversation.
    ClearConversation,
    /// Replace the conversation with a specific message list (used by /rewind).
    SetMessages(Vec<Message>),
    /// Load a previously saved session into the live REPL.
    ResumeSession(claurst_core::history::ConversationSession),
    /// Update the current session title.
    RenameSession(String),
    /// Trigger the OAuth login flow (handled by the REPL in main.rs).
    /// The bool indicates whether to use Claude.ai auth (true) or Console auth (false).
    StartOAuthFlow(bool),
    /// Trigger the OAuth login flow for a specific provider with optional
    /// human-friendly label for the new account profile.
    ///
    /// `provider` is one of `claurst_core::accounts::PROVIDER_ANTHROPIC` or
    /// `PROVIDER_CODEX`. `login_with_claude_ai` is only meaningful for
    /// Anthropic.
    StartLoginForProvider {
        provider: String,
        login_with_claude_ai: bool,
        label: Option<String>,
    },
    /// Exit the REPL.
    Exit,
    /// No visible output.
    Silent,
    /// An error.
    Error(String),
    /// Open the rewind/message-selector overlay in the TUI.
    /// The TUI will call SetMessages when the user confirms.
    OpenRewindOverlay,
    /// Open the hooks configuration browser overlay in the TUI.
    /// Falls back to a text listing in non-TUI contexts.
    OpenHooksOverlay,
    /// Open the import-config overlay in the TUI.
    OpenImportConfigOverlay,
    /// Clear saved provider auth, model selection, and model caches, then
    /// rebuild the live runtime state.
    RefreshProviderState,
    /// Activate a speech mode (caveman/rocky) with level, or deactivate (normal).
    /// (mode, level) — mode=None means deactivate.
    SpeechMode { mode: Option<String>, level: String },
}

/// Every slash command implements this trait.
#[async_trait]
pub trait SlashCommand: Send + Sync {
    /// The primary name (without the leading `/`).
    fn name(&self) -> &str;
    /// Alias names (e.g. `["h"]` for `/help`).
    fn aliases(&self) -> Vec<&str> {
        vec![]
    }
    /// One-line description for /help.
    fn description(&self) -> &str;
    /// Detailed help text (shown by `/help <command>`).
    fn help(&self) -> &str {
        self.description()
    }
    /// Whether this command is visible in /help output.
    fn hidden(&self) -> bool {
        false
    }
    /// Execute the command with the given arguments string.
    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult;
}

fn stripped_model_for_provider<'a>(provider_id: &str, model_id: &'a str) -> &'a str {
    model_id
        .strip_prefix(&format!("{provider_id}/"))
        .unwrap_or(model_id)
}

fn canonical_model_for_provider(provider_id: &str, model_id: &str) -> String {
    if provider_id == "anthropic" || model_id.contains('/') {
        model_id.to_string()
    } else {
        format!("{provider_id}/{model_id}")
    }
}

fn provider_lookup_ids(provider_id: &str) -> Vec<&str> {
    match provider_id {
        "togetherai" | "together-ai" => vec!["togetherai", "together-ai"],
        "lmstudio" | "lm-studio" => vec!["lmstudio", "lm-studio"],
        "llamacpp" | "llama-cpp" | "llama-server" => {
            vec!["llamacpp", "llama-cpp", "llama-server"]
        }
        "moonshot" | "moonshotai" => vec!["moonshot", "moonshotai"],
        "zhipu" | "zhipuai" => vec!["zhipu", "zhipuai"],
        "vultr" | "vultr-ai" => vec!["vultr", "vultr-ai"],
        "google" | "google-vertex" => vec!["google", "google-vertex"],
        _ => vec![provider_id],
    }
}

fn resolve_fast_model_id(config: &Config) -> String {
    let provider_id = config.selected_provider_id();
    let registry = claurst_api::ModelRegistry::new();

    provider_lookup_ids(provider_id)
        .into_iter()
        .find_map(|lookup_id| registry.best_small_model_for_provider(lookup_id))
        .unwrap_or_else(|| stripped_model_for_provider(provider_id, config.effective_model()).to_string())
}

async fn provider_for_config(config: &Config) -> Option<std::sync::Arc<dyn claurst_api::LlmProvider>> {
    let anthropic_auth = config.resolve_anthropic_auth_async().await;
    let registry = claurst_api::ProviderRegistry::from_config(
        config,
        claurst_api::client::ClientConfig {
            api_key: anthropic_auth
                .as_ref()
                .map(|(credential, _)| credential.clone())
                .unwrap_or_default(),
            api_base: config.resolve_anthropic_api_base(),
            use_bearer_auth: anthropic_auth
                .as_ref()
                .is_some_and(|(_, use_bearer)| *use_bearer),
            ..Default::default()
        },
    );

    provider_lookup_ids(config.selected_provider_id())
        .into_iter()
        .find_map(|lookup_id| registry.get(&claurst_core::ProviderId::new(lookup_id)).cloned())
}

fn text_from_content_blocks(blocks: &[ContentBlock]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

// ---------------------------------------------------------------------------
// Feature command modules (extracted per issue #232 to shrink this file).
// Each module owns a cohesive group of SlashCommand impls plus its private
// helpers. Command structs are re-exported so the public surface is unchanged.
// ---------------------------------------------------------------------------
mod goal;
pub use goal::*;
mod speech;
pub use speech::*;
mod config_cmd;
pub use config_cmd::*;
mod plugin;
pub use plugin::*;
mod doctor;
pub use doctor::*;
mod accounts;
pub use accounts::*;
mod review;
pub use review::*;
mod mcp;
pub use mcp::*;
mod export;
pub use export::*;
mod share;
pub use share::*;
mod copy;
pub use copy::*;
mod chrome;
pub use chrome::*;
mod teleport;
pub use teleport::*;
mod managed_agents;
pub use managed_agents::*;

// ---------------------------------------------------------------------------
// Built-in commands
// ---------------------------------------------------------------------------

pub struct HelpCommand;
pub struct ClearCommand;
pub struct CompactCommand;
pub struct CostCommand;
pub struct ExitCommand;
pub struct ModelCommand;
pub struct ColorCommand;
pub struct VersionCommand;
pub struct ResumeCommand;
pub struct StatusCommand;
pub struct DiffCommand;
pub struct MemoryCommand;
pub struct BugCommand;
pub struct UsageCommand;
pub struct InitCommand;
pub struct HooksCommand;
pub struct ImportConfigCommand;
pub struct PermissionsCommand;
pub struct PlanCommand;
pub struct TasksCommand;
pub struct SessionCommand;
pub struct ThinkingCommand;
// New commands
pub struct SkillsCommand;
pub struct RewindCommand;
pub struct StatsCommand;
pub struct FilesCommand;
pub struct RenameCommand;
pub struct EffortCommand;
pub struct SummaryCommand;
pub struct CommitCommand;
pub struct ThemeCommand;
pub struct OutputStyleCommand;
pub struct KeybindingsCommand;
pub struct PrivacySettingsCommand;
// Batch-1 new commands
pub struct RemoteControlCommand;
pub struct RemoteEnvCommand;
pub struct ContextCommand;
pub struct VimCommand;
pub struct VoiceCommand;
pub struct UpgradeCommand;
pub struct ReleaseNotesCommand;
pub struct RateLimitOptionsCommand;
pub struct StatuslineCommand;
pub struct SecurityReviewCommand;
pub struct TerminalSetupCommand;
pub struct ExtraUsageCommand;
pub struct FastCommand;
pub struct ThinkBackCommand;
pub struct ThinkBackPlayCommand;
pub struct FeedbackCommand;
pub struct ColorSetCommand;
// New commands: teleport, btw, ctx-viz, sandbox-toggle
pub struct BtwCommand;
pub struct CtxVizCommand;
pub struct SandboxToggleCommand;
pub struct HeapdumpCommand;
pub struct InsightsCommand;
pub struct UltrareviewCommand;
pub struct AdvisorCommand;
pub struct InstallSlackAppCommand;
pub struct UndoCommand;
pub struct RevertCommand;
pub struct CheckpointsCommand;
pub struct SnapshotDiffCommand;
pub struct ProvidersCommand;
pub struct ConnectCommand;
pub struct AgentCommand;
pub struct SearchCommand;
pub struct ForkCommand;
pub struct NamedCommandAdapter {
    pub slash_name: &'static str,
    pub target_name: &'static str,
    pub slash_aliases: &'static [&'static str],
    pub slash_description: &'static str,
    pub slash_help: &'static str,
}

#[derive(serde::Serialize)]
struct KeybindingTemplateFile {
    #[serde(rename = "$schema")]
    schema: &'static str,
    #[serde(rename = "$docs")]
    docs: &'static str,
    bindings: Vec<KeybindingTemplateBlock>,
}

#[derive(serde::Serialize)]
struct KeybindingTemplateBlock {
    context: String,
    bindings: BTreeMap<String, Option<String>>,
}

fn save_settings_mutation<F>(mutate: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut Settings),
{
    let mut settings = Settings::load_sync()?;
    mutate(&mut settings);
    settings.save_sync()
}

fn open_with_system(target: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let ps_cmd = format!("Start-Process '{}'", target.replace('\'', "''"));
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(target)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(target)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        Ok(())
    }
}

fn format_keystroke(keystroke: &claurst_core::keybindings::ParsedKeystroke) -> String {
    let mut parts = Vec::new();
    if keystroke.ctrl {
        parts.push("ctrl".to_string());
    }
    if keystroke.alt {
        parts.push("alt".to_string());
    }
    if keystroke.shift {
        parts.push("shift".to_string());
    }
    if keystroke.meta {
        parts.push("meta".to_string());
    }
    parts.push(match keystroke.key.as_str() {
        "space" => "space".to_string(),
        other => other.to_string(),
    });
    parts.join("+")
}

fn format_chord(chord: &[claurst_core::keybindings::ParsedKeystroke]) -> String {
    chord
        .iter()
        .map(format_keystroke)
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_keybindings_template() -> anyhow::Result<String> {
    let mut grouped: BTreeMap<String, BTreeMap<String, Option<String>>> = BTreeMap::new();
    for binding in claurst_core::keybindings::default_bindings() {
        let chord = format_chord(&binding.chord);
        if claurst_core::keybindings::NON_REBINDABLE.contains(&chord.as_str()) {
            continue;
        }
        grouped
            .entry(format!("{:?}", binding.context))
            .or_default()
            .insert(chord, binding.action.clone());
    }

    let template = KeybindingTemplateFile {
        schema: "https://www.schemastore.org/claude-code-keybindings.json",
        docs: "https://code.claude.com/docs/en/keybindings",
        bindings: grouped
            .into_iter()
            .map(|(context, bindings)| KeybindingTemplateBlock { context, bindings })
            .collect(),
    };

    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&template)?
    ))
}

fn parse_theme(name: &str) -> Option<Theme> {
    match name.trim().to_lowercase().as_str() {
        "default" | "system" => Some(Theme::Default),
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        custom if !custom.is_empty() => Some(Theme::Custom(custom.to_string())),
        _ => None,
    }
}

fn current_output_style_name(config: &Config) -> &str {
    config.output_style.as_deref().unwrap_or("default")
}

fn available_output_style_names() -> Vec<String> {
    claurst_core::output_styles::all_styles(&Settings::config_dir())
        .into_iter()
        .map(|style| style.name)
        .collect()
}

fn split_command_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escape = false;

    for ch in args.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }

        match ch {
            '\\' => escape = true,
            '\'' | '"' if quote == Some(ch) => quote = None,
            '\'' | '"' if quote.is_none() => quote = Some(ch),
            ch if ch.is_whitespace() && quote.is_none() => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        out.push(current);
    }

    out
}

fn execute_named_command_from_slash(
    target_name: &str,
    args: &str,
    ctx: &CommandContext,
) -> CommandResult {
    let Some(cmd) = named_commands::find_named_command(target_name) else {
        return CommandResult::Error(format!(
            "Named command '{}' is not available in this build.",
            target_name
        ));
    };

    let parsed_args = split_command_args(args);
    let parsed_refs = parsed_args.iter().map(String::as_str).collect::<Vec<_>>();
    cmd.execute_named(&parsed_refs, ctx)
}

// ---- /help ---------------------------------------------------------------

/// Category labels for help grouping.
fn command_category(name: &str) -> &'static str {
    match name {
        "clear" | "compact" | "rewind" | "summary" | "export" | "rename" | "branch" | "fork" => {
            "Conversation"
        }
        "model" | "config" | "theme" | "color" | "vim" | "fast" | "effort"
        | "voice" | "statusline" | "output-style" | "keybindings"
        | "privacy-settings" | "rate-limit-options" | "sandbox-toggle" => "Settings",
        "cost" | "stats" | "usage" | "extra-usage" | "context" | "ctx-viz" => "Usage & Cost",
        "status" | "doctor" | "terminal-setup" | "version" | "update" | "upgrade"
        | "release-notes" => "System",
        "login" | "logout" | "refresh" | "permissions" => "Auth & Permissions",
        "memory" | "files" | "diff" | "init" | "commit" | "review"
        | "security-review" | "import-config" => "Project",
        "mcp" | "hooks" | "ide" | "chrome" => "Integrations",
        "session" | "resume" | "remote-control" | "remote-env"
        | "teleport" => "Sessions & Remote",
        "help" | "exit" | "feedback" | "bug" => "General",
        "think-back" | "thinkback-play" | "thinking" | "plan" | "tasks" => "AI & Thinking",
        "copy" | "skills" | "agents" | "plugin" | "reload-plugins"
        | "stickers" | "passes" | "desktop" | "mobile" | "btw" => "Tools & Extras",
        _ => "Other",
    }
}

#[async_trait]
impl SlashCommand for HelpCommand {
    fn name(&self) -> &str { "help" }
    fn aliases(&self) -> Vec<&str> { vec!["h", "?"] }
    fn description(&self) -> &str { "Show available commands and usage information" }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        if !args.is_empty() {
            // Show help for a specific command
            if let Some(cmd) = find_command(args) {
                let aliases = cmd.aliases();
                let alias_line = if aliases.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\nAliases: {}",
                        aliases.iter().map(|a| format!("/{}", a)).collect::<Vec<_>>().join(", ")
                    )
                };
                return CommandResult::Message(format!(
                    "/{name}{aliases}\n{desc}\n\n{help}",
                    name = cmd.name(),
                    aliases = alias_line,
                    desc = cmd.description(),
                    help = cmd.help(),
                ));
            }
            return CommandResult::Error(format!("Unknown command: /{}", args));
        }

        // Grouped output
        let commands = all_commands();
        let visible: Vec<_> = commands.iter().filter(|c| !c.hidden()).collect();

        // Collect categories in stable order
        let category_order = [
            "Conversation",
            "Settings",
            "Usage & Cost",
            "System",
            "Auth & Permissions",
            "Project",
            "Integrations",
            "Sessions & Remote",
            "AI & Thinking",
            "Tools & Extras",
            "General",
            "Other",
        ];

        let mut by_cat: std::collections::HashMap<&str, Vec<String>> =
            std::collections::HashMap::new();

        for cmd in &visible {
            let cat = command_category(cmd.name());
            let aliases = cmd.aliases();
            let alias_str = if aliases.is_empty() {
                String::new()
            } else {
                format!(
                    " ({})",
                    aliases.iter().map(|a| format!("/{}", a)).collect::<Vec<_>>().join(", ")
                )
            };
            by_cat
                .entry(cat)
                .or_default()
                .push(format!("  /{:<20} {}", format!("{}{}", cmd.name(), alias_str), cmd.description()));
        }

        let mut output = String::from("Claurst — Slash Commands\n");
        output.push_str("════════════════════════════\n");

        for cat in &category_order {
            if let Some(entries) = by_cat.get(cat) {
                output.push_str(&format!("\n{}\n", cat));
                for entry in entries {
                    output.push_str(&format!("{}\n", entry));
                }
            }
        }

        output.push_str("\nType /help <command> for detailed help on a specific command.");
        CommandResult::Message(output)
    }
}

// ---- /clear --------------------------------------------------------------

#[async_trait]
impl SlashCommand for ClearCommand {
    fn name(&self) -> &str { "clear" }
    fn aliases(&self) -> Vec<&str> { vec!["c", "reset", "new"] }
    fn description(&self) -> &str { "Clear the conversation history" }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        CommandResult::ClearConversation
    }
}

// ---- /compact ------------------------------------------------------------

#[async_trait]
impl SlashCommand for CompactCommand {
    fn name(&self) -> &str { "compact" }
    fn description(&self) -> &str { "Compact the conversation to reduce token usage" }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let msg_count = ctx.messages.len();
        let instruction = if args.is_empty() {
            "Provide a detailed summary of our conversation so far, preserving all \
             key technical details, decisions made, file paths mentioned, and current \
             task status."
                .to_string()
        } else {
            args.to_string()
        };

        CommandResult::UserMessage(format!(
            "[Compact requested ({} messages). Instruction: {}]",
            msg_count, instruction
        ))
    }
}

// ---- /cost ---------------------------------------------------------------

#[async_trait]
impl SlashCommand for CostCommand {
    fn name(&self) -> &str { "cost" }
    fn description(&self) -> &str { "Show token usage and cost for this session" }
    fn help(&self) -> &str {
        "Usage: /cost\n\n\
         Shows per-category token counts and the estimated cost for this session.\n\
         Cache write tokens are priced slightly higher than input; cache read tokens\n\
         are ~10x cheaper — caching reduces cost significantly in long sessions.\n\
         For per-call breakdown use /extra-usage. For account quotas use /usage."
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let tracker = &ctx.cost_tracker;
        let model = ctx.config.effective_model();
        let pricing = claurst_core::cost::ModelPricing::for_model(model);

        let input = tracker.input_tokens();
        let output = tracker.output_tokens();
        let cache_create = tracker.cache_creation_tokens();
        let cache_read = tracker.cache_read_tokens();
        let total = tracker.total_tokens();
        let cost = tracker.total_cost_usd();

        // Per-category cost breakdown.
        let input_cost    = (input as f64 * pricing.input_per_mtk) / 1_000_000.0;
        let output_cost   = (output as f64 * pricing.output_per_mtk) / 1_000_000.0;
        let cc_cost       = (cache_create as f64 * pricing.cache_creation_per_mtk) / 1_000_000.0;
        let cr_cost       = (cache_read as f64 * pricing.cache_read_per_mtk) / 1_000_000.0;

        // Pricing info line.
        let pricing_line = format!(
            "  Rates ($/MTok): input ${:.2} | output ${:.2} | cache-write ${:.3} | cache-read ${:.3}",
            pricing.input_per_mtk,
            pricing.output_per_mtk,
            pricing.cache_creation_per_mtk,
            pricing.cache_read_per_mtk,
        );

        // Cache savings note: how much input cost was avoided by using cache-read
        // instead of re-sending those tokens as normal input.
        let savings = if cache_read > 0 {
            let saved =
                (cache_read as f64 * (pricing.input_per_mtk - pricing.cache_read_per_mtk))
                    / 1_000_000.0;
            format!("\n  Cache savings:  ${:.4}  ({} tokens served from cache)", saved, cache_read)
        } else {
            String::new()
        };

        CommandResult::Message(format!(
            "Session Cost — {model}\n\
             ──────────────────────────────\n\
             {pricing_line}\n\n\
               Input tokens:   {input:>10}   ${input_cost:.4}\n\
               Output tokens:  {output:>10}   ${output_cost:.4}\n\
               Cache write:    {cache_create:>10}   ${cc_cost:.4}\n\
               Cache read:     {cache_read:>10}   ${cr_cost:.4}\n\
             ─────────────────────────────\n\
               Total tokens:   {total:>10}\n\
               Total cost:              ${cost:.4}{savings}\n\n\
             Use /usage for quota info · /extra-usage for per-call breakdown",
            model = model,
            pricing_line = pricing_line,
            input = input,
            input_cost = input_cost,
            output = output,
            output_cost = output_cost,
            cache_create = cache_create,
            cc_cost = cc_cost,
            cache_read = cache_read,
            cr_cost = cr_cost,
            total = total,
            cost = cost,
            savings = savings,
        ))
    }
}

// ---- /exit ---------------------------------------------------------------

#[async_trait]
impl SlashCommand for ExitCommand {
    fn name(&self) -> &str { "exit" }
    fn aliases(&self) -> Vec<&str> { vec!["quit", "q"] }
    fn description(&self) -> &str { "Exit Claurst" }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        CommandResult::Exit
    }
}

// ---- /model --------------------------------------------------------------

#[async_trait]
impl SlashCommand for ModelCommand {
    fn name(&self) -> &str { "model" }
    fn description(&self) -> &str { "Show or change the current model" }
    fn help(&self) -> &str {
        "Usage: /model [<model-id>]\n\n\
         Without arguments, shows the current model.\n\n\
         With a model ID, switches to that model.  Accepts both bare model\n\
         names (e.g. claude-sonnet-4-6) and provider-prefixed format\n\
         (e.g. openai/gpt-4o, google/gemini-2.0-flash).\n\n\
         Examples:\n\
           /model                        — show current model\n\
           /model claude-opus-4-6        — switch to Claude Opus 4.6\n\
           /model openai/gpt-4o          — switch to GPT-4o via OpenAI\n\
           /model google/gemini-2.0-flash — switch to Gemini 2.0 Flash"
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let args = args.trim();
        if args.is_empty() {
            CommandResult::Message(format!(
                "Current model: {}",
                ctx.config.effective_model()
            ))
        } else {
            // Accept both "provider/model" and bare model names.
            // The config stores the full string (including provider prefix when present)
            // so that downstream dispatch can route to the correct provider.
            let model_str = args.to_string();
            let confirmation = if let Some((provider, model)) = model_str.split_once('/') {
                if provider == "anthropic" {
                    format!("Switched to {}", model)
                } else {
                    format!("Switched to {}/{}", provider, model)
                }
            } else {
                format!("Switched to {}", model_str)
            };
            let mut new_config = ctx.config.clone();
            new_config.model = Some(model_str.clone());
            if let Some((provider, _)) = model_str.split_once('/') {
                new_config.provider = Some(provider.to_string());
            }
            CommandResult::ConfigChangeMessage(new_config, confirmation)
        }
    }
}

// ---- /color --------------------------------------------------------------

#[async_trait]
impl SlashCommand for ColorCommand {
    fn name(&self) -> &str { "color" }
    fn description(&self) -> &str { "Set or show the prompt bar color for this session" }
    fn help(&self) -> &str {
        "Usage: /color [<name|#RRGGBB|default>]\n\n\
         Sets the accent color for the prompt bar in this session.\n\
         Named colors: red, green, blue, yellow, cyan, magenta, white, orange, purple\n\
         Hex codes:    #RGB or #RRGGBB\n\
         Reset:        /color default\n\n\
         The color is persisted to ~/.claurst/ui-settings.json and\n\
         applied on the next REPL startup."
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let color = args.trim();
        if color.is_empty() {
            let current = load_ui_settings();
            return CommandResult::Message(format!(
                "Current prompt color: {}\n\
                 Use /color <name|#RRGGBB|default> to change it.\n\n\
                 Named colors: red, green, blue, yellow, cyan, magenta, white, orange, purple",
                current.prompt_color.as_deref().unwrap_or("default"),
            ));
        }

        let normalized = if color == "default" {
            None
        } else {
            let known_colors = [
                "red", "green", "blue", "yellow", "cyan", "magenta",
                "white", "orange", "purple", "pink", "gray", "grey",
            ];
            let is_hex = color.starts_with('#') && (color.len() == 4 || color.len() == 7)
                && color[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !is_hex && !known_colors.contains(&color.to_lowercase().as_str()) {
                return CommandResult::Error(format!(
                    "Unknown color '{}'. Use a color name (red, green, …) or a hex code (#RGB or #RRGGBB).",
                    color
                ));
            }
            Some(color.to_string())
        };

        match mutate_ui_settings(|s| s.prompt_color = normalized.clone()) {
            Ok(_) => CommandResult::Message(format!(
                "Prompt color set to {}.\n\
                 Restart the REPL for the change to take effect.",
                normalized.as_deref().unwrap_or("default")
            )),
            Err(e) => CommandResult::Error(format!("Failed to save color: {}", e)),
        }
    }
}

// ---- /theme --------------------------------------------------------------

#[async_trait]
impl SlashCommand for ThemeCommand {
    fn name(&self) -> &str { "theme" }
    fn description(&self) -> &str { "Show or change the current theme" }
    fn help(&self) -> &str {
        "Usage: /theme [default|dark|light]\n\
         Without arguments, shows the active theme. With an argument, updates the theme for this and future sessions."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let args = args.trim();
        if args.is_empty() {
            return CommandResult::Message(format!(
                "Current theme: {:?}\nUse /theme <default|dark|light> to change it.",
                ctx.config.theme
            ));
        }

        let Some(theme) = parse_theme(args) else {
            return CommandResult::Error(
                "Theme must be one of: default, dark, light".to_string(),
            );
        };

        let mut new_config = ctx.config.clone();
        new_config.theme = theme.clone();
        if let Err(err) = save_settings_mutation(|settings| settings.config.theme = theme.clone())
        {
            return CommandResult::Error(format!("Failed to save theme: {}", err));
        }

        CommandResult::ConfigChangeMessage(
            new_config,
            format!("Theme set to {}.", args.to_lowercase()),
        )
    }
}

// ---- /output-style -------------------------------------------------------

#[async_trait]
impl SlashCommand for OutputStyleCommand {
    fn name(&self) -> &str { "output-style" }
    fn description(&self) -> &str { "Show or switch the current output style" }
    fn help(&self) -> &str {
        "Usage: /output-style [style-name]\n\n\
         With no argument: list available styles and show the current one.\n\
         With a style name: switch to that style (persisted to settings).\n\n\
         Built-in styles: default, verbose, concise\n\
         Plugin-defined styles are listed automatically.\n\n\
         Changes take effect on the next request."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let arg = args.trim();
        let valid_styles = available_output_style_names();
        let current = current_output_style_name(&ctx.config);

        if arg.is_empty() {
            // List available styles
            let mut lines = format!("Current output style: {}\n\nAvailable styles:\n", current);
            for style in &valid_styles {
                let marker = if style == current { " *" } else { "" };
                lines.push_str(&format!("  {}{}\n", style, marker));
            }
            lines.push_str("\nUse /output-style <name> to switch.");
            return CommandResult::Message(lines);
        }

        let normalized = arg.to_lowercase();
        if !valid_styles.iter().any(|name| name == &normalized) {
            return CommandResult::Error(format!(
                "Unknown output style '{}'. Available styles: {}",
                arg,
                valid_styles.join(", ")
            ));
        }

        let mut new_config = ctx.config.clone();
        new_config.output_style = (normalized != "default").then(|| normalized.clone());
        if let Err(err) = save_settings_mutation(|settings| {
            settings.config.output_style =
                (normalized != "default").then(|| normalized.clone());
        }) {
            return CommandResult::Error(format!("Failed to save configuration: {}", err));
        }

        CommandResult::ConfigChangeMessage(
            new_config,
            format!(
                "Output style set to '{}'. Changes take effect on the next request.",
                normalized
            ),
        )
    }
}

// ---- /keybindings --------------------------------------------------------

#[async_trait]
impl SlashCommand for KeybindingsCommand {
    fn name(&self) -> &str { "keybindings" }
    fn description(&self) -> &str { "Create or open ~/.claurst/keybindings.json" }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let config_dir = Settings::config_dir();
        let path = config_dir.join("keybindings.json");
        let existed = path.exists();

        if !existed {
            if let Err(err) = std::fs::create_dir_all(&config_dir) {
                return CommandResult::Error(format!(
                    "Failed to create {}: {}",
                    config_dir.display(),
                    err
                ));
            }

            let template = match generate_keybindings_template() {
                Ok(template) => template,
                Err(err) => {
                    return CommandResult::Error(format!(
                        "Failed to generate keybindings template: {}",
                        err
                    ))
                }
            };

            if let Err(err) = std::fs::write(&path, template) {
                return CommandResult::Error(format!(
                    "Failed to write {}: {}",
                    path.display(),
                    err
                ));
            }
        }

        match open_with_system(&path.display().to_string()) {
            Ok(_) => CommandResult::Message(if existed {
                format!("Opened {} in your editor.", path.display())
            } else {
                format!(
                    "Created {} with a template and opened it in your editor.",
                    path.display()
                )
            }),
            Err(err) => CommandResult::Message(if existed {
                format!(
                    "Opened {}. Could not launch an editor automatically: {}",
                    path.display(),
                    err
                )
            } else {
                format!(
                    "Created {} with a template. Could not launch an editor automatically: {}",
                    path.display(),
                    err
                )
            }),
        }
    }
}

// ---- /privacy-settings ---------------------------------------------------

#[async_trait]
impl SlashCommand for PrivacySettingsCommand {
    fn name(&self) -> &str { "privacy-settings" }
    fn description(&self) -> &str { "Open Claurst privacy settings" }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let url = "https://claude.ai/settings/data-privacy-controls";
        let fallback = format!("Review and manage your privacy settings at {}", url);
        match open_with_system(url) {
            Ok(_) => CommandResult::Message(format!("Opened privacy settings: {}", url)),
            Err(_) => CommandResult::Message(fallback),
        }
    }
}

// ---- /version ------------------------------------------------------------

#[async_trait]
impl SlashCommand for VersionCommand {
    fn name(&self) -> &str { "version" }
    fn aliases(&self) -> Vec<&str> { vec!["v"] }
    fn description(&self) -> &str { "Show version information" }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        CommandResult::Message(format!(
            "Claurst v{}",
            claurst_core::constants::APP_VERSION
        ))
    }
}

// ---- /resume -------------------------------------------------------------

#[async_trait]
impl SlashCommand for ResumeCommand {
    fn name(&self) -> &str { "resume" }
    fn aliases(&self) -> Vec<&str> { vec!["r", "continue"] }
    fn description(&self) -> &str { "Resume a previous conversation" }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        if args.is_empty() {
            let sessions = claurst_core::history::list_sessions().await;
            if sessions.is_empty() {
                return CommandResult::Message("No previous sessions found.".to_string());
            }
            let last = &sessions[0];
            match claurst_core::history::load_session(&last.id).await {
                Ok(session) => CommandResult::ResumeSession(session),
                Err(e) => CommandResult::Error(format!(
                    "Failed to load session {}: {}",
                    last.id, e
                )),
            }
        } else {
            match claurst_core::history::load_session(args.trim()).await {
                Ok(session) => CommandResult::ResumeSession(session),
                Err(e) => CommandResult::Error(format!(
                    "Failed to load session {}: {}",
                    args.trim(),
                    e
                )),
            }
        }
    }
}

// ---- /status -------------------------------------------------------------

#[async_trait]
impl SlashCommand for StatusCommand {
    fn name(&self) -> &str { "status" }
    fn description(&self) -> &str { "Show comprehensive system and session status" }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        // Auth status
        let auth_status = match claurst_core::oauth::OAuthTokens::load().await {
            Some(tokens) => {
                let sub = tokens.subscription_type.as_deref().unwrap_or("oauth");
                format!("Authenticated ({})", sub)
            }
            None => {
                if ctx.config.resolve_api_key().is_some() {
                    "Authenticated (API key)".to_string()
                } else {
                    "Not authenticated".to_string()
                }
            }
        };

        // MCP status
        let mcp_count = ctx.config.mcp_servers.len();
        let mcp_status = if mcp_count == 0 {
            "none configured".to_string()
        } else {
            format!("{} server(s) configured", mcp_count)
        };

        // Hook status
        let hook_count: usize = ctx.config.hooks.values().map(|v| v.len()).sum();

        // UI settings
        let ui = load_ui_settings();
        let editor_mode = ui.editor_mode.as_deref().unwrap_or("normal");
        let fast_mode = ui.fast_mode.unwrap_or(false);

        // Git status
        let git_branch = tokio::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&ctx.working_dir)
            .output()
            .await
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "n/a".to_string());

        CommandResult::Message(format!(
            "Claurst Status\n\
             ══════════════════\n\
             Auth:           {auth_status}\n\
             Model:          {model}\n\
             Permission mode: {perm:?}\n\
             Fast mode:      {fast}\n\
             Editor mode:    {editor}\n\n\
             Session\n\
             ───────\n\
             Session ID:     {sid}\n\
             Title:          {title}\n\
             Messages:       {msgs}\n\
             Working dir:    {wd}\n\
             Git branch:     {branch}\n\n\
             Integrations\n\
             ────────────\n\
             MCP servers:    {mcp}\n\
             Hooks:          {hooks} configured\n\n\
             Usage\n\
             ─────\n\
             {summary}",
            auth_status = auth_status,
            model = ctx.config.effective_model(),
            perm = ctx.config.permission_mode,
            fast = if fast_mode { "on" } else { "off" },
            editor = editor_mode,
            sid = &ctx.session_id[..ctx.session_id.len().min(12)],
            title = ctx.session_title.as_deref().unwrap_or("(untitled)"),
            msgs = ctx.messages.len(),
            wd = ctx.working_dir.display(),
            branch = git_branch,
            mcp = mcp_status,
            hooks = hook_count,
            summary = ctx.cost_tracker.summary(),
        ))
    }
}

// ---- /diff ---------------------------------------------------------------

#[async_trait]
impl SlashCommand for DiffCommand {
    fn name(&self) -> &str { "diff" }
    fn description(&self) -> &str { "Show git diff of changes in the working directory" }
    fn help(&self) -> &str {
        "Usage: /diff [--stat|--staged|<ref>]\n\n\
         Shows git diff output for the current working directory.\n\n\
         Options:\n\
           /diff           — diff of all unstaged changes (git diff)\n\
           /diff --stat    — summary of changed files\n\
           /diff --staged  — diff of staged changes (git diff --cached)\n\
           /diff <ref>     — diff against a branch, tag, or commit"
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let args = args.trim();

        let git_args: Vec<&str> = if args == "--stat" {
            vec!["diff", "--stat"]
        } else if args == "--staged" || args == "--cached" {
            vec!["diff", "--cached"]
        } else if args.is_empty() {
            vec!["diff"]
        } else {
            // Treat as a ref
            vec!["diff", args]
        };

        let output = tokio::process::Command::new("git")
            .args(&git_args)
            .current_dir(&ctx.working_dir)
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() || out.status.code() == Some(1) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.trim().is_empty() {
                    CommandResult::Message(
                        "No changes found. Working tree is clean (or not a git repository)."
                            .to_string(),
                    )
                } else {
                    // Truncate very long diffs
                    let text = stdout.as_ref();
                    let display = if text.len() > 8000 {
                        format!(
                            "{}\n… (truncated — {} total bytes; use `git diff` for full output)",
                            &text[..8000],
                            text.len()
                        )
                    } else {
                        text.to_string()
                    };
                    CommandResult::Message(format!("Changes:\n{}", display))
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                CommandResult::Error(format!(
                    "git diff failed (exit {}): {}",
                    out.status.code().unwrap_or(-1),
                    stderr.trim()
                ))
            }
            Err(e) => CommandResult::Error(format!("Failed to run git diff: {}", e)),
        }
    }
}

// ---- /memory -------------------------------------------------------------

#[async_trait]
impl SlashCommand for MemoryCommand {
    fn name(&self) -> &str { "memory" }
    fn description(&self) -> &str { "View, edit, or clear AGENTS.md memory files" }
    fn help(&self) -> &str {
        "Usage: /memory [edit|clear] [global]\n\n\
         Shows the content of AGENTS.md files that provide project context to Claurst.\n\
         Claurst reads these files automatically at session start.\n\n\
         Subcommands:\n\
           /memory              — show all AGENTS.md files\n\
           /memory edit         — open project AGENTS.md in your editor\n\
           /memory edit global  — open global ~/.claurst/AGENTS.md in your editor\n\
           /memory clear        — clear the project AGENTS.md\n\
           /memory clear global — clear the global ~/.claurst/AGENTS.md\n\n\
         Locations checked (in priority order):\n\
           1. <project>/.claurst/AGENTS.md\n\
           2. <project>/AGENTS.md\n\
           3. ~/.claurst/AGENTS.md  (global)\n\n\
         Use /init to create a new AGENTS.md from a template."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let project_claude_dir = ctx.working_dir.join(".claurst").join("AGENTS.md");
        let project_root = ctx.working_dir.join("AGENTS.md");
        let global_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".claurst")
            .join("AGENTS.md");

        let locations = [
            ("project (.claurst/AGENTS.md)", project_claude_dir.clone()),
            ("project (AGENTS.md)", project_root.clone()),
            ("global (~/.claurst/AGENTS.md)", global_path.clone()),
        ];

        let cmd = args.trim();

        // ---- /memory edit [global|project] ------------------------------------
        if cmd == "edit" || cmd.starts_with("edit ") {
            let target_hint = cmd.strip_prefix("edit").map(|s| s.trim()).unwrap_or("project");
            let target = match target_hint {
                "global" => {
                    // Ensure global dir exists
                    if let Some(parent) = global_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    global_path.clone()
                }
                _ => {
                    // Best project AGENTS.md
                    if project_root.exists() {
                        project_root.clone()
                    } else if project_claude_dir.exists() {
                        project_claude_dir.clone()
                    } else {
                        project_root.clone() // will be created by editor
                    }
                }
            };
            // Create file if it doesn't exist yet
            if !target.exists() {
                if let Some(parent) = target.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&target, "");
            }
            let editor = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| {
                    if cfg!(target_os = "windows") {
                        "notepad".to_string()
                    } else {
                        "vi".to_string()
                    }
                });
            let editor_hint = if let Ok(visual) = std::env::var("VISUAL") {
                format!("Using $VISUAL=\"{}\".", visual)
            } else if let Ok(ed) = std::env::var("EDITOR") {
                format!("Using $EDITOR=\"{}\".", ed)
            } else {
                "To use a different editor, set the $EDITOR or $VISUAL environment variable.".to_string()
            };
            let spawn_result = std::process::Command::new(&editor)
                .arg(&target)
                .status();
            return match spawn_result {
                Ok(_) => CommandResult::Message(format!(
                    "Opened {} in your editor.\n{}",
                    target.display(),
                    editor_hint
                )),
                Err(e) => CommandResult::Message(format!(
                    "Could not launch '{}': {}. Edit {} manually.\n{}",
                    editor, e, target.display(), editor_hint
                )),
            };
        }

        // ---- /memory clear [global|project] -----------------------------------
        if cmd == "clear" || cmd.starts_with("clear ") {
            let target_hint = cmd.strip_prefix("clear").map(|s| s.trim()).unwrap_or("project");
            let (label, target) = match target_hint {
                "global" => ("global (~/.claurst/AGENTS.md)", global_path.clone()),
                _ => {
                    if project_claude_dir.exists() {
                        ("project (.claurst/AGENTS.md)", project_claude_dir.clone())
                    } else {
                        ("project (AGENTS.md)", project_root.clone())
                    }
                }
            };
            if !target.exists() {
                return CommandResult::Message(format!(
                    "No {} memory file found (nothing to clear).",
                    label
                ));
            }
            return match tokio::fs::write(&target, "").await {
                Ok(_) => CommandResult::Message(format!(
                    "Cleared {} memory file at {}.\n\
                     Claurst will no longer see this content at session start.",
                    label,
                    target.display()
                )),
                Err(e) => CommandResult::Error(format!(
                    "Failed to clear {}: {}", target.display(), e
                )),
            };
        }

        // ---- /memory (show all) -----------------------------------------------
        let mut output = String::from("AGENTS.md Memory Files\n══════════════════════\n");
        let mut found_any = false;

        for (label, path) in &locations {
            if path.exists() {
                found_any = true;
                match tokio::fs::read_to_string(path).await {
                    Ok(content) => {
                        let lines: usize = content.lines().count();
                        let chars = content.len();
                        output.push_str(&format!(
                            "\n[{label}]\nPath: {path}\nSize: {lines} lines, {chars} chars\n\
                             ─────────────────────────────────\n\
                             {content}\n",
                            label = label,
                            path = path.display(),
                            lines = lines,
                            chars = chars,
                            content = if content.len() > 2000 {
                                format!("{}…\n(truncated — file is {} chars)", &content[..2000], chars)
                            } else {
                                content.clone()
                            }
                        ));
                    }
                    Err(e) => output.push_str(&format!(
                        "\n[{label}] — Error reading {}: {}\n",
                        path.display(), e, label = label
                    )),
                }
            }
        }

        if !found_any {
            output.push_str(
                "\nNo AGENTS.md files found.\n\
                 Use /init to create one in the current project.\n\
                 Use /memory edit to create and open a memory file."
            );
        } else {
            output.push_str(
                "\nSubcommands:\n\
                 /memory edit          — edit project AGENTS.md\n\
                 /memory edit global   — edit global ~/.claurst/AGENTS.md\n\
                 /memory clear         — clear project AGENTS.md\n\
                 /memory clear global  — clear global AGENTS.md"
            );
        }

        CommandResult::Message(output)
    }
}

// ---- /bug ----------------------------------------------------------------

#[async_trait]
impl SlashCommand for BugCommand {
    fn name(&self) -> &str { "feedback" }
    fn aliases(&self) -> Vec<&str> { vec!["bug"] }
    fn description(&self) -> &str { "Submit feedback about Claurst" }
    fn help(&self) -> &str { "Usage: /feedback [report]" }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let report = args.trim();
        if report.is_empty() {
            CommandResult::Message(
                "To submit feedback or report a bug, visit: https://github.com/anthropics/claude-code/issues"
                    .to_string(),
            )
        } else {
            CommandResult::Message(format!(
                "To submit feedback or report a bug, visit: https://github.com/anthropics/claude-code/issues\nSuggested report summary: {}",
                report
            ))
        }
    }
}

// ---- /usage --------------------------------------------------------------

#[async_trait]
impl SlashCommand for UsageCommand {
    fn name(&self) -> &str { "usage" }
    fn description(&self) -> &str { "Show API usage, quotas, and rate limit status" }
    fn help(&self) -> &str {
        "Usage: /usage\n\n\
         Shows current session API usage and account quota information.\n\
         For detailed per-call breakdown, use /extra-usage.\n\
         For cost details, use /cost."
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let input = ctx.cost_tracker.input_tokens();
        let output = ctx.cost_tracker.output_tokens();
        let cache_creation = ctx.cost_tracker.cache_creation_tokens();
        let cache_read = ctx.cost_tracker.cache_read_tokens();
        let total = ctx.cost_tracker.total_tokens();
        let cost = ctx.cost_tracker.total_cost_usd();

        // Try to get account tier from OAuth tokens
        let account_info = match claurst_core::oauth::OAuthTokens::load().await {
            Some(tokens) => {
                let sub = tokens.subscription_type.as_deref().unwrap_or("unknown");
                format!("Plan: {}", sub)
            }
            None => {
                if ctx.config.resolve_api_key().is_some() {
                    "Plan: API key (Console billing)".to_string()
                } else {
                    "Plan: not authenticated — run /login".to_string()
                }
            }
        };

        CommandResult::Message(format!(
            "API Usage — Current Session\n\
             ────────────────────────────\n\
             {account_info}\n\
             Model:          {model}\n\n\
             Tokens used this session:\n\
               Input:        {input:>10}\n\
               Output:       {output:>10}\n\
               Cache write:  {cache_creation:>10}\n\
               Cache read:   {cache_read:>10}\n\
               Total:        {total:>10}\n\n\
             Estimated cost: ${cost:.4}\n\n\
             Use /extra-usage for per-call breakdown.\n\
             Use /rate-limit-options to see your plan limits.",
            account_info = account_info,
            model = ctx.config.effective_model(),
            input = input,
            output = output,
            cache_creation = cache_creation,
            cache_read = cache_read,
            total = total,
            cost = cost,
        ))
    }
}

// ---- /init ---------------------------------------------------------------

#[async_trait]
impl SlashCommand for InitCommand {
    fn name(&self) -> &str { "init" }
    fn description(&self) -> &str { "Initialize a new project with AGENTS.md" }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let path = ctx.working_dir.join("AGENTS.md");
        if path.exists() {
            return CommandResult::Message(format!(
                "AGENTS.md already exists at {}",
                path.display()
            ));
        }

        let default_content = "# Project Instructions\n\n\
            Add project-specific instructions and context here.\n\n\
            ## Guidelines\n\n\
            - Describe your project structure\n\
            - Note any coding conventions\n\
            - List important files and their purposes\n";

        match tokio::fs::write(&path, default_content).await {
            Ok(()) => CommandResult::Message(format!(
                "Created AGENTS.md at {}",
                path.display()
            )),
            Err(e) => CommandResult::Error(format!("Failed to create AGENTS.md: {}", e)),
        }
    }
}

// ---- /import-config ------------------------------------------------------

#[async_trait]
impl SlashCommand for ImportConfigCommand {
    fn name(&self) -> &str { "import-config" }
    fn description(&self) -> &str { "Import CLAUDE.md and settings.json from ~/.claude" }
    fn help(&self) -> &str {
        "Usage: /import-config\n\
         Import user-level Claude Code configuration from ~/.claude:\n\
           - ~/.claude/CLAUDE.md\n\
           - ~/.claude/settings.json\n\n\
         This command opens an interactive import dialog with preview and confirmation."
    }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        CommandResult::OpenImportConfigOverlay
    }
}

// ---- /hooks --------------------------------------------------------------

#[async_trait]
impl SlashCommand for HooksCommand {
    fn name(&self) -> &str { "hooks" }
    fn description(&self) -> &str { "Show configured event hooks" }
    fn help(&self) -> &str {
        "Usage: /hooks\n\
         Show hooks configured in settings.json under 'hooks'.\n\
         Hooks fire shell commands on events: PreToolUse, PostToolUse, Stop, UserPromptSubmit."
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        // In TUI mode this command is intercepted by intercept_slash_command("hooks")
        // before execute() is ever called, so this path only runs in non-TUI
        // contexts (e.g., `claude hooks` on the CLI, pipes, or tests).
        //
        // Signal to the CLI driver that it should open the TUI overlay if possible;
        // the CLI will fall back to the text listing when no TUI is active.
        if ctx.config.hooks.is_empty() {
            // If there is nothing to show in the overlay, emit a helpful message
            // so the user knows what to do.
            return CommandResult::Message(
                "No hooks configured.\n\
                 Add hooks to ~/.claurst/settings.json under the 'hooks' key.\n\
                 Example:\n\
                 \x20 \"hooks\": {\n\
                 \x20   \"PreToolUse\": [{ \"matcher\": \"*\", \"hooks\": [{ \"type\": \"command\", \"command\": \"echo $STDIN\" }] }]\n\
                 \x20 }"
                    .to_string(),
            );
        }

        // Return the overlay-open signal; the CLI driver will call
        // app.hooks_config_menu.open() or fall back to text output if running
        // without a TUI.
        CommandResult::OpenHooksOverlay
    }
}

// ---- /permissions --------------------------------------------------------

#[async_trait]
impl SlashCommand for PermissionsCommand {
    fn name(&self) -> &str { "permissions" }
    fn description(&self) -> &str { "View or change tool permission settings" }
    fn help(&self) -> &str {
        "Usage: /permissions [set <mode>|allow <tool>|deny <tool>|reset]\n\n\
         Modes: default, accept-edits, bypass-permissions, plan\n\n\
         Examples:\n\
           /permissions                    — show current permissions\n\
           /permissions set accept-edits   — auto-accept file edits\n\
           /permissions allow Bash         — allow a specific tool\n\
           /permissions deny Write         — deny a specific tool\n\
           /permissions reset              — clear overrides"
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let args = args.trim();

        if args.is_empty() {
            let allowed_display = if ctx.config.allowed_tools.is_empty() {
                "(all tools allowed)".to_string()
            } else {
                ctx.config.allowed_tools.join(", ")
            };
            let denied_display = if ctx.config.disallowed_tools.is_empty() {
                "(none)".to_string()
            } else {
                ctx.config.disallowed_tools.join(", ")
            };
            return CommandResult::Message(format!(
                "Permission Settings\n\
                 ───────────────────\n\
                 Mode:          {:?}\n\
                 Allowed tools: {}\n\
                 Denied tools:  {}\n\n\
                 Use /permissions set <mode> to change the permission mode.\n\
                 Use /permissions allow|deny <tool> to override individual tools.\n\
                 Use /permissions reset to clear all overrides.",
                ctx.config.permission_mode,
                allowed_display,
                denied_display,
            ));
        }

        let mut parts = args.splitn(2, ' ');
        let sub = parts.next().unwrap_or("").trim();
        let arg = parts.next().unwrap_or("").trim();

        match sub {
            "set" => {
                let mode = match arg.to_lowercase().as_str() {
                    "default" => claurst_core::config::PermissionMode::Default,
                    "accept-edits" | "accept_edits" => claurst_core::config::PermissionMode::AcceptEdits,
                    "bypass-permissions" | "bypass_permissions" => claurst_core::config::PermissionMode::BypassPermissions,
                    "plan" => claurst_core::config::PermissionMode::Plan,
                    _ => return CommandResult::Error(
                        "Mode must be: default, accept-edits, bypass-permissions, or plan".to_string()
                    ),
                };
                let mut new_config = ctx.config.clone();
                new_config.permission_mode = mode.clone();
                if let Err(e) = save_settings_mutation(|s| s.config.permission_mode = mode.clone()) {
                    return CommandResult::Error(format!("Failed to save: {}", e));
                }
                CommandResult::ConfigChangeMessage(
                    new_config,
                    format!("Permission mode set to {:?}.", mode),
                )
            }
            "allow" => {
                if arg.is_empty() {
                    return CommandResult::Error("Usage: /permissions allow <tool>".to_string());
                }
                let tool = arg.to_string();
                let mut new_config = ctx.config.clone();
                if !new_config.allowed_tools.contains(&tool) {
                    new_config.allowed_tools.push(tool.clone());
                }
                new_config.disallowed_tools.retain(|t| t != &tool);
                if let Err(e) = save_settings_mutation(|s| {
                    if !s.config.allowed_tools.contains(&tool) {
                        s.config.allowed_tools.push(tool.clone());
                    }
                    s.config.disallowed_tools.retain(|t| t != &tool);
                }) {
                    return CommandResult::Error(format!("Failed to save: {}", e));
                }
                CommandResult::ConfigChangeMessage(new_config, format!("Allowed tool: {}", tool))
            }
            "deny" => {
                if arg.is_empty() {
                    return CommandResult::Error("Usage: /permissions deny <tool>".to_string());
                }
                let tool = arg.to_string();
                let mut new_config = ctx.config.clone();
                if !new_config.disallowed_tools.contains(&tool) {
                    new_config.disallowed_tools.push(tool.clone());
                }
                new_config.allowed_tools.retain(|t| t != &tool);
                if let Err(e) = save_settings_mutation(|s| {
                    if !s.config.disallowed_tools.contains(&tool) {
                        s.config.disallowed_tools.push(tool.clone());
                    }
                    s.config.allowed_tools.retain(|t| t != &tool);
                }) {
                    return CommandResult::Error(format!("Failed to save: {}", e));
                }
                CommandResult::ConfigChangeMessage(new_config, format!("Denied tool: {}", tool))
            }
            "reset" => {
                let mut new_config = ctx.config.clone();
                new_config.allowed_tools.clear();
                new_config.disallowed_tools.clear();
                new_config.permission_mode = claurst_core::config::PermissionMode::Default;
                if let Err(e) = save_settings_mutation(|s| {
                    s.config.allowed_tools.clear();
                    s.config.disallowed_tools.clear();
                    s.config.permission_mode = claurst_core::config::PermissionMode::Default;
                }) {
                    return CommandResult::Error(format!("Failed to save: {}", e));
                }
                CommandResult::ConfigChangeMessage(
                    new_config,
                    "Permissions reset to defaults.".to_string(),
                )
            }
            other => CommandResult::Error(format!(
                "Unknown subcommand '{}'. Use: /permissions [set|allow|deny|reset]",
                other
            )),
        }
    }
}

// ---- /plan ---------------------------------------------------------------

#[async_trait]
impl SlashCommand for PlanCommand {
    fn name(&self) -> &str { "plan" }
    fn description(&self) -> &str { "Enter plan mode – model outputs a plan for approval before acting" }
    fn help(&self) -> &str {
        "Usage: /plan [description]\n\n\
         Switches to plan mode where the model will create a detailed plan before executing.\n\
         The plan must be approved before any file writes or command executions are performed.\n\
         Use /plan exit to leave plan mode."
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        if args.trim() == "exit" {
            return CommandResult::UserMessage(
                "[Exiting plan mode. Resuming normal execution.]".to_string()
            );
        }
        let task_desc = if args.is_empty() {
            "the current task".to_string()
        } else {
            args.to_string()
        };
        CommandResult::UserMessage(format!(
            "[Entering plan mode for: {}]\n\
             Please create a detailed step-by-step plan. Do not execute any commands or \
             write any files until the plan has been reviewed and approved.",
            task_desc
        ))
    }
}

// ---- /tasks --------------------------------------------------------------

#[async_trait]
impl SlashCommand for TasksCommand {
    fn name(&self) -> &str { "tasks" }
    fn aliases(&self) -> Vec<&str> { vec!["bashes"] }
    fn description(&self) -> &str { "List and manage background tasks" }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        CommandResult::UserMessage(
            "Please list all current tasks using the TaskList tool and show their status.".to_string()
        )
    }
}

// ---- /session ------------------------------------------------------------

#[async_trait]
impl SlashCommand for SessionCommand {
    fn name(&self) -> &str { "session" }
    fn aliases(&self) -> Vec<&str> { vec!["remote"] }
    fn description(&self) -> &str { "Show or manage conversation sessions" }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        match args.trim() {
            "list" => {
                let sessions = claurst_core::history::list_sessions().await;
                if sessions.is_empty() {
                    CommandResult::Message("No saved sessions found.".to_string())
                } else {
                    let mut output = String::from("Recent sessions:\n\n");
                    for sess in sessions.iter().take(10) {
                        let updated = sess.updated_at.format("%Y-%m-%d %H:%M").to_string();
                        let id_short = &sess.id[..sess.id.len().min(8)];
                        output.push_str(&format!(
                            "  {} | {} | {} messages | {}\n",
                            id_short,
                            updated,
                            sess.messages.len(),
                            sess.title.as_deref().unwrap_or("(untitled)")
                        ));
                    }
                    output.push_str("\nUse /resume <id> to resume a session.");
                    CommandResult::Message(output)
                }
            }
            "" => {
                // If a bridge remote URL is active, show it prominently.
                if let Some(ref url) = ctx.remote_session_url {
                    let border = "─".repeat(url.len().min(60) + 4);
                    let display_url = if url.len() > 60 {
                        format!("{}…", &url[..60])
                    } else {
                        url.clone()
                    };
                    CommandResult::Message(format!(
                        "Remote session active\n\
                         ┌{border}┐\n\
                         │  {display_url}  │\n\
                         └{border}┘\n\n\
                         Open the URL above on any device to connect remotely.\n\
                         Session ID: {}",
                        ctx.session_id,
                    ))
                } else {
                    // Show current session info + recent sessions list.
                    let sessions = claurst_core::history::list_sessions().await;
                    let mut output = format!(
                        "Current session\n\
                         ───────────────\n\
                         ID:       {}\n\
                         Title:    {}\n\
                         Messages: {}\n\
                         Model:    {}\n",
                        ctx.session_id,
                        ctx.session_title.as_deref().unwrap_or("(untitled)"),
                        ctx.messages.len(),
                        ctx.config.effective_model()
                    );

                    if !sessions.is_empty() {
                        output.push_str("\nRecent sessions:\n\n");
                        for sess in sessions.iter().take(5) {
                            let updated = sess.updated_at.format("%Y-%m-%d %H:%M").to_string();
                            let id_short = &sess.id[..sess.id.len().min(8)];
                            let marker = if sess.id == ctx.session_id { " ◀ current" } else { "" };
                            output.push_str(&format!(
                                "  {} | {} | {} messages | {}{}\n",
                                id_short,
                                updated,
                                sess.messages.len(),
                                sess.title.as_deref().unwrap_or("(untitled)"),
                                marker,
                            ));
                        }
                        output.push_str("\nUse /session list for all sessions, /resume <id> to switch.");
                    }

                    CommandResult::Message(output)
                }
            }
            _ => CommandResult::Error(format!("Unknown subcommand: {}\n\nUsage: /session [list]", args)),
        }
    }
}

// ---- /fork ---------------------------------------------------------------

#[async_trait]
impl SlashCommand for ForkCommand {
    fn name(&self) -> &str { "fork" }
    fn description(&self) -> &str { "Fork the current session into a new branch" }
    fn help(&self) -> &str {
        "Usage: /fork [message_index]\n\n\
         Fork the current session at the specified message index (or at the\n\
         current point if no index is given).  Creates a new session containing\n\
         messages up to the fork point.\n\n\
         Examples:\n\
           /fork        \u{2014} fork at the current end of the conversation\n\
           /fork 5      \u{2014} fork after message 5"
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let fork_index: Option<usize> = args.trim().parse().ok();
        let messages = &ctx.messages;
        let fork_at = fork_index.unwrap_or(messages.len()).min(messages.len());
        let forked_messages: Vec<_> = messages[..fork_at].to_vec();

        let mut new_session = claurst_core::history::ConversationSession::new(
            ctx.config.effective_model().to_string(),
        );
        new_session.messages = forked_messages;
        new_session.parent_session_id = Some(ctx.session_id.clone());
        new_session.fork_point_message_index = Some(fork_at);
        new_session.title = Some(format!(
            "Fork of {}",
            ctx.session_title.as_deref().unwrap_or("session")
        ));
        new_session.working_dir = Some(
            ctx.working_dir.to_string_lossy().to_string(),
        );

        let new_id = new_session.id.clone();
        match claurst_core::history::save_session(&new_session).await {
            Ok(()) => CommandResult::Message(format!(
                "Session forked at message {}. New session: {}\nUse /resume {} to switch to it.",
                fork_at, new_id, new_id
            )),
            Err(e) => CommandResult::Error(format!("Failed to save forked session: {}", e)),
        }
    }
}

// ---- /thinking -----------------------------------------------------------

#[async_trait]
impl SlashCommand for ThinkingCommand {
    fn name(&self) -> &str { "thinking" }
    fn description(&self) -> &str { "Toggle extended thinking mode" }
    fn aliases(&self) -> Vec<&str> { vec!["think"] }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        // Extended thinking is configured through the model; just inform the user
        let model = ctx.config.effective_model();
        if model.contains("claude-3-5") || model.contains("claude-3.5") {
            CommandResult::Message(
                "Extended thinking is not available for Claude 3.5 models.\n\
                 Use claude-opus-4-6 or claude-sonnet-4-6 for extended thinking.".to_string()
            )
        } else {
            CommandResult::Message(format!(
                "Extended thinking is available with {}.\n\
                 You can request thinking by asking Claurst to 'think step by step' or \
                 'think carefully before answering'.",
                model
            ))
        }
    }
}

// ---- /skills -------------------------------------------------------------

#[async_trait]
impl SlashCommand for SkillsCommand {
    fn name(&self) -> &str { "skills" }
    fn aliases(&self) -> Vec<&str> { vec!["skill"] }
    fn description(&self) -> &str { "List available skills in .claurst/commands/" }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let mut found: Vec<String> = Vec::new();
        let dirs = [
            ctx.working_dir.join(".claurst").join("commands"),
            dirs::home_dir()
                .unwrap_or_default()
                .join(".claurst")
                .join("commands"),
        ];

        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map_or(false, |e| e == "md") {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            let name = stem.to_string();
                            if !found.contains(&name) {
                                found.push(name);
                            }
                        }
                    }
                }
            }
        }

        // Include skills contributed by installed plugins.
        if let Some(registry) = claurst_plugins::global_plugin_registry() {
            for skill_dir in registry.all_skill_paths() {
                if let Ok(entries) = std::fs::read_dir(&skill_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        // Skills can be individual .md files or subdirs with SKILL.md.
                        if p.is_dir() {
                            if p.join("SKILL.md").exists() || p.join("skill.md").exists() {
                                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                    let skill_name = name.to_string();
                                    if !found.contains(&skill_name) {
                                        found.push(skill_name);
                                    }
                                }
                            }
                        } else if p.extension().map_or(false, |e| e == "md") {
                            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                                let name = stem.to_string();
                                if !found.contains(&name) {
                                    found.push(name);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Include discovered skills from .claurst/skills/ and configured paths/URLs.
        let discovered = claurst_core::discover_skills(
            &ctx.working_dir,
            &ctx.config.skills,
        );

        let mut output = if found.is_empty() && discovered.is_empty() {
            return CommandResult::Message(
                "No skills found.\nCreate .md files in .claurst/commands/ to define skills.\n\
                 Example: .claurst/commands/review.md".to_string(),
            );
        } else if found.is_empty() {
            String::new()
        } else {
            found.sort();
            format!(
                "Available skills ({}):\n{}",
                found.len(),
                found.iter().map(|s| format!("  /{}", s)).collect::<Vec<_>>().join("\n")
            )
        };

        if !discovered.is_empty() {
            let mut disc_list: Vec<(&String, &claurst_core::DiscoveredSkill)> =
                discovered.iter().collect();
            disc_list.sort_by_key(|(name, _)| name.as_str());

            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&format!("\nDiscovered skills ({}):\n", disc_list.len()));
            for (name, skill) in disc_list {
                output.push_str(&format!(
                    "  /{} — {} ({})\n",
                    name,
                    skill.description,
                    skill.source_path.display()
                ));
            }
        }

        CommandResult::Message(output.trim_end().to_string())
    }
}

// ---- /rewind -------------------------------------------------------------

#[async_trait]
impl SlashCommand for RewindCommand {
    fn name(&self) -> &str { "rewind" }
    fn description(&self) -> &str { "Interactively select a message to rewind to" }
    fn help(&self) -> &str {
        "Usage: /rewind\n\
         Opens an interactive overlay to select the message to rewind to.\n\
         Use ↑↓ to navigate, Enter to select, y/n to confirm."
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        if ctx.messages.is_empty() {
            return CommandResult::Message("Nothing to rewind — conversation is empty.".to_string());
        }
        CommandResult::OpenRewindOverlay
    }
}

// ---- /stats --------------------------------------------------------------

#[async_trait]
impl SlashCommand for StatsCommand {
    fn name(&self) -> &str { "stats" }
    fn description(&self) -> &str { "Show token usage and cost statistics" }
    fn help(&self) -> &str {
        "Usage: /stats\n\n\
         Shows detailed token usage and cost breakdown for the current session,\n\
         including cache creation/read token counts, turn counts, and session duration.\n\
         Use /usage for quota and account info. Use /cost for a quick cost summary."
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let input = ctx.cost_tracker.input_tokens();
        let output = ctx.cost_tracker.output_tokens();
        let cache_creation = ctx.cost_tracker.cache_creation_tokens();
        let cache_read = ctx.cost_tracker.cache_read_tokens();
        let total = ctx.cost_tracker.total_tokens();
        let cost = ctx.cost_tracker.total_cost_usd();
        let model = ctx.config.effective_model();

        // Count user/assistant turns separately.
        let user_turns = ctx.messages.iter()
            .filter(|m| m.role == claurst_core::types::Role::User)
            .count();
        let assistant_turns = ctx.messages.iter()
            .filter(|m| m.role == claurst_core::types::Role::Assistant)
            .count();

        // Count tool-use invocations.
        let tool_calls: usize = ctx.messages.iter()
            .map(|m| m.get_tool_use_blocks().len())
            .sum();

        // Cost breakdown note: cache-read tokens are cheaper than input, and
        // cache-creation tokens are slightly more expensive. Provide a note if
        // caching is active.
        let cache_note = if cache_creation > 0 || cache_read > 0 {
            format!(
                "\n  (Cache write: {:>10}    Cache read: {:>10})",
                cache_creation, cache_read
            )
        } else {
            String::new()
        };

        CommandResult::Message(format!(
            "Session Statistics\n\
             ══════════════════\n\
             Model:          {model}\n\
             \n\
             Conversation:\n\
               User turns:     {user_turns:>10}\n\
               Assistant turns:{assistant_turns:>10}\n\
               Tool calls:     {tool_calls:>10}\n\
             \n\
             Token usage:\n\
               Input:          {input:>10}\n\
               Output:         {output:>10}\n\
               Total:          {total:>10}{cache_note}\n\
             \n\
             Estimated cost:   ${cost:.4}\n\
             \n\
             Use /usage for quota info · /cost for quick cost · /extra-usage for per-call breakdown",
            model = model,
            user_turns = user_turns,
            assistant_turns = assistant_turns,
            tool_calls = tool_calls,
            input = input,
            output = output,
            total = total,
            cache_note = cache_note,
            cost = cost,
        ))
    }
}

// ---- /files --------------------------------------------------------------

#[async_trait]
impl SlashCommand for FilesCommand {
    fn name(&self) -> &str { "files" }
    fn description(&self) -> &str { "List files referenced in the current conversation" }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        use std::collections::HashSet;
        // Scan message content for file paths (simple heuristic)
        let mut files: HashSet<String> = HashSet::new();
        let path_re = regex::Regex::new(r#"(?m)([A-Za-z]:[\\/][^\s,;:"'<>]+|/[^\s,;:"'<>]{3,})"#).ok();

        for msg in &ctx.messages {
            let text = msg.get_all_text();
            if let Some(ref re) = path_re {
                for cap in re.captures_iter(&text) {
                    let path = cap[1].trim().to_string();
                    if std::path::Path::new(&path).exists() {
                        files.insert(path);
                    }
                }
            }
        }

        if files.is_empty() {
            return CommandResult::Message(
                "No referenced files detected in the conversation.".to_string(),
            );
        }

        let mut sorted: Vec<String> = files.into_iter().collect();
        sorted.sort();

        CommandResult::Message(format!(
            "Referenced files ({}):\n{}",
            sorted.len(),
            sorted.iter().map(|f| format!("  {}", f)).collect::<Vec<_>>().join("\n")
        ))
    }
}

// ---- /rename -------------------------------------------------------------

#[async_trait]
impl SlashCommand for RenameCommand {
    fn name(&self) -> &str { "rename" }
    fn description(&self) -> &str { "Rename the current session" }
    fn help(&self) -> &str {
        "Usage: /rename [new name]\n\n\
         With a name: sets the session title immediately.\n\
         With no argument: auto-generates a kebab-case name from the conversation.\n\n\
         Examples:\n\
           /rename fix-login-bug\n\
           /rename              — auto-generate from conversation history"
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let name = args.trim();

        if !name.is_empty() {
            // Explicit name provided: rename immediately.
            return CommandResult::RenameSession(name.to_string());
        }

        // No name given — auto-generate from conversation context.
        if ctx.messages.is_empty() {
            return CommandResult::Error(
                "No conversation context yet. Usage: /rename <name>".to_string(),
            );
        }

        // Build a short conversation excerpt (up to ~2000 chars) for the model.
        let excerpt: String = ctx
            .messages
            .iter()
            .take(20)
            .filter_map(|m| {
                let text = m.get_all_text();
                if text.is_empty() { return None; }
                let role = match m.role {
                    claurst_core::types::Role::User => "User",
                    claurst_core::types::Role::Assistant => "Assistant",
                };
                Some(format!("{}: {}", role, text.chars().take(300).collect::<String>()))
            })
            .collect::<Vec<_>>()
            .join("\n");

        if excerpt.is_empty() {
            return CommandResult::Error(
                "No text content in conversation. Usage: /rename <name>".to_string(),
            );
        }

        let provider = match provider_for_config(&ctx.config).await {
            Some(provider) => provider,
            None => {
                return CommandResult::Error(
                    "Could not create a provider client for auto-naming.\n\
                     Use /rename <name> to set the name manually."
                        .to_string(),
                );
            }
        };
        let rename_model = resolve_fast_model_id(&ctx.config);

        let system_prompt = "Generate a short kebab-case name (2-4 words) that captures the \
            main topic of this conversation. Use lowercase words separated by hyphens. \
            Examples: fix-login-bug, add-auth-feature, refactor-api-client. \
            Respond with ONLY the name, nothing else.";

        let request = claurst_api::ProviderRequest {
            model: rename_model,
            messages: vec![Message::user(format!(
                "Conversation to name:\n\n{}",
                &excerpt[..excerpt.len().min(2000)]
            ))],
            system_prompt: Some(claurst_api::SystemPrompt::Text(system_prompt.to_string())),
            tools: vec![],
            max_tokens: 64,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: vec![],
            thinking: None,
            provider_options: serde_json::Value::Object(Default::default()),
        };

        match provider.create_message(request).await {
            Ok(response) => {
                let raw_text = text_from_content_blocks(&response.content).trim().to_string();

                let generated = raw_text
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-')
                    .collect::<String>();

                // Trim leading/trailing hyphens and ensure non-empty.
                let cleaned = generated.trim_matches('-').to_string();
                if cleaned.is_empty() {
                    return CommandResult::Error(
                        "Could not generate a valid name from conversation. \
                         Use /rename <name> to set manually.".to_string(),
                    );
                }

                CommandResult::RenameSession(cleaned)
            }
            Err(e) => CommandResult::Error(format!(
                "Auto-name generation failed: {e}\n\
                 Use /rename <name> to set the name manually."
            )),
        }
    }
}

// ---- /effort -------------------------------------------------------------

#[async_trait]
impl SlashCommand for EffortCommand {
    fn name(&self) -> &str { "effort" }
    fn description(&self) -> &str { "Set the model's thinking effort (low | normal | high)" }
    fn help(&self) -> &str {
        "Usage: /effort [low|normal|high]\n\
         Sets how much computation the model uses for reasoning.\n\
         'high' enables extended thinking with a larger budget."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        match args.trim() {
            "" => CommandResult::Message(format!(
                "Current effort: normal\nUse /effort [low|normal|high] to change."
            )),
            "low" => {
                // Low effort: smaller max_tokens
                ctx.config.max_tokens = Some(4096);
                CommandResult::ConfigChange(ctx.config.clone())
            }
            "normal" => {
                ctx.config.max_tokens = None; // use default
                CommandResult::ConfigChange(ctx.config.clone())
            }
            "high" => {
                ctx.config.max_tokens = Some(32768);
                CommandResult::ConfigChange(ctx.config.clone())
            }
            other => CommandResult::Error(format!(
                "Unknown effort level '{}'. Use: low | normal | high",
                other
            )),
        }
    }
}

// ---- /summary ------------------------------------------------------------

#[async_trait]
impl SlashCommand for SummaryCommand {
    fn name(&self) -> &str { "summary" }
    fn description(&self) -> &str { "Generate a brief summary of the conversation so far" }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let count = ctx.messages.len();
        if count == 0 {
            return CommandResult::Message("No messages in conversation yet.".to_string());
        }

        // Ask the model to summarize by injecting a hidden user message
        CommandResult::UserMessage(
            "Please provide a brief (3-5 sentence) summary of our conversation so far, \
             focusing on what has been accomplished and the current state."
                .to_string(),
        )
    }
}

// ---- /commit -------------------------------------------------------------

#[async_trait]
impl SlashCommand for CommitCommand {
    fn name(&self) -> &str { "commit" }
    fn description(&self) -> &str { "Ask Claurst to commit staged changes" }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let extra = if args.trim().is_empty() {
            String::new()
        } else {
            format!(" with message: {}", args.trim())
        };

        CommandResult::UserMessage(format!(
            "Please commit the currently staged git changes{}. \
             Run `git diff --cached` to see what's staged, \
             write an appropriate commit message following the repository's conventions, \
             and run `git commit`.",
            extra
        ))
    }
}

// ---------------------------------------------------------------------------
// UI settings helpers (stored in ~/.claurst/ui-settings.json)
// These hold things not present in the core Config struct.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct UiSettings {
    #[serde(default)]
    pub editor_mode: Option<String>,       // "vim" or "normal"
    #[serde(default)]
    pub fast_mode: Option<bool>,
    #[serde(default)]
    pub voice_enabled: Option<bool>,
    #[serde(default)]
    pub statusline_show_cost: Option<bool>,
    #[serde(default)]
    pub statusline_show_tokens: Option<bool>,
    #[serde(default)]
    pub statusline_show_model: Option<bool>,
    #[serde(default)]
    pub statusline_show_time: Option<bool>,
    #[serde(default)]
    pub prompt_color: Option<String>,
    #[serde(default)]
    pub sandbox_mode: Option<bool>,
    /// Shell command patterns excluded from sandboxing (glob-style strings).
    /// Mirrors TS `excludedCommands` in settings.local.json.
    #[serde(default)]
    pub sandbox_excluded_commands: Vec<String>,
}

fn ui_settings_path() -> std::path::PathBuf {
    claurst_core::config::Settings::config_dir().join("ui-settings.json")
}

fn load_ui_settings() -> UiSettings {
    let path = ui_settings_path();
    if !path.exists() {
        return UiSettings::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_ui_settings(settings: &UiSettings) -> anyhow::Result<()> {
    let path = ui_settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(&path, json)?;
    Ok(())
}

fn mutate_ui_settings<F>(f: F) -> anyhow::Result<UiSettings>
where
    F: FnOnce(&mut UiSettings),
{
    let mut s = load_ui_settings();
    f(&mut s);
    save_ui_settings(&s)?;
    Ok(s)
}

// ---- /remote-control (/rc) -----------------------------------------------

#[async_trait]
impl SlashCommand for RemoteControlCommand {
    fn name(&self) -> &str { "remote-control" }
    fn aliases(&self) -> Vec<&str> { vec!["rc"] }
    fn description(&self) -> &str { "Show or manage the remote control (Bridge) connection" }
    fn help(&self) -> &str {
        "Usage: /remote-control [start|stop|status]\n\n\
         The Bridge feature lets you connect your local Claurst CLI to the\n\
         claude.ai web UI or mobile app.\n\n\
         Subcommands:\n\
         /remote-control          Show current bridge status and connection URL\n\
         /remote-control start    Start the remote-control bridge listener\n\
         /remote-control stop     Stop the bridge listener\n\
         /remote-control status   Show bridge status"
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let settings = match claurst_core::config::Settings::load().await {
            Ok(s) => s,
            Err(e) => return CommandResult::Error(format!("Failed to load settings: {}", e)),
        };

        let remote_at_startup = settings.remote_control_at_startup;

        match args.trim() {
            "" | "status" => {
                let hostname = hostname::get()
                    .map(|h| h.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| "(unknown host)".to_string());

                let bridge_url = std::env::var("CLAURST_BRIDGE_URL")
                    .unwrap_or_else(|_| "https://claude.ai".to_string());

                let token_status = if std::env::var("CLAURST_BRIDGE_TOKEN").is_ok()
                    || std::env::var("CLAUDE_BRIDGE_OAUTH_TOKEN").is_ok()
                {
                    "configured via environment variable"
                } else {
                    "not set (required to connect)"
                };

                let startup_status =
                    if remote_at_startup { "enabled at startup" } else { "disabled" };

                // Active session info from context
                let session_section = if let Some(ref url) = ctx.remote_session_url {
                    format!(
                        "\nActive Session\n\
                         ──────────────\n\
                         Session URL:  {url}\n\
                         Share this URL or QR code with others to let them connect\n\
                         to this Claurst session from the claude.ai web UI.\n",
                        url = url
                    )
                } else {
                    "\nNo active bridge session in this process.\n".to_string()
                };

                // Device fingerprint (first 12 chars are enough for display)
                let fingerprint = claurst_bridge::device_fingerprint();
                let fp_short = &fingerprint[..fingerprint.len().min(12)];

                CommandResult::Message(format!(
                    "Remote Control (Bridge)\n\
                     ═══════════════════════\n\
                     What it does: lets you connect the claude.ai web UI or mobile app\n\
                     to this running Claurst CLI session on your local machine.\n\
                     All prompts and responses are relayed bidirectionally.\n\
                     \n\
                     Local Machine\n\
                     ─────────────\n\
                     Hostname:     {hostname}\n\
                     Device ID:    {fp_short}… (SHA-256 fingerprint)\n\
                     \n\
                     Bridge Configuration\n\
                     ────────────────────\n\
                     Bridge server:   {bridge_url}\n\
                     Session token:   {token_status}\n\
                     Startup mode:    {startup_status}\n\
                     {session_section}\n\
                     How to connect\n\
                     ──────────────\n\
                     1. Obtain a session token from claude.ai (Settings → Remote Control)\n\
                     2. Set it:  export CLAURST_BRIDGE_TOKEN=<your-token>\n\
                     3. Enable:  /remote-control start\n\
                     4. Restart Claurst — the bridge will connect automatically\n\
                     5. Open {bridge_url}/claude-code in your browser\n\
                     \n\
                     Note: Full bridge polling requires server-side session infrastructure.\n\
                     The cc-bridge crate implements the complete protocol (register → poll\n\
                     → events) and is ready to use once a valid session token is provided.\n\
                     \n\
                     Use /remote-control start   to enable bridge at next startup\n\
                     Use /remote-control stop    to disable bridge at startup",
                    hostname = hostname,
                    fp_short = fp_short,
                    bridge_url = bridge_url,
                    token_status = token_status,
                    startup_status = startup_status,
                    session_section = session_section,
                ))
            }
            "start" => {
                if let Err(e) = save_settings_mutation(|s| s.remote_control_at_startup = true) {
                    return CommandResult::Error(format!("Failed to save settings: {}", e));
                }
                let bridge_url = std::env::var("CLAURST_BRIDGE_URL")
                    .unwrap_or_else(|_| "https://claude.ai".to_string());
                let token_note = if std::env::var("CLAURST_BRIDGE_TOKEN").is_ok()
                    || std::env::var("CLAUDE_BRIDGE_OAUTH_TOKEN").is_ok()
                {
                    "Session token detected in environment — bridge will connect on next start."
                        .to_string()
                } else {
                    format!(
                        "No session token found.\n\
                         Get a token from {bridge_url} (Settings → Remote Control)\n\
                         then run:  export CLAURST_BRIDGE_TOKEN=<token>",
                        bridge_url = bridge_url
                    )
                };
                CommandResult::Message(format!(
                    "Remote control bridge enabled at startup.\n\
                     Restart Claurst to activate the bridge connection.\n\n\
                     {token_note}",
                    token_note = token_note
                ))
            }
            "stop" => {
                if let Err(e) = save_settings_mutation(|s| s.remote_control_at_startup = false) {
                    return CommandResult::Error(format!("Failed to save settings: {}", e));
                }
                CommandResult::Message(
                    "Remote control bridge disabled.\n\
                     The bridge will not start on next launch."
                        .to_string(),
                )
            }
            other => CommandResult::Error(format!(
                "Unknown subcommand: '{}'\nUsage: /remote-control [start|stop|status]",
                other
            )),
        }
    }
}

// ---- /remote-env ---------------------------------------------------------

#[async_trait]
impl SlashCommand for RemoteEnvCommand {
    fn name(&self) -> &str { "remote-env" }
    fn description(&self) -> &str { "Show and manage environment variables for remote sessions" }
    fn help(&self) -> &str {
        "Usage: /remote-env [set <KEY> <VALUE> | unset <KEY> | list]\n\n\
         Manages env vars stored in config that are forwarded to remote Claurst sessions.\n\
         These are persisted to settings under the 'env' key."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let args = args.trim();

        if args.is_empty() || args == "list" {
            if ctx.config.env.is_empty() {
                return CommandResult::Message(
                    "No remote environment variables configured.\n\
                     Use /remote-env set <KEY> <VALUE> to add one."
                        .to_string(),
                );
            }
            let mut lines = vec!["Remote environment variables:".to_string()];
            let mut keys: Vec<_> = ctx.config.env.keys().collect();
            keys.sort();
            for key in keys {
                let val = &ctx.config.env[key];
                // Mask values that look like secrets
                let display = if key.to_uppercase().contains("KEY")
                    || key.to_uppercase().contains("TOKEN")
                    || key.to_uppercase().contains("SECRET")
                    || key.to_uppercase().contains("PASSWORD")
                {
                    format!("{}***", &val[..val.len().min(4)])
                } else {
                    val.clone()
                };
                lines.push(format!("  {} = {}", key, display));
            }
            return CommandResult::Message(lines.join("\n"));
        }

        let mut parts = args.splitn(3, ' ');
        let sub = parts.next().unwrap_or("").trim();
        let key = parts.next().unwrap_or("").trim();
        let val = parts.next().unwrap_or("").trim();

        match sub {
            "set" => {
                if key.is_empty() || val.is_empty() {
                    return CommandResult::Error(
                        "Usage: /remote-env set <KEY> <VALUE>".to_string(),
                    );
                }
                let key_owned = key.to_string();
                let val_owned = val.to_string();
                if let Err(e) = save_settings_mutation(|s| {
                    s.config.env.insert(key_owned.clone(), val_owned.clone());
                }) {
                    return CommandResult::Error(format!("Failed to save: {}", e));
                }
                let mut new_config = ctx.config.clone();
                new_config.env.insert(key.to_string(), val.to_string());
                CommandResult::ConfigChangeMessage(
                    new_config,
                    format!("Set remote env: {} = {}", key, val),
                )
            }
            "unset" | "remove" | "delete" => {
                if key.is_empty() {
                    return CommandResult::Error(
                        "Usage: /remote-env unset <KEY>".to_string(),
                    );
                }
                if !ctx.config.env.contains_key(key) {
                    return CommandResult::Message(format!("Key '{}' is not set.", key));
                }
                let key_owned = key.to_string();
                if let Err(e) = save_settings_mutation(|s| {
                    s.config.env.remove(&key_owned);
                }) {
                    return CommandResult::Error(format!("Failed to save: {}", e));
                }
                let mut new_config = ctx.config.clone();
                new_config.env.remove(key);
                CommandResult::ConfigChangeMessage(
                    new_config,
                    format!("Removed remote env var: {}", key),
                )
            }
            other => CommandResult::Error(format!(
                "Unknown subcommand: '{}'\nUsage: /remote-env [list|set <K> <V>|unset <K>]",
                other
            )),
        }
    }
}

// ---- /context ------------------------------------------------------------

#[async_trait]
impl SlashCommand for ContextCommand {
    fn name(&self) -> &str { "context" }
    fn description(&self) -> &str { "Show context window usage (tokens used / available)" }
    fn help(&self) -> &str {
        "Usage: /context\n\n\
         Displays the current context window utilization:\n\
         - Estimated tokens consumed by current conversation\n\
         - Context window limit for the active model\n\
         - Percentage used"
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let model = ctx.config.effective_model();

        // Determine context window size from known model names
        let context_window: u64 = if model.contains("claude-3-5") || model.contains("claude-3.5") {
            200_000
        } else if model.contains("opus") {
            200_000
        } else if model.contains("sonnet") {
            200_000
        } else if model.contains("haiku") {
            200_000
        } else {
            200_000 // safe default for any Claude model
        };

        let used_tokens = ctx.cost_tracker.total_tokens();
        let pct = if context_window > 0 {
            (used_tokens as f64 / context_window as f64) * 100.0
        } else {
            0.0
        };

        let bar_width = 40usize;
        let filled = ((pct / 100.0) * bar_width as f64).round() as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));

        // Estimate approximate message tokens from the message list
        let msg_char_count: usize = ctx.messages.iter().map(|m| m.get_all_text().len()).sum();
        // Rough estimate: ~4 chars per token for message text
        let msg_token_estimate = msg_char_count / 4;

        CommandResult::Message(format!(
            "Context Window Usage\n\
             ────────────────────\n\
             Model:          {model}\n\
             Context window: {window:>10} tokens\n\
             API tokens used:{used:>10} tokens  ({pct:.1}%)\n\
             Est. msg size:  {msg:>10} tokens  (approx)\n\
             Messages:       {msgs:>10}\n\n\
             [{bar}] {pct:.1}%\n\n\
             Use /compact to reduce context usage.",
            model = model,
            window = context_window,
            used = used_tokens,
            pct = pct,
            msg = msg_token_estimate,
            msgs = ctx.messages.len(),
            bar = bar,
        ))
    }
}

// ---- /vim (/vi) ----------------------------------------------------------

#[async_trait]
impl SlashCommand for VimCommand {
    fn name(&self) -> &str { "vim" }
    fn aliases(&self) -> Vec<&str> { vec!["vi"] }
    fn description(&self) -> &str { "Toggle vim keybinding mode on/off" }
    fn help(&self) -> &str {
        "Usage: /vim [on|off]\n\n\
         Toggles vim keybinding mode in the REPL input.\n\
         When enabled, use Esc to switch between INSERT and NORMAL modes.\n\n\
         The setting is persisted to ~/.claurst/ui-settings.json."
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let current = load_ui_settings();
        let current_mode = current.editor_mode.as_deref().unwrap_or("normal");

        let new_mode = match args.trim() {
            "on" | "vim" => "vim",
            "off" | "normal" => "normal",
            "" => {
                // Toggle
                if current_mode == "vim" { "normal" } else { "vim" }
            }
            other => {
                return CommandResult::Error(format!(
                    "Unknown argument '{}'. Use: /vim [on|off]",
                    other
                ))
            }
        };

        match mutate_ui_settings(|s| s.editor_mode = Some(new_mode.to_string())) {
            Ok(_) => CommandResult::Message(format!(
                "Editor mode set to {}.\n{}",
                new_mode,
                if new_mode == "vim" {
                    "Use Esc to switch between INSERT and NORMAL modes.\n\
                     Restart the REPL for the change to take effect."
                } else {
                    "Using standard (readline-style) keyboard bindings.\n\
                     Restart the REPL for the change to take effect."
                }
            )),
            Err(e) => CommandResult::Error(format!("Failed to save setting: {}", e)),
        }
    }
}

// ---- /voice --------------------------------------------------------------

#[async_trait]
impl SlashCommand for VoiceCommand {
    fn name(&self) -> &str { "voice" }
    fn description(&self) -> &str { "Toggle voice input mode on/off" }
    fn help(&self) -> &str {
        "Usage: /voice [on|off|status]\n\n\
         Enables or disables voice input (push-to-talk).\n\
         Setting is persisted to ~/.claurst/ui-settings.json.\n\n\
         Transcription is performed via a Whisper-compatible API.\n\
         Set one of these env vars for the API key:\n\
           OPENAI_API_KEY   — OpenAI Whisper (default endpoint)\n\
           ANTHROPIC_API_KEY — used as a fallback key\n\n\
         To use a local Whisper server instead of OpenAI:\n\
           export WHISPER_ENDPOINT_URL=http://localhost:8080/v1/audio/transcriptions\n\
           export OPENAI_API_KEY=any-value  (local servers often ignore the key)\n\n\
         On Linux, ALSA must be set up: sudo apt install libasound2-dev\n\
         Check available devices with: arecord -l\n\n\
         Controls:\n\
           Alt+V — start recording; Alt+V or Esc — stop and transcribe"
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let current = load_ui_settings();
        let currently_enabled = current.voice_enabled.unwrap_or(false);

        let enable = match args.trim() {
            "on" | "enable" | "enabled" | "true" | "1" => true,
            "off" | "disable" | "disabled" | "false" | "0" => false,
            "" => !currently_enabled, // toggle
            "status" => {
                let state = if currently_enabled { "enabled" } else { "disabled" };
                let endpoint = std::env::var("WHISPER_ENDPOINT_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1/audio/transcriptions (default)".to_string());
                let key_source = if std::env::var("OPENAI_API_KEY").is_ok() {
                    "OPENAI_API_KEY"
                } else if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                    "ANTHROPIC_API_KEY"
                } else {
                    "(none — transcription will fail)"
                };
                return CommandResult::Message(format!(
                    "Voice mode: {}\n\
                     Endpoint:   {}\n\
                     API key:    {}",
                    state, endpoint, key_source
                ));
            }
            other => {
                return CommandResult::Error(format!(
                    "Unknown argument '{}'. Use: /voice [on|off|status]",
                    other
                ))
            }
        };

        match mutate_ui_settings(|s| s.voice_enabled = Some(enable)) {
            Ok(_) => {
                if enable {
                    let endpoint = std::env::var("WHISPER_ENDPOINT_URL")
                        .unwrap_or_else(|_| "OpenAI Whisper (default)".to_string());
                    let key_hint = if std::env::var("OPENAI_API_KEY").is_ok()
                        || std::env::var("ANTHROPIC_API_KEY").is_ok()
                    {
                        String::new()
                    } else {
                        "\nWarning: no OPENAI_API_KEY found — transcription will fail. \
                         Set OPENAI_API_KEY or WHISPER_ENDPOINT_URL for a local server."
                            .to_string()
                    };
                    CommandResult::Message(format!(
                        "Voice recording activated.\n\
                         Press Alt+V to start recording; Alt+V or Esc to stop and transcribe.\n\
                         Endpoint: {}{}",
                        endpoint, key_hint
                    ))
                } else {
                    CommandResult::Message(
                        "Voice recording deactivated.".to_string(),
                    )
                }
            }
            Err(e) => CommandResult::Error(format!("Failed to save voice setting: {}", e)),
        }
    }
}

// ---- /upgrade ------------------------------------------------------------

#[async_trait]
impl SlashCommand for UpgradeCommand {
    fn name(&self) -> &str { "update" }
    fn aliases(&self) -> Vec<&str> { vec!["upgrade"] }
    fn description(&self) -> &str { "Check for updates and download the latest release" }
    fn help(&self) -> &str {
        "Usage: /update\n\n\
         Checks GitHub releases for the latest version of Claurst.\n\
         If a newer version is available, shows where to download it."
    }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let current = claurst_core::constants::APP_VERSION;

        // Check GitHub releases API for latest version
        let client = reqwest::Client::builder()
            .user_agent(format!("claurst/{}", current))
            .timeout(std::time::Duration::from_secs(8))
            .build();

        let client = match client {
            Ok(c) => c,
            Err(e) => {
                return CommandResult::Message(format!(
                    "Current version: {current}\n\
                     Could not check for updates (HTTP client error: {e})\n\
                     Visit https://github.com/kuberwastaken/claurst/releases for updates."
                ))
            }
        };

        let resp = client
            .get("https://api.github.com/repos/kuberwastaken/claurst/releases/latest")
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let json: serde_json::Value =
                    r.json().await.unwrap_or(serde_json::Value::Null);

                let tag = json
                    .get("tag_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .trim_start_matches('v');

                let url = json
                    .get("html_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("https://github.com/kuberwastaken/claurst/releases");

                if tag == current || tag == "unknown" {
                    CommandResult::Message(format!(
                        "Claurst v{current} - you are up to date.\n\
                         Release page: {url}"
                    ))
                } else {
                    CommandResult::Message(format!(
                        "Update available!\n\
                         Current version:  v{current}\n\
                         Latest version:   v{tag}\n\
                         Release page:     {url}\n\n\
                         Upgrade in place (recommended):\n\
                           claurst upgrade\n\n\
                         Or build from source:\n\
                           cargo install claurst --force"
                    ))
                }
            }
            Ok(r) => {
                let status = r.status();
                CommandResult::Message(format!(
                    "Current version: v{current}\n\
                     Could not check for updates (HTTP {status}).\n\
                     Visit https://github.com/kuberwastaken/claurst/releases for updates."
                ))
            }
            Err(e) => CommandResult::Message(format!(
                "Current version: v{current}\n\
                 Could not check for updates: {e}\n\
                 Visit https://github.com/kuberwastaken/claurst/releases for updates."
            )),
        }
    }
}

// ---- /release-notes ------------------------------------------------------

#[async_trait]
impl SlashCommand for ReleaseNotesCommand {
    fn name(&self) -> &str { "release-notes" }
    fn description(&self) -> &str { "Show release notes for the current version" }
    fn help(&self) -> &str {
        "Usage: /release-notes [version]\n\n\
         Fetches and displays release notes from GitHub.\n\
         Without an argument, shows notes for the current version."
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let current = claurst_core::constants::APP_VERSION;
        let version = args.trim();

        let tag = if version.is_empty() {
            format!("v{}", current)
        } else if version.starts_with('v') {
            version.to_string()
        } else {
            format!("v{}", version)
        };

        let client = reqwest::Client::builder()
            .user_agent(format!("claurst/{}", current))
            .timeout(std::time::Duration::from_secs(8))
            .build();

        let client = match client {
            Ok(c) => c,
            Err(_) => {
                return CommandResult::Message(format!(
                    "Claurst {tag} release notes:\n\
                     Visit https://github.com/kuberwastaken/claurst/releases/tag/{tag}"
                ))
            }
        };

        let url = format!(
            "https://api.github.com/repos/kuberwastaken/claurst/releases/tags/{}",
            tag
        );

        match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => {
                let json: serde_json::Value =
                    r.json().await.unwrap_or(serde_json::Value::Null);

                let body = json
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No release notes found.");

                let published = json
                    .get("published_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown date");

                let html_url = json
                    .get("html_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                CommandResult::Message(format!(
                    "Release Notes: Claurst {tag}\n\
                     Published: {published}\n\
                     URL: {html_url}\n\
                     ─────────────────────────────────\n\
                     {body}"
                ))
            }
            Ok(r) if r.status().as_u16() == 404 => CommandResult::Message(format!(
                "No release found for {tag}.\n\
                 View all releases: https://github.com/kuberwastaken/claurst/releases"
            )),
            Ok(r) => CommandResult::Message(format!(
                "Could not fetch release notes (HTTP {}).\n\
                 View at: https://github.com/kuberwastaken/claurst/releases/tag/{}",
                r.status(),
                tag
            )),
            Err(e) => CommandResult::Message(format!(
                "Could not fetch release notes: {e}\n\
                 View at: https://github.com/kuberwastaken/claurst/releases/tag/{tag}"
            )),
        }
    }
}

// ---- /rate-limit-options -------------------------------------------------

#[async_trait]
impl SlashCommand for RateLimitOptionsCommand {
    fn name(&self) -> &str { "rate-limit-options" }
    fn description(&self) -> &str { "Show rate limit tiers and current rate limit status" }
    fn help(&self) -> &str {
        "Usage: /rate-limit-options\n\n\
         Displays available rate limit tiers and the current tier for your account.\n\
         Rate limits depend on your Claurst plan (Free, Pro, Max, API)."
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        // Try to read from OAuth tokens file to get subscription/tier info
        let tier_info = match claurst_core::oauth::OAuthTokens::load().await {
            Some(tokens) => {
                let sub_type = tokens.subscription_type.as_deref().unwrap_or("unknown");
                format!(
                    "Account type:    {}\n\
                     Scopes:          {}",
                    sub_type,
                    if tokens.scopes.is_empty() { "none".to_string() } else { tokens.scopes.join(", ") }
                )
            }
            None => {
                // Check for API key auth
                if ctx.config.resolve_api_key().is_some() {
                    "Account type:    API key (Console)\n\
                     Rate limit tier: Depends on your API plan tier"
                        .to_string()
                } else {
                    "Not logged in. Run /login to see your rate limit tier.".to_string()
                }
            }
        };

        CommandResult::Message(format!(
            "Rate Limit Status\n\
             ─────────────────\n\
             {tier_info}\n\n\
             Available tiers:\n\
             ┌─────────────────────────────────────────────────┐\n\
             │ Free          │ Limited daily usage             │\n\
             │ Pro           │ Higher limits, faster resets    │\n\
             │ Max (5x)      │ 5× Pro limits                   │\n\
             │ Max (20x)     │ 20× Pro limits (highest tier)   │\n\
             │ API / Console │ Usage-billed, no hard cap       │\n\
             └─────────────────────────────────────────────────┘\n\n\
             To upgrade: /upgrade\n\
             Manage billing: https://claude.ai/settings/billing",
            tier_info = tier_info,
        ))
    }
}

// ---- /statusline ---------------------------------------------------------

#[async_trait]
impl SlashCommand for StatuslineCommand {
    fn name(&self) -> &str { "statusline" }
    fn description(&self) -> &str { "Configure what is shown in the status line" }
    fn help(&self) -> &str {
        "Usage: /statusline [show|hide] [cost|tokens|model|time|all]\n\n\
         Controls which items appear in the TUI status bar at the bottom.\n\
         Settings are persisted to ~/.claurst/ui-settings.json.\n\n\
         Examples:\n\
           /statusline               — show current configuration\n\
           /statusline show cost     — show cost in status line\n\
           /statusline hide tokens   — hide token count\n\
           /statusline show all      — show everything\n\
           /statusline hide all      — hide everything"
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let args = args.trim();
        let current = load_ui_settings();

        if args.is_empty() {
            return CommandResult::Message(format!(
                "Status line configuration\n\
                 ─────────────────────────\n\
                 Show cost:   {cost}\n\
                 Show tokens: {tokens}\n\
                 Show model:  {model}\n\
                 Show time:   {time}\n\n\
                 Use /statusline [show|hide] [cost|tokens|model|time|all] to change.",
                cost = fmt_bool(current.statusline_show_cost.unwrap_or(true)),
                tokens = fmt_bool(current.statusline_show_tokens.unwrap_or(true)),
                model = fmt_bool(current.statusline_show_model.unwrap_or(true)),
                time = fmt_bool(current.statusline_show_time.unwrap_or(true)),
            ));
        }

        let mut parts = args.splitn(2, ' ');
        let verb = parts.next().unwrap_or("").trim();
        let item = parts.next().unwrap_or("").trim();

        let show = match verb {
            "show" | "enable" | "on" => true,
            "hide" | "disable" | "off" => false,
            _ => {
                return CommandResult::Error(
                    "Usage: /statusline [show|hide] [cost|tokens|model|time|all]".to_string(),
                )
            }
        };

        if item.is_empty() || item == "all" {
            match mutate_ui_settings(|s| {
                s.statusline_show_cost = Some(show);
                s.statusline_show_tokens = Some(show);
                s.statusline_show_model = Some(show);
                s.statusline_show_time = Some(show);
            }) {
                Ok(_) => return CommandResult::Message(format!(
                    "Status line: all items {}.",
                    if show { "shown" } else { "hidden" }
                )),
                Err(e) => return CommandResult::Error(format!("Failed to save: {}", e)),
            }
        }

        let result = match item {
            "cost" => mutate_ui_settings(|s| s.statusline_show_cost = Some(show)),
            "tokens" | "token" => mutate_ui_settings(|s| s.statusline_show_tokens = Some(show)),
            "model" => mutate_ui_settings(|s| s.statusline_show_model = Some(show)),
            "time" | "clock" => mutate_ui_settings(|s| s.statusline_show_time = Some(show)),
            other => {
                return CommandResult::Error(format!(
                    "Unknown item '{}'. Use: cost, tokens, model, time, or all.",
                    other
                ))
            }
        };

        match result {
            Ok(_) => CommandResult::Message(format!(
                "Status line: {} {}.",
                item,
                if show { "shown" } else { "hidden" }
            )),
            Err(e) => CommandResult::Error(format!("Failed to save: {}", e)),
        }
    }
}

fn fmt_bool(v: bool) -> &'static str {
    if v { "on" } else { "off" }
}

// ---- /security-review ----------------------------------------------------

#[async_trait]
impl SlashCommand for SecurityReviewCommand {
    fn name(&self) -> &str { "security-review" }
    fn description(&self) -> &str { "Run a security review of the current project" }
    fn help(&self) -> &str {
        "Usage: /security-review [path]\n\n\
         Asks Claurst to perform a security review of the codebase.\n\
         Analyzes for common vulnerabilities: injection attacks, auth issues,\n\
         secrets exposure, unsafe deserialization, path traversal, etc."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let target = if args.trim().is_empty() {
            ctx.working_dir.display().to_string()
        } else {
            args.trim().to_string()
        };

        CommandResult::UserMessage(format!(
            "Please perform a comprehensive security review of the code in `{target}`.\n\n\
             Focus on identifying:\n\
             1. Injection vulnerabilities (SQL, command, LDAP, XSS, SSTI)\n\
             2. Authentication and authorization flaws\n\
             3. Hardcoded secrets, API keys, or passwords\n\
             4. Insecure deserialization\n\
             5. Path traversal or file inclusion vulnerabilities\n\
             6. Cryptographic weaknesses (weak algorithms, bad IV usage, key reuse)\n\
             7. Dependency vulnerabilities (check for outdated packages)\n\
             8. Race conditions and TOCTOU issues\n\
             9. Information disclosure (verbose errors, debug endpoints)\n\
             10. Any OWASP Top 10 issues relevant to this codebase\n\n\
             For each finding, provide:\n\
             - Severity: Critical/High/Medium/Low/Informational\n\
             - File and line number\n\
             - Description of the vulnerability\n\
             - Proof of concept or reproduction steps\n\
             - Recommended remediation\n\n\
             Start by reading the main source files and any dependency manifests.",
            target = target,
        ))
    }
}

// ---- /terminal-setup -----------------------------------------------------

#[async_trait]
impl SlashCommand for TerminalSetupCommand {
    fn name(&self) -> &str { "terminal-setup" }
    fn description(&self) -> &str { "Help configure your terminal for optimal Claurst use" }
    fn help(&self) -> &str {
        "Usage: /terminal-setup\n\n\
         Diagnoses your terminal environment and gives recommendations for\n\
         optimal Claurst display (font, color support, Unicode, etc.)."
    }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let mut checks: Vec<String> = Vec::new();

        // Check TERM variable
        let term = std::env::var("TERM").unwrap_or_default();
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();

        // Terminal identification
        let terminal_name = if !term_program.is_empty() {
            term_program.clone()
        } else {
            term.clone()
        };
        checks.push(format!("Terminal:      {}", terminal_name));

        // Color depth
        let color_depth = if colorterm == "truecolor" || colorterm == "24bit" {
            "24-bit true color (optimal)"
        } else if term.contains("256color") || colorterm == "256color" {
            "256 colors (good)"
        } else if !term.is_empty() {
            "Basic colors (limited)"
        } else {
            "Unknown"
        };
        checks.push(format!("Colors:        {}", color_depth));

        // Check if UNICODE is likely supported
        let lang = std::env::var("LANG").unwrap_or_default();
        let lc_all = std::env::var("LC_ALL").unwrap_or_default();
        let unicode_env = lang.to_lowercase().contains("utf") || lc_all.to_lowercase().contains("utf");
        checks.push(format!(
            "Unicode/UTF-8: {}",
            if unicode_env { "likely supported (LANG/LC_ALL contains UTF)" } else { "check LANG env var" }
        ));

        // Check for known good terminals
        let is_good_terminal = matches!(
            term_program.to_lowercase().as_str(),
            "iterm.app" | "iterm2" | "hyper" | "warp" | "alacritty" | "kitty" | "wezterm"
        ) || term_program.to_lowercase().contains("vscode")
          || term_program.to_lowercase().contains("terminal");

        checks.push(format!(
            "Terminal type: {}",
            if is_good_terminal { "well-known terminal (good)" } else { "verify settings below" }
        ));

        // Shell detection
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());
        checks.push(format!("Shell:         {}", shell));

        // Check for Nerd Fonts (heuristic: environment variable set by some terminals)
        let nerd_font = std::env::var("NERD_FONT").is_ok()
            || std::env::var("TERM_NERD_FONT").is_ok();

        CommandResult::Message(format!(
            "Terminal Setup Diagnostic\n\
             ─────────────────────────\n\
             {checks}\n\n\
             Recommendations for optimal Claurst experience:\n\
             ─────────────────────────────────────────────────\n\
             1. Font: Use a Nerd Font for box-drawing characters and icons\n\
                {nerd_hint}\n\
                Download: https://www.nerdfonts.com/\n\
             2. Color: Enable 24-bit true color:\n\
                export COLORTERM=truecolor\n\
             3. Unicode: Ensure UTF-8 locale:\n\
                export LANG=en_US.UTF-8\n\
             4. Recommended terminals:\n\
                - WezTerm (all platforms)\n\
                - Alacritty (all platforms)\n\
                - Kitty (macOS/Linux)\n\
                - Windows Terminal (Windows)\n\
                - iTerm2 (macOS)\n\
             5. Set terminal to unlimited scrollback for long conversations",
            checks = checks.join("\n  "),
            nerd_hint = if nerd_font {
                "[ok] Nerd Font detected"
            } else {
                "[!] Nerd Font not detected — box-drawing may appear broken"
            },
        ))
    }
}

// ---- /extra-usage --------------------------------------------------------

#[async_trait]
impl SlashCommand for ExtraUsageCommand {
    fn name(&self) -> &str { "extra-usage" }
    fn description(&self) -> &str { "Show detailed usage statistics: calls, cache, tools" }
    fn help(&self) -> &str {
        "Usage: /extra-usage\n\n\
         Displays extended usage statistics beyond /cost:\n\
         - API call count\n\
         - Cache hit/miss ratio\n\
         - Token breakdown by type\n\
         - Effective cost per call"
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let input = ctx.cost_tracker.input_tokens();
        let output = ctx.cost_tracker.output_tokens();
        let cache_creation = ctx.cost_tracker.cache_creation_tokens();
        let cache_read = ctx.cost_tracker.cache_read_tokens();
        let total = ctx.cost_tracker.total_tokens();
        let cost = ctx.cost_tracker.total_cost_usd();

        // Estimate API calls from messages (each assistant message ~ 1 API call)
        let api_calls = ctx.messages.iter()
            .filter(|m| m.role == claurst_core::types::Role::Assistant)
            .count();
        let api_calls = api_calls.max(1); // at least 1 if we have any data

        // Cache efficiency
        let cache_total = cache_creation + cache_read;
        let cache_hit_pct = if cache_total > 0 {
            (cache_read as f64 / cache_total as f64) * 100.0
        } else {
            0.0
        };

        let cost_per_call = if api_calls > 0 {
            cost / api_calls as f64
        } else {
            0.0
        };

        CommandResult::Message(format!(
            "Detailed Usage Statistics\n\
             ─────────────────────────\n\
             API calls:           {api_calls}\n\
             Avg cost/call:       ${cost_per_call:.4}\n\n\
             Token Breakdown:\n\
               Input tokens:      {input:>10}\n\
               Output tokens:     {output:>10}\n\
               Cache creation:    {cache_creation:>10}\n\
               Cache read:        {cache_read:>10}\n\
               Total tokens:      {total:>10}\n\n\
             Cache Performance:\n\
               Cache hit rate:    {cache_hit_pct:.1}%\n\
               Cache efficiency:  {cache_eff}\n\n\
             Cost:\n\
               Total cost:        ${cost:.4}\n\
               Cost/1k tokens:    ${cost_per_k:.4}",
            api_calls = api_calls,
            cost_per_call = cost_per_call,
            input = input,
            output = output,
            cache_creation = cache_creation,
            cache_read = cache_read,
            total = total,
            cache_hit_pct = cache_hit_pct,
            cache_eff = if cache_hit_pct > 70.0 {
                "Excellent"
            } else if cache_hit_pct > 40.0 {
                "Good"
            } else if cache_total > 0 {
                "Low — prompts may not be stable enough to cache"
            } else {
                "No cache activity"
            },
            cost = cost,
            cost_per_k = if total > 0 { cost / (total as f64 / 1000.0) } else { 0.0 },
        ))
    }
}

// ---- /advisor ------------------------------------------------------------

#[async_trait]
impl SlashCommand for AdvisorCommand {
    fn name(&self) -> &str { "advisor" }
    fn description(&self) -> &str { "Set or unset the server-side advisor model" }
    fn help(&self) -> &str {
        "Usage: /advisor [<model>|off|unset]\n\n\
         Sets the advisor model used for server-side suggestions.\n\
         Examples:\n\
           /advisor claude-opus-4-6   — set advisor model\n\
           /advisor off               — disable the advisor\n\
           /advisor                   — show current advisor setting"
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let arg = args.trim();
        let settings_dir = claurst_core::config::Settings::config_dir();
        let settings_path = settings_dir.join("settings.json");

        // Read or create settings JSON
        let mut settings_val: serde_json::Value = settings_path
            .exists()
            .then(|| std::fs::read_to_string(&settings_path).ok())
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        match arg {
            "" => {
                let current = settings_val
                    .get("advisorModel")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(not set)");
                CommandResult::Message(format!("Advisor model: {current}"))
            }
            "off" | "unset" | "none" => {
                settings_val
                    .as_object_mut()
                    .map(|m| m.remove("advisorModel"));
                if let Ok(json) = serde_json::to_string_pretty(&settings_val) {
                    let _ = std::fs::write(&settings_path, json);
                }
                CommandResult::Message("Advisor model unset.".to_string())
            }
            model => {
                // Basic validation: must look like a model identifier
                if model.starts_with("claude-") || model.contains('/') {
                    settings_val["advisorModel"] = serde_json::Value::String(model.to_string());
                    if let Ok(json) = serde_json::to_string_pretty(&settings_val) {
                        let _ = std::fs::write(&settings_path, json);
                    }
                    CommandResult::Message(format!("Advisor model set to: {model}"))
                } else {
                    CommandResult::Message(format!(
                        "Unknown model '{model}'. Model IDs should start with 'claude-'.\n\
                         Use /model to see available models."
                    ))
                }
            }
        }
    }
}

// ---- /install-slack-app --------------------------------------------------

#[async_trait]
impl SlashCommand for InstallSlackAppCommand {
    fn name(&self) -> &str { "install-slack-app" }
    fn description(&self) -> &str { "Install the Claurst Slack integration" }
    fn help(&self) -> &str {
        "Usage: /install-slack-app\n\n\
         Opens instructions for installing the Claurst Slack app.\n\
         Requires a Claurst for Enterprise subscription."
    }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        CommandResult::Message(
            "Claurst Slack Integration\n\
             ─────────────────────────────\n\
             To install Claurst in Slack:\n\n\
             1. Ensure you have a Claurst for Enterprise subscription\n\
             2. Visit your Anthropic Console → Integrations → Slack\n\
             3. Click \"Add to Slack\" and authorize the app\n\
             4. Invite @Claurst to any channel with: /invite @Claurst\n\n\
             In Slack, you can then:\n\
             • Mention @Claurst to ask questions in any channel\n\
             • Use /claude for direct commands\n\
             • Share code snippets for review\n\n\
             See: https://docs.anthropic.com/claude-code/slack"
                .to_string(),
        )
    }
}

// ---- /fast (/speed) ------------------------------------------------------

#[async_trait]
impl SlashCommand for FastCommand {
    fn name(&self) -> &str { "fast" }
    fn aliases(&self) -> Vec<&str> { vec!["speed"] }
    fn description(&self) -> &str { "Toggle fast mode (uses a faster/cheaper model)" }
    fn help(&self) -> &str {
        "Usage: /fast [on|off]\n\n\
         Fast mode switches to the active provider's smaller, faster model\n\
         for quick responses. Toggle without argument to switch.\n\
         The setting is persisted to ~/.claurst/ui-settings.json."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let current = load_ui_settings();
        let currently_on = current.fast_mode.unwrap_or(false);

        let enable = match args.trim() {
            "on" | "enable" | "true" | "1" => true,
            "off" | "disable" | "false" | "0" => false,
            "" => !currently_on,
            other => {
                return CommandResult::Error(format!(
                    "Unknown argument '{}'. Use: /fast [on|off]",
                    other
                ))
            }
        };

        if let Err(e) = mutate_ui_settings(|s| s.fast_mode = Some(enable)) {
            return CommandResult::Error(format!("Failed to save setting: {}", e));
        }

        let provider_id = ctx.config.selected_provider_id();
        let fast_model = resolve_fast_model_id(&ctx.config);
        let normal_model = stripped_model_for_provider(
            provider_id,
            ctx.config.effective_model(),
        )
        .to_string();

        if enable {
            let mut new_config = ctx.config.clone();
            new_config.model = Some(canonical_model_for_provider(provider_id, &fast_model));
            CommandResult::ConfigChangeMessage(
                new_config,
                format!(
                    "Fast mode ON. Using {} for quicker, cheaper responses.\n\
                     Use /fast off to return to {}.",
                    fast_model, normal_model
                ),
            )
        } else {
            let mut new_config = ctx.config.clone();
            // Restore default / saved model
            new_config.model = None;
            let restored_model = stripped_model_for_provider(
                provider_id,
                new_config.effective_model(),
            )
            .to_string();
            CommandResult::ConfigChangeMessage(
                new_config,
                format!(
                    "Fast mode OFF. Restored to default model ({}).",
                    restored_model
                ),
            )
        }
    }
}

// ---- /think-back ---------------------------------------------------------

#[async_trait]
impl SlashCommand for ThinkBackCommand {
    fn name(&self) -> &str { "think-back" }
    fn aliases(&self) -> Vec<&str> { vec!["thinkback"] }
    fn description(&self) -> &str { "Show thinking traces from previous responses in this session" }
    fn help(&self) -> &str {
        "Usage: /think-back [n]\n\n\
         Displays the thinking/reasoning traces from the most recent model responses.\n\
         Pass a number to show the Nth most recent thinking block."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let n: usize = args.trim().parse().unwrap_or(1).max(1);

        // Scan messages for thinking blocks
        let thinking_blocks: Vec<(usize, String)> = ctx
            .messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.role == claurst_core::types::Role::Assistant)
            .filter_map(|(idx, m)| {
                let blocks = m.get_thinking_blocks();
                if blocks.is_empty() {
                    return None;
                }
                let thinking: String = blocks
                    .iter()
                    .filter_map(|b| {
                        if let claurst_core::types::ContentBlock::Thinking { thinking, .. } = b {
                            Some(thinking.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");
                if thinking.is_empty() { None } else { Some((idx, thinking)) }
            })
            .collect();

        if thinking_blocks.is_empty() {
            return CommandResult::Message(
                "No thinking traces found in this session.\n\
                 Thinking traces appear when the model uses extended thinking mode.\n\
                 Try asking Claurst to 'think step by step' or 'think carefully'."
                    .to_string(),
            );
        }

        // Show the Nth most recent (1-indexed)
        let total = thinking_blocks.len();
        let target_idx = total.saturating_sub(n);
        let (msg_idx, trace) = &thinking_blocks[target_idx];

        CommandResult::Message(format!(
            "Thinking trace ({n} of {total} found, from message {msg}):\n\
             ─────────────────────────────────────\n\
             {trace}\n\
             ─────────────────────────────────────\n\
             Use /think-back <n> to see older traces.",
            n = n,
            total = total,
            msg = msg_idx + 1,
            trace = trace,
        ))
    }
}

// ---- /thinkback-play -----------------------------------------------------

#[async_trait]
impl SlashCommand for ThinkBackPlayCommand {
    fn name(&self) -> &str { "thinkback-play" }
    fn description(&self) -> &str { "Replay a thinking trace as an animated walkthrough" }
    fn help(&self) -> &str {
        "Usage: /thinkback-play [n]\n\n\
         Replays a previous thinking trace, formatted for easy reading.\n\
         Pass a number to replay the Nth most recent trace."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let n: usize = args.trim().parse().unwrap_or(1).max(1);

        let thinking_blocks: Vec<String> = ctx
            .messages
            .iter()
            .filter(|m| m.role == claurst_core::types::Role::Assistant)
            .filter_map(|m| {
                let blocks = m.get_thinking_blocks();
                if blocks.is_empty() {
                    return None;
                }
                let t: String = blocks
                    .iter()
                    .filter_map(|b| {
                        if let claurst_core::types::ContentBlock::Thinking { thinking, .. } = b {
                            Some(thinking.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");
                if t.is_empty() { None } else { Some(t) }
            })
            .collect();

        if thinking_blocks.is_empty() {
            return CommandResult::Message(
                "No thinking traces to replay in this session.".to_string(),
            );
        }

        let total = thinking_blocks.len();
        let idx = total.saturating_sub(n);
        let trace = &thinking_blocks[idx];

        // Format the trace with step numbering
        let steps: Vec<&str> = trace.split('\n').filter(|l| !l.trim().is_empty()).collect();
        let mut formatted = format!(
            "Thinking Trace Replay ({}/{total})\n\
             ══════════════════════════════════\n",
            n,
            total = total
        );
        for (i, step) in steps.iter().enumerate() {
            formatted.push_str(&format!("  Step {}: {}\n", i + 1, step));
        }
        formatted.push_str("══════════════════════════════════\n");
        formatted.push_str(&format!(
            "{} steps shown. Use /think-back for raw traces.",
            steps.len()
        ));

        CommandResult::Message(formatted)
    }
}

// ---- /feedback (standalone, supplements BugCommand alias) ----------------

#[async_trait]
impl SlashCommand for FeedbackCommand {
    fn name(&self) -> &str { "report" }
    fn aliases(&self) -> Vec<&str> { vec![] }
    fn description(&self) -> &str { "Open the GitHub issues page to report a bug or request a feature" }
    fn hidden(&self) -> bool { true } // surfaced via BugCommand alias; hidden to avoid duplicate
    fn help(&self) -> &str {
        "Usage: /report [description]\n\n\
         Opens the GitHub issues tracker. If a description is provided,\n\
         it is shown as a suggested pre-fill for the issue body."
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let url = "https://github.com/anthropics/claude-code/issues/new";
        let report = args.trim();
        let display_url = if report.is_empty() {
            url.to_string()
        } else {
            // Append as a body query param
            format!(
                "{}?body={}",
                url,
                urlencoding::encode(report)
            )
        };

        match open_with_system(&display_url) {
            Ok(_) => CommandResult::Message(format!("Opened issue tracker: {}", url)),
            Err(_) => CommandResult::Message(format!(
                "Please visit {} to submit a report.",
                url
            )),
        }
    }
}

// ---- /color (full implementation) ----------------------------------------

#[async_trait]
impl SlashCommand for ColorSetCommand {
    fn name(&self) -> &str { "color-set" }
    fn hidden(&self) -> bool { true }
    fn description(&self) -> &str { "Internal: set prompt color — use /color instead" }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let color = args.trim();
        if color.is_empty() {
            let current = load_ui_settings();
            return CommandResult::Message(format!(
                "Current prompt color: {}\n\
                 Use /color <name|#RRGGBB|default> to change it.\n\n\
                 Named colors: red, green, blue, yellow, cyan, magenta, white, orange, purple",
                current.prompt_color.as_deref().unwrap_or("default"),
            ));
        }

        let normalized = if color == "default" {
            None
        } else {
            // Validate hex or named color
            let known_colors = [
                "red", "green", "blue", "yellow", "cyan", "magenta",
                "white", "orange", "purple", "pink", "gray", "grey",
            ];
            let is_hex = color.starts_with('#') && (color.len() == 4 || color.len() == 7)
                && color[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !is_hex && !known_colors.contains(&color.to_lowercase().as_str()) {
                return CommandResult::Error(format!(
                    "Unknown color '{}'. Use a color name (red, green, …) or a hex code (#RGB or #RRGGBB).",
                    color
                ));
            }
            Some(color.to_string())
        };

        match mutate_ui_settings(|s| s.prompt_color = normalized.clone()) {
            Ok(_) => CommandResult::Message(format!(
                "Prompt color set to {}.\n\
                 Restart the REPL for the change to take effect.",
                normalized.as_deref().unwrap_or("default")
            )),
            Err(e) => CommandResult::Error(format!("Failed to save color: {}", e)),
        }
    }
}

// ---- /search -------------------------------------------------------------

#[async_trait]
impl SlashCommand for SearchCommand {
    fn name(&self) -> &str { "search" }
    fn description(&self) -> &str { "Search across all sessions" }
    fn help(&self) -> &str {
        "Usage: /search <query>\n\n\
         Searches session titles and message content in the local SQLite\n\
         session database (~/.claurst/sessions.db).  Returns the 50 best\n\
         matching sessions, ordered by most recently updated.\n\n\
         Example: /search refactor authentication"
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let query = args.trim();
        if query.is_empty() {
            return CommandResult::Error(
                "Usage: /search <query>\n\
                 Provide a search term to look up across all sessions."
                    .to_string(),
            );
        }

        let db_path = claurst_core::config::Settings::config_dir().join("sessions.db");

        let store = match claurst_core::SqliteSessionStore::open(&db_path) {
            Ok(s) => s,
            Err(e) => {
                return CommandResult::Error(format!(
                    "Failed to open session database: {}\n\
                     The database is created automatically once sessions are stored.",
                    e
                ))
            }
        };

        let results = match store.search_sessions(query) {
            Ok(r) => r,
            Err(e) => {
                return CommandResult::Error(format!(
                    "Search failed: {}",
                    e
                ))
            }
        };

        if results.is_empty() {
            return CommandResult::Message(format!(
                "No sessions found matching \"{}\".",
                query
            ));
        }

        let mut out = format!(
            "Search results for \"{}\": {} session(s)\n\n",
            query,
            results.len()
        );
        for s in &results {
            let title = s.title.as_deref().unwrap_or("(untitled)");
            out.push_str(&format!(
                "  [{}] {} — {} ({} messages, updated {})\n",
                &s.id[..s.id.len().min(12)],
                title,
                s.model,
                s.message_count,
                &s.updated_at[..s.updated_at.len().min(10)],
            ));
        }
        out.push_str("\nTip: use /resume <session-id> to continue a session.");
        CommandResult::Message(out)
    }
}

// ---- /btw ----------------------------------------------------------------

#[async_trait]
impl SlashCommand for BtwCommand {
    fn name(&self) -> &str { "btw" }
    fn description(&self) -> &str { "Ask a side question without adding it to conversation history" }
    fn help(&self) -> &str {
        "Usage: /btw <question>\n\n\
         Submits a background question to the model without it becoming part of\n\
         the main conversation context. The response is shown inline but not\n\
         stored in the message history.\n\n\
         Example:\n\
           /btw what is the capital of France?"
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let question = args.trim();
        if question.is_empty() {
            return CommandResult::Error(
                "Usage: /btw <question>  — provide a question after /btw".to_string(),
            );
        }

        // Surface as a special user message tagged as a side-question so the
        // REPL/TUI can handle it as a non-history query. We inject a system tag
        // that tells the backend to answer but not record the exchange.
        CommandResult::UserMessage(format!(
            "[/btw side-question — answer inline, do not store in history]: {}",
            question
        ))
    }
}

// ---- /ctx-viz (context visualizer) ---------------------------------------

#[async_trait]
impl SlashCommand for CtxVizCommand {
    fn name(&self) -> &str { "ctx-viz" }
    fn aliases(&self) -> Vec<&str> { vec!["context-visualizer", "ctx"] }
    fn description(&self) -> &str { "Visualize context window usage breakdown by category" }
    fn help(&self) -> &str {
        "Usage: /ctx-viz\n\n\
         Shows a detailed breakdown of how the context window is being used:\n\
         - System prompt token estimate\n\
         - Conversation messages token estimate\n\
         - Tool results token estimate\n\
         - Total vs context window limit"
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let model = ctx.config.effective_model().to_string();
        let context_window: u64 = 200_000; // all current Claude models

        // Estimate system prompt tokens: rough chars/4 approximation
        // Build a minimal system prompt to estimate its size.
        let sys_prompt_chars: usize = ctx.config.custom_system_prompt
            .as_deref()
            .map(|s| s.len())
            .unwrap_or(2400 * 4); // fallback: ~2400 tokens worth
        let sys_prompt_tokens = (sys_prompt_chars / 4).max(1) as u64;

        // Estimate conversation tokens from messages
        let (conv_chars, tool_chars): (usize, usize) = ctx.messages.iter().fold(
            (0, 0),
            |(conv, tool), msg| {
                let text = msg.get_all_text();
                // Heuristic: if the message looks like a tool result, count separately
                if msg.role == claurst_core::types::Role::User && text.starts_with('[') {
                    (conv, tool + text.len())
                } else {
                    (conv + text.len(), tool)
                }
            },
        );

        let conv_tokens = (conv_chars / 4) as u64;
        let tool_tokens = (tool_chars / 4) as u64;
        let total_tokens = sys_prompt_tokens + conv_tokens + tool_tokens;
        let pct = (total_tokens as f64 / context_window as f64) * 100.0;

        let bar_width = 40usize;
        let filled = ((pct / 100.0) * bar_width as f64).round() as usize;
        let bar = "█".repeat(filled) + &"░".repeat(bar_width.saturating_sub(filled));

        CommandResult::Message(format!(
            "Context Window Usage\n\
             ────────────────────────────────────────\n\
             Model:            {model}\n\
             System prompt:    ~{sys:>7} tokens\n\
             Conversation:     ~{conv:>7} tokens\n\
             Tool results:     ~{tool:>7} tokens\n\
             ────────────────────────────────────────\n\
             Total:            ~{total:>7} / {window} tokens ({pct:.1}%)\n\
             [{bar}] {pct:.1}%\n\n\
             Use /compact to reduce context usage.",
            model = model,
            sys = sys_prompt_tokens,
            conv = conv_tokens,
            tool = tool_tokens,
            total = total_tokens,
            window = context_window,
            pct = pct,
            bar = bar,
        ))
    }
}

// ---- /sandbox-toggle -----------------------------------------------------

#[async_trait]
impl SlashCommand for SandboxToggleCommand {
    fn name(&self) -> &str { "sandbox-toggle" }
    fn aliases(&self) -> Vec<&str> { vec!["sandbox"] }
    fn description(&self) -> &str { "Enable or disable sandboxed execution of shell commands" }
    fn help(&self) -> &str {
        "Usage: /sandbox-toggle [on|off|exclude <pattern>|status]\n\n\
         Toggles sandboxed execution of bash/shell commands.\n\
         When sandbox mode is enabled, shell commands run in an isolated\n\
         environment to prevent unintended side effects.\n\n\
         Subcommands:\n\
           /sandbox-toggle           — toggle the current state\n\
           /sandbox-toggle on        — enable sandbox mode\n\
           /sandbox-toggle off       — disable sandbox mode\n\
           /sandbox-toggle status    — show current state and excluded patterns\n\
           /sandbox-toggle exclude <pattern>  — add a command pattern to exclusions\n\n\
         Sandbox is supported on macOS, Linux, and WSL2.\n\
         Note: A restart is recommended for full effect."
    }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let args = args.trim();

        // Platform support check: sandbox requires macOS or Linux (not Windows native).
        let platform = std::env::consts::OS;
        let is_wsl = std::env::var("WSL_DISTRO_NAME").is_ok()
            || std::env::var("WSL_INTEROP").is_ok();
        let is_supported = matches!(platform, "linux" | "macos") || is_wsl;

        // Handle subcommand: status
        if args == "status" {
            let ui = load_ui_settings();
            let mode = if ui.sandbox_mode.unwrap_or(false) { "enabled" } else { "disabled" };
            let excl = if ui.sandbox_excluded_commands.is_empty() {
                "(none)".to_string()
            } else {
                ui.sandbox_excluded_commands
                    .iter()
                    .map(|p| format!("  - {}", p))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let platform_note = if is_supported {
                format!("\u{2713} Supported on this platform ({})", platform)
            } else {
                format!("\u{2717} Not supported on this platform ({}). Requires macOS, Linux, or WSL2.", platform)
            };
            return CommandResult::Message(format!(
                "Sandbox mode: {}\n\
                 Platform:     {}\n\
                 Excluded command patterns:\n{}\n\n\
                 Use /sandbox-toggle [on|off] to change mode.\n\
                 Use /sandbox-toggle exclude <pattern> to add exclusions.",
                mode, platform_note, excl
            ));
        }

        // Handle subcommand: exclude <pattern>
        if let Some(rest) = args.strip_prefix("exclude").map(str::trim) {
            if rest.is_empty() {
                return CommandResult::Error(
                    "Usage: /sandbox-toggle exclude <command-pattern>\n\
                     Example: /sandbox-toggle exclude \"npm run test:*\"".to_string()
                );
            }
            // Strip surrounding quotes if present
            let pattern = rest.trim_matches(|c| c == '"' || c == '\'').to_string();
            if pattern.is_empty() {
                return CommandResult::Error("Pattern cannot be empty.".to_string());
            }
            match mutate_ui_settings(|s| {
                if !s.sandbox_excluded_commands.contains(&pattern) {
                    s.sandbox_excluded_commands.push(pattern.clone());
                }
            }) {
                Ok(_) => {
                    let settings_path = ui_settings_path();
                    return CommandResult::Message(format!(
                        "Added \"{}\" to sandbox excluded commands.\n\
                         Saved to: {}",
                        pattern,
                        settings_path.display()
                    ));
                }
                Err(e) => return CommandResult::Error(format!("Failed to save exclusion: {}", e)),
            }
        }

        // Platform guard for toggling on/off
        if !is_supported && (args == "on" || args == "enable" || args == "enabled"
            || args == "true" || args == "1" || args.is_empty())
        {
            let msg = if is_wsl {
                "Error: Sandboxing requires WSL2. WSL1 is not supported.".to_string()
            } else {
                format!(
                    "Error: Sandboxing is currently only supported on macOS, Linux, and WSL2.\n\
                     Current platform: {}",
                    platform
                )
            };
            // Only hard-block enabling; allow off/status even on unsupported platforms.
            if args != "off" && args != "disable" && args != "disabled"
                && args != "false" && args != "0"
            {
                return CommandResult::Error(msg);
            }
        }

        // Read current sandbox state from ui-settings
        let current_ui = load_ui_settings();
        let currently_enabled = current_ui.sandbox_mode.unwrap_or(false);

        let enable = match args {
            "on" | "enable" | "enabled" | "true" | "1" => true,
            "off" | "disable" | "disabled" | "false" | "0" => false,
            "" => !currently_enabled,
            other => {
                return CommandResult::Error(format!(
                    "Unknown argument '{}'. Use: /sandbox-toggle [on|off|status|exclude <pattern>]",
                    other
                ))
            }
        };

        match mutate_ui_settings(|s| s.sandbox_mode = Some(enable)) {
            Ok(_) => {
                let state = if enable { "enabled" } else { "disabled" };
                CommandResult::Message(format!(
                    "Sandbox mode {}. Restart recommended for full effect.\n\
                     Use /sandbox-toggle exclude <pattern> to bypass sandboxing for specific commands.",
                    state
                ))
            }
            Err(e) => CommandResult::Error(format!("Failed to save sandbox setting: {}", e)),
        }
    }
}

// ---- /heapdump -----------------------------------------------------------

#[async_trait]
impl SlashCommand for HeapdumpCommand {
    fn name(&self) -> &str { "heapdump" }
    fn description(&self) -> &str { "Show process memory and diagnostic information" }
    fn help(&self) -> &str {
        "Usage: /heapdump\n\n\
         Displays a diagnostic snapshot of the current process:\n\
         process ID, platform, architecture, and available memory info.\n\
         On Linux, reads /proc/self/status for RSS/VmPeak figures.\n\
         On other platforms, reports what is available from the OS."
    }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let pid = std::process::id();
        let platform = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("  Process ID : {}", pid));
        lines.push(format!("  Platform   : {}", platform));
        lines.push(format!("  Arch       : {}", arch));

        // On Linux, pull memory figures from /proc/self/status
        #[cfg(target_os = "linux")]
        {
            match std::fs::read_to_string("/proc/self/status") {
                Ok(status) => {
                    for line in status.lines() {
                        let key = line.split(':').next().unwrap_or("").trim();
                        if matches!(key, "VmPeak" | "VmRSS" | "VmSize" | "VmData" | "Threads") {
                            let value = line.split(':').nth(1).unwrap_or("").trim();
                            lines.push(format!("  {:10} : {}", key, value));
                        }
                    }
                }
                Err(e) => {
                    lines.push(format!("  (could not read /proc/self/status: {})", e));
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            lines.push("  Memory stats: not available on this platform".to_string());
            lines.push("  (Linux /proc/self/status required for detailed figures)".to_string());
        }

        let body = lines.join("\n");
        CommandResult::Message(format!(
            "Heap Diagnostic\n\
             ─────────────────────────────\n\
             {body}"
        ))
    }
}

// ---- /insights -----------------------------------------------------------

#[async_trait]
impl SlashCommand for InsightsCommand {
    fn name(&self) -> &str { "insights" }
    fn description(&self) -> &str { "Generate a session analysis report with conversation statistics" }
    fn help(&self) -> &str {
        "Usage: /insights\n\n\
         Analyses the current conversation and prints a statistics report:\n\
         turn count, token usage, tools invoked, most-used tool, and more."
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let messages = &ctx.messages;

        // Count turns (user / assistant pairs)
        let user_turns: usize = messages.iter()
            .filter(|m| matches!(m.role, claurst_core::types::Role::User))
            .count();
        let assistant_turns: usize = messages.iter()
            .filter(|m| matches!(m.role, claurst_core::types::Role::Assistant))
            .count();
        let total_turns = user_turns.min(assistant_turns);

        // Count tool_use blocks and track frequency
        let mut tool_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for msg in messages {
            for block in msg.get_tool_use_blocks() {
                if let claurst_core::types::ContentBlock::ToolUse { name, .. } = block {
                    *tool_counts.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }
        let total_tool_calls: usize = tool_counts.values().sum();
        let most_frequent_tool = tool_counts
            .iter()
            .max_by_key(|(_, &v)| v)
            .map(|(k, v)| format!("{} ({} calls)", k, v))
            .unwrap_or_else(|| "none".to_string());

        // Token stats from cost_tracker
        let input_tokens = ctx.cost_tracker.input_tokens();
        let output_tokens = ctx.cost_tracker.output_tokens();
        let total_tokens = ctx.cost_tracker.total_tokens();
        let total_cost = ctx.cost_tracker.total_cost_usd();

        let avg_tokens_per_turn = if total_turns > 0 {
            total_tokens / total_turns as u64
        } else {
            0
        };

        CommandResult::Message(format!(
            "Session Insights\n\
             ──────────────────────────────────────\n\
             Conversation\n\
             ├─ User turns          : {user_turns}\n\
             ├─ Assistant turns     : {assistant_turns}\n\
             └─ Completed exchanges : {total_turns}\n\
             \n\
             Tokens\n\
             ├─ Input               : {input_tokens}\n\
             ├─ Output              : {output_tokens}\n\
             ├─ Total               : {total_tokens}\n\
             └─ Avg per exchange    : {avg_tokens_per_turn}\n\
             \n\
             Cost\n\
             └─ Estimated USD       : ${total_cost:.4}\n\
             \n\
             Tools\n\
             ├─ Total calls         : {total_tool_calls}\n\
             └─ Most used           : {most_frequent_tool}",
            user_turns = user_turns,
            assistant_turns = assistant_turns,
            total_turns = total_turns,
            input_tokens = input_tokens,
            output_tokens = output_tokens,
            total_tokens = total_tokens,
            avg_tokens_per_turn = avg_tokens_per_turn,
            total_cost = total_cost,
            total_tool_calls = total_tool_calls,
            most_frequent_tool = most_frequent_tool,
        ))
    }
}

// ---- /ultrareview --------------------------------------------------------

#[async_trait]
impl SlashCommand for UltrareviewCommand {
    fn name(&self) -> &str { "ultrareview" }
    fn description(&self) -> &str { "Run an exhaustive multi-dimensional code review" }
    fn help(&self) -> &str {
        "Usage: /ultrareview [path]\n\n\
         Runs a comprehensive code review that goes beyond /review and\n\
         /security-review. Covers: security (OWASP Top 10), performance,\n\
         maintainability, test coverage, error handling, API design,\n\
         documentation, accessibility, and architectural concerns.\n\
         Each finding is tagged by category and severity."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let target = if args.trim().is_empty() {
            ctx.working_dir.display().to_string()
        } else {
            args.trim().to_string()
        };

        CommandResult::UserMessage(format!(
            "Please perform an **ultra-comprehensive code review** of the code in `{target}`.\n\n\
             This review must go beyond a standard review and cover ALL of the following dimensions:\n\n\
             ## 1. Security (OWASP Top 10 + extras)\n\
             - Injection vulnerabilities (SQL, command, LDAP, XSS, SSTI, CRLF)\n\
             - Broken authentication / session management\n\
             - Sensitive data exposure (secrets, PII, tokens in logs or source)\n\
             - XML/JSON External Entity (XXE) processing\n\
             - Broken access control and privilege escalation paths\n\
             - Security misconfiguration (default creds, open ports, verbose errors)\n\
             - Cross-site scripting (Stored, Reflected, DOM-based)\n\
             - Insecure deserialization\n\
             - Using components with known vulnerabilities (outdated deps)\n\
             - Insufficient logging and monitoring\n\
             - Path traversal and file inclusion\n\
             - Race conditions, TOCTOU, deadlocks\n\
             - Cryptographic weaknesses (weak algorithms, key reuse, bad IV)\n\
             - Supply chain / dependency confusion risks\n\n\
             ## 2. Performance\n\
             - Algorithmic complexity: O(n²) or worse in hot paths\n\
             - Unnecessary allocations, copies, or clones\n\
             - Database N+1 query patterns\n\
             - Missing indexes on frequently queried fields\n\
             - Blocking I/O in async contexts\n\
             - Unbounded loops or recursion\n\
             - Memory leaks or resource leaks (file handles, sockets)\n\
             - Caching opportunities\n\n\
             ## 3. Maintainability & Code Quality\n\
             - Functions / methods exceeding 50 lines\n\
             - Deep nesting (>4 levels)\n\
             - Duplicated logic (DRY violations)\n\
             - Magic numbers and strings without named constants\n\
             - Misleading names (variables, functions, types)\n\
             - Dead code and unused imports\n\
             - Overly complex conditionals\n\
             - Coupling: tight coupling between unrelated modules\n\n\
             ## 4. Error Handling\n\
             - Swallowed errors (empty catch blocks, `unwrap()` without context)\n\
             - Panic-able paths in library code\n\
             - Missing input validation at trust boundaries\n\
             - Unclear error messages that hinder debugging\n\
             - Error type inconsistency across the codebase\n\n\
             ## 5. Test Coverage\n\
             - Missing unit tests for critical logic\n\
             - Missing integration tests for external boundaries\n\
             - Tests with no assertions\n\
             - Tests that are brittle (time-dependent, order-dependent)\n\
             - Missing negative / edge-case tests\n\
             - Mocking strategy concerns\n\n\
             ## 6. API Design\n\
             - Unclear or inconsistent naming conventions\n\
             - Functions with too many parameters (>5)\n\
             - Mutable global state\n\
             - Missing or incorrect use of visibility modifiers\n\
             - Breaking changes risk in public interfaces\n\
             - Lack of builder or fluent patterns where appropriate\n\n\
             ## 7. Documentation\n\
             - Missing doc comments on public items\n\
             - Outdated or misleading comments\n\
             - Undocumented panics, unsafe blocks, or invariants\n\
             - Missing README or high-level architectural overview\n\n\
             ## 8. Architectural Concerns\n\
             - Single Responsibility Principle violations\n\
             - Circular dependencies\n\
             - Missing abstraction layers\n\
             - Hardcoded configuration that should be externalised\n\
             - Observability gaps (missing tracing, metrics, structured logs)\n\n\
             ## Output Format\n\
             For **every** finding, provide:\n\
             - **Category** (from the dimensions above)\n\
             - **Severity**: Critical / High / Medium / Low / Informational\n\
             - **File** and **line number** (if applicable)\n\
             - **Description** of the issue\n\
             - **Impact**: what can go wrong\n\
             - **Recommended fix** with a code snippet where helpful\n\n\
             Start by reading the main source files, dependency manifests, and any CI/CD configuration.\n\
             Group findings by severity (Critical first). Conclude with a prioritised action plan.",
            target = target,
        ))
    }
}

// ---- Named-command slash adapters ----------------------------------------

#[async_trait]
impl SlashCommand for NamedCommandAdapter {
    fn name(&self) -> &str { self.slash_name }

    fn aliases(&self) -> Vec<&str> { self.slash_aliases.to_vec() }

    fn description(&self) -> &str { self.slash_description }

    fn help(&self) -> &str { self.slash_help }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        execute_named_command_from_slash(self.target_name, args, ctx)
    }
}

// ---- /undo (alias for /revert targeting the most recent assistant turn) ----

#[async_trait]
impl SlashCommand for UndoCommand {
    fn name(&self) -> &str { "undo" }
    fn aliases(&self) -> Vec<&str> { vec![] }
    fn description(&self) -> &str { "Revert all file changes from the last assistant turn (alias: /revert)" }
    fn help(&self) -> &str {
        "Usage: /undo\n\nReverts all file changes made during the most recent assistant turn.\n\
         For finer control use /revert. To list what changed, use /checkpoints."
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        RevertCommand.execute("", ctx).await
    }
}

// ---- /revert ---------------------------------------------------------------

#[async_trait]
impl SlashCommand for RevertCommand {
    fn name(&self) -> &str { "revert" }
    fn description(&self) -> &str { "Revert file changes from an assistant turn back to pre-turn state" }
    fn help(&self) -> &str {
        "Usage: /revert [<n>|<uuid>]\n\n\
         Without args: revert the most recent assistant turn.\n\
         With a number n: revert the n-th most recent assistant turn (1 = latest).\n\
         With a uuid: revert the turn whose message id starts with that string.\n\n\
         This uses the shadow-git snapshot to restore all files that were\n\
         changed during the target turn, and removes that turn (and any later\n\
         turns) from the session transcript.\n\n\
         Examples:\n\
           /revert        — revert last turn\n\
           /revert 2      — revert the second-to-last turn\n\
           /revert abc123 — revert the turn with uuid starting 'abc123'"
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let snap = match claurst_core::snapshot::get_or_create(&ctx.working_dir) {
            Some(s) => s,
            None => return CommandResult::Error(
                "Snapshot system unavailable (git not found or not a git repo).".into()
            ),
        };

        // Collect assistant messages that have a snapshot patch (newest last).
        let checkpoints: Vec<&claurst_core::types::Message> = ctx.messages.iter()
            .filter(|m| {
                m.role == claurst_core::types::Role::Assistant
                    && m.snapshot_patch.is_some()
            })
            .collect();

        if checkpoints.is_empty() {
            return CommandResult::Message(
                "No revertible turns found. Run /checkpoints to see recorded file changes.".into()
            );
        }

        // Select the target turn.
        let args = args.trim();
        let target = if args.is_empty() {
            checkpoints.last().copied()
        } else if let Ok(n) = args.parse::<usize>() {
            if n == 0 || n > checkpoints.len() {
                return CommandResult::Error(format!(
                    "Turn {} out of range (1–{}).", n, checkpoints.len()
                ));
            }
            Some(checkpoints[checkpoints.len() - n])
        } else {
            checkpoints.iter().copied()
                .find(|m| m.uuid.as_deref().map_or(false, |u| u.starts_with(args)))
        };

        let target = match target {
            Some(m) => m,
            None => return CommandResult::Error(format!("No turn found matching '{args}'.")),
        };

        // Collect all patches from this turn onward to revert.
        let target_uuid = match target.uuid.clone() {
            Some(u) => u,
            None => return CommandResult::Error("Target turn has no uuid; cannot revert.".into()),
        };

        let patches: Vec<claurst_core::snapshot::Patch> = ctx.messages.iter()
            .skip_while(|m| m.uuid.as_deref() != Some(&target_uuid))
            .filter_map(|m| m.snapshot_patch.clone())
            .collect();

        if patches.is_empty() {
            return CommandResult::Message("No file changes recorded for that turn.".into());
        }

        // Revert files.
        snap.revert(&patches).await;

        // Record the revert in the session transcript. NON-DESTRUCTIVE (#234):
        // rather than truncating, point the active leaf at the turn *before* the
        // target so the reverted turn (and everything after it) is retained on a
        // sibling branch that can be returned to. `branch_before` only falls
        // back to a destructive truncate for legacy/unchained transcripts.
        let project_root = claurst_core::git_utils::get_repo_root(&ctx.working_dir)
            .unwrap_or_else(|| ctx.working_dir.clone());
        let path = claurst_core::session_storage::transcript_path(&project_root, &ctx.session_id);
        if path.exists() {
            if let Err(e) = claurst_core::session_storage::branch_before(&path, &target_uuid).await {
                return CommandResult::Error(format!("Reverted files but could not update transcript: {e}"));
            }
        }

        let file_count: usize = patches.iter().map(|p| p.files.len()).sum();
        CommandResult::Message(format!(
            "Reverted {} file(s) changed during turn {}. Later turns kept on a branch.",
            file_count,
            &target_uuid[..target_uuid.len().min(8)],
        ))
    }
}

// ---- /checkpoints ----------------------------------------------------------

#[async_trait]
impl SlashCommand for CheckpointsCommand {
    fn name(&self) -> &str { "checkpoints" }
    fn description(&self) -> &str { "List assistant turns that have recorded file changes" }
    fn help(&self) -> &str {
        "Usage: /checkpoints\n\nShows all assistant turns in this session that modified files,\n\
         with file counts.  Use /revert <n> to roll back to a specific turn."
    }

    async fn execute(&self, _args: &str, ctx: &mut CommandContext) -> CommandResult {
        let checkpoints: Vec<(usize, &claurst_core::types::Message)> = ctx.messages.iter()
            .enumerate()
            .filter(|(_, m)| {
                m.role == claurst_core::types::Role::Assistant
                    && m.snapshot_patch.is_some()
            })
            .collect();

        if checkpoints.is_empty() {
            return CommandResult::Message(
                "No file-change checkpoints recorded yet for this session.\n\
                 Checkpoints are created automatically when the assistant modifies files.".into()
            );
        }

        let total = checkpoints.len();
        let mut lines = vec![format!("{} checkpoint(s):", total)];
        for (rank, (_, msg)) in checkpoints.iter().rev().enumerate() {
            let uuid_short = msg.uuid.as_deref()
                .map(|u| &u[..u.len().min(8)])
                .unwrap_or("?");
            let file_count = msg.snapshot_patch.as_ref().map_or(0, |p| p.files.len());
            let preview: Vec<String> = msg.snapshot_patch.as_ref()
                .map(|p| {
                    p.files.iter().take(3)
                        .map(|f| f.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default())
                        .collect()
                })
                .unwrap_or_default();
            let preview_str = if preview.len() == file_count {
                preview.join(", ")
            } else {
                format!("{}, …", preview.join(", "))
            };
            lines.push(format!(
                "  [{}] {} — {} file(s): {}",
                rank + 1, uuid_short, file_count, preview_str
            ));
        }
        lines.push(String::new());
        lines.push("Use /revert <n> to revert to before turn [n].".into());
        CommandResult::Message(lines.join("\n"))
    }
}

// ---- /snapshot (show snapshot diff for a recorded turn) ------------------

#[async_trait]
impl SlashCommand for SnapshotDiffCommand {
    fn name(&self) -> &str { "snapshot" }
    fn description(&self) -> &str { "Show shadow-git diff of file changes from an assistant turn" }
    fn help(&self) -> &str {
        "Usage: /snapshot [<n>|<hash>]\n\n\
         Without args: show unified diff for the most recent assistant turn.\n\
         With a number: show diff for the n-th most recent turn (1 = latest).\n\
         With a hash: show diff against that explicit snapshot tree hash.\n\n\
         See also: /checkpoints (list turns), /revert (roll back files)."
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        let snap = match claurst_core::snapshot::get_or_create(&ctx.working_dir) {
            Some(s) => s,
            None => return CommandResult::Error(
                "Snapshot system unavailable (git not found or not a git repo).".into()
            ),
        };

        let args = args.trim();

        // If a raw hash was passed, use it directly.
        let hash = if !args.is_empty() && args.chars().all(|c| c.is_ascii_hexdigit()) && args.len() >= 8 {
            args.to_string()
        } else {
            // Otherwise find the n-th most recent checkpoint.
            let checkpoints: Vec<&claurst_core::snapshot::Patch> = ctx.messages.iter()
                .filter_map(|m| {
                    if m.role == claurst_core::types::Role::Assistant {
                        m.snapshot_patch.as_ref()
                    } else {
                        None
                    }
                })
                .collect();

            if checkpoints.is_empty() {
                return CommandResult::Message(
                    "No snapshot checkpoints recorded yet. File changes will appear here after the next assistant turn.".into()
                );
            }

            let idx = if args.is_empty() {
                0
            } else {
                match args.parse::<usize>() {
                    Ok(n) if n >= 1 && n <= checkpoints.len() => n - 1,
                    _ => return CommandResult::Error(format!(
                        "Turn '{}' out of range (1–{}).", args, checkpoints.len()
                    )),
                }
            };
            // Reverse so idx=0 is newest.
            let patch = checkpoints[checkpoints.len() - 1 - idx];
            patch.hash.clone()
        };

        let diff = snap.diff(&hash).await;
        if diff.is_empty() {
            CommandResult::Message(format!("No changes since snapshot {}.", &hash[..hash.len().min(8)]))
        } else {
            CommandResult::Message(diff)
        }
    }
}


// ---- /providers -------------------------------------------------------------

#[async_trait]
impl SlashCommand for ProvidersCommand {
    fn name(&self) -> &str { "providers" }
    fn description(&self) -> &str { "List available AI providers and their status" }
    fn help(&self) -> &str {
        "Usage: /providers\n\nList all providers registered in the model registry with their\nmodel counts, context windows, and pricing information."
    }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let registry = claurst_api::ModelRegistry::new();
        let all = registry.list_all();

        if all.is_empty() {
            return CommandResult::Message("No providers available.".to_string());
        }

        // Group by provider
        use std::collections::HashMap;
        let mut by_provider: HashMap<String, Vec<_>> = HashMap::new();
        for entry in &all {
            by_provider
                .entry(entry.info.provider_id.to_string())
                .or_default()
                .push(entry);
        }

        // Sort providers alphabetically for stable output
        let mut provider_keys: Vec<String> = by_provider.keys().cloned().collect();
        provider_keys.sort();

        let mut lines = vec!["Available providers:\n".to_string()];
        for provider in &provider_keys {
            let models = &by_provider[provider];
            lines.push(format!("\n{} ({} model{})", provider.to_uppercase(), models.len(),
                if models.len() == 1 { "" } else { "s" }));
            for m in models.iter().take(3) {
                let cost_str = match (m.cost_input, m.cost_output) {
                    (Some(i), Some(o)) => format!("${:.2}/${:.2} per 1M", i, o),
                    _ => "free/local".to_string(),
                };
                lines.push(format!("  {} — {}K ctx, {}",
                    m.info.id, m.info.context_window / 1000, cost_str));
            }
            if models.len() > 3 {
                lines.push(format!("  ... and {} more", models.len() - 3));
            }
        }

        CommandResult::Message(lines.join("\n"))
    }
}

// ---- /connect -------------------------------------------------------------

#[async_trait]
impl SlashCommand for ConnectCommand {
    fn name(&self) -> &str { "connect" }
    fn description(&self) -> &str { "Connect an AI provider" }
    fn help(&self) -> &str {
        "Usage: /connect\n\nOpens the interactive provider picker dialog.\nSelect a provider to see setup instructions."
    }

    async fn execute(&self, _args: &str, _ctx: &mut CommandContext) -> CommandResult {
        // This is handled by the TUI interceptor — opening the connect dialog.
        CommandResult::Message("Use the connect dialog to set up a provider.".to_string())
    }
}

// ---- /agent ---------------------------------------------------------------

#[async_trait]
impl SlashCommand for AgentCommand {
    fn name(&self) -> &str { "agent" }
    fn description(&self) -> &str { "List available agents or get info about a specific agent" }
    fn help(&self) -> &str {
        "Usage: /agent [name]\n\nWithout arguments, lists all available named agents.\nWith a name, shows details for that agent.\n\nTo use an agent, start Claurst with: --agent <name>"
    }

    async fn execute(&self, args: &str, ctx: &mut CommandContext) -> CommandResult {
        use std::collections::HashMap;

        // Merge built-in defaults with user-defined agents (user wins on collision).
        let mut all_agents: HashMap<String, claurst_core::AgentDefinition> =
            claurst_core::default_agents();
        all_agents.extend(ctx.config.agents.clone());

        let agent_name = args.trim();

        if agent_name.is_empty() {
            // List all visible agents.
            let mut keys: Vec<&String> = all_agents
                .iter()
                .filter(|(_, d)| d.visible)
                .map(|(k, _)| k)
                .collect();
            keys.sort();

            let mut output = "Available agents:\n\n".to_string();
            for name in keys {
                let def = &all_agents[name];
                output.push_str(&format!(
                    "  @{} — {}\n    access: {}{}\n",
                    name,
                    def.description.as_deref().unwrap_or(""),
                    def.access,
                    def.max_turns
                        .map(|t| format!(", max_turns: {}", t))
                        .unwrap_or_default(),
                ));
            }
            output.push_str("\nUse --agent <name> when starting Claurst to activate an agent.");
            CommandResult::Message(output)
        } else if let Some(def) = all_agents.get(agent_name) {
            // Show details for the named agent.
            let mut output = format!("Agent: @{}\n", agent_name);
            if let Some(ref desc) = def.description {
                output.push_str(&format!("Description: {}\n", desc));
            }
            output.push_str(&format!("Access: {}\n", def.access));
            if let Some(ref model) = def.model {
                output.push_str(&format!("Model: {}\n", model));
            }
            if let Some(t) = def.max_turns {
                output.push_str(&format!("Max turns: {}\n", t));
            }
            if let Some(ref color) = def.color {
                output.push_str(&format!("Color: {}\n", color));
            }
            if let Some(ref prompt) = def.prompt {
                output.push_str(&format!("\nSystem prompt prefix:\n  {}\n", prompt));
            }
            output.push_str(&format!(
                "\nTo activate: claurst --agent {}", agent_name
            ));
            CommandResult::Message(output)
        } else {
            CommandResult::Error(format!(
                "Unknown agent '{}'. Run /agent to see available agents.",
                agent_name
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Return all built-in slash commands.
pub fn all_commands() -> Vec<Box<dyn SlashCommand>> {
    vec![
        Box::new(HelpCommand),
        Box::new(ClearCommand),
        Box::new(CompactCommand),
        Box::new(CostCommand),
        Box::new(ExitCommand),
        Box::new(ModelCommand),
        Box::new(ConfigCommand),
        Box::new(ColorCommand),
        Box::new(PluginCommand),
        Box::new(VersionCommand),
        Box::new(ResumeCommand),
        Box::new(ReloadPluginsCommand),
        Box::new(StatusCommand),
        Box::new(DiffCommand),
        Box::new(MemoryCommand),
        Box::new(BugCommand),
        Box::new(UsageCommand),
        Box::new(DoctorCommand),
        Box::new(LoginCommand),
        Box::new(LogoutCommand),
        Box::new(AccountsCommand),
        Box::new(SwitchCommand),
        Box::new(RefreshCommand),
        Box::new(CavemanCommand),
        Box::new(RockyCommand),
        Box::new(NormalCommand),
        Box::new(InitCommand),
        Box::new(ReviewCommand),
        Box::new(HooksCommand),
        Box::new(ImportConfigCommand),
        Box::new(McpCommand),
        Box::new(PermissionsCommand),
        Box::new(PlanCommand),
        Box::new(TasksCommand),
        Box::new(SessionCommand),
        Box::new(ForkCommand),
        Box::new(ThinkingCommand),
        Box::new(ThemeCommand),
        Box::new(OutputStyleCommand),
        Box::new(KeybindingsCommand),
        Box::new(PrivacySettingsCommand),
        // New commands
        Box::new(ExportCommand),
        Box::new(ShareCommand),
        Box::new(LinksCommand),
        Box::new(SkillsCommand),
        Box::new(RewindCommand),
        Box::new(StatsCommand),
        Box::new(FilesCommand),
        Box::new(RenameCommand),
        Box::new(EffortCommand),
        Box::new(SummaryCommand),
        Box::new(CommitCommand),
        Box::new(NamedCommandAdapter {
            slash_name: "add-dir",
            target_name: "add-dir",
            slash_aliases: &[],
            slash_description: "Add a directory to Claurst's allowed workspace paths",
            slash_help: "Usage: /add-dir <path>",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "agents",
            target_name: "agents",
            slash_aliases: &[],
            slash_description: "Manage and configure sub-agents",
            slash_help: "Usage: /agents [list|create|edit|delete] [name]",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "branch",
            target_name: "branch",
            slash_aliases: &[],
            slash_description: "Create a branch of the current conversation at this point",
            slash_help: "Usage: /branch [create|switch|list] [name]",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "tag",
            target_name: "tag",
            slash_aliases: &[],
            slash_description: "Toggle a searchable tag on the current session",
            slash_help: "Usage: /tag [list|add|remove] [tag]",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "passes",
            target_name: "passes",
            slash_aliases: &[],
            slash_description: "Share a free week of Claurst with friends",
            slash_help: "Usage: /passes",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "ide",
            target_name: "ide",
            slash_aliases: &[],
            slash_description: "Manage IDE integrations and show status",
            slash_help: "Usage: /ide [status|connect|disconnect|open]",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "pr-comments",
            target_name: "pr-comments",
            slash_aliases: &[],
            slash_description: "Get comments from a GitHub pull request",
            slash_help: "Usage: /pr-comments <PR-number>",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "desktop",
            target_name: "desktop",
            slash_aliases: &[],
            slash_description: "Open the Claurst desktop app",
            slash_help: "Usage: /desktop",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "mobile",
            target_name: "mobile",
            slash_aliases: &[],
            slash_description: "Set up Claurst on mobile",
            slash_help: "Usage: /mobile",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "install-github-app",
            target_name: "install-github-app",
            slash_aliases: &[],
            slash_description: "Set up Claurst GitHub Actions for a repository",
            slash_help: "Usage: /install-github-app",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "web-setup",
            target_name: "remote-setup",
            slash_aliases: &["remote-setup"],
            slash_description: "Configure a remote Claurst environment",
            slash_help: "Usage: /web-setup",
        }),
        Box::new(NamedCommandAdapter {
            slash_name: "stickers",
            target_name: "stickers",
            slash_aliases: &[],
            slash_description: "View collected stickers",
            slash_help: "Usage: /stickers",
        }),
        // Batch-1 new commands
        Box::new(RemoteControlCommand),
        Box::new(RemoteEnvCommand),
        Box::new(ContextCommand),
        Box::new(CopyCommand),
        Box::new(ChromeCommand),
        Box::new(VimCommand),
        Box::new(VoiceCommand),
        Box::new(UpgradeCommand),
        Box::new(ReleaseNotesCommand),
        Box::new(RateLimitOptionsCommand),
        Box::new(StatuslineCommand),
        Box::new(SecurityReviewCommand),
        Box::new(TerminalSetupCommand),
        Box::new(ExtraUsageCommand),
        Box::new(FastCommand),
        Box::new(ThinkBackCommand),
        Box::new(ThinkBackPlayCommand),
        Box::new(FeedbackCommand),
        Box::new(ColorSetCommand),
        // New commands: teleport, btw, ctx-viz, sandbox-toggle
        Box::new(TeleportCommand),
        Box::new(BtwCommand),
        Box::new(CtxVizCommand),
        Box::new(SandboxToggleCommand),
        // Advisor and Slack integration
        Box::new(AdvisorCommand),
        Box::new(InstallSlackAppCommand),
        // Diagnostics / analysis
        Box::new(HeapdumpCommand),
        Box::new(InsightsCommand),
        Box::new(UltrareviewCommand),
        // Snapshot / revert system
        Box::new(UndoCommand),
        Box::new(RevertCommand),
        Box::new(CheckpointsCommand),
        Box::new(SnapshotDiffCommand),
        // Multi-provider support
        Box::new(ProvidersCommand),
        Box::new(ConnectCommand),
        // Named agent system
        Box::new(AgentCommand),
        // Session search (SQLite)
        Box::new(SearchCommand),
        // Managed agent (manager-executor) architecture
        Box::new(ManagedAgentsCommand),
        // Durable long-running goals
        Box::new(GoalCommand),
    ]
}

/// Find a command by name or alias.
pub fn find_command(name: &str) -> Option<Box<dyn SlashCommand>> {
    let name = name.trim_start_matches('/');
    all_commands().into_iter().find(|c| {
        c.name() == name || c.aliases().contains(&name)
    })
}

/// Build `HelpEntry` values for all non-hidden commands, suitable for
/// populating `HelpOverlay::commands` at startup.
pub fn build_help_entries() -> Vec<claurst_tui::overlays::HelpEntry> {
    all_commands()
        .iter()
        .filter(|c| !c.hidden())
        .map(|c| claurst_tui::overlays::HelpEntry {
            name: c.name().to_string(),
            aliases: c.aliases().join(", "),
            description: c.description().to_string(),
            category: command_category(c.name()).to_string(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// User-defined command templates (Feature 2)
// ---------------------------------------------------------------------------

/// A slash command backed by a user-defined template in `settings.json`.
struct TemplateCommand {
    name: String,
    template: claurst_core::CommandTemplate,
}

#[async_trait]
impl SlashCommand for TemplateCommand {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str {
        self.template.description.as_deref().unwrap_or("Custom command")
    }
    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let mut words = args.split_whitespace();
        let arg1 = words.next().unwrap_or("");
        let arg2 = words.next().unwrap_or("");
        let prompt = self.template.template
            .replace("$ARGUMENTS", args)
            .replace("$1", arg1)
            .replace("$2", arg2);
        CommandResult::UserMessage(prompt)
    }
}

/// Build slash commands from user-defined command templates stored in
/// `settings.commands`.
pub fn commands_from_settings(settings: &claurst_core::Settings) -> Vec<Box<dyn SlashCommand>> {
    settings.commands.iter().map(|(name, template)| {
        Box::new(TemplateCommand {
            name: name.clone(),
            template: template.clone(),
        }) as Box<dyn SlashCommand>
    }).collect()
}

// ---------------------------------------------------------------------------
// Discovered skill commands (from .claurst/skills/ and git URLs)
// ---------------------------------------------------------------------------

/// A slash command backed by a discovered skill markdown file.
struct SkillCommand {
    name: String,
    description: String,
    template: String,
}

#[async_trait]
impl SlashCommand for SkillCommand {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { &self.description }

    async fn execute(&self, args: &str, _ctx: &mut CommandContext) -> CommandResult {
        let mut words = args.split_whitespace();
        let arg1 = words.next().unwrap_or("");
        let arg2 = words.next().unwrap_or("");
        let prompt = self.template
            .replace("$ARGUMENTS", args)
            .replace("$1", arg1)
            .replace("$2", arg2);
        CommandResult::UserMessage(prompt)
    }
}

/// Build slash commands from skill markdown files discovered on the filesystem
/// and from configured git URLs.
///
/// Pass the project `cwd` and the `skills` section of the effective config.
/// Bundled skills take precedence — any discovered skill whose name clashes
/// with a built-in command will be silently skipped.
pub fn commands_from_discovered_skills(
    cwd: &std::path::Path,
    skills_config: &claurst_core::SkillsConfig,
) -> Vec<Box<dyn SlashCommand>> {
    let discovered = claurst_core::discover_skills(cwd, skills_config);
    // Build a set of built-in command names so we can skip collisions.
    let all_cmds = all_commands();
    let builtin_names: std::collections::HashSet<&str> = all_cmds
        .iter()
        .map(|c| c.name())
        .collect();

    discovered
        .into_values()
        .filter(|skill| !builtin_names.contains(skill.name.as_str()))
        .map(|skill| {
            Box::new(SkillCommand {
                name: skill.name,
                description: skill.description,
                template: skill.template,
            }) as Box<dyn SlashCommand>
        })
        .collect()
}

/// Execute a slash command string (with leading /).
pub async fn execute_command(
    input: &str,
    ctx: &mut CommandContext,
) -> Option<CommandResult> {
    if !claurst_tui::input::is_slash_command(input) { return None; }
    let (name, args) = claurst_tui::input::parse_slash_command(input);

    // First check built-in commands.
    if let Some(cmd) = find_command(name) {
        return Some(cmd.execute(args, ctx).await);
    }

    // Check user-defined command templates from settings.
    let cmd_name = name.trim_start_matches('/');
    if let Some(tmpl) = ctx.config.commands.get(cmd_name).cloned() {
        let tc = TemplateCommand { name: cmd_name.to_string(), template: tmpl };
        return Some(tc.execute(args, ctx).await);
    }

    // Check discovered skill commands (from .claurst/skills/, git URLs, etc.).
    {
        let discovered = claurst_core::discover_skills(&ctx.working_dir, &ctx.config.skills);
        if let Some(skill) = discovered.get(cmd_name) {
            let sc = SkillCommand {
                name: skill.name.clone(),
                description: skill.description.clone(),
                template: skill.template.clone(),
            };
            return Some(sc.execute(args, ctx).await);
        }
    }

    // Then check plugin-defined slash commands.
    let project_dir = ctx.working_dir.clone();
    let registry = claurst_plugins::load_plugins(&project_dir, &[]).await;
    for cmd_def in registry.all_command_defs() {
        if cmd_def.name == cmd_name {
            let adapter = PluginSlashCommandAdapter { def: cmd_def };
            return Some(adapter.execute(args, ctx).await);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Named commands module (top-level `claude <name>` subcommands)
// ---------------------------------------------------------------------------
pub mod named_commands;

// ---------------------------------------------------------------------------
// Stats analytics (persisted transcript aggregation) — backs `claurst stats`.
// The current-session `/stats` slash command lives above; this module reads
// JSONL transcripts on disk.
// ---------------------------------------------------------------------------
pub mod stats;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use claurst_core::cost::CostTracker;

    fn make_ctx() -> CommandContext {
        CommandContext {
            config: claurst_core::config::Config::default(),
            cost_tracker: CostTracker::new(),
            messages: vec![],
            working_dir: std::path::PathBuf::from("."),
            session_id: "test-session".to_string(),
            session_title: None,
            remote_session_url: None,
            mcp_manager: None,
            mcp_auth_runner: None,
        }
    }

    // ---- Command registry tests ---------------------------------------------

    #[test]
    fn test_all_commands_non_empty() {
        assert!(!all_commands().is_empty());
    }

    #[test]
    fn test_all_commands_have_unique_names() {
        let mut names = std::collections::HashSet::new();
        for cmd in all_commands() {
            assert!(
                names.insert(cmd.name().to_string()),
                "Duplicate command name: {}",
                cmd.name()
            );
        }
    }

    #[test]
    fn test_find_command_by_name() {
        assert!(find_command("help").is_some());
        assert!(find_command("clear").is_some());
        assert!(find_command("exit").is_some());
        assert!(find_command("model").is_some());
        assert!(find_command("refresh").is_some());
        assert!(find_command("version").is_some());
    }

    #[test]
    fn test_find_command_with_slash_prefix() {
        // find_command should strip the leading / before lookup
        assert!(find_command("/help").is_some());
        assert!(find_command("/clear").is_some());
    }

    #[test]
    fn test_find_command_by_alias() {
        // /help has aliases "h" and "?"
        assert!(find_command("h").is_some());
        assert!(find_command("?").is_some());
        // /clear has alias "c"
        assert!(find_command("c").is_some());
        assert!(find_command("settings").is_some());
        assert!(find_command("continue").is_some());
        assert!(find_command("bug").is_some());
        assert!(find_command("bashes").is_some());
        assert!(find_command("remote").is_some());
        assert!(find_command("remote-setup").is_some());
    }

    #[test]
    fn test_find_command_not_found() {
        assert!(find_command("nonexistent_command_xyz").is_none());
    }

    #[test]
    fn test_core_commands_present() {
        let expected = [
            "help", "clear", "compact", "cost", "exit", "model",
            "config", "version", "status", "diff", "memory", "hooks",
            "permissions", "plan", "tasks", "session", "login", "logout", "refresh",
            "feedback", "usage", "plugin", "reload-plugins",
            "add-dir", "agents", "branch", "tag",
            "passes", "ide", "pr-comments", "desktop", "mobile",
            "install-github-app", "web-setup", "stickers",
        ];
        for name in &expected {
            assert!(
                find_command(name).is_some(),
                "Expected command '{}' not in all_commands()",
                name
            );
        }
    }

    // ---- Command execution tests --------------------------------------------

    #[tokio::test]
    async fn test_clear_command_returns_clear_conversation() {
        let mut ctx = make_ctx();
        let cmd = find_command("clear").unwrap();
        let result = cmd.execute("", &mut ctx).await;
        assert!(matches!(result, CommandResult::ClearConversation));
    }

    #[tokio::test]
    async fn test_refresh_command_requests_provider_reset() {
        let mut ctx = make_ctx();
        let cmd = find_command("refresh").unwrap();
        let result = cmd.execute("", &mut ctx).await;
        assert!(matches!(result, CommandResult::RefreshProviderState));
    }

    #[tokio::test]
    async fn test_exit_command_returns_exit() {
        let mut ctx = make_ctx();
        let cmd = find_command("exit").unwrap();
        let result = cmd.execute("", &mut ctx).await;
        assert!(matches!(result, CommandResult::Exit));
    }

    #[tokio::test]
    async fn test_version_command_returns_message() {
        let mut ctx = make_ctx();
        let cmd = find_command("version").unwrap();
        let result = cmd.execute("", &mut ctx).await;
        assert!(matches!(result, CommandResult::Message(_)));
        if let CommandResult::Message(msg) = result {
            assert!(
                msg.contains("claude") || msg.contains("Claurst") || msg.contains('.'),
                "Version message should contain version number, got: {}",
                msg
            );
        }
    }

    #[tokio::test]
    async fn test_cost_command_returns_message() {
        let mut ctx = make_ctx();
        let cmd = find_command("cost").unwrap();
        let result = cmd.execute("", &mut ctx).await;
        assert!(matches!(result, CommandResult::Message(_)));
    }

    #[tokio::test]
    async fn test_login_command_starts_oauth_flow() {
        let mut ctx = make_ctx();
        let cmd = find_command("login").unwrap();
        // Default (no --console) → Anthropic, login_with_claude_ai = true
        let result = cmd.execute("", &mut ctx).await;
        match result {
            CommandResult::StartLoginForProvider {
                provider,
                login_with_claude_ai,
                label,
            } => {
                assert_eq!(provider, claurst_core::accounts::PROVIDER_ANTHROPIC);
                assert!(login_with_claude_ai);
                assert!(label.is_none());
            }
            other => panic!("expected StartLoginForProvider, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_login_command_console_flag() {
        let mut ctx = make_ctx();
        let cmd = find_command("login").unwrap();
        let result = cmd.execute("--console", &mut ctx).await;
        match result {
            CommandResult::StartLoginForProvider {
                provider,
                login_with_claude_ai,
                ..
            } => {
                assert_eq!(provider, claurst_core::accounts::PROVIDER_ANTHROPIC);
                assert!(!login_with_claude_ai);
            }
            other => panic!("expected StartLoginForProvider, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_login_command_codex_flag() {
        let mut ctx = make_ctx();
        let cmd = find_command("login").unwrap();
        let result = cmd.execute("--codex --label work", &mut ctx).await;
        match result {
            CommandResult::StartLoginForProvider {
                provider,
                label,
                ..
            } => {
                assert_eq!(provider, claurst_core::accounts::PROVIDER_CODEX);
                assert_eq!(label.as_deref(), Some("work"));
            }
            other => panic!("expected StartLoginForProvider, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_accounts_command_returns_message() {
        let mut ctx = make_ctx();
        let cmd = find_command("accounts").unwrap();
        let result = cmd.execute("", &mut ctx).await;
        // Should return a Message regardless of registry contents.
        assert!(matches!(result, CommandResult::Message(_)));
    }

    #[tokio::test]
    async fn test_switch_command_requires_id() {
        let mut ctx = make_ctx();
        let cmd = find_command("switch").unwrap();
        let result = cmd.execute("", &mut ctx).await;
        assert!(matches!(result, CommandResult::Error(_)));
    }

    #[tokio::test]
    async fn test_help_command_returns_message() {
        let mut ctx = make_ctx();
        let cmd = find_command("help").unwrap();
        let result = cmd.execute("", &mut ctx).await;
        // help returns either Message or Silent
        assert!(
            matches!(result, CommandResult::Message(_) | CommandResult::Silent),
            "help should return Message or Silent"
        );
    }

    #[tokio::test]
    async fn test_web_setup_proxy_executes_named_command() {
        let mut ctx = make_ctx();
        let cmd = find_command("web-setup").unwrap();
        let result = cmd.execute("", &mut ctx).await;
        assert!(matches!(result, CommandResult::Message(_)));
    }

    #[tokio::test]
    async fn test_import_config_command_opens_overlay() {
        let mut ctx = make_ctx();
        let cmd = find_command("import-config").unwrap();
        let result = cmd.execute("", &mut ctx).await;
        assert!(matches!(result, CommandResult::OpenImportConfigOverlay));
    }

    #[test]
    fn test_split_command_args_preserves_quoted_segments() {
        assert_eq!(
            split_command_args("create \"agent alpha\" 'second value'"),
            vec![
                "create".to_string(),
                "agent alpha".to_string(),
                "second value".to_string(),
            ]
        );
    }
}
