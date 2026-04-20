//! `nix_typescape` — a typed model of the `pleme-io/nix` repository architecture.
//!
//! This is a ninth dimension of the pleme-io typescape family, complementary
//! to `arch-synthesizer`'s eight dimensions (vocabulary, domains, lattice,
//! stack, DAG, render, compliance, modules). It encodes the fleet of nodes,
//! the layered profile system, the blackmatter ecosystem, the WireGuard VPN
//! topology, the K3s cluster registry, the flake-input graph, the substrate
//! builder catalog, and the secret-path convention — each as Rust types with
//! structural invariants proven by unit tests and property-based proofs.
//!
//! The top-level abstraction is `NixTypescape` (the universe) and
//! `NixTypescapeSummary` (a snapshot suitable for display / attestation).
//! The canonical registry is available via `pleme_nix_registry()` in
//! `crate::typescape::registry`.

pub mod blackmatter;
pub mod cluster;
pub mod flake;
pub mod invariants;
pub mod node;
pub mod platform;
pub mod profile;
pub mod registry;
pub mod secret;
pub mod substrate_builder;
pub mod vpn;

use self::blackmatter::BlackmatterComponent;
use self::cluster::Cluster;
use self::flake::FlakeInput;
use self::node::Node;
use self::platform::identity_hash;
use self::profile::Profile;
use self::secret::{SecretBackend, SecretPath};
use self::substrate_builder::SubstrateBuilder;
use self::vpn::VpnLink;

/// The complete pleme-io nix architecture as typed data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NixTypescape {
    pub nodes: Vec<Node>,
    pub profiles: Vec<Profile>,
    pub blackmatter_components: Vec<BlackmatterComponent>,
    pub vpn_links: Vec<VpnLink>,
    pub clusters: Vec<Cluster>,
    pub flake_inputs: Vec<FlakeInput>,
    pub substrate_builders: Vec<SubstrateBuilder>,
    pub secrets: Vec<(String, SecretPath)>,
    pub default_secret_backend: SecretBackend,
}

impl NixTypescape {
    /// Empty typescape — useful for negative tests where we expect "required
    /// thing missing" invariants to fire.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            profiles: Vec::new(),
            blackmatter_components: Vec::new(),
            vpn_links: Vec::new(),
            clusters: Vec::new(),
            flake_inputs: Vec::new(),
            substrate_builders: Vec::new(),
            secrets: Vec::new(),
            default_secret_backend: SecretBackend::Sops,
        }
    }

    /// Count of `Darwin` nodes in the registry.
    #[must_use]
    pub fn darwin_node_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_darwin()).count()
    }

    /// Count of `NixOS` nodes in the registry.
    #[must_use]
    pub fn nixos_node_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_nixos()).count()
    }

    /// Count of foundation-layer profiles.
    #[must_use]
    pub fn foundation_profile_count(&self) -> usize {
        self.profiles.iter().filter(|p| p.is_foundation()).count()
    }

    /// Count of flake inputs owned by pleme-io.
    #[must_use]
    pub fn pleme_flake_input_count(&self) -> usize {
        self.flake_inputs.iter().filter(|f| f.is_pleme()).count()
    }

    /// Count of substrate builders.
    #[must_use]
    pub fn substrate_builder_count(&self) -> usize {
        self.substrate_builders.len()
    }

    /// Stable content identity hash.
    #[must_use]
    pub fn type_hash(&self) -> u64 {
        identity_hash(self)
    }

    /// Build the dimension summary for display / attestation.
    #[must_use]
    pub fn summary(&self) -> NixTypescapeSummary {
        let violations = self.all_violations();
        NixTypescapeSummary {
            node_count: self.nodes.len(),
            darwin_node_count: self.darwin_node_count(),
            nixos_node_count: self.nixos_node_count(),
            profile_count: self.profiles.len(),
            foundation_profile_count: self.foundation_profile_count(),
            blackmatter_component_count: self.blackmatter_components.len(),
            vpn_link_count: self.vpn_links.len(),
            cluster_count: self.clusters.len(),
            flake_input_count: self.flake_inputs.len(),
            pleme_flake_input_count: self.pleme_flake_input_count(),
            substrate_builder_count: self.substrate_builder_count(),
            secret_path_count: self.secrets.len(),
            type_hash: self.type_hash(),
            invariants_total: invariants::ALL_INVARIANTS.len(),
            violations_count: violations.len(),
            is_consistent: violations.is_empty(),
        }
    }
}

/// A compact summary of the typescape — suitable for logs, dashboards, attestation leaves.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NixTypescapeSummary {
    pub node_count: usize,
    pub darwin_node_count: usize,
    pub nixos_node_count: usize,
    pub profile_count: usize,
    pub foundation_profile_count: usize,
    pub blackmatter_component_count: usize,
    pub vpn_link_count: usize,
    pub cluster_count: usize,
    pub flake_input_count: usize,
    pub pleme_flake_input_count: usize,
    pub substrate_builder_count: usize,
    pub secret_path_count: usize,
    pub type_hash: u64,
    pub invariants_total: usize,
    pub violations_count: usize,
    pub is_consistent: bool,
}

impl std::fmt::Display for NixTypescapeSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "─────── NixTypescape Summary ───────")?;
        writeln!(f, "  type_hash         : {:#018x}", self.type_hash)?;
        writeln!(f, "  nodes             : {} ({} darwin, {} nixos)",
            self.node_count, self.darwin_node_count, self.nixos_node_count)?;
        writeln!(f, "  profiles          : {} ({} foundations)",
            self.profile_count, self.foundation_profile_count)?;
        writeln!(f, "  blackmatter       : {} components", self.blackmatter_component_count)?;
        writeln!(f, "  vpn links         : {}", self.vpn_link_count)?;
        writeln!(f, "  clusters          : {}", self.cluster_count)?;
        writeln!(f, "  flake inputs      : {} ({} pleme-owned)",
            self.flake_input_count, self.pleme_flake_input_count)?;
        writeln!(f, "  substrate builders: {}", self.substrate_builder_count)?;
        writeln!(f, "  secrets           : {} paths", self.secret_path_count)?;
        writeln!(f, "  invariants        : {} declared, {} violations",
            self.invariants_total, self.violations_count)?;
        writeln!(f, "  consistent        : {}", self.is_consistent)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_typescape_renders() {
        let t = NixTypescape::empty();
        let summary = t.summary();
        assert_eq!(summary.node_count, 0);
        assert_eq!(summary.invariants_total, invariants::ALL_INVARIANTS.len());
    }

    #[test]
    fn summary_displays_without_panic() {
        let t = registry::pleme_nix_registry();
        let _ = format!("{}", t.summary());
    }

    #[test]
    fn summary_type_hash_is_deterministic() {
        let a = registry::pleme_nix_registry();
        let b = registry::pleme_nix_registry();
        assert_eq!(a.summary().type_hash, b.summary().type_hash);
    }

    #[test]
    fn pleme_registry_darwin_count_is_two() {
        let t = registry::pleme_nix_registry();
        assert_eq!(t.darwin_node_count(), 2);
    }

    #[test]
    fn pleme_registry_has_at_least_two_foundations() {
        let t = registry::pleme_nix_registry();
        assert!(t.foundation_profile_count() >= 2);
    }

    #[test]
    fn pleme_registry_has_many_flake_inputs() {
        let t = registry::pleme_nix_registry();
        // Conservative lower bound — we encode a representative sample, not every input.
        assert!(t.flake_inputs.len() >= 25);
    }
}
