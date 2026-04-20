//! Blackmatter ecosystem catalog — the components, repos, and aggregation graph.

use super::platform::Platform;

/// A blackmatter component — exposes a `blackmatter.components.<name>` option namespace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlackmatterComponent {
    pub name: String,
    pub repo: String,
    pub option_namespace: String,
    pub provides_hm: bool,
    pub provides_nixos: bool,
    pub provides_darwin: bool,
    pub platforms: Vec<Platform>,
    pub exposes_overlay: bool,
    pub role: ComponentRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentRole {
    /// The master aggregator that re-exports every other module.
    Aggregator,
    /// A user-facing capability module (shell, nvim, desktop…).
    Capability,
    /// Infrastructure abstraction (secrets, kubernetes, services, vpn…).
    Infrastructure,
    /// Developer tooling integration (claude, cursor, opencode, anvil…).
    DevTool,
    /// Security primitives.
    Security,
    /// Organization-specific conventions (pleme).
    Org,
}

impl BlackmatterComponent {
    #[must_use]
    pub fn new(name: &str, repo: &str, role: ComponentRole) -> Self {
        Self {
            name: name.to_string(),
            repo: repo.to_string(),
            option_namespace: format!("blackmatter.components.{name}"),
            provides_hm: true,
            provides_nixos: false,
            provides_darwin: false,
            platforms: vec![Platform::Linux, Platform::Darwin],
            exposes_overlay: true,
            role,
        }
    }

    #[must_use]
    pub fn with_namespace(mut self, ns: &str) -> Self {
        self.option_namespace = ns.to_string();
        self
    }

    #[must_use]
    pub fn with_modules(mut self, hm: bool, nixos: bool, darwin: bool) -> Self {
        self.provides_hm = hm;
        self.provides_nixos = nixos;
        self.provides_darwin = darwin;
        self
    }

    #[must_use]
    pub fn with_platforms(mut self, platforms: &[Platform]) -> Self {
        self.platforms = platforms.to_vec();
        self
    }

    #[must_use]
    pub fn with_overlay(mut self, exposes: bool) -> Self {
        self.exposes_overlay = exposes;
        self
    }

    /// True if the component provides at least one of HM / NixOS / Darwin modules.
    #[must_use]
    pub fn provides_any_module(&self) -> bool {
        self.provides_hm || self.provides_nixos || self.provides_darwin
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_namespace_uses_component_name() {
        let c = BlackmatterComponent::new("secrets", "blackmatter-secrets", ComponentRole::Infrastructure);
        assert_eq!(c.option_namespace, "blackmatter.components.secrets");
    }

    #[test]
    fn every_component_provides_hm_by_default() {
        let c = BlackmatterComponent::new("shell", "blackmatter-shell", ComponentRole::Capability);
        assert!(c.provides_hm);
        assert!(c.provides_any_module());
    }

    #[test]
    fn component_can_be_both_hm_and_nixos() {
        let c = BlackmatterComponent::new("kubernetes", "blackmatter-kubernetes", ComponentRole::Infrastructure)
            .with_modules(true, true, false);
        assert!(c.provides_hm);
        assert!(c.provides_nixos);
        assert!(!c.provides_darwin);
    }
}
