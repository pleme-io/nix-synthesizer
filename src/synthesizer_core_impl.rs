//! Conformance to [`synthesizer_core`] traits.
//!
//! Wave 3 of the compound-knowledge refactor: Raw variants have been
//! removed entirely. The no-raw invariant is now structural — invalid
//! states are unrepresentable at the type level.
//!
//! - [`synthesizer_core::SynthesizerNode`] unifies the emit contract so
//!   generic code can render any synthesizer's AST without knowing the
//!   concrete type.
//! - [`synthesizer_core::NoRawAttestation`] documents the structural
//!   absence of Raw variants, retained as a defensive guard against
//!   reintroduction via the scan test in
//!   `tests/synthesizer_core_conformance.rs`.

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
        }
    }
}

impl NoRawAttestation for NixNode {
    fn attestation() -> &'static str {
        "Raw variants were removed in Wave 3. NixNode and NixType are \
         structurally incapable of carrying arbitrary strings — invalid \
         states are unrepresentable at the type level. The source-scan \
         test in tests/synthesizer_core_conformance.rs::\
         no_raw_constructor_in_production_source is retained as a \
         defensive guard against reintroduction (now trivially satisfied)."
    }
}
