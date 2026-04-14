use proptest::prelude::*;

use nix_synthesizer::*;

// ── Proptest strategies ─────────────────────────────────────────────

fn arb_simple_node() -> impl Strategy<Value = NixNode> {
    prop_oneof![
        any::<i64>().prop_map(NixNode::Int),
        any::<bool>().prop_map(NixNode::Bool),
        Just(NixNode::Null),
        "[a-z][a-z0-9_]{0,10}".prop_map(|s| NixNode::Ident(s)),
        "[a-zA-Z0-9 _-]{0,20}".prop_map(|s| NixNode::Str(s)),
        "[a-z][a-z0-9_]{0,10}".prop_map(|s| NixNode::Comment(s)),
        Just(NixNode::Blank),
    ]
}

fn arb_nix_type() -> impl Strategy<Value = NixType> {
    prop_oneof![
        Just(NixType::Str),
        Just(NixType::Int),
        Just(NixType::Float),
        Just(NixType::Bool),
        Just(NixType::Path),
        Just(NixType::Package),
        Just(NixType::Attrs),
        Just(NixType::Anything),
    ]
}

// ── Property: all simple nodes emit non-panicking ───────────────────

proptest! {
    #[test]
    fn simple_node_emit_does_not_panic(node in arb_simple_node()) {
        let _ = node.emit(0);
    }

    #[test]
    fn simple_node_emit_at_any_indent(node in arb_simple_node(), indent in 0usize..10) {
        let _ = node.emit(indent);
    }

    // ── Property: emit is deterministic ────────────────��────────────

    #[test]
    fn emit_deterministic(node in arb_simple_node()) {
        let a = node.emit(0);
        let b = node.emit(0);
        prop_assert_eq!(a, b);
    }

    // ── Property: emit_file always ends with newline ────────────────

    #[test]
    fn emit_file_trailing_newline(nodes in proptest::collection::vec(arb_simple_node(), 0..5)) {
        let out = emit_file(&nodes);
        prop_assert!(out.ends_with('\n'), "emit_file must end with newline");
    }

    // ── Property: attr sets have balanced braces ────────────────────

    #[test]
    fn attr_set_balanced(
        keys in proptest::collection::vec("[a-z]{1,5}", 1..5),
        values in proptest::collection::vec(any::<i64>(), 1..5)
    ) {
        let len = keys.len().min(values.len());
        let bindings: Vec<Binding> = keys[..len].iter().zip(&values[..len])
            .map(|(k, v)| Binding::new(k, NixNode::Int(*v)))
            .collect();
        let node = NixNode::AttrSet(bindings);
        let out = node.emit(0);
        let opens = out.chars().filter(|&c| c == '{').count();
        let closes = out.chars().filter(|&c| c == '}').count();
        prop_assert!(opens == closes, "braces must be balanced");
    }

    // ── Property: lists have balanced brackets ──────────────────────

    #[test]
    fn list_balanced(values in proptest::collection::vec(any::<i64>(), 0..10)) {
        let elems: Vec<NixNode> = values.into_iter().map(NixNode::Int).collect();
        let node = NixNode::List(elems);
        let out = node.emit(0);
        let opens = out.chars().filter(|&c| c == '[').count();
        let closes = out.chars().filter(|&c| c == ']').count();
        prop_assert!(opens == closes, "brackets must be balanced");
    }

    // ── Property: strings always have balanced quotes ───────────────

    #[test]
    fn string_balanced_quotes(s in "[a-zA-Z0-9 ]{0,20}") {
        let out = NixNode::Str(s).emit(0);
        let count = out.chars().filter(|&c| c == '"').count();
        prop_assert!(count == 2, "string must have exactly 2 quotes");
    }

    // ── Property: NixType always starts with "types." ───────────────

    #[test]
    fn nix_type_starts_with_types(ty in arb_nix_type()) {
        let out = ty.emit();
        prop_assert!(out.starts_with("types."), "NixType must start with 'types.': {out}");
    }

    // ── Property: NixType emit is deterministic ─────────────────────

    #[test]
    fn nix_type_deterministic(ty in arb_nix_type()) {
        let a = ty.emit();
        let b = ty.emit();
        prop_assert_eq!(a, b);
    }

    // ── Property: null_or is idempotent ─────────────────────────────

    #[test]
    fn null_or_idempotent(ty in arb_nix_type()) {
        let once = NixType::null_or(ty.clone());
        let twice = NixType::null_or(once.clone());
        prop_assert_eq!(once, twice, "null_or must be idempotent");
    }

    // ── Property: one_of with single variant degenerates ────────────

    #[test]
    fn one_of_single_degenerates(ty in arb_nix_type()) {
        let result = NixType::one_of(vec![ty.clone()]);
        prop_assert_eq!(result, ty, "one_of with 1 variant must degenerate");
    }

    // ── Property: no control chars except newline/tab ───────────────

    #[test]
    fn no_unexpected_control_chars(node in arb_simple_node()) {
        let out = node.emit(0);
        for ch in out.chars() {
            if ch.is_control() {
                prop_assert!(
                    ch == '\n' || ch == '\t',
                    "unexpected control char: {:?} in: {}",
                    ch, out
                );
            }
        }
    }

    // ── Property: indentation is always multiples of 2 ──────────────

    #[test]
    fn indentation_multiples_of_two(
        keys in proptest::collection::vec("[a-z]{1,3}", 1..3),
        values in proptest::collection::vec(any::<i64>(), 1..3)
    ) {
        let len = keys.len().min(values.len());
        let bindings: Vec<Binding> = keys[..len].iter().zip(&values[..len])
            .map(|(k, v)| Binding::new(k, NixNode::Int(*v)))
            .collect();
        let node = NixNode::AttrSet(bindings);
        let out = node.emit(0);
        for line in out.lines() {
            if line.starts_with(' ') {
                let spaces = line.len() - line.trim_start().len();
                prop_assert!(
                    spaces % 2 == 0,
                    "indentation must be multiples of 2"
                );
            }
        }
    }
}
