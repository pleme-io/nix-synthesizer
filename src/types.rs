use crate::node::NixNode;

/// NixOS module option types (`lib.types.*`).
///
/// Each variant maps to exactly one `types.*` expression. Construction
/// helpers enforce structural invariants (e.g., `optional` is idempotent,
/// `one_of` with a single variant degenerates).
#[derive(Debug, Clone, PartialEq)]
pub enum NixType {
    /// `types.str`
    Str,
    /// `types.int`
    Int,
    /// `types.float`
    Float,
    /// `types.bool`
    Bool,
    /// `types.path`
    Path,
    /// `types.package`
    Package,
    /// `types.attrs`
    Attrs,
    /// `types.anything`
    Anything,
    /// `types.listOf inner`
    ListOf(Box<NixType>),
    /// `types.attrsOf inner`
    AttrsOf(Box<NixType>),
    /// `types.enum [ "a" "b" ... ]`
    Enum(Vec<String>),
    /// `types.nullOr inner`
    NullOr(Box<NixType>),
    /// `types.submodule { ... }`
    Submodule(Vec<SubmoduleOption>),
    /// `types.oneOf [ type1 type2 ... ]`
    OneOf(Vec<NixType>),
    /// `types.either a b`
    Either(Box<NixType>, Box<NixType>),
    /// Raw type expression (escape hatch)
    Raw(String),
}

/// An option within a submodule type.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmoduleOption {
    pub name: String,
    pub option_type: NixType,
    pub default: Option<NixNode>,
    pub description: Option<String>,
}

// ── Constructors ────────────────────────────────────────────────────

impl NixType {
    #[must_use]
    pub fn list_of(inner: Self) -> Self {
        Self::ListOf(Box::new(inner))
    }

    #[must_use]
    pub fn attrs_of(inner: Self) -> Self {
        Self::AttrsOf(Box::new(inner))
    }

    /// Wrap in `types.nullOr`. Idempotent: `null_or(null_or(x)) == null_or(x)`.
    #[must_use]
    pub fn null_or(inner: Self) -> Self {
        if matches!(inner, Self::NullOr(_)) {
            return inner;
        }
        Self::NullOr(Box::new(inner))
    }

    /// Union of types. 0 variants panics, 1 variant degenerates,
    /// 2 variants become `either`, 3+ become `oneOf`.
    #[must_use]
    pub fn one_of(variants: Vec<Self>) -> Self {
        match variants.len() {
            0 => panic!("NixType::one_of with 0 variants is invalid"),
            1 => variants.into_iter().next().expect("checked len"),
            2 => {
                let mut iter = variants.into_iter();
                let a = iter.next().expect("checked len");
                let b = iter.next().expect("checked len");
                Self::Either(Box::new(a), Box::new(b))
            }
            _ => Self::OneOf(variants),
        }
    }

    #[must_use]
    pub fn enum_of(values: Vec<&str>) -> Self {
        Self::Enum(values.into_iter().map(|s| s.to_string()).collect())
    }

    /// Emit as a Nix expression string (e.g., `types.str`).
    #[must_use]
    pub fn emit(&self) -> String {
        match self {
            Self::Str => "types.str".into(),
            Self::Int => "types.int".into(),
            Self::Float => "types.float".into(),
            Self::Bool => "types.bool".into(),
            Self::Path => "types.path".into(),
            Self::Package => "types.package".into(),
            Self::Attrs => "types.attrs".into(),
            Self::Anything => "types.anything".into(),
            Self::ListOf(inner) => format!("types.listOf {}", wrap_complex(&inner.emit())),
            Self::AttrsOf(inner) => format!("types.attrsOf {}", wrap_complex(&inner.emit())),
            Self::Enum(values) => {
                let vals: Vec<String> = values.iter().map(|v| format!("\"{v}\"")).collect();
                format!("types.enum [ {} ]", vals.join(" "))
            }
            Self::NullOr(inner) => format!("types.nullOr {}", wrap_complex(&inner.emit())),
            Self::OneOf(variants) => {
                let parts: Vec<String> = variants.iter().map(|v| v.emit()).collect();
                format!("types.oneOf [ {} ]", parts.join(" "))
            }
            Self::Either(a, b) => {
                format!("types.either {} {}", wrap_complex(&a.emit()), wrap_complex(&b.emit()))
            }
            Self::Submodule(options) => {
                if options.is_empty() {
                    return "types.submodule { }".into();
                }
                let mut lines = vec!["types.submodule {".to_string()];
                lines.push("  options = {".to_string());
                for opt in options {
                    let mk = emit_mk_option(opt);
                    lines.push(format!("    {} = {};", opt.name, mk));
                }
                lines.push("  };".to_string());
                lines.push("}".to_string());
                lines.join("\n")
            }
            Self::Raw(expr) => expr.clone(),
        }
    }

    /// Convert this type expression to a NixNode for embedding in ASTs.
    #[must_use]
    pub fn to_node(&self) -> NixNode {
        NixNode::Raw(self.emit())
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Wrap compound type expressions in parens for clarity.
fn wrap_complex(s: &str) -> String {
    if s.contains(' ') && !s.starts_with('(') {
        format!("({s})")
    } else {
        s.to_string()
    }
}

fn emit_mk_option(opt: &SubmoduleOption) -> String {
    let mut parts = vec![format!("type = {}", opt.option_type.emit())];
    if let Some(ref def) = opt.default {
        parts.push(format!("default = {}", def.emit(0)));
    }
    if let Some(ref desc) = opt.description {
        let escaped = desc.replace('"', "\\\"");
        parts.push(format!("description = \"{escaped}\""));
    }
    format!("lib.mkOption {{ {} }}", parts.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_emits_types_str() {
        assert_eq!(NixType::Str.emit(), "types.str");
    }

    #[test]
    fn int_emits_types_int() {
        assert_eq!(NixType::Int.emit(), "types.int");
    }

    #[test]
    fn bool_emits_types_bool() {
        assert_eq!(NixType::Bool.emit(), "types.bool");
    }

    #[test]
    fn path_emits_types_path() {
        assert_eq!(NixType::Path.emit(), "types.path");
    }

    #[test]
    fn package_emits_types_package() {
        assert_eq!(NixType::Package.emit(), "types.package");
    }

    #[test]
    fn list_of_wraps_inner() {
        assert_eq!(NixType::list_of(NixType::Str).emit(), "types.listOf types.str");
    }

    #[test]
    fn list_of_complex_wraps_parens() {
        let ty = NixType::list_of(NixType::list_of(NixType::Str));
        assert_eq!(ty.emit(), "types.listOf (types.listOf types.str)");
    }

    #[test]
    fn attrs_of_wraps_inner() {
        assert_eq!(NixType::attrs_of(NixType::Str).emit(), "types.attrsOf types.str");
    }

    #[test]
    fn enum_emits_values() {
        let ty = NixType::enum_of(vec!["a", "b", "c"]);
        assert_eq!(ty.emit(), r#"types.enum [ "a" "b" "c" ]"#);
    }

    #[test]
    fn null_or_wraps_inner() {
        assert_eq!(NixType::null_or(NixType::Str).emit(), "types.nullOr types.str");
    }

    #[test]
    fn null_or_is_idempotent() {
        let once = NixType::null_or(NixType::Str);
        let twice = NixType::null_or(once.clone());
        assert_eq!(once, twice);
    }

    #[test]
    fn one_of_single_degenerates() {
        let ty = NixType::one_of(vec![NixType::Str]);
        assert_eq!(ty, NixType::Str);
    }

    #[test]
    fn one_of_two_becomes_either() {
        let ty = NixType::one_of(vec![NixType::Str, NixType::Int]);
        assert_eq!(ty.emit(), "types.either types.str types.int");
    }

    #[test]
    fn one_of_three_plus_becomes_one_of() {
        let ty = NixType::one_of(vec![NixType::Str, NixType::Int, NixType::Bool]);
        assert_eq!(ty.emit(), "types.oneOf [ types.str types.int types.bool ]");
    }

    #[test]
    #[should_panic(expected = "0 variants")]
    fn one_of_zero_panics() {
        let _ = NixType::one_of(vec![]);
    }

    #[test]
    fn anything_emits_types_anything() {
        assert_eq!(NixType::Anything.emit(), "types.anything");
    }

    #[test]
    fn submodule_emits_structure() {
        let ty = NixType::Submodule(vec![SubmoduleOption {
            name: "port".into(),
            option_type: NixType::Int,
            default: Some(NixNode::Int(8080)),
            description: Some("Service port".into()),
        }]);
        let out = ty.emit();
        assert!(out.contains("types.submodule"));
        assert!(out.contains("port"));
        assert!(out.contains("types.int"));
        assert!(out.contains("8080"));
    }

    #[test]
    fn raw_emits_verbatim() {
        let ty = NixType::Raw("types.functionTo types.str".into());
        assert_eq!(ty.emit(), "types.functionTo types.str");
    }
}
