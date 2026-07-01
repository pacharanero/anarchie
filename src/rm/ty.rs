// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Type discriminator (`_type`) support for openEHR canonical JSON.
//!
//! openEHR canonical JSON tags polymorphic objects with a `_type` field whose
//! value is the upper-case RM class name (e.g. `"DV_TEXT"`). For *polymorphic*
//! slots we model this with `#[serde(tag = "_type")]` enums. For *monomorphic*
//! concrete structs (e.g. `EVENT_CONTEXT`) the class is fixed, yet serialisers
//! such as EHRbase still emit `_type`. [`Ty`] is a zero-sized field that
//! serialises to a fixed class name and tolerates (or validates) it on input,
//! so our canonical form round-trips faithfully with those serialisers.

use std::marker::PhantomData;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A compile-time association between a marker type and its RM class name.
pub trait RmType {
    /// The upper-case openEHR class name, e.g. `"EVENT_CONTEXT"`.
    const TYPE: &'static str;
}

/// Zero-sized `_type` discriminator that (de)serialises a fixed class name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ty<T: RmType>(PhantomData<T>);

impl<T: RmType> Default for Ty<T> {
    fn default() -> Self {
        Ty(PhantomData)
    }
}

impl<T: RmType> Serialize for Ty<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(T::TYPE)
    }
}

impl<'de, T: RmType> Deserialize<'de> for Ty<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let found = String::deserialize(deserializer)?;
        if found != T::TYPE {
            return Err(D::Error::custom(format!(
                "expected _type \"{}\", found \"{}\"",
                T::TYPE,
                found
            )));
        }
        Ok(Ty(PhantomData))
    }
}

/// Declare a marker type implementing [`RmType`] for a given class name.
macro_rules! rm_type_marker {
    ($name:ident, $class:literal) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        pub struct $name;
        impl $crate::rm::ty::RmType for $name {
            const TYPE: &'static str = $class;
        }
    };
}

/// Marker types for the monomorphic RM classes that carry a `_type` field.
pub mod tags {
    rm_type_marker!(TerminologyId, "TERMINOLOGY_ID");
    rm_type_marker!(ArchetypeId, "ARCHETYPE_ID");
    rm_type_marker!(TemplateId, "TEMPLATE_ID");
    rm_type_marker!(HierObjectId, "HIER_OBJECT_ID");
    rm_type_marker!(CodePhrase, "CODE_PHRASE");
    rm_type_marker!(PartyRef, "PARTY_REF");
    rm_type_marker!(LocatableRef, "LOCATABLE_REF");
    rm_type_marker!(Archetyped, "ARCHETYPED");
    rm_type_marker!(Composition, "COMPOSITION");
    rm_type_marker!(Ehr, "EHR");
    rm_type_marker!(EhrStatus, "EHR_STATUS");
    rm_type_marker!(EventContext, "EVENT_CONTEXT");
    rm_type_marker!(History, "HISTORY");
    rm_type_marker!(Activity, "ACTIVITY");
    rm_type_marker!(IsmTransition, "ISM_TRANSITION");
    rm_type_marker!(InstructionDetails, "INSTRUCTION_DETAILS");
    rm_type_marker!(Participation, "PARTICIPATION");
    rm_type_marker!(TermMapping, "TERM_MAPPING");
    rm_type_marker!(Link, "LINK");
}
