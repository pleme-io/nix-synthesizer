//! Integration tests proving `NixNode` conforms to `synthesizer_core` traits.
//!
//! Wave 2 of the compound-knowledge refactor. Every test calls one of
//! `synthesizer_core::node::laws::*` on a real `NixNode` value, compounding
//! proof surface: the same laws prove properties of every synthesizer that
//! conforms.

use nix_synthesizer::NixNode;
use synthesizer_core::node::laws;
use synthesizer_core::{NoRawAttestation, SynthesizerNode};

// ─── Trait shape ────────────────────────────────────────────────────

#[test]
fn indent_unit_is_two_spaces() {
    assert_eq!(<NixNode as SynthesizerNode>::indent_unit(), "  ");
}

#[test]
fn variant_ids_distinct_across_disjoint_variants() {
    let samples: Vec<NixNode> = vec![
        NixNode::Blank,
        NixNode::Null,
        NixNode::Int(42),
        NixNode::Bool(true),
        NixNode::Str("s".into()),
        NixNode::MultilineStr("m".into()),
        NixNode::ident("pkgs"),
        NixNode::path("./x"),
        NixNode::AttrSet(vec![]),
        NixNode::List(vec![]),
        NixNode::Comment("c".into()),
    ];
    let before = samples.len();
    let mut ids: Vec<u8> = samples.iter().map(SynthesizerNode::variant_id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        before,
        "variant_id must be distinct for disjoint variants"
    );
}

// ─── SynthesizerNode laws ───────────────────────────────────────────

#[test]
fn law_determinism_holds_on_simple_nodes() {
    for n in [NixNode::Null, NixNode::Int(0), NixNode::Bool(true)] {
        assert!(laws::is_deterministic(&n, 0));
        assert!(laws::is_deterministic(&n, 3));
    }
}

#[test]
fn law_determinism_holds_on_attr_set() {
    let n = NixNode::attr_set(vec![("k", NixNode::Int(1))]);
    assert!(laws::is_deterministic(&n, 2));
}

#[test]
fn law_honors_indent_unit_on_attr_set() {
    let n = NixNode::attr_set(vec![
        ("alpha", NixNode::Int(1)),
        ("beta", NixNode::Int(2)),
    ]);
    assert!(laws::honors_indent_unit(&n, 0));
    assert!(laws::honors_indent_unit(&n, 3));
}

#[test]
fn law_indent_monotone_len_on_attr_set() {
    let n = NixNode::attr_set(vec![("k", NixNode::Str("v".into()))]);
    assert!(laws::indent_monotone_len(&n, 0));
    assert!(laws::indent_monotone_len(&n, 1));
}

#[test]
fn law_variant_id_valid_on_all_sample_variants() {
    let samples = [
        NixNode::Blank,
        NixNode::Null,
        NixNode::Int(0),
        NixNode::Bool(false),
        NixNode::Str("x".into()),
        NixNode::ident("foo"),
        NixNode::path("./p"),
        NixNode::AttrSet(vec![]),
        NixNode::List(vec![]),
    ];
    for n in &samples {
        assert!(laws::variant_id_is_valid(n));
    }
}

// ─── NoRawAttestation ───────────────────────────────────────────────

#[test]
fn attestation_is_nonempty() {
    assert!(!<NixNode as NoRawAttestation>::attestation().is_empty());
}

#[test]
fn attestation_mentions_raw() {
    let s = <NixNode as NoRawAttestation>::attestation();
    assert!(
        s.to_lowercase().contains("raw"),
        "attestation must explain how no-raw is enforced — got: {s}"
    );
}

// ─── No-raw source invariant ────────────────────────────────────────

#[test]
fn no_raw_constructor_in_production_source() {
    // Scan src/ for `NixNode::Raw(...)` or `Self::Raw(...)` constructor
    // uses. Legitimate non-constructions (variant declaration, match arms,
    // and #[allow(deprecated)]-pinned references) are exempted.
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    for path in walk_rust_files(&src_dir) {
        let content = std::fs::read_to_string(&path).expect("read src file");
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            // Skip comments.
            if trimmed.starts_with("//") || trimmed.starts_with("*") {
                continue;
            }
            // Skip the variant declaration line.
            if line.contains("Raw(String)") {
                continue;
            }
            // Skip match arms (patterns, not constructions).
            if line.contains("=>") {
                continue;
            }
            // Skip when the preceding line explicitly allows deprecated
            // use (intentional reference like in the variant_id match).
            let prev_allows = i > 0 && lines[i - 1].contains("#[allow(deprecated)]");
            if prev_allows {
                continue;
            }
            // Skip attribute lines themselves.
            if trimmed.starts_with("#[") {
                continue;
            }
            if line.contains("NixNode::Raw(") || line.contains("Self::Raw(") {
                violations.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "NixNode::Raw construction in production source is forbidden \
         (use a typed variant). Violations: {violations:?}"
    );
}

fn walk_rust_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root).expect("read src dir") {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_rust_files(&path));
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}
