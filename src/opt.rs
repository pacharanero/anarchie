// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! # anarchie-opt
//!
//! Loads a flattened **Operational Template** into the [`crate::aom`]
//! constraint model that the validator walks a Composition against.
//!
//! An [`Opt`] is `anarchie`'s native, already-flattened template form: a small
//! amount of identifying metadata plus a [`CComplexObject`](crate::aom::CComplexObject)
//! definition rooted at the template's top RM type (usually `COMPOSITION`).
//! Templates are registered as *data* (`anarchie template add`), parsed once
//! into this model, and walked at validation time - the file-first philosophy
//! applied to the schema as well as the data.
//!
//! Legacy ADL 1.4 OPT XML is lowered into this model at the format boundary.
//! The stored form remains native JSON. Web Template input is future work; see
//! `specs/serialisation-formats.md`.

use std::collections::BTreeMap;

use crate::aom::{
    CAttribute, CCodePhrase, CComplexObject, CDvQuantity, CObject, CQuantityItem, Cardinality,
    Interval, MultiplicityInterval,
};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};

const ADL14_XML_NAMESPACE: &str = "http://schemas.openehr.org/v1";
const XML_SCHEMA_INSTANCE_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";
const MAX_OPT_XML_BYTES: usize = 8 * 1024 * 1024;
const MAX_OPT_XML_DEPTH: usize = 256;
const MAX_OPT_XML_NODES: usize = 100_000;

/// Errors arising from loading an Operational Template.
#[derive(Debug, thiserror::Error)]
pub enum OptError {
    /// The bytes were not valid `anarchie` OPT JSON.
    #[error("template JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// The bytes were not well-formed UTF-8 XML.
    #[error("template XML error: {0}")]
    Xml(#[from] quick_xml::Error),
    /// The XML document could not be lowered into anarchie's supported AOM subset.
    #[error("unsupported or malformed OPT XML: {0}")]
    XmlStructure(String),
    /// The template's root constraint was not a `COMPOSITION`.
    #[error("template root must constrain a COMPOSITION, found {0}")]
    NotAComposition(String),
}

/// A flattened Operational Template in `anarchie`'s native form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Opt {
    /// The template identifier, e.g. `"blood_pressure"`. Matches the
    /// `archetype_details.template_id` a Composition claims.
    pub template_id: String,
    /// The root archetype concept id, e.g.
    /// `"openEHR-EHR-COMPOSITION.encounter.v1"`.
    pub concept: String,
    /// The constraint tree, rooted at the template's top RM object.
    pub definition: CComplexObject,
}

impl Opt {
    /// Parse an Operational Template from `anarchie` OPT JSON.
    pub fn from_json(json: &str) -> Result<Self, OptError> {
        let opt: Opt = serde_json::from_str(json)?;
        if opt.definition.rm_type != "COMPOSITION" {
            return Err(OptError::NotAComposition(opt.definition.rm_type.clone()));
        }
        Ok(opt)
    }

    /// Parse legacy ADL 1.4 OPT XML into anarchie's flattened template model.
    ///
    /// This accepts the portion of an exported OPT that the native validator can
    /// represent. Unsupported constraints fail rather than being discarded.
    pub fn from_xml(xml: &str) -> Result<Self, OptError> {
        if xml.len() > MAX_OPT_XML_BYTES {
            return Err(OptError::XmlStructure(format!(
                "OPT XML exceeds the {} MiB input limit",
                MAX_OPT_XML_BYTES / (1024 * 1024)
            )));
        }
        let root = XmlNode::parse(xml)?;
        if root.name != "template" {
            return Err(OptError::XmlStructure(format!(
                "root element must be `template`, found `{}`",
                root.name
            )));
        }
        if root.attributes.get("xmlns").map(String::as_str) != Some(ADL14_XML_NAMESPACE) {
            return Err(OptError::XmlStructure(format!(
                "template must use the ADL 1.4 XML namespace `{ADL14_XML_NAMESPACE}`"
            )));
        }
        if root.attributes.get("xmlns:xsi").map(String::as_str)
            != Some(XML_SCHEMA_INSTANCE_NAMESPACE)
        {
            return Err(OptError::XmlStructure(format!(
                "template must bind xsi to `{XML_SCHEMA_INSTANCE_NAMESPACE}`"
            )));
        }
        if root.child("rules").is_some() {
            return Err(OptError::XmlStructure(
                "template rules are not supported".to_string(),
            ));
        }

        let template_id = root
            .child("template_id")
            .and_then(|node| node.child("value"))
            .and_then(XmlNode::required_text)
            .ok_or_else(|| OptError::XmlStructure("missing template_id/value".to_string()))?;
        let concept = root
            .child("concept")
            .and_then(XmlNode::required_text)
            .ok_or_else(|| OptError::XmlStructure("missing concept".to_string()))?;
        let definition = root
            .child("definition")
            .ok_or_else(|| OptError::XmlStructure("missing definition".to_string()))?;
        let definition = parse_complex(definition, true)?;
        if definition.rm_type != "COMPOSITION" {
            return Err(OptError::NotAComposition(definition.rm_type));
        }

        Ok(Self {
            template_id,
            concept,
            definition,
        })
    }

    /// Serialise the template to `anarchie` OPT JSON (pretty, newline-terminated).
    pub fn to_json(&self) -> Result<String, OptError> {
        let mut out = serde_json::to_string_pretty(self)?;
        out.push('\n');
        Ok(out)
    }
}

#[derive(Debug)]
struct XmlNode {
    name: String,
    attributes: BTreeMap<String, String>,
    children: Vec<Self>,
    text: String,
}

impl XmlNode {
    fn parse(xml: &str) -> Result<Self, OptError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut stack = Vec::new();
        let mut root = None;
        let mut node_count = 0;

        loop {
            match reader.read_event()? {
                Event::Start(event) => {
                    if stack.len() >= MAX_OPT_XML_DEPTH {
                        return Err(OptError::XmlStructure(format!(
                            "OPT XML exceeds the {MAX_OPT_XML_DEPTH}-element nesting limit"
                        )));
                    }
                    node_count += 1;
                    if node_count > MAX_OPT_XML_NODES {
                        return Err(OptError::XmlStructure(format!(
                            "OPT XML exceeds the {MAX_OPT_XML_NODES}-node limit"
                        )));
                    }
                    stack.push(Self::from_start(&event)?);
                }
                Event::Empty(event) => {
                    node_count += 1;
                    if node_count > MAX_OPT_XML_NODES {
                        return Err(OptError::XmlStructure(format!(
                            "OPT XML exceeds the {MAX_OPT_XML_NODES}-node limit"
                        )));
                    }
                    let node = Self::from_start(&event)?;
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else if root.replace(node).is_some() {
                        return Err(OptError::XmlStructure(
                            "XML document has more than one root element".to_string(),
                        ));
                    }
                }
                Event::Text(text) => {
                    let text = decode_xml_text(text.as_ref())?;
                    if let Some(node) = stack.last_mut() {
                        node.text.push_str(&text);
                    }
                }
                Event::CData(text) => {
                    let text = std::str::from_utf8(text.as_ref()).map_err(|error| {
                        OptError::XmlStructure(format!("XML is not UTF-8: {error}"))
                    })?;
                    if let Some(node) = stack.last_mut() {
                        node.text.push_str(text);
                    }
                }
                Event::End(_) => {
                    let node = stack.pop().ok_or_else(|| {
                        OptError::XmlStructure("unexpected closing XML element".to_string())
                    })?;
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else if root.replace(node).is_some() {
                        return Err(OptError::XmlStructure(
                            "XML document has more than one root element".to_string(),
                        ));
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        if !stack.is_empty() {
            return Err(OptError::XmlStructure("unclosed XML element".to_string()));
        }
        root.ok_or_else(|| OptError::XmlStructure("empty XML document".to_string()))
    }

    fn from_start(event: &quick_xml::events::BytesStart<'_>) -> Result<Self, OptError> {
        let mut attributes = BTreeMap::new();
        for attribute in event.attributes() {
            let attribute = attribute.map_err(|error| OptError::XmlStructure(error.to_string()))?;
            let key = std::str::from_utf8(attribute.key.as_ref())
                .map_err(|error| OptError::XmlStructure(format!("invalid XML attribute: {error}")))?
                .to_string();
            let value = decode_xml_text(attribute.value.as_ref())?;
            attributes.insert(key, value);
        }
        Ok(Self {
            name: local_name(event.name().as_ref()),
            attributes,
            children: Vec::new(),
            text: String::new(),
        })
    }

    fn child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|child| child.name == name)
    }

    fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Self> + 'a {
        self.children.iter().filter(move |child| child.name == name)
    }

    fn required_text(&self) -> Option<String> {
        (!self.text.is_empty()).then(|| self.text.clone())
    }

    fn constraint_type(&self) -> Result<&str, OptError> {
        self.attributes
            .get("xsi:type")
            .map(String::as_str)
            .ok_or_else(|| OptError::XmlStructure(format!("{} has no xsi:type", self.name)))
    }
}

fn local_name(name: &[u8]) -> String {
    String::from_utf8_lossy(name)
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn decode_xml_text(bytes: &[u8]) -> Result<String, OptError> {
    let value = std::str::from_utf8(bytes)
        .map_err(|error| OptError::XmlStructure(format!("XML is not UTF-8: {error}")))?;
    quick_xml::escape::unescape(value)
        .map(|value| value.into_owned())
        .map_err(|error| OptError::XmlStructure(format!("invalid XML escape: {error}")))
}

fn parse_complex(node: &XmlNode, root: bool) -> Result<CComplexObject, OptError> {
    reject_unexpected_children(
        node,
        &[
            "rm_type_name",
            "node_id",
            "archetype_id",
            "occurrences",
            "attributes",
        ],
    )?;
    let rm_type = text(node, "rm_type_name")?;
    let node_id = if root || node.constraint_type().ok() == Some("C_ARCHETYPE_ROOT") {
        nested_text(node, "archetype_id", "value")?
    } else {
        text(node, "node_id")?
    };

    Ok(CComplexObject {
        rm_type,
        node_id,
        occurrences: parse_multiplicity(node.child("occurrences"))?,
        attributes: node
            .children("attributes")
            .map(parse_attribute)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_attribute(node: &XmlNode) -> Result<CAttribute, OptError> {
    match node.constraint_type()? {
        "C_SINGLE_ATTRIBUTE" | "C_MULTIPLE_ATTRIBUTE" => {}
        kind => {
            return Err(OptError::XmlStructure(format!(
                "unsupported attribute xsi:type `{kind}`"
            )));
        }
    }
    reject_unexpected_children(
        node,
        &["rm_attribute_name", "existence", "cardinality", "children"],
    )?;
    let cardinality = node
        .child("cardinality")
        .map(parse_cardinality)
        .transpose()?;
    Ok(CAttribute {
        rm_attribute: text(node, "rm_attribute_name")?,
        existence: parse_multiplicity(node.child("existence"))?,
        cardinality,
        children: node
            .children("children")
            .map(parse_object)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_cardinality(node: &XmlNode) -> Result<Cardinality, OptError> {
    reject_unexpected_children(node, &["interval", "is_ordered", "is_unique"])?;
    let is_ordered = bool_text(node, "is_ordered")?.unwrap_or(false);
    let is_unique = bool_text(node, "is_unique")?.unwrap_or(false);
    if is_ordered || is_unique {
        return Err(OptError::XmlStructure(
            "ordered or unique cardinality is not supported".to_string(),
        ));
    }
    Ok(Cardinality {
        interval: parse_multiplicity(node.child("interval"))?,
        is_ordered,
        is_unique,
    })
}

fn parse_object(node: &XmlNode) -> Result<CObject, OptError> {
    match node.constraint_type()? {
        "C_COMPLEX_OBJECT" | "C_ARCHETYPE_ROOT" => {
            if matches!(text(node, "rm_type_name").as_deref(), Ok("DV_CODED_TEXT"))
                && node
                    .child("node_id")
                    .and_then(XmlNode::required_text)
                    .is_none()
            {
                return collapse_coded_text(node);
            }
            Ok(CObject::Complex(parse_complex(node, false)?))
        }
        "C_DV_QUANTITY" => Ok(CObject::DvQuantity(parse_quantity(node)?)),
        "C_CODE_PHRASE" => Ok(CObject::CodePhrase(parse_code_phrase(node)?)),
        unknown => Err(OptError::XmlStructure(format!(
            "unsupported constraint xsi:type `{unknown}`"
        ))),
    }
}

fn collapse_coded_text(node: &XmlNode) -> Result<CObject, OptError> {
    let attributes: Vec<&XmlNode> = node.children("attributes").collect();
    if attributes.len() != 1 {
        return Err(OptError::XmlStructure(
            "anonymous DV_CODED_TEXT must constrain only defining_code".to_string(),
        ));
    }
    let attribute = parse_attribute(attributes[0])?;
    if attribute.rm_attribute != "defining_code"
        || (attribute.existence != MultiplicityInterval::ANY
            && attribute.existence != MultiplicityInterval::MANDATORY)
        || attribute.cardinality.is_some()
        || attribute.children.len() != 1
    {
        return Err(OptError::XmlStructure(
            "anonymous DV_CODED_TEXT defining_code has unsupported constraints".to_string(),
        ));
    }
    match attribute
        .children
        .into_iter()
        .next()
        .expect("one checked child")
    {
        CObject::CodePhrase(code_phrase) => Ok(CObject::CodePhrase(code_phrase)),
        _ => Err(OptError::XmlStructure(
            "anonymous DV_CODED_TEXT defining_code must be C_CODE_PHRASE".to_string(),
        )),
    }
}

fn parse_quantity(node: &XmlNode) -> Result<CDvQuantity, OptError> {
    if node.child("property").is_some() {
        return Err(OptError::XmlStructure(
            "C_DV_QUANTITY property constraints are not supported".to_string(),
        ));
    }
    reject_unexpected_children(node, &["list"])?;
    Ok(CDvQuantity {
        property: None,
        list: node
            .children("list")
            .map(|item| {
                reject_unexpected_children(item, &["magnitude", "precision", "units"])?;
                Ok(CQuantityItem {
                    units: text(item, "units")?,
                    magnitude: item
                        .child("magnitude")
                        .map(parse_interval_f64)
                        .transpose()?,
                    precision: item
                        .child("precision")
                        .map(parse_interval_i64)
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, OptError>>()?,
    })
}

fn parse_code_phrase(node: &XmlNode) -> Result<CCodePhrase, OptError> {
    reject_unexpected_children(node, &["terminology_id", "code_list"])?;
    Ok(CCodePhrase {
        terminology: nested_text_opt(node, "terminology_id", "value"),
        codes: node
            .children("code_list")
            .map(|code| {
                code.required_text().ok_or_else(|| {
                    OptError::XmlStructure("empty C_CODE_PHRASE code_list".to_string())
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_multiplicity(node: Option<&XmlNode>) -> Result<MultiplicityInterval, OptError> {
    let Some(node) = node else {
        return Ok(MultiplicityInterval::ANY);
    };
    reject_unexpected_children(
        node,
        &[
            "lower",
            "upper",
            "lower_unbounded",
            "upper_unbounded",
            "lower_included",
            "upper_included",
        ],
    )?;
    if bool_text(node, "lower_unbounded")?.unwrap_or(false)
        || !bool_text(node, "lower_included")?.unwrap_or(true)
        || !bool_text(node, "upper_included")?.unwrap_or(true)
    {
        return Err(OptError::XmlStructure(
            "multiplicity intervals must have inclusive, bounded lower limits".to_string(),
        ));
    }
    let lower = integer_text(node, "lower")?.unwrap_or(0);
    let upper = if bool_text(node, "upper_unbounded")?.unwrap_or(false) {
        None
    } else {
        integer_text(node, "upper")?
    };
    if upper.is_some_and(|upper| upper < lower) {
        return Err(OptError::XmlStructure(format!(
            "multiplicity interval {lower}..{} is invalid",
            upper.unwrap()
        )));
    }
    Ok(MultiplicityInterval { lower, upper })
}

fn parse_interval_f64(node: &XmlNode) -> Result<Interval<f64>, OptError> {
    reject_unexpected_children(
        node,
        &["lower", "upper", "lower_included", "upper_included"],
    )?;
    Ok(Interval {
        lower: decimal_text(node, "lower")?,
        upper: decimal_text(node, "upper")?,
        lower_included: bool_text(node, "lower_included")?.unwrap_or(true),
        upper_included: bool_text(node, "upper_included")?.unwrap_or(true),
    })
}

fn parse_interval_i64(node: &XmlNode) -> Result<Interval<i64>, OptError> {
    reject_unexpected_children(
        node,
        &["lower", "upper", "lower_included", "upper_included"],
    )?;
    Ok(Interval {
        lower: integer_text(node, "lower")?.map(i64::from),
        upper: integer_text(node, "upper")?.map(i64::from),
        lower_included: bool_text(node, "lower_included")?.unwrap_or(true),
        upper_included: bool_text(node, "upper_included")?.unwrap_or(true),
    })
}

fn text(node: &XmlNode, name: &str) -> Result<String, OptError> {
    node.child(name)
        .and_then(XmlNode::required_text)
        .ok_or_else(|| OptError::XmlStructure(format!("missing {name}")))
}

fn nested_text(node: &XmlNode, parent: &str, child: &str) -> Result<String, OptError> {
    nested_text_opt(node, parent, child)
        .ok_or_else(|| OptError::XmlStructure(format!("missing {parent}/{child}")))
}

fn nested_text_opt(node: &XmlNode, parent: &str, child: &str) -> Option<String> {
    node.child(parent)
        .and_then(|node| node.child(child))
        .and_then(XmlNode::required_text)
}

fn bool_text(node: &XmlNode, name: &str) -> Result<Option<bool>, OptError> {
    node.child(name)
        .map(|node| match node.required_text().as_deref() {
            Some("true") => Ok(true),
            Some("false") => Ok(false),
            _ => Err(OptError::XmlStructure(format!(
                "{name} must be true or false"
            ))),
        })
        .transpose()
}

fn integer_text(node: &XmlNode, name: &str) -> Result<Option<u32>, OptError> {
    node.child(name)
        .map(|node| {
            node.required_text()
                .ok_or_else(|| OptError::XmlStructure(format!("empty {name}")))?
                .parse()
                .map_err(|error| OptError::XmlStructure(format!("invalid {name}: {error}")))
        })
        .transpose()
}

fn decimal_text(node: &XmlNode, name: &str) -> Result<Option<f64>, OptError> {
    node.child(name)
        .map(|node| {
            node.required_text()
                .ok_or_else(|| OptError::XmlStructure(format!("empty {name}")))?
                .parse()
                .map_err(|error| OptError::XmlStructure(format!("invalid {name}: {error}")))
        })
        .transpose()
}

fn reject_unexpected_children(node: &XmlNode, allowed: &[&str]) -> Result<(), OptError> {
    if let Some(child) = node
        .children
        .iter()
        .find(|child| !allowed.contains(&child.name.as_str()))
    {
        return Err(OptError::XmlStructure(format!(
            "unsupported {} child `{}`",
            node.name, child.name
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_minimal_template() {
        let json = r#"{
            "template_id": "blood_pressure",
            "concept": "openEHR-EHR-COMPOSITION.encounter.v1",
            "definition": {
                "type": "COMPLEX",
                "rm_type": "COMPOSITION",
                "node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
                "occurrences": { "lower": 1, "upper": 1 },
                "attributes": []
            }
        }"#;
        let opt = Opt::from_json(json).expect("parses");
        assert_eq!(opt.template_id, "blood_pressure");
        assert_eq!(opt.definition.rm_type, "COMPOSITION");
        let reparsed = Opt::from_json(&opt.to_json().unwrap()).unwrap();
        assert_eq!(opt, reparsed);
    }

    #[test]
    fn rejects_non_composition_root() {
        let json = r#"{
            "template_id": "x",
            "concept": "openEHR-EHR-OBSERVATION.blood_pressure.v2",
            "definition": {
                "type": "COMPLEX",
                "rm_type": "OBSERVATION",
                "node_id": "openEHR-EHR-OBSERVATION.blood_pressure.v2"
            }
        }"#;
        assert!(matches!(
            Opt::from_json(json),
            Err(OptError::NotAComposition(_))
        ));
    }

    #[test]
    fn lowers_a_legacy_adl14_opt_xml_fixture() {
        let xml = include_str!("../tests/fixtures/legacy-vital-signs.opt.xml");
        let opt = Opt::from_xml(xml).expect("legacy OPT XML parses");

        assert_eq!(opt.template_id, "legacy_vital_signs.v1");
        assert_eq!(opt.concept, "Legacy vital signs");
        assert_eq!(
            opt.definition.node_id,
            "openEHR-EHR-COMPOSITION.encounter.v1"
        );

        let CObject::CodePhrase(category) = &opt.definition.attributes[0].children[0] else {
            panic!("category is lowered to C_CODE_PHRASE");
        };
        assert_eq!(category.terminology.as_deref(), Some("openehr"));
        assert_eq!(category.codes, ["433"]);

        let content = &opt.definition.attributes[1];
        assert_eq!(content.cardinality.unwrap().interval.upper, Some(1));
        let CObject::Complex(observation) = &content.children[0] else {
            panic!("content is an archetype root");
        };
        assert_eq!(
            observation.node_id,
            "openEHR-EHR-OBSERVATION.blood_pressure.v2"
        );
        let CObject::Complex(item_tree) = &observation.attributes[0].children[0] else {
            panic!("observation data is an item tree");
        };
        let CObject::Complex(element) = &item_tree.attributes[0].children[0] else {
            panic!("item tree contains an element");
        };
        let CObject::DvQuantity(quantity) = &element.attributes[0].children[0] else {
            panic!("element value is a quantity constraint");
        };
        assert_eq!(quantity.property, None);
        assert_eq!(quantity.list[0].units, "mm[Hg]");
        assert_eq!(quantity.list[0].magnitude.unwrap().upper, Some(300.0));
        assert_eq!(quantity.list[0].precision.unwrap().upper, Some(1));
    }

    #[test]
    fn rejects_unknown_xml_constraints() {
        let xml = include_str!("../tests/fixtures/legacy-vital-signs.opt.xml").replacen(
            "C_DV_QUANTITY",
            "C_ARCHETYPE_SLOT",
            1,
        );
        assert!(matches!(
            Opt::from_xml(&xml),
            Err(OptError::XmlStructure(message)) if message.contains("C_ARCHETYPE_SLOT")
        ));
    }

    #[test]
    fn rejects_xml_constraints_that_cannot_be_lowered_without_loss() {
        let xml = include_str!("../tests/fixtures/legacy-vital-signs.opt.xml").replacen(
            "    </attributes>\n    <attributes xsi:type=\"C_MULTIPLE_ATTRIBUTE\">",
            "    </attributes>\n    <assumed_value>event</assumed_value>\n    <attributes xsi:type=\"C_MULTIPLE_ATTRIBUTE\">",
            1,
        );
        assert!(matches!(
            Opt::from_xml(&xml),
            Err(OptError::XmlStructure(message)) if message.contains("assumed_value")
        ));

        let xml = include_str!("../tests/fixtures/legacy-vital-signs.opt.xml").replacen(
            "<is_unique>false</is_unique>",
            "<is_unique>true</is_unique>",
            1,
        );
        assert!(matches!(
            Opt::from_xml(&xml),
            Err(OptError::XmlStructure(message)) if message.contains("unique cardinality")
        ));
    }

    #[test]
    fn requires_the_adl14_and_xml_schema_instance_namespaces() {
        let xml = include_str!("../tests/fixtures/legacy-vital-signs.opt.xml").replacen(
            "http://schemas.openehr.org/v1",
            "https://example.invalid/template",
            1,
        );
        assert!(matches!(
            Opt::from_xml(&xml),
            Err(OptError::XmlStructure(message)) if message.contains("ADL 1.4 XML namespace")
        ));
    }

    #[test]
    fn rejects_anonymous_coded_text_with_additional_constraints() {
        let xml = include_str!("../tests/fixtures/legacy-vital-signs.opt.xml").replacen(
            "        </attributes>\n      </children>\n    </attributes>",
            "        </attributes>\n        <attributes xsi:type=\"C_SINGLE_ATTRIBUTE\">\n          <rm_attribute_name>value</rm_attribute_name>\n          <children xsi:type=\"C_STRING\" />\n        </attributes>\n      </children>\n    </attributes>",
            1,
        );
        assert!(matches!(
            Opt::from_xml(&xml),
            Err(OptError::XmlStructure(message)) if message.contains("only defining_code")
        ));
    }

    #[test]
    fn rejects_quantity_property_constraints() {
        let xml = include_str!("../tests/fixtures/legacy-vital-signs.opt.xml").replacen(
            "<list>",
            "<property><code_string>124</code_string></property><list>",
            1,
        );
        assert!(matches!(
            Opt::from_xml(&xml),
            Err(OptError::XmlStructure(message)) if message.contains("property constraints")
        ));
    }

    #[test]
    fn lowered_xml_template_rejects_an_incompatible_leaf_value() {
        let opt = Opt::from_xml(include_str!("../tests/fixtures/legacy-vital-signs.opt.xml"))
            .expect("legacy OPT XML parses");
        let composition = serde_json::json!({
            "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
            "category": {
                "_type": "DV_CODED_TEXT",
                "value": "event",
                "defining_code": {
                    "terminology_id": { "value": "openehr" },
                    "code_string": "433"
                }
            },
            "content": [{
                "archetype_node_id": "openEHR-EHR-OBSERVATION.blood_pressure.v2",
                "data": {
                    "archetype_node_id": "at0003",
                    "items": [{
                        "archetype_node_id": "at0004",
                        "value": { "_type": "DV_TEXT", "value": "not a quantity" }
                    }]
                }
            }]
        });

        let report = crate::validate::validate_opt(&composition, &opt);
        assert!(!report.valid);
        assert!(report
            .violations
            .iter()
            .any(|violation| violation.constraint == "C_DV_QUANTITY"));
    }
}
