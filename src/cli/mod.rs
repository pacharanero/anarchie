// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `anarchie` command-line interface.
//!
//! `main.rs` is a thin shim: it calls [`run`]. All behaviour lives here in the
//! library so the CLI can be embedded. Each command family is a submodule with
//! `run`-style handlers; this module owns the argument surface and dispatch.
//!
//! Output convention: **data on stdout, hints/errors on stderr.** A global
//! `--format text|json` is honoured by every command that emits structured
//! data (text for humans, json for scripts and agents).

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

use crate::store::Deployment;

mod inspect;
mod query;
mod record;
mod schema;
mod serve;

/// Output format for commands that emit structured data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Human-readable text (the default).
    #[default]
    Text,
    /// JSON, for scripts and agents.
    Json,
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Format::Text => "text",
            Format::Json => "json",
        })
    }
}

/// A flat-file, git-native openEHR clinical data repository.
#[derive(Parser)]
#[command(name = "anarchie", version, about, long_about = None)]
pub struct Cli {
    /// Output format for commands that emit structured data.
    #[arg(long, short = 'f', global = true, value_name = "FORMAT", default_value_t = Format::Text)]
    format: Format,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Summarise an openEHR composition stored as canonical JSON.
    Info {
        /// Path to a canonical-JSON composition file.
        file: PathBuf,
    },
    /// Re-emit a composition in anarchie's canonical JSON form.
    Canonicalise {
        /// Path to a canonical-JSON composition file.
        file: PathBuf,
    },
    /// Scaffold a new anarchie deployment in the current (or given) directory.
    ///
    /// By default the deployment is seeded with a curated set of bundled
    /// "starter" Operational Templates so it can store real clinical data
    /// immediately; pass --minimal for an empty CDR.
    Init {
        /// Directory to create the deployment in (defaults to the current dir).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// The creating-system identity used in every version_uid.
        #[arg(long, default_value = "anarchie.local")]
        system_id: String,
        /// Create an empty CDR without the bundled starter templates.
        #[arg(long)]
        minimal: bool,
    },
    /// Manage EHRs (one git repository per patient).
    Ehr {
        #[command(subcommand)]
        command: Option<EhrCommand>,
    },
    /// Commit a composition into an EHR as a CONTRIBUTION.
    Commit {
        /// The EHR id to commit into.
        ehr: String,
        /// Path to a canonical-JSON composition file.
        file: PathBuf,
        /// Object id of an existing composition to create a new version of.
        #[arg(long)]
        object_id: Option<String>,
        /// Committer name for the audit trail.
        #[arg(long, default_value = "anarchie")]
        committer: String,
        /// Committer email for the audit trail.
        #[arg(long, default_value = "anarchie@localhost")]
        email: String,
        /// Contribution description, used as the commit subject. Defaults to a
        /// generated summary (e.g. "Create composition <id>") when omitted.
        #[arg(long, short = 'm')]
        message: Option<String>,
        /// Skip validation and commit even if the composition is nonconformant.
        #[arg(long)]
        no_validate: bool,
    },
    /// Validate a composition against the RM and, optionally, a template.
    Validate {
        /// Path to a canonical-JSON composition file.
        file: PathBuf,
        /// A registered template id to validate against (otherwise RM only).
        #[arg(long)]
        template: Option<String>,
    },
    /// Manage registered Operational Templates (the schema).
    Template {
        #[command(subcommand)]
        command: Option<TemplateCommand>,
    },
    /// Print a composition: its head version, or a specific version_uid.
    Cat {
        /// The EHR id.
        ehr: String,
        /// An object_id (head) or a full version_uid (history).
        target: String,
    },
    /// Show the version history of a composition.
    Log {
        /// The EHR id.
        ehr: String,
        /// The composition object_id.
        object_id: String,
    },
    /// Diff two versions of a composition.
    Diff {
        /// The EHR id.
        ehr: String,
        /// The composition object_id.
        object_id: String,
        /// The earlier version_tree_id.
        from: u32,
        /// The later version_tree_id.
        to: u32,
    },
    /// Build or refresh the derived query index from the Composition files.
    ///
    /// The index is the read model (CQRS): rebuildable from the files, never
    /// authoritative. By default only EHRs whose git HEAD changed are
    /// re-indexed; --rebuild drops and rebuilds everything.
    Index {
        /// Drop and rebuild the entire index from scratch.
        #[arg(long)]
        rebuild: bool,
    },
    /// Run an ad-hoc AQL query against the index.
    Aql {
        /// The AQL query text.
        query: String,
        /// A `$`-parameter binding as `NAME=VALUE` (repeatable).
        #[arg(long = "param", value_name = "NAME=VALUE")]
        params: Vec<String>,
    },
    /// Manage and run stored (named) AQL queries.
    Query {
        #[command(subcommand)]
        command: Option<QueryCommand>,
    },
    /// Serve the openEHR REST API over HTTP (binds to localhost).
    Serve {
        /// Address to bind, as `host:port`.
        #[arg(long, default_value = "127.0.0.1:8080")]
        addr: String,
    },
    /// Run the stdio MCP server, exposing the store to LLM agents.
    Mcp,
    /// Check every stored Composition against the RM (and its template).
    Fsck,
    /// Install and inspect archetype packs (sets of Operational Templates).
    Pack {
        #[command(subcommand)]
        command: Option<PackCommand>,
    },
    /// Report the anarchie version.
    Version,
    /// Generate shell completion scripts.
    Completions {
        /// The shell to generate for (bash, zsh, fish, powershell, elvish).
        /// Omit together with --install to auto-detect the current shell.
        shell: Option<clap_complete::Shell>,
        /// Write the completion file into this directory instead of stdout.
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Install into the standard user completion directory for the shell.
        #[arg(long)]
        install: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum PackCommand {
    /// Install a bundled pack by name (e.g. `ips-core`) or a local directory.
    Add {
        /// A bundled pack name or a path to a directory of `*.opt.json` files.
        source: String,
    },
    /// List the bundled packs available to install.
    List,
}

#[derive(Subcommand)]
pub(crate) enum EhrCommand {
    /// Create a new, empty EHR and print its id.
    New {
        /// Committer name for the creation audit.
        #[arg(long, default_value = "anarchie")]
        committer: String,
        /// Committer email for the creation audit.
        #[arg(long, default_value = "anarchie@localhost")]
        email: String,
    },
    /// List the EHRs in the deployment.
    List,
}

#[derive(Subcommand)]
pub(crate) enum TemplateCommand {
    /// Register an Operational Template (anarchie OPT JSON) as the schema.
    Add {
        /// Path to an anarchie OPT JSON file.
        file: PathBuf,
    },
    /// List the registered template ids.
    List,
}

#[derive(Subcommand)]
pub(crate) enum QueryCommand {
    /// Register an AQL query under a name (and optional semantic version).
    Add {
        /// The query name.
        name: String,
        /// Path to a file containing the AQL text.
        file: PathBuf,
        /// Semantic version to register under (defaults to 1.0.0).
        #[arg(long)]
        version: Option<String>,
    },
    /// List the registered stored queries.
    List,
    /// Run a stored query by name (and optional version).
    Run {
        /// The query name.
        name: String,
        /// The version to run (defaults to the highest registered).
        #[arg(long)]
        version: Option<String>,
        /// A `$`-parameter binding as `NAME=VALUE` (repeatable).
        #[arg(long = "param", value_name = "NAME=VALUE")]
        params: Vec<String>,
    },
}

/// Entry point. Resets SIGPIPE, dispatches, and maps errors to an exit code.
pub fn run() -> ExitCode {
    reset_sigpipe();

    // Friendly version aliases beyond clap's canonical -V/--version.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && matches!(args[1].as_str(), "-v" | "-version") {
        println!("{}", version_line());
        return ExitCode::SUCCESS;
    }

    match dispatch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch() -> Result<()> {
    let cli = Cli::parse();
    let format = cli.format;
    let Some(command) = cli.command else {
        // A bare invocation is helpful, not an error: print the command list.
        Cli::command().print_help()?;
        println!();
        return Ok(());
    };
    match command {
        Command::Info { file } => inspect::info(format, &file),
        Command::Canonicalise { file } => inspect::canonicalise(&file),
        Command::Init {
            path,
            system_id,
            minimal,
        } => record::init(format, &path, &system_id, minimal),
        Command::Ehr { command } => match command {
            Some(c) => record::ehr(format, c),
            None => subcommand_help("ehr"),
        },
        Command::Commit {
            ehr,
            file,
            object_id,
            committer,
            email,
            message,
            no_validate,
        } => record::commit(
            format,
            &ehr,
            &file,
            object_id,
            &committer,
            &email,
            message,
            no_validate,
        ),
        Command::Validate { file, template } => {
            schema::validate(format, &file, template.as_deref())
        }
        Command::Template { command } => match command {
            Some(c) => schema::template(format, c),
            None => subcommand_help("template"),
        },
        Command::Cat { ehr, target } => record::cat(&ehr, &target),
        Command::Log { ehr, object_id } => record::log(format, &ehr, &object_id),
        Command::Diff {
            ehr,
            object_id,
            from,
            to,
        } => record::diff(format, &ehr, &object_id, from, to),
        Command::Index { rebuild } => query::index(format, rebuild),
        Command::Aql { query, params } => query::aql(format, &query, &params),
        Command::Query { command } => match command {
            Some(c) => query::query(format, c),
            None => subcommand_help("query"),
        },
        Command::Serve { addr } => serve::serve(&addr),
        Command::Mcp => serve::mcp(),
        Command::Fsck => record::fsck(format),
        Command::Pack { command } => match command {
            Some(c) => schema::pack(format, c),
            None => subcommand_help("pack"),
        },
        Command::Version => version(format),
        Command::Completions {
            shell,
            dir,
            install,
        } => completions(shell, dir, install),
    }
}

// --- shared helpers, visible to the command submodules -----------------------

/// Open the anarchie deployment containing the current working directory.
fn open_deployment() -> Result<Deployment> {
    let cwd = std::env::current_dir().context("determining current directory")?;
    Deployment::open(&cwd).context("opening anarchie deployment")
}

/// Load and parse a canonical-JSON Composition file.
fn load(file: &std::path::Path) -> Result<crate::rm::Composition> {
    let json =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    crate::rm::from_canonical_str(&json)
        .with_context(|| format!("parsing {} as a COMPOSITION", file.display()))
}

/// Path to the deployment's derived AQL index (git-ignored, disposable).
fn index_db_path(deployment: &Deployment) -> PathBuf {
    deployment.root().join("index").join("aql.db")
}

/// Parse repeated `NAME=VALUE` CLI args into an AQL parameter map.
fn parse_params(pairs: &[String]) -> Result<crate::query::Params> {
    let mut params = crate::query::Params::new();
    for pair in pairs {
        let (name, value) = pair
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("--param must be NAME=VALUE, got `{pair}`"))?;
        params.insert(name.to_string(), value.to_string());
    }
    Ok(params)
}

/// Emit `value` as pretty JSON when the format is JSON, otherwise run the
/// `text` closure to render the human-readable form.
fn emit(format: Format, value: &serde_json::Value, text: impl FnOnce()) -> Result<()> {
    match format {
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(value)?);
            Ok(())
        }
        Format::Text => {
            text();
            Ok(())
        }
    }
}

/// Print the help for a command family invoked with no subcommand, and exit
/// successfully - a bare `anarchie ehr` is a helpful listing, not an error.
fn subcommand_help(name: &str) -> Result<()> {
    let mut cmd = Cli::command();
    if let Some(sub) = cmd.find_subcommand_mut(name) {
        sub.print_help()?;
        println!();
    }
    Ok(())
}

// --- meta commands -----------------------------------------------------------

fn version_line() -> String {
    format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

fn version(format: Format) -> Result<()> {
    let value = serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
    });
    emit(format, &value, || println!("{}", version_line()))
}

fn completions(
    shell: Option<clap_complete::Shell>,
    dir: Option<PathBuf>,
    install: bool,
) -> Result<()> {
    let shell = match shell {
        Some(s) => s,
        None => clap_complete::Shell::from_env()
            .context("could not detect the current shell; pass one, e.g. `completions bash`")?,
    };
    let mut cmd = Cli::command();

    if install {
        let target = dir
            .map(Ok)
            .unwrap_or_else(|| default_completion_dir(shell))?;
        std::fs::create_dir_all(&target)
            .with_context(|| format!("creating {}", target.display()))?;
        let path = clap_complete::generate_to(shell, &mut cmd, "anarchie", &target)
            .context("writing completion file")?;
        eprintln!("Installed {shell} completions to {}", path.display());
        eprintln!("You may need to restart your shell (or source the file) to pick them up.");
        return Ok(());
    }

    if let Some(dir) = dir {
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = clap_complete::generate_to(shell, &mut cmd, "anarchie", &dir)
            .context("writing completion file")?;
        eprintln!("Wrote {}", path.display());
        return Ok(());
    }

    clap_complete::generate(shell, &mut cmd, "anarchie", &mut std::io::stdout());
    Ok(())
}

/// The standard per-user completion directory for a shell, honouring
/// `XDG_DATA_HOME` where relevant.
fn default_completion_dir(shell: clap_complete::Shell) -> Result<PathBuf> {
    use clap_complete::Shell;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")?;
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("share"));
    Ok(match shell {
        Shell::Bash => data.join("bash-completion").join("completions"),
        Shell::Zsh => data.join("zsh").join("site-functions"),
        Shell::Fish => home.join(".config").join("fish").join("completions"),
        _ => data.join("anarchie").join("completions"),
    })
}

/// Reset SIGPIPE to its default on Unix so writing to a closed pipe (e.g.
/// `anarchie aql … | head`) terminates the process cleanly instead of
/// panicking on a broken pipe.
fn reset_sigpipe() {
    #[cfg(unix)]
    // SAFETY: resetting a signal disposition to the default handler is sound;
    // it is the standard startup fix for CLIs that stream to pipes.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

/// Print a validation report's violations to stdout.
fn print_report(report: &crate::validate::ValidationReport) {
    for violation in &report.violations {
        println!(
            "  [{}] {} ({})\n        {}",
            violation.severity.as_str(),
            violation.rm_path,
            violation.constraint,
            violation.message
        );
    }
}
