use nix_synthesizer::iac_bridge::iac_type_to_nix;
use nix_synthesizer::NixType;

// ── Totality: every IacType variant is handled ──────────────────────

#[test]
fn bridge_handles_string() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::String);
    assert_eq!(ty, NixType::Str);
}

#[test]
fn bridge_handles_integer() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::Integer);
    assert_eq!(ty, NixType::Int);
}

#[test]
fn bridge_handles_float() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::Float);
    assert_eq!(ty, NixType::Float);
}

#[test]
fn bridge_handles_numeric() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::Numeric);
    assert_eq!(ty.emit(), "types.either types.int types.float");
}

#[test]
fn bridge_handles_boolean() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::Boolean);
    assert_eq!(ty, NixType::Bool);
}

#[test]
fn bridge_handles_list() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::List(Box::new(
        iac_forge::ir::IacType::String,
    )));
    assert_eq!(ty.emit(), "types.listOf types.str");
}

#[test]
fn bridge_handles_set() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::Set(Box::new(
        iac_forge::ir::IacType::Integer,
    )));
    assert_eq!(ty.emit(), "types.listOf types.int");
}

#[test]
fn bridge_handles_map() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::Map(Box::new(
        iac_forge::ir::IacType::String,
    )));
    assert_eq!(ty.emit(), "types.attrsOf types.str");
}

#[test]
fn bridge_handles_object() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::Object {
        name: "test".into(),
        fields: vec![],
    });
    assert_eq!(ty, NixType::Attrs);
}

#[test]
fn bridge_handles_enum_with_values() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::Enum {
        values: vec!["a".into(), "b".into()],
        underlying: Box::new(iac_forge::ir::IacType::String),
    });
    assert_eq!(ty.emit(), r#"types.enum [ "a" "b" ]"#);
}

#[test]
fn bridge_handles_enum_without_values() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::Enum {
        values: vec![],
        underlying: Box::new(iac_forge::ir::IacType::String),
    });
    assert_eq!(ty, NixType::Str);
}

#[test]
fn bridge_handles_any() {
    let ty = iac_type_to_nix(&iac_forge::ir::IacType::Any);
    assert_eq!(ty, NixType::Anything);
}

// ── Injectivity: distinct inputs → distinct outputs ─────────────────

#[test]
fn bridge_injective_base_types() {
    use iac_forge::ir::IacType;
    let types = vec![
        IacType::String,
        IacType::Integer,
        IacType::Float,
        IacType::Boolean,
        IacType::Any,
    ];
    let outputs: Vec<String> = types.iter().map(|t| iac_type_to_nix(t).emit()).collect();
    for i in 0..outputs.len() {
        for j in (i + 1)..outputs.len() {
            assert_ne!(
                outputs[i], outputs[j],
                "bridge must be injective: {} vs {}",
                outputs[i], outputs[j]
            );
        }
    }
}

// ── Determinism ─────────────────────────────────────────────────────

#[test]
fn bridge_deterministic() {
    use iac_forge::ir::IacType;
    let types = vec![
        IacType::String,
        IacType::Integer,
        IacType::List(Box::new(IacType::Boolean)),
        IacType::Map(Box::new(IacType::String)),
        IacType::Enum {
            values: vec!["x".into()],
            underlying: Box::new(IacType::String),
        },
    ];
    for ty in &types {
        let a = iac_type_to_nix(ty).emit();
        let b = iac_type_to_nix(ty).emit();
        assert_eq!(a, b, "bridge must be deterministic for {:?}", ty);
    }
}

// ── Recursive composition ───────────────────────────────────────────

#[test]
fn bridge_nested_list_of_map() {
    use iac_forge::ir::IacType;
    let ty = iac_type_to_nix(&IacType::List(Box::new(IacType::Map(Box::new(
        IacType::Boolean,
    )))));
    assert_eq!(ty.emit(), "types.listOf (types.attrsOf types.bool)");
}

#[test]
fn bridge_deeply_nested() {
    use iac_forge::ir::IacType;
    let ty = iac_type_to_nix(&IacType::List(Box::new(IacType::List(Box::new(
        IacType::Set(Box::new(IacType::String)),
    )))));
    assert_eq!(
        ty.emit(),
        "types.listOf (types.listOf (types.listOf types.str))"
    );
}
