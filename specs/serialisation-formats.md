# Serialisation Formats

Canonical JSON is `anarchie`'s on-disk and primary wire format - it is the only representation the store ever persists. But the openEHR ecosystem speaks several other formats at the REST boundary, and form renderers in particular lean heavily on the FLAT and STRUCTURED conventions. This document fixes which formats `anarchie` supports, where, and how they relate to the canonical store.

The governing rule: **the store persists canonical JSON only. Every other format is a derived, lossless-where-possible conversion applied at the REST boundary, never a second source of truth.** This is the same discipline as the derived query index - convenience views over one authoritative representation.

---

## The formats

| Format | Media type | Where it is used | Authoritative? |
|---|---|---|---|
| **Canonical JSON** | `application/json` | On disk, default wire format | **Yes** - the source of truth |
| **Canonical XML** | `application/xml` | Optional wire format; OPT upload arrives as XML | No - converted to/from canonical JSON |
| **FLAT JSON** | `application/openehr.flat+json` | Wire only - form renderers, simple clients | No - generated from canonical JSON via the Web Template |
| **STRUCTURED JSON** | `application/openehr.structured+json` | Wire only - nested convenience form | No - generated from canonical JSON |
| **Web Template** | `application/openehr.wt+json` | Wire only - returned for a registered template | No - derived from the OPT at registration time |

Canonical JSON with `_type` discriminators is the spine. Everything else is a transformation layered on top of it, performed only when a client asks for it through content negotiation.

---

## Canonical JSON (the spine)

Already specified in [on-disk-format.md](on-disk-format.md): RM-faithful JSON with `_type` discriminators, serialised deterministically (stable key order, normalised number and whitespace formatting) so that semantically identical Compositions are byte-identical, diffs are minimal, and a content hash is a stable identity. This is what `serde` produces from the `rm` types, and byte-stability is the hard dependency of the whole design.

```json
{ "_type": "DV_QUANTITY", "magnitude": 120.0, "units": "mm[Hg]" }
```

---

## Canonical XML

The canonical XML form is RM-faithful XML, semantically equivalent to canonical JSON. It matters for one unavoidable reason: **Operational Templates are distributed as XML.** Even if `anarchie`'s own native template form is flattened JSON (see [validation.md](validation.md)), ingesting a real `.opt` exported from Archetype Designer or the ADL Workbench means parsing OPT XML. Beyond OPT ingestion, canonical XML on the wire is an optional, later convenience; canonical JSON is the default for everything.

Implementation note: XML (de)serialisation is a feature-gated concern (a `quick-xml`-class dependency), kept out of the core so the base binary stays light.

---

## FLAT and STRUCTURED (the renderer formats)

These two formats are not in the normative openEHR ITS, but they originate in the openEHR SDK and EHRbase and are so widely used by form renderers and integration clients that supporting them materially affects whether real clients can talk to `anarchie serve`.

**FLAT JSON** collapses the RM tree into a flat map of path-keyed scalar pairs:

```jsonc
{
  "vital_signs/blood_pressure/any_event:0/systolic|magnitude": 120.0,
  "vital_signs/blood_pressure/any_event:0/systolic|unit": "mm[Hg]",
  "ctx/language": "en",
  "ctx/territory": "GB"
}
```

FLAT path conventions:

- `|` separates an RM leaf's sub-properties: `systolic|magnitude`, `systolic|unit`.
- `:N` is a zero-indexed suffix for repeated items: `any_event:0`, `any_event:1`.
- Paths use human-readable Web Template node names, not at-codes.
- `ctx/` prefixes composition-level defaults (language, territory, composer, start_time).
- A leading `_` marks RM metadata keys (for example `_uid`).

**STRUCTURED JSON** is the same information as a nested JSON tree (objects and arrays mirroring the Web Template shape) rather than a flat key map - a middle ground between FLAT's terseness and canonical JSON's RM fidelity.

Both formats are **lossy without a Web Template**: the path keys are defined by the template, not by the RM. Converting FLAT or STRUCTURED to canonical JSON (and back) therefore requires the Web Template derived from the Composition's template. This is why FLAT/STRUCTURED are wire-only and the store always holds canonical JSON.

---

## Web Template generation

A **Web Template** is a JSON projection of an Operational Template, generated once when a template is registered (`anarchie template add` / the definition endpoint) and cached alongside the OPT. It provides:

- a flattened input tree keyed by human-readable node names,
- the path mapping that FLAT and STRUCTURED conversion needs,
- per-node constraint metadata (occurrences, allowed values, units, terminology bindings) for form renderers.

The Web Template is **derived and disposable** in exactly the same sense as the query index: it can be regenerated from the OPT at any time and is never an independent source of truth. Storing it is a cache, not a commitment.

```
OPT (XML or native JSON)
   │  anarchie template add
   ▼
AOM constraint tree  ──►  Web Template (wt.json, cached)
                              │
                              ▼
                    FLAT / STRUCTURED  ⇄  canonical JSON
```

---

## Scope and phasing

- **Now (shipped):** canonical JSON on disk and on the wire. The native flattened-JSON template form.
- **Validation phase:** OPT XML ingestion (parse a real `.opt` into the AOM tree).
- **Services phase:** Web Template generation on template registration; FLAT and STRUCTURED conversion at the REST boundary; canonical XML content negotiation.

Each conversion is a boundary concern. None of them changes what the store holds: one canonical JSON document per Composition version, byte-stable and git-diffable.
