use clap::CommandFactory;
use clap::Parser;
use clap_complete::Shell;
use clap_complete::generate;
use codex_arg0::arg0_dispatch_or_else;
use codex_chatgpt::apply_command::ApplyCommand;
use codex_chatgpt::apply_command::run_apply_command;
use codex_cli::LandlockCommand;
use codex_cli::SeatbeltCommand;
use codex_cli::login::run_login_status;
use codex_cli::login::run_login_with_api_key;
use codex_cli::login::run_login_with_chatgpt;
use codex_cli::login::run_logout;
use codex_cli::proto;
use codex_common::CliConfigOverrides;
use codex_exec::Cli as ExecCli;
use codex_tui::Cli as TuiCli;
use std::path::PathBuf;

use crate::proto::ProtoCli;

/// Codex CLI
///
/// If no subcommand is specified, options will be forwarded to the interactive CLI.
#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    // If a sub‑command is given, ignore requirements of the default args.
    subcommand_negates_reqs = true,
    // The executable is sometimes invoked via a platform‑specific name like
    // `codex-x86_64-unknown-linux-musl`, but the help output should always use
    // the generic `codex` command name that users run.
    bin_name = "codex"
)]
struct MultitoolCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    interactive: TuiCli,

    #[clap(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Run Codex non-interactively.
    #[clap(visible_alias = "e")]
    Exec(ExecCli),

    /// Manage login.
    Login(LoginCommand),

    /// Remove stored authentication credentials.
    Logout(LogoutCommand),

    /// Experimental: run Codex as an MCP server.
    Mcp,

    /// Run the Protocol stream via stdin/stdout
    #[clap(visible_alias = "p")]
    Proto(ProtoCli),

    /// Generate shell completion scripts.
    Completion(CompletionCommand),

    /// Internal debugging commands.
    Debug(DebugArgs),

    /// Apply the latest diff produced by Codex agent as a `git apply` to your local working tree.
    #[clap(visible_alias = "a")]
    Apply(ApplyCommand),

    /// Internal: generate TypeScript protocol bindings.
    #[clap(hide = true)]
    GenerateTs(GenerateTsCommand),
}

#[derive(Debug, Parser)]
struct CompletionCommand {
    /// Shell to generate completions for
    #[clap(value_enum, default_value_t = Shell::Bash)]
    shell: Shell,
}

#[derive(Debug, Parser)]
struct DebugArgs {
    #[command(subcommand)]
    cmd: DebugCommand,
}

#[derive(Debug, clap::Subcommand)]
enum DebugCommand {
    /// Run a command under Seatbelt (macOS only).
    Seatbelt(SeatbeltCommand),

    /// Run a command under Landlock+seccomp (Linux only).
    Landlock(LandlockCommand),

    /// Show the final system prompt after applying an output style.
    OutputStyle(OutputStyleDebugCommand),
}

#[derive(Debug, Parser)]
struct LoginCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,

    #[arg(long = "api-key", value_name = "API_KEY")]
    api_key: Option<String>,

    #[command(subcommand)]
    action: Option<LoginSubcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum LoginSubcommand {
    /// Show login status.
    Status,
}

#[derive(Debug, Parser)]
struct LogoutCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,
}

#[derive(Debug, Parser)]
struct GenerateTsCommand {
    /// Output directory where .ts files will be written
    #[arg(short = 'o', long = "out", value_name = "DIR")]
    out_dir: PathBuf,

    /// Optional path to the Prettier executable to format generated files
    #[arg(short = 'p', long = "prettier", value_name = "PRETTIER_BIN")]
    prettier: Option<PathBuf>,
}

#[derive(Debug, Parser)]
struct OutputStyleDebugCommand {
    /// Name of the style to apply (case-insensitive). If omitted, lists available styles.
    #[arg(value_name = "STYLE_NAME")]
    style: Option<String>,

    /// Model slug for feature flags (defaults to gpt-4.1)
    #[arg(long, value_name = "MODEL", default_value = "gpt-4.1")]
    model: String,

    /// Limit output to selected sections (comma-separated): personality,presentation,preambles,planning,progress
    #[arg(long = "sections", value_name = "LIST")]
    sections: Option<String>,
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|codex_linux_sandbox_exe| async move {
        cli_main(codex_linux_sandbox_exe).await?;
        Ok(())
    })
}

async fn cli_main(codex_linux_sandbox_exe: Option<PathBuf>) -> anyhow::Result<()> {
    let cli = MultitoolCli::parse();

    match cli.subcommand {
        None => {
            let mut tui_cli = cli.interactive;
            prepend_config_flags(&mut tui_cli.config_overrides, cli.config_overrides);
            let usage = codex_tui::run_main(tui_cli, codex_linux_sandbox_exe).await?;
            if !usage.is_zero() {
                println!("{}", codex_core::protocol::FinalOutput::from(usage));
            }
        }
        Some(Subcommand::Exec(mut exec_cli)) => {
            prepend_config_flags(&mut exec_cli.config_overrides, cli.config_overrides);
            codex_exec::run_main(exec_cli, codex_linux_sandbox_exe).await?;
        }
        Some(Subcommand::Mcp) => {
            codex_mcp_server::run_main(codex_linux_sandbox_exe, cli.config_overrides).await?;
        }
        Some(Subcommand::Login(mut login_cli)) => {
            prepend_config_flags(&mut login_cli.config_overrides, cli.config_overrides);
            match login_cli.action {
                Some(LoginSubcommand::Status) => {
                    run_login_status(login_cli.config_overrides).await;
                }
                None => {
                    if let Some(api_key) = login_cli.api_key {
                        run_login_with_api_key(login_cli.config_overrides, api_key).await;
                    } else {
                        run_login_with_chatgpt(login_cli.config_overrides).await;
                    }
                }
            }
        }
        Some(Subcommand::Logout(mut logout_cli)) => {
            prepend_config_flags(&mut logout_cli.config_overrides, cli.config_overrides);
            run_logout(logout_cli.config_overrides).await;
        }
        Some(Subcommand::Proto(mut proto_cli)) => {
            prepend_config_flags(&mut proto_cli.config_overrides, cli.config_overrides);
            proto::run_main(proto_cli).await?;
        }
        Some(Subcommand::Completion(completion_cli)) => {
            print_completion(completion_cli);
        }
        Some(Subcommand::Debug(debug_args)) => match debug_args.cmd {
            DebugCommand::Seatbelt(mut seatbelt_cli) => {
                prepend_config_flags(&mut seatbelt_cli.config_overrides, cli.config_overrides);
                codex_cli::debug_sandbox::run_command_under_seatbelt(
                    seatbelt_cli,
                    codex_linux_sandbox_exe,
                )
                .await?;
            }
            DebugCommand::Landlock(mut landlock_cli) => {
                prepend_config_flags(&mut landlock_cli.config_overrides, cli.config_overrides);
                codex_cli::debug_sandbox::run_command_under_landlock(
                    landlock_cli,
                    codex_linux_sandbox_exe,
                )
                .await?;
            }
            DebugCommand::OutputStyle(opts) => {
                if opts.style.is_none() {
                    let names: Vec<&'static str> = codex_tui::builtin_style_names().collect();
                    println!("Available styles ({}):", names.len());
                    for n in names {
                        println!("- {n}");
                    }
                } else if let Some(name) = opts.style {
                    match codex_tui::builtin_style_yaml_by_name(&name) {
                        Some(yaml) => {
                            let mf = match codex_core::model_family::find_family_for_model(&opts.model) {
                                Some(m) => m,
                                None => match codex_core::model_family::find_family_for_model("gpt-4.1") {
                                    Some(m) => m,
                                    None => {
                                        eprintln!("Error: Could not find model family for {} or gpt-4.1", opts.model);
                                        std::process::exit(1);
                                    }
                                }
                            };
                            let full = codex_core::debug_full_instructions(yaml, &mf);
                            if let Some(list) = opts.sections.as_ref() {
                                let wanted: Vec<&str> = list
                                    .split(",")
                                    .map(|s| s.trim())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                let sections: [(&str, &str, &'static [&'static str]); 5] = [
                                    ("personality", "## Personality", &["\n## "]),
                                    (
                                        "presentation",
                                        "## Presenting your work and final message",
                                        &["\n## "],
                                    ),
                                    (
                                        "preambles",
                                        "### Preamble messages",
                                        &[("\n### "), ("\n## ")],
                                    ),
                                    ("planning", "## Planning", &["\n## "]),
                                    ("progress", "## Sharing progress updates", &["\n## "]),
                                ];
                                fn find_range(
                                    base: &str,
                                    heading: &str,
                                    next_markers: &[&str],
                                ) -> Option<(usize, usize)> {
                                    let target_bof = format!("{heading}\n");
                                    let target = format!("\n{heading}\n");
                                    let start = if base.starts_with(&target_bof) {
                                        Some(0)
                                    } else {
                                        base.find(&target).map(|p| p + 1)
                                    }?;
                                    let after_heading = start + heading.len();
                                    let content_start = base[after_heading..]
                                        .find("\n")
                                        .map(|off| after_heading + off + 1)
                                        .unwrap_or(after_heading);
                                    let mut next = base.len();
                                    for m in next_markers {
                                        if let Some(p) = base[content_start..].find(m) {
                                            let idx = content_start + p + 1;
                                            if idx < next {
                                                next = idx;
                                            }
                                        }
                                    }
                                    Some((start, next))
                                }
                                let mut out_s = String::new();
                                for (key, heading, next) in sections {
                                    if wanted.iter().any(|w| w.eq_ignore_ascii_case(key))
                                        && let Some((a, b)) = find_range(&full, heading, next) {
                                        if !out_s.is_empty() {
                                            out_s.push('\n');
                                        }
                                        out_s.push_str(&full[a..b]);
                                    }
                                }
                                println!("{out_s}");
                            } else {
                                println!("{full}");
                            }
                        }
                        None => {
                            eprintln!("Unknown style: {name}");
                            let names: Vec<&'static str> =
                                codex_tui::builtin_style_names().collect();
                            eprintln!("Available: {}", names.join(", "));
                            std::process::exit(2);
                        }
                    }
                }
            }
        },
        Some(Subcommand::Apply(mut apply_cli)) => {
            prepend_config_flags(&mut apply_cli.config_overrides, cli.config_overrides);
            run_apply_command(apply_cli, None).await?;
        }
        Some(Subcommand::GenerateTs(gen_cli)) => {
            codex_protocol_ts::generate_ts(&gen_cli.out_dir, gen_cli.prettier.as_deref())?;
        }
    }

    Ok(())
}

/// Prepend root-level overrides so they have lower precedence than
/// CLI-specific ones specified after the subcommand (if any).
fn prepend_config_flags(
    subcommand_config_overrides: &mut CliConfigOverrides,
    cli_config_overrides: CliConfigOverrides,
) {
    subcommand_config_overrides
        .raw_overrides
        .splice(0..0, cli_config_overrides.raw_overrides);
}

fn print_completion(cmd: CompletionCommand) {
    let mut app = MultitoolCli::command();
    let name = "codex";
    generate(cmd.shell, &mut app, name, &mut std::io::stdout());
}
