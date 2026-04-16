//! Conformance to [`synthesizer_core`] traits.
//!
//! Wave 2 of the compound-knowledge refactor: purely additive. No behavior
//! change to nix-synthesizer's existing APIs — this module only adds trait
//! impls that downstream generic code can consume.
//!
//! - [`synthesizer_core::SynthesizerNode`] unifies the emit contract so
//!   generic code can render any synthesizer's AST without knowing the
//!   concrete type.
//! - [`synthesizer_core::NoRawAttestation`] documents how the no-raw
//!   invariant is enforced here (the `#[deprecated]` attribute on
//!   `NixNode::Raw` plus the scan test in
//!   `tests/synthesizer_core_conformance.rs`).

use crate::node::NixNode;
use synthesizer_core::{NoRawAttestation, SynthesizerNode};

impl SynthesizerNode for NixNode {
    fn emit(&self, indent: usize) -> String {
        // Delegate to the inherent `NixNode::emit` method — unambiguous
        // because inherent methods take priority over trait methods in
        // UFCS path lookup.
        NixNode::emit(self, indent)
    }

    fn indent_unit() -> &'static str {
        "  "
    }

    fn variant_id(&self) -> u8 {
        match self {
            Self::Comment(_) => 0,
            Self::Blank => 1,
            Self::Str(_) => 2,
            Self::MultilineStr(_) => 3,
            Self::Int(_) => 4,
            Self::Bool(_) => 5,
            Self::Null => 6,
            Self::Path(_) => 7,
            Self::Ident(_) => 8,
            Self::Select { .. } => 9,
            Self::SelectOr { .. } => 10,
            Self::AttrSet(_) => 11,
            Self::RecAttrSet(_) => 12,
            Self::List(_) => 13,
            Self::LetIn { .. } => 14,
            Self::With { .. } => 15,
            Self::Inherit(_) => 16,
            Self::InheritFrom { .. } => 17,
            Self::Function { .. } => 18,
            Self::Lambda { .. } => 19,
            Self::Apply { .. } => 20,
            Self::If { .. } => 21,
            Self::BinOp { .. } => 22,
            Self::Interpolation { .. } => 23,
            Self::Import(_) => 24,
            Self::MkOption { .. } => 25,
            Self::MkEnableOption(_) => 26,
            Self::ModuleFile { .. } => 27,
            Self::FlakeFile { .. } => 28,
            Self::FlakeInput { .. } => 29,
            Self::WriteShellApp { .. } => 30,
            Self::TypeExpr(_) => 31,
            #[allow(deprecated)]
            Self::Raw(_) => 32,
        }
    }
}

impl NoRawAttestation for NixNode {
    fn attestation() -> &'static str {
        "NixNode::Raw carries #[deprecated] in src/node.rs and is scheduled \
         for removal in Wave 3 of the compound-knowledge refactor. \
         tests/synthesizer_core_conformance.rs::no_raw_constructor_in_production_source \
         scans src/ for Raw constructions; any accidental reintroduction \
         fails CI. The #[allow(deprecated)] pin in synthesizer_core_impl.rs \
         is the one intentional reference — to a match arm pattern, not a \
         construction."
    }
}
