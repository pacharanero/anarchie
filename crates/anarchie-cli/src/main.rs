// SPDX-License-Identifier: AGPL-3.0-or-later
//! `anarchie` command-line interface.

use std::fs;
use std::path::PathBuf;

use anarchie_rm::{
    Composition, ContentItem, DataValue, Item, ItemStructure,
};
use anyhow::{Context, Result};
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Info { file } => info(&file),
        Command::Canonicalise { file } => canonicalise(&file),
    }
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
