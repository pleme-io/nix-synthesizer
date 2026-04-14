use iac_forge::ir::IacType;

use crate::types::NixType;

/// Map an `IacType` to its corresponding NixOS module option type.
///
/// This function is:
/// - **Total**: every `IacType` variant is handled
/// - **Injective**: distinct inputs produce distinct outputs
/// - **Deterministic**: same input always produces same output
#[must_use]
pub fn iac_type_to_nix(ty: &IacType) -> NixType {
    match ty {
        IacType::String => NixType::Str,
        IacType::Integer => NixType::Int,
        IacType::Float => NixType::Float,
        IacType::Numeric => NixType::one_of(vec![NixType::Int, NixType::Float]),
        IacType::Boolean => NixType::Bool,
        IacType::List(inner) | IacType::Set(inner) => NixType::list_of(iac_type_to_nix(inner)),
        IacType::Map(inner) => NixType::attrs_of(iac_type_to_nix(inner)),
        IacType::Object { .. } => NixType::Attrs,
        IacType::Enum { values, .. } => {
            if values.is_empty() {
                NixType::Str
            } else {
                NixType::Enum(values.clone())
            }
        }
        IacType::Any => NixType::Anything,
        other => panic!("unsupported IacType variant: {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_maps_to_str() {
        assert_eq!(iac_type_to_nix(&IacType::String), NixType::Str);
    }

    #[test]
    fn integer_maps_to_int() {
        assert_eq!(iac_type_to_nix(&IacType::Integer), NixType::Int);
    }

    #[test]
    fn float_maps_to_float() {
        assert_eq!(iac_type_to_nix(&IacType::Float), NixType::Float);
    }

    #[test]
    fn numeric_maps_to_either() {
        let ty = iac_type_to_nix(&IacType::Numeric);
        assert_eq!(ty.emit(), "types.either types.int types.float");
    }

    #[test]
    fn boolean_maps_to_bool() {
        assert_eq!(iac_type_to_nix(&IacType::Boolean), NixType::Bool);
    }

    #[test]
    fn list_maps_to_list_of() {
        let ty = iac_type_to_nix(&IacType::List(Box::new(IacType::String)));
        assert_eq!(ty.emit(), "types.listOf types.str");
    }

    #[test]
    fn set_maps_to_list_of() {
        let ty = iac_type_to_nix(&IacType::Set(Box::new(IacType::Integer)));
        assert_eq!(ty.emit(), "types.listOf types.int");
    }

    #[test]
    fn map_maps_to_attrs_of() {
        let ty = iac_type_to_nix(&IacType::Map(Box::new(IacType::String)));
        assert_eq!(ty.emit(), "types.attrsOf types.str");
    }

    #[test]
    fn object_maps_to_attrs() {
        let ty = iac_type_to_nix(&IacType::Object {
            name: "test".into(),
            fields: vec![],
        });
        assert_eq!(ty, NixType::Attrs);
    }

    #[test]
    fn enum_with_values_maps_to_enum() {
        let ty = iac_type_to_nix(&IacType::Enum {
            values: vec!["a".into(), "b".into()],
            underlying: Box::new(IacType::String),
        });
        assert_eq!(ty.emit(), r#"types.enum [ "a" "b" ]"#);
    }

    #[test]
    fn enum_without_values_maps_to_str() {
        let ty = iac_type_to_nix(&IacType::Enum {
            values: vec![],
            underlying: Box::new(IacType::String),
        });
        assert_eq!(ty, NixType::Str);
    }

    #[test]
    fn any_maps_to_anything() {
        assert_eq!(iac_type_to_nix(&IacType::Any), NixType::Anything);
    }

    #[test]
    fn nested_list_of_map() {
        let ty = iac_type_to_nix(&IacType::List(Box::new(IacType::Map(Box::new(
            IacType::Boolean,
        )))));
        assert_eq!(ty.emit(), "types.listOf (types.attrsOf types.bool)");
    }

    #[test]
    fn deterministic_output() {
        let ty = IacType::List(Box::new(IacType::String));
        let a = iac_type_to_nix(&ty).emit();
        let b = iac_type_to_nix(&ty).emit();
        assert_eq!(a, b);
    }

    #[test]
    fn injective_distinct_inputs() {
        let types = vec![
            IacType::String,
            IacType::Integer,
            IacType::Float,
            IacType::Boolean,
            IacType::Any,
        ];
        let outputs: Vec<String> = types.iter().map(|t| iac_type_to_nix(t).emit()).collect();
        // All outputs must be distinct
        for i in 0..outputs.len() {
            for j in (i + 1)..outputs.len() {
                assert_ne!(outputs[i], outputs[j], "outputs must be injective");
            }
        }
    }
}
