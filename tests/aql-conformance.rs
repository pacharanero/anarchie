// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::path::Path;

#[test]
fn parser_matches_the_declared_aql_conformance_corpus() {
    let suite = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/aql-conformance");
    let manifest = fs::read_to_string(suite.join("cases.tsv")).expect("reads conformance manifest");
    let mut failures = Vec::new();

    for line in manifest.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 5, "invalid conformance case: {line}");

        let query = fs::read_to_string(suite.join(fields[1])).expect("reads AQL fixture");
        let actual = if anarchie::query::parse(&query).is_ok() {
            "accept"
        } else {
            "reject"
        };

        if actual != fields[3] {
            failures.push(format!(
                "{}: anarchie returned {actual}, expected {} ({})",
                fields[0], fields[3], fields[4]
            ));
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
