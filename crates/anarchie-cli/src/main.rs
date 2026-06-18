// SPDX-License-Identifier: AGPL-3.0-or-later
//! `anarchie` command-line interface.

use std::fs;
use std::path::PathBuf;

use anarchie_rm::{
    Composition, ContentItem, DataValue, Item, ItemStructure,
};
use anarchie_store::{Audit, ChangeType, Deployment, DeploymentConfig, StoreError};
use anarchie_validate::{Opt, ValidationReport};
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

/// A flat-file, git-native openEHR clinical data repository.
#[derive(Parser)]
#[command(name = "anarchie", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
    Init {
        /// Directory to create the deployment in (defaults to the current dir).
        #[arg(default_value = ".")]
        path: PathBuf,
        /// The creating-system identity used in every version_uid.
        #[arg(long, default_value = "anarchie.local")]
        system_id: String,
    },
    /// Manage EHRs (one git repository per patient).
    Ehr {
        #[command(subcommand)]
        command: EhrCommand,
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
        /// Contribution description (the commit subject).
        #[arg(long, short = 'm', default_value = "Commit composition")]
        message: String,
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
        /// Emit the validation report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Manage registered Operational Templates (the schema).
    Template {
        #[command(subcommand)]
        command: TemplateCommand,
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
}

#[derive(Subcommand)]
enum EhrCommand {
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
enum TemplateCommand {
    /// Register an Operational Template (anarchie OPT JSON) as the schema.
    Add {
        /// Path to an anarchie OPT JSON file.
        file: PathBuf,
    },
    /// List the registered template ids.
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Info { file } => info(&file),
        Command::Canonicalise { file } => canonicalise(&file),
        Command::Init { path, system_id } => init(&path, &system_id),
        Command::Ehr { command } => ehr(command),
        Command::Commit {
            ehr,
            file,
            object_id,
            committer,
            email,
            message,
            no_validate,
        } => commit(&ehr, &file, object_id, &committer, &email, &message, no_validate),
        Command::Validate {
            file,
            template,
            json,
        } => validate(&file, template.as_deref(), json),
        Command::Template { command } => template(command),
        Command::Cat { ehr, target } => cat(&ehr, &target),
        Command::Log { ehr, object_id } => log(&ehr, &object_id),
        Command::Diff {
            ehr,
            object_id,
            from,
            to,
        } => diff(&ehr, &object_id, from, to),
    }
}

fn open_deployment() -> Result<Deployment> {
    let cwd = std::env::current_dir().context("determining current directory")?;
    Deployment::open(&cwd).context("opening anarchie deployment")
}

fn init(path: &PathBuf, system_id: &str) -> Result<()> {
    let config = DeploymentConfig::new(system_id);
    let deployment = Deployment::init(path, config).context("initialising deployment")?;
    println!("Initialised anarchie deployment at {}", deployment.root().display());
    println!("  system_id: {}", deployment.config().system_id);
    Ok(())
}

fn ehr(command: EhrCommand) -> Result<()> {
    let deployment = open_deployment()?;
    match command {
        EhrCommand::New { committer, email } => {
            let audit = Audit::now(committer, email, ChangeType::Creation, "Create EHR");
            let repo = deployment.create_ehr(&audit).context("creating EHR")?;
            println!("{}", repo.ehr_id());
            Ok(())
        }
        EhrCommand::List => {
            for id in deployment.list_ehrs().context("listing EHRs")? {
                println!("{id}");
            }
            Ok(())
        }
    }
}

fn commit(
    ehr_id: &str,
    file: &PathBuf,
    object_id: Option<String>,
    committer: &str,
    email: &str,
    message: &str,
    no_validate: bool,
) -> Result<()> {
    let deployment = open_deployment()?;
    let repo = deployment.open_ehr(ehr_id).context("opening EHR")?;
    let composition = load(file)?;
    let change_type = if object_id.is_some() {
        ChangeType::Modification
    } else {
        ChangeType::Creation
    };
    let audit = Audit::now(committer, email, change_type, message);
    let result = if no_validate {
        repo.commit_composition_unchecked(composition, object_id, &audit)
    } else {
        repo.commit_composition(composition, object_id, &audit)
    };
    let outcome = match result {
        Ok(outcome) => outcome,
        Err(StoreError::Invalid(report)) => {
            eprintln!("Rejected: composition failed validation");
            print_report(&report);
            bail!("{} validation error(s)", report.error_count());
        }
        Err(err) => return Err(anyhow::Error::new(err).context("committing composition")),
    };
    println!("Committed {}", outcome.version_uid);
    println!("  object_id:       {}", outcome.object_id);
    println!("  commit:          {}", outcome.commit_sha);
    println!("  contribution_id: {}", outcome.contribution_id);
    Ok(())
}

fn validate(file: &PathBuf, template_id: Option<&str>, json: bool) -> Result<()> {
    let composition = load(file)?;
    let opt = match template_id {
        Some(id) => {
            let deployment = open_deployment()?;
            let opt = deployment
                .get_template(id)
                .context("loading template")?
                .ok_or_else(|| anyhow::anyhow!("template `{id}` is not registered"))?;
            Some(opt)
        }
        None => None,
    };
    let report = anarchie_validate::validate(&composition, opt.as_ref());
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
        if report.valid {
            println!("valid");
        }
    }
    if report.error_count() > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn template(command: TemplateCommand) -> Result<()> {
    let deployment = open_deployment()?;
    match command {
        TemplateCommand::Add { file } => {
            let json = fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let opt = Opt::from_json(&json)
                .with_context(|| format!("parsing {} as an anarchie OPT", file.display()))?;
            let id = deployment.add_template(&opt).context("registering template")?;
            println!("Registered template {id}");
            Ok(())
        }
        TemplateCommand::List => {
            for id in deployment.list_templates().context("listing templates")? {
                println!("{id}");
            }
            Ok(())
        }
    }
}

fn print_report(report: &ValidationReport) {
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

fn cat(ehr_id: &str, target: &str) -> Result<()> {
    let deployment = open_deployment()?;
    let repo = deployment.open_ehr(ehr_id).context("opening EHR")?;
    // A version_uid has the form object_id::system_id::version_tree_id.
    let body = if target.contains("::") {
        repo.cat_version(target).context("reading version")?
    } else {
        repo.cat_head(target).context("reading head version")?
    };
    print!("{body}");
    if !body.ends_with('\n') {
        println!();
    }
    Ok(())
}

fn log(ehr_id: &str, object_id: &str) -> Result<()> {
    let deployment = open_deployment()?;
    let repo = deployment.open_ehr(ehr_id).context("opening EHR")?;
    for entry in repo.log(object_id).context("reading history")? {
        println!("{}  {}  {}", entry.version_uid, entry.time_committed, entry.subject);
        println!("  commit {}", entry.commit_sha);
    }
    Ok(())
}

fn diff(ehr_id: &str, object_id: &str, from: u32, to: u32) -> Result<()> {
    if from == 0 || to == 0 {
        bail!("version_tree_id is 1-based; v0 does not exist");
    }
    let deployment = open_deployment()?;
    let repo = deployment.open_ehr(ehr_id).context("opening EHR")?;
    print!("{}", repo.diff(object_id, from, to).context("diffing versions")?);
    Ok(())
}

fn load(file: &PathBuf) -> Result<Composition> {
    let json = fs::read_to_string(file)
        .with_context(|| format!("reading {}", file.display()))?;
    anarchie_rm::from_canonical_str(&json)
        .with_context(|| format!("parsing {} as a COMPOSITION", file.display()))
}

fn info(file: &PathBuf) -> Result<()> {
    let composition = load(file)?;

    let mut counts = Counts::default();
    for item in &composition.content {
        counts.visit_content(item);
    }

    println!("Composition: {}", composition.name.value());
    println!("  archetype:  {}", composition.archetype_details.archetype_id.value);
    if let Some(template) = &composition.archetype_details.template_id {
        println!("  template:   {}", template.value);
    }
    println!("  rm_version: {}", composition.archetype_details.rm_version);
    println!("  language:   {}", composition.language.code_string);
    println!("  territory:  {}", composition.territory.code_string);
    println!("  category:   {}", composition.category.value);
    if let Some(composer) = composer_name(&composition) {
        println!("  composer:   {composer}");
    }
    println!("  content items: {}", composition.content.len());
    println!("  sections:      {}", counts.sections);
    println!("  entries:       {}", counts.entries);
    println!("  elements:      {}", counts.elements);

    Ok(())
}

fn canonicalise(file: &PathBuf) -> Result<()> {
    let composition = load(file)?;
    print!("{}", anarchie_rm::to_canonical_string(&composition)?);
    Ok(())
}

fn composer_name(composition: &Composition) -> Option<String> {
    use anarchie_rm::PartyProxy::*;
    match &composition.composer {
        PartyIdentified(p) => p.name.clone(),
        PartyRelated(p) => p.name.clone(),
        PartySelf(_) => Some("(self)".to_string()),
    }
}

#[derive(Default)]
struct Counts {
    sections: usize,
    entries: usize,
    elements: usize,
}

impl Counts {
    fn visit_content(&mut self, item: &ContentItem) {
        match item {
            ContentItem::Section(section) => {
                self.sections += 1;
                for child in &section.items {
                    self.visit_content(child);
                }
            }
            ContentItem::Observation(obs) => {
                self.entries += 1;
                for event in &obs.data.events {
                    self.visit_structure(event_data(event));
                }
            }
            ContentItem::Evaluation(ev) => {
                self.entries += 1;
                self.visit_structure(&ev.data);
            }
            ContentItem::Instruction(_) => {
                self.entries += 1;
            }
            ContentItem::Action(action) => {
                self.entries += 1;
                self.visit_structure(&action.description);
            }
            ContentItem::AdminEntry(admin) => {
                self.entries += 1;
                self.visit_structure(&admin.data);
            }
        }
    }

    fn visit_structure(&mut self, structure: &ItemStructure) {
        match structure {
            ItemStructure::ItemTree(tree) => {
                for item in &tree.items {
                    self.visit_item(item);
                }
            }
            ItemStructure::ItemList(list) => {
                self.elements += list.items.len();
            }
            ItemStructure::ItemSingle(single) => {
                self.count_element(&single.item.value);
            }
            ItemStructure::ItemTable(table) => {
                for row in &table.rows {
                    for item in &row.items {
                        self.visit_item(item);
                    }
                }
            }
        }
    }

    fn visit_item(&mut self, item: &Item) {
        match item {
            Item::Cluster(cluster) => {
                for child in &cluster.items {
                    self.visit_item(child);
                }
            }
            Item::Element(element) => {
                self.count_element(&element.value);
            }
        }
    }

    fn count_element(&mut self, _value: &Option<DataValue>) {
        self.elements += 1;
    }
}

fn event_data(event: &anarchie_rm::Event) -> &ItemStructure {
    match event {
        anarchie_rm::Event::PointEvent(e) => &e.data,
        anarchie_rm::Event::IntervalEvent(e) => &e.data,
    }
}
