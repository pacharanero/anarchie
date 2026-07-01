// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! `info` and `canonicalise` - inspect a Composition file, no repository needed.

use std::path::Path;

use anyhow::Result;

use super::{emit, load, Format};
use crate::rm::{Composition, ContentItem, DataValue, Item, ItemStructure};

pub(crate) fn info(format: Format, file: &Path) -> Result<()> {
    let composition = load(file)?;

    let mut counts = Counts::default();
    for item in &composition.content {
        counts.visit_content(item);
    }
    let composer = composer_name(&composition);

    let value = serde_json::json!({
        "name": composition.name.value(),
        "archetype": composition.archetype_details.archetype_id.value,
        "template": composition.archetype_details.template_id.as_ref().map(|t| &t.value),
        "rm_version": composition.archetype_details.rm_version,
        "language": composition.language.code_string,
        "territory": composition.territory.code_string,
        "category": composition.category.value,
        "composer": composer,
        "content_items": composition.content.len(),
        "sections": counts.sections,
        "entries": counts.entries,
        "elements": counts.elements,
    });

    emit(format, &value, || {
        println!("Composition: {}", composition.name.value());
        println!(
            "  archetype:  {}",
            composition.archetype_details.archetype_id.value
        );
        if let Some(template) = &composition.archetype_details.template_id {
            println!("  template:   {}", template.value);
        }
        println!("  rm_version: {}", composition.archetype_details.rm_version);
        println!("  language:   {}", composition.language.code_string);
        println!("  territory:  {}", composition.territory.code_string);
        println!("  category:   {}", composition.category.value);
        if let Some(composer) = &composer {
            println!("  composer:   {composer}");
        }
        println!("  content items: {}", composition.content.len());
        println!("  sections:      {}", counts.sections);
        println!("  entries:       {}", counts.entries);
        println!("  elements:      {}", counts.elements);
    })
}

pub(crate) fn canonicalise(file: &Path) -> Result<()> {
    let composition = load(file)?;
    print!("{}", crate::rm::to_canonical_string(&composition)?);
    Ok(())
}

fn composer_name(composition: &Composition) -> Option<String> {
    use crate::rm::PartyProxy::*;
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

fn event_data(event: &crate::rm::Event) -> &ItemStructure {
    match event {
        crate::rm::Event::PointEvent(e) => &e.data,
        crate::rm::Event::IntervalEvent(e) => &e.data,
    }
}
