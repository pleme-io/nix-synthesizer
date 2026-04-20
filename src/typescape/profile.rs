//! Profile registry: the layered presets nodes compose to assemble their configuration.
//!
//! Profiles are how the "blackmatter modules → profiles → nodes" stack is realized.
//! The key structural rule: every `Specialization` profile requires a `Foundation`
//! profile, and every `Foundation` profile is usable on its own. `Standalone`
//! profiles stack orthogonally with any foundation.

/// A stackable profile — aggregates blackmatter components + opinionated defaults.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Profile {
    pub name: String,
    pub kind: ProfileKind,
    pub layer: ProfileLayer,
    /// If `Specialization`, the foundation profile it requires as a base.
    pub requires_foundation: Option<String>,
    /// Blackmatter components this profile enables.
    pub enables_components: Vec<String>,
    /// Variant string set by the profile (e.g. `blizzard.variant = "server"`).
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileKind {
    /// macOS-only (darwin-rebuild).
    Darwin,
    /// NixOS-only (nixos-rebuild).
    NixOs,
    /// Kindling profile consumable from either platform.
    Kindling,
}

/// Where a profile sits in the layering order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileLayer {
    /// Base layer — exactly one per node. Provides the foundation every other layer
    /// assumes (user, shell, secrets, base HM modules, …).
    Foundation,
    /// Adds a specific concern on top of a foundation (e.g. k3s-server, laptop-server).
    /// Must stack on a foundation.
    Specialization,
    /// Stackable orthogonally with any foundation (e.g. security-hardened).
    Standalone,
}

impl Profile {
    pub fn new(name: &str, kind: ProfileKind, layer: ProfileLayer) -> Self {
        Self {
            name: name.to_string(),
            kind,
            layer,
            requires_foundation: None,
            enables_components: Vec::new(),
            variant: None,
        }
    }

    #[must_use]
    pub fn requiring(mut self, foundation: &str) -> Self {
        self.requires_foundation = Some(foundation.to_string());
        self
    }

    #[must_use]
    pub fn enabling(mut self, components: &[&str]) -> Self {
        self.enables_components = components.iter().map(|s| (*s).to_string()).collect();
        self
    }

    #[must_use]
    pub fn with_variant(mut self, variant: &str) -> Self {
        self.variant = Some(variant.to_string());
        self
    }

    #[must_use]
    pub fn is_foundation(&self) -> bool {
        self.layer == ProfileLayer::Foundation
    }

    #[must_use]
    pub fn is_specialization(&self) -> bool {
        self.layer == ProfileLayer::Specialization
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundation_profile_has_no_requirement() {
        let p = Profile::new("nixos-pleme-base", ProfileKind::NixOs, ProfileLayer::Foundation);
        assert!(p.is_foundation());
        assert_eq!(p.requires_foundation, None);
    }

    #[test]
    fn specialization_profile_can_require_foundation() {
        let p = Profile::new("nixos-k3s-server", ProfileKind::NixOs, ProfileLayer::Specialization)
            .requiring("nixos-pleme-base")
            .with_variant("server");
        assert!(p.is_specialization());
        assert_eq!(p.requires_foundation.as_deref(), Some("nixos-pleme-base"));
        assert_eq!(p.variant.as_deref(), Some("server"));
    }

    #[test]
    fn standalone_profile_stacks_orthogonally() {
        let p = Profile::new("nixos-security-hardened", ProfileKind::NixOs, ProfileLayer::Standalone);
        assert!(!p.is_foundation());
        assert!(!p.is_specialization());
        assert_eq!(p.layer, ProfileLayer::Standalone);
    }

    #[test]
    fn profile_enabling_accumulates() {
        let p = Profile::new("darwin-developer", ProfileKind::Darwin, ProfileLayer::Foundation)
            .enabling(&["blackmatter", "akeyless", "zoekt-mcp"]);
        assert_eq!(p.enables_components.len(), 3);
    }
}
