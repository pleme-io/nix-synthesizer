//! substrate builder catalog — every Nix build pattern substrate exposes.
//!
//! This is an exhaustive enum of every `lib/build/**` builder in substrate.
//! Downstream code can encode a repo's flake.nix as a single variant and the
//! invariants will verify the chosen variant is being used correctly.

use super::platform::{Architecture, Target};

/// A substrate builder — one per `lib/build/*/{tool,library,service,...}-flake.nix`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubstrateBuilder {
    // ── Rust ────────────────────────────────────────────────────────────
    RustToolRelease {
        tool_name: String,
        targets: Vec<Target>,
        has_hm_module: bool,
    },
    RustWorkspaceRelease {
        tool_name: String,
        package_name: String,
        targets: Vec<Target>,
        has_hm_module: bool,
    },
    RustToolImage {
        tool_name: String,
        archs: Vec<Architecture>,
    },
    RustService {
        service_name: String,
        has_hm_module: bool,
        has_nixos_module: bool,
    },
    RustLibrary {
        crate_name: String,
    },
    LeptosBuild {
        app_name: String,
        port: u16,
    },

    // ── Go ──────────────────────────────────────────────────────────────
    GoTool {
        pname: String,
        has_version_ldflags: bool,
        completions: bool,
    },
    GoMonorepoSource {
        owner: String,
        repo: String,
    },
    GoMonorepoBinary {
        pname: String,
        sub_packages: Vec<String>,
    },

    // ── TypeScript ──────────────────────────────────────────────────────
    TypescriptTool {
        tool_name: String,
        needs_pleme_linker: bool,
    },
    TypescriptLibrary {
        name: String,
    },

    // ── Ruby ────────────────────────────────────────────────────────────
    RubyGem {
        name: String,
    },

    // ── Zig ─────────────────────────────────────────────────────────────
    ZigToolRelease {
        tool_name: String,
        targets: Vec<Target>,
    },

    // ── WASM ────────────────────────────────────────────────────────────
    WasiService {
        service_name: String,
        capabilities: Vec<String>,
    },

    // ── NixOS ───────────────────────────────────────────────────────────
    NixOsAmiBuild {
        ami_name: String,
    },
}

impl SubstrateBuilder {
    #[must_use]
    pub fn kind(&self) -> BuilderKind {
        match self {
            Self::RustToolRelease { .. } => BuilderKind::RustToolRelease,
            Self::RustWorkspaceRelease { .. } => BuilderKind::RustWorkspaceRelease,
            Self::RustToolImage { .. } => BuilderKind::RustToolImage,
            Self::RustService { .. } => BuilderKind::RustService,
            Self::RustLibrary { .. } => BuilderKind::RustLibrary,
            Self::LeptosBuild { .. } => BuilderKind::LeptosBuild,
            Self::GoTool { .. } => BuilderKind::GoTool,
            Self::GoMonorepoSource { .. } => BuilderKind::GoMonorepoSource,
            Self::GoMonorepoBinary { .. } => BuilderKind::GoMonorepoBinary,
            Self::TypescriptTool { .. } => BuilderKind::TypescriptTool,
            Self::TypescriptLibrary { .. } => BuilderKind::TypescriptLibrary,
            Self::RubyGem { .. } => BuilderKind::RubyGem,
            Self::ZigToolRelease { .. } => BuilderKind::ZigToolRelease,
            Self::WasiService { .. } => BuilderKind::WasiService,
            Self::NixOsAmiBuild { .. } => BuilderKind::NixOsAmiBuild,
        }
    }

    #[must_use]
    pub fn target_count(&self) -> Option<usize> {
        match self {
            Self::RustToolRelease { targets, .. }
            | Self::RustWorkspaceRelease { targets, .. }
            | Self::ZigToolRelease { targets, .. } => Some(targets.len()),
            Self::RustToolImage { archs, .. } => Some(archs.len()),
            _ => None,
        }
    }

    /// True iff this builder produces a home-manager module output.
    #[must_use]
    pub fn produces_hm_module(&self) -> bool {
        match self {
            Self::RustToolRelease { has_hm_module, .. }
            | Self::RustWorkspaceRelease { has_hm_module, .. } => *has_hm_module,
            Self::RustService { has_hm_module, .. } => *has_hm_module,
            _ => false,
        }
    }
}

/// Builder kinds without their payload — useful for category-level invariants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuilderKind {
    RustToolRelease,
    RustWorkspaceRelease,
    RustToolImage,
    RustService,
    RustLibrary,
    LeptosBuild,
    GoTool,
    GoMonorepoSource,
    GoMonorepoBinary,
    TypescriptTool,
    TypescriptLibrary,
    RubyGem,
    ZigToolRelease,
    WasiService,
    NixOsAmiBuild,
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_tool_release_has_four_targets_by_convention() {
        let b = SubstrateBuilder::RustToolRelease {
            tool_name: "tobira".to_string(),
            targets: Target::all_canonical().to_vec(),
            has_hm_module: true,
        };
        assert_eq!(b.target_count(), Some(4));
        assert_eq!(b.kind(), BuilderKind::RustToolRelease);
        assert!(b.produces_hm_module());
    }

    #[test]
    fn rust_library_has_no_target_count() {
        let b = SubstrateBuilder::RustLibrary { crate_name: "irodori".to_string() };
        assert_eq!(b.target_count(), None);
    }

    #[test]
    fn rust_tool_image_counts_architectures() {
        let b = SubstrateBuilder::RustToolImage {
            tool_name: "hanabi".to_string(),
            archs: vec![Architecture::Aarch64, Architecture::X86_64],
        };
        assert_eq!(b.target_count(), Some(2));
    }

    #[test]
    fn go_builders_have_distinct_kinds() {
        let t = SubstrateBuilder::GoTool { pname: "kubectl".to_string(), has_version_ldflags: true, completions: true };
        let s = SubstrateBuilder::GoMonorepoSource { owner: "kubernetes".to_string(), repo: "kubernetes".to_string() };
        let b = SubstrateBuilder::GoMonorepoBinary { pname: "kubectl".to_string(), sub_packages: vec!["cmd/kubectl".to_string()] };
        assert_ne!(t.kind(), s.kind());
        assert_ne!(s.kind(), b.kind());
    }

    #[test]
    fn builder_kind_copy_and_eq() {
        let k = BuilderKind::RustLibrary;
        assert_eq!(k, k);
    }
}
