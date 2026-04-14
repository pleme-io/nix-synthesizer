use nix_synthesizer::*;

// ── Type algebra: null_or ────────────────────────────��──────────────

#[test]
fn null_or_wrapping_adds_null_or() {
    let base = NixType::Str;
    let wrapped = NixType::null_or(base);
    assert_eq!(wrapped.emit(), "types.nullOr types.str");
}

#[test]
fn null_or_idempotent_for_all_base_types() {
    let types = vec![
        NixType::Str,
        NixType::Int,
        NixType::Float,
        NixType::Bool,
        NixType::Path,
        NixType::Package,
        NixType::Attrs,
        NixType::Anything,
    ];
    for ty in types {
        let once = NixType::null_or(ty.clone());
        let twice = NixType::null_or(once.clone());
        assert_eq!(once, twice, "null_or must be idempotent for {:?}", ty);
    }
}

// ── Type algebra: one_of ────────────────────────────────────────────

#[test]
fn one_of_two_is_either() {
    let ty = NixType::one_of(vec![NixType::Str, NixType::Int]);
    match ty {
        NixType::Either(_, _) => {}
        other => panic!("expected Either, got {:?}", other),
    }
}

#[test]
fn one_of_three_is_one_of() {
    let ty = NixType::one_of(vec![NixType::Str, NixType::Int, NixType::Bool]);
    match ty {
        NixType::OneOf(v) => assert_eq!(v.len(), 3),
        other => panic!("expected OneOf, got {:?}", other),
    }
}

#[test]
fn one_of_single_returns_inner() {
    let ty = NixType::one_of(vec![NixType::Bool]);
    assert_eq!(ty, NixType::Bool);
}

// ── Type algebra: list_of ──────────────────────────────────���────────

#[test]
fn list_of_nests_correctly() {
    let inner = NixType::list_of(NixType::Str);
    let outer = NixType::list_of(inner);
    assert_eq!(outer.emit(), "types.listOf (types.listOf types.str)");
}

// ── Type algebra: attrs_of ────────────────────────────��─────────────

#[test]
fn attrs_of_nests_correctly() {
    let inner = NixType::attrs_of(NixType::Int);
    let outer = NixType::attrs_of(inner);
    assert_eq!(outer.emit(), "types.attrsOf (types.attrsOf types.int)");
}

// ── Type injectivity: distinct types → distinct emissions ───────────

#[test]
fn all_base_types_emit_distinctly() {
    let types: Vec<NixType> = vec![
        NixType::Str,
        NixType::Int,
        NixType::Float,
        NixType::Bool,
        NixType::Path,
        NixType::Package,
        NixType::Attrs,
        NixType::Anything,
    ];
    let emissions: Vec<String> = types.iter().map(|t| t.emit()).collect();
    for i in 0..emissions.len() {
        for j in (i + 1)..emissions.len() {
            assert_ne!(
                emissions[i], emissions[j],
                "types must emit distinctly: {} vs {}",
                emissions[i], emissions[j]
            );
        }
    }
}

// ── Type to node conversion ─────────────────────────────────────────

#[test]
fn to_node_preserves_emission() {
    let types = vec![
        NixType::Str,
        NixType::Int,
        NixType::list_of(NixType::Bool),
        NixType::null_or(NixType::Str),
    ];
    for ty in types {
        let node = ty.to_node();
        let type_emit = ty.emit();
        let node_emit = node.emit(0);
        assert_eq!(type_emit, node_emit, "to_node must preserve emission");
    }
}

// ── Compound type parenthesization ──────────��───────────────────────

#[test]
fn compound_types_get_parenthesized() {
    // types.listOf (types.attrsOf types.str)
    let ty = NixType::list_of(NixType::attrs_of(NixType::Str));
    assert!(ty.emit().contains('('), "compound inner types need parens");
}

#[test]
fn simple_types_no_parens() {
    let ty = NixType::list_of(NixType::Str);
    assert!(!ty.emit().contains('('), "simple inner types should not have parens");
}

// ── Enum type ───────────────────────���───────────────────────────────

#[test]
fn enum_preserves_order() {
    let ty = NixType::enum_of(vec!["z", "a", "m"]);
    assert_eq!(ty.emit(), r#"types.enum [ "z" "a" "m" ]"#);
}

#[test]
fn enum_handles_special_characters() {
    let ty = NixType::Enum(vec!["us-east-1".into(), "eu-west-1".into()]);
    let out = ty.emit();
    assert!(out.contains("us-east-1"));
    assert!(out.contains("eu-west-1"));
}
