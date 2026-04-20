//! Negative-case tests for every structural invariant.
//!
//! Each invariant claims "X should hold". These tests verify each invariant
//! actually detects the violation of its claim by constructing a typescape
//! that deliberately breaks X and asserting the corresponding invariant
//! function emits ≥ 1 violation. Without these, a broken invariant
//! (always returning empty) would silently pass the canonical check.

use nix_synthesizer::typescape::{
    blackmatter::{BlackmatterComponent, ComponentRole},
    cluster::{Cluster, FluxAuth, K3sRole},
    flake::{FlakeInput, FlakeInputUrl, InputOrigin},
    invariants::{self, Violation},
    node::{Node, NodeRole},
    platform::{IpV4Cidr, Target, WireguardInterface},
    profile::{Profile, ProfileKind, ProfileLayer},
    registry::pleme_nix_registry,
    secret::SecretPath,
    substrate_builder::SubstrateBuilder,
    vpn::{SideName, VpnLink, VpnProfile, VpnSide},
    NixTypescape,
};

// Helper: clone the canonical registry so negative tests start from a
// known-good baseline, then mutate to break exactly one invariant.
fn base() -> NixTypescape {
    pleme_nix_registry()
}

fn has_violation(violations: &[Violation], id: &str) -> bool {
    violations.iter().any(|v| v.id.0 == id)
}

// ── node invariants ────────────────────────────────────────────────────────

#[test]
fn detects_duplicate_hostname() {
    let mut t = base();
    let plo = t.nodes.iter().find(|n| n.short_name == "plo").cloned().unwrap();
    let mut dup = plo.clone();
    dup.short_name = "plo-alias".into();
    t.nodes.push(dup);
    let v = invariants::node_hostnames_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_duplicate_short_name() {
    let mut t = base();
    let plo = t.nodes.iter().find(|n| n.short_name == "plo").cloned().unwrap();
    t.nodes.push(plo);
    let v = invariants::node_short_names_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_darwin_node_not_aarch64() {
    let mut t = base();
    // Force a darwin node to x86_64 → should fail.
    t.nodes.push(Node::new("old-mac", "old.local", Target::X86_64_DARWIN, NodeRole::DarwinWorkstation, "drzzln"));
    let v = invariants::darwin_nodes_use_aarch64(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_k3s_vm_without_managing_node() {
    let mut t = base();
    t.nodes.push(Node::new("wild-vm", "192.168.99.1", Target::AARCH64_LINUX, NodeRole::K3sVm, "root"));
    let v = invariants::k3s_vm_nodes_have_managing_node(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_k3s_vm_target_coherence_violation() {
    let mut t = base();
    // K3s VM forced to x86_64-linux instead of aarch64-linux.
    t.nodes.push(
        Node::new("bad-vm", "10.0.0.99", Target::X86_64_LINUX, NodeRole::K3sVm, "root")
            .with_managing_node("cid"),
    );
    let v = invariants::node_target_coherence(&t);
    assert!(!v.is_empty());
}

// ── profile invariants ─────────────────────────────────────────────────────

#[test]
fn detects_duplicate_profile_name() {
    let mut t = base();
    t.profiles.push(Profile::new("nixos-pleme-base", ProfileKind::NixOs, ProfileLayer::Foundation));
    let v = invariants::profile_names_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_specialization_without_foundation() {
    let mut t = base();
    t.profiles.push(Profile::new("rogue-spec", ProfileKind::NixOs, ProfileLayer::Specialization));
    let v = invariants::specialization_has_foundation(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_specialization_referencing_missing_foundation() {
    let mut t = base();
    t.profiles.push(
        Profile::new("rogue-spec", ProfileKind::NixOs, ProfileLayer::Specialization)
            .requiring("does-not-exist"),
    );
    let v = invariants::specialization_foundation_exists(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_missing_foundation_for_platform() {
    let mut t = base();
    // Remove all darwin foundations.
    t.profiles.retain(|p| !(p.is_foundation() && p.kind == ProfileKind::Darwin));
    let v = invariants::foundation_profile_per_platform_exists(&t);
    assert!(!v.is_empty());
}

// ── blackmatter invariants ─────────────────────────────────────────────────

#[test]
fn detects_duplicate_blackmatter_component() {
    let mut t = base();
    t.blackmatter_components.push(BlackmatterComponent::new(
        "secrets",
        "blackmatter-secrets-dup",
        ComponentRole::Infrastructure,
    ));
    let v = invariants::blackmatter_component_names_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_duplicate_blackmatter_repo() {
    let mut t = base();
    t.blackmatter_components.push(BlackmatterComponent::new(
        "secrets-alt",
        "blackmatter-secrets",
        ComponentRole::Infrastructure,
    ));
    let v = invariants::blackmatter_component_repo_names_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_missing_aggregator() {
    let mut t = base();
    t.blackmatter_components.retain(|c| c.role != ComponentRole::Aggregator);
    let v = invariants::blackmatter_aggregator_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_double_aggregator() {
    let mut t = base();
    t.blackmatter_components.push(
        BlackmatterComponent::new("extra-aggregator", "bm-dup", ComponentRole::Aggregator),
    );
    let v = invariants::blackmatter_aggregator_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_component_with_no_modules() {
    let mut t = base();
    t.blackmatter_components.push(
        BlackmatterComponent::new("silent", "blackmatter-silent", ComponentRole::Capability)
            .with_modules(false, false, false),
    );
    let v = invariants::blackmatter_component_has_at_least_one_module(&t);
    assert!(!v.is_empty());
}

// ── VPN invariants ─────────────────────────────────────────────────────────

fn bad_link(name: &str, subnet: &str, iface: &str, a_node: &str, b_node: &str) -> VpnLink {
    VpnLink {
        name: name.to_string(),
        profile: VpnProfile::K8sControlPlane,
        interface: WireguardInterface::new(iface).unwrap(),
        subnet: IpV4Cidr::parse(subnet).unwrap(),
        mtu: 1420,
        persistent_keepalive: None,
        side_a: VpnSide::initiator(a_node, "10.200.0.1", "x/y/z"),
        side_b: VpnSide::responder(b_node, "10.200.0.2", 51900, "host:51900", "p/q/r"),
        psk_on_side: SideName::A,
    }
}

#[test]
fn detects_duplicate_vpn_link_name() {
    let mut t = base();
    t.vpn_links.push(bad_link("ryn-k3s", "10.200.0.0/24", "wg-dup", "ryn", "x"));
    let v = invariants::vpn_link_names_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_duplicate_vpn_interface_name() {
    let mut t = base();
    t.vpn_links.push(bad_link("fresh", "10.200.0.0/24", "wg-ryn-k3s", "ryn", "x"));
    let v = invariants::vpn_interface_names_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_identical_vpn_sides() {
    let mut t = base();
    let mut link = bad_link("self-loop", "10.200.0.0/24", "wg-loop", "ryn", "x");
    link.side_b.node = "ryn".into();
    t.vpn_links.push(link);
    let v = invariants::vpn_sides_distinct(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_vpn_addresses_outside_subnet() {
    let mut t = base();
    let mut link = bad_link("wrong-subnet", "10.250.0.0/24", "wg-bad-sub", "ryn", "x");
    link.side_a.address = nix_synthesizer::typescape::platform::IpV4Address::parse("10.1.1.1").unwrap();
    t.vpn_links.push(link);
    let v = invariants::vpn_addresses_in_subnet(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_overlapping_vpn_subnets() {
    let mut t = base();
    // Overlaps with ryn-k3s (10.100.1.0/24).
    t.vpn_links.push(bad_link("overlap", "10.100.1.0/25", "wg-over", "ryn", "x"));
    let v = invariants::vpn_subnets_non_overlapping(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_both_sides_listening() {
    let mut t = base();
    let mut link = bad_link("two-listen", "10.200.0.0/24", "wg-2l", "ryn", "x");
    link.side_a.listen_port = Some(51900);
    t.vpn_links.push(link);
    let v = invariants::vpn_exactly_one_side_listens(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_responder_without_endpoint() {
    let mut t = base();
    let mut link = bad_link("no-endpoint", "10.200.0.0/24", "wg-ne", "ryn", "x");
    link.side_b.endpoint = None;
    t.vpn_links.push(link);
    let v = invariants::vpn_responder_has_endpoint(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_psk_on_responder() {
    let mut t = base();
    let mut link = bad_link("psk-b", "10.200.0.0/24", "wg-pskb", "ryn", "x");
    link.psk_on_side = SideName::B;
    t.vpn_links.push(link);
    let v = invariants::vpn_psk_on_initiator(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_vpn_initiator_missing_from_registry() {
    let mut t = base();
    t.vpn_links.push(bad_link("phantom", "10.200.0.0/24", "wg-phant", "ghost-node", "x"));
    let v = invariants::vpn_local_node_exists(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_vpn_subnet_not_slash_24() {
    let mut t = base();
    t.vpn_links.push(VpnLink {
        name: "slash16".into(),
        profile: VpnProfile::K8sControlPlane,
        interface: WireguardInterface::new("wg-16").unwrap(),
        subnet: IpV4Cidr::parse("10.200.0.0/16").unwrap(),
        mtu: 1420,
        persistent_keepalive: None,
        side_a: VpnSide::initiator("ryn", "10.200.0.1", "x"),
        side_b: VpnSide::responder("y", "10.200.0.2", 51900, "a:51900", "p"),
        psk_on_side: SideName::A,
    });
    let v = invariants::vpn_cidr_is_slash_24(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_internet_link_without_keepalive() {
    let mut t = base();
    t.vpn_links.push(VpnLink {
        name: "no-keepalive".into(),
        profile: VpnProfile::K8sControlPlane,
        interface: WireguardInterface::new("wg-nk").unwrap(),
        subnet: IpV4Cidr::parse("10.200.0.0/24").unwrap(),
        mtu: 1380, // internet-sized MTU
        persistent_keepalive: None, // missing!
        side_a: VpnSide::initiator("ryn", "10.200.0.1", "x"),
        side_b: VpnSide::responder("y", "10.200.0.2", 51900, "a:51900", "p"),
        psk_on_side: SideName::A,
    });
    let v = invariants::vpn_keepalive_set_for_internet_links(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_vpn_endpoint_malformed() {
    let mut t = base();
    let mut link = bad_link("bad-ep", "10.200.0.0/24", "wg-bep", "ryn", "x");
    link.side_b.endpoint = Some("not-a-port".into());
    t.vpn_links.push(link);
    let v = invariants::vpn_endpoint_format_valid(&t);
    assert!(!v.is_empty());
}

// ── cluster invariants ─────────────────────────────────────────────────────

#[test]
fn detects_duplicate_cluster_name() {
    let mut t = base();
    t.clusters.push(Cluster::new("plo", "plo", K3sRole::Server));
    let v = invariants::cluster_names_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_cluster_referencing_missing_node() {
    let mut t = base();
    t.clusters.push(Cluster::new("ghost-cluster", "ghost-node", K3sRole::Server));
    let v = invariants::cluster_node_exists(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_cluster_with_non_default_kubeconfig() {
    let mut t = base();
    let mut c = Cluster::new("nonstd", "plo", K3sRole::Server);
    c.kubeconfig_path = "/home/root/kubeconfig.yaml".to_string();
    t.clusters.push(c);
    let v = invariants::cluster_kubeconfig_convention(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_cluster_referencing_missing_vpn_link() {
    let mut t = base();
    let c = Cluster::new("orphan-vpn", "plo", K3sRole::Server)
        .with_vpn_links(&["missing-link"]);
    t.clusters.push(c);
    let v = invariants::cluster_vpn_link_exists(&t);
    assert!(!v.is_empty());
}

// ── flake input invariants ─────────────────────────────────────────────────

#[test]
fn detects_duplicate_flake_input() {
    let mut t = base();
    t.flake_inputs.push(FlakeInput::new(
        "nixpkgs",
        FlakeInputUrl::GitHub { org: "x".into(), repo: "y".into(), branch: None },
        InputOrigin::NixCommunity,
    ));
    let v = invariants::flake_input_names_unique(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_pleme_input_not_following_nixpkgs() {
    let mut t = base();
    t.flake_inputs.push(FlakeInput::new(
        "rogue-pleme",
        FlakeInputUrl::pleme_gh("rogue"),
        InputOrigin::PlemeIo,
    )); // no follows → should fail
    let v = invariants::pleme_inputs_follow_nixpkgs(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_missing_nixpkgs_input() {
    let mut t = base();
    t.flake_inputs.retain(|f| f.name != "nixpkgs");
    let v = invariants::nixpkgs_input_present(&t);
    assert!(!v.is_empty());
}

// ── substrate builder invariants ───────────────────────────────────────────

#[test]
fn detects_rust_tool_release_with_wrong_target_count() {
    let mut t = base();
    t.substrate_builders.push(SubstrateBuilder::RustToolRelease {
        tool_name: "short-release".into(),
        targets: vec![Target::AARCH64_DARWIN], // 1 target instead of 4
        has_hm_module: true,
    });
    let v = invariants::substrate_rust_release_has_four_targets(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_duplicate_substrate_builder_per_kind() {
    let mut t = base();
    t.substrate_builders.push(SubstrateBuilder::RustLibrary { crate_name: "irodori".into() });
    let v = invariants::substrate_builder_names_unique_per_kind(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_rust_service_without_nixos_module() {
    let mut t = base();
    t.substrate_builders.push(SubstrateBuilder::RustService {
        service_name: "bad-service".into(),
        has_hm_module: true,
        has_nixos_module: false,
    });
    let v = invariants::substrate_services_expose_nixos_module(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_rust_tool_image_with_empty_archs() {
    let mut t = base();
    t.substrate_builders.push(SubstrateBuilder::RustToolImage {
        tool_name: "hollow-image".into(),
        archs: vec![],
    });
    let v = invariants::substrate_rust_tool_image_archs_are_linux(&t);
    assert!(!v.is_empty());
}

// ── secret invariants ──────────────────────────────────────────────────────

#[test]
fn detects_duplicate_secret_path_per_node() {
    let mut t = base();
    let path = SecretPath::new("github/token").unwrap();
    t.secrets.push(("plo".into(), path.clone()));
    t.secrets.push(("plo".into(), path));
    let v = invariants::secret_paths_unique_per_node(&t);
    assert!(!v.is_empty());
}

#[test]
fn detects_secret_referencing_missing_node() {
    let mut t = base();
    t.secrets.push((
        "phantom-node".into(),
        SecretPath::new("phantom/token").unwrap(),
    ));
    let v = invariants::secret_node_reference_exists_or_is_shared(&t);
    assert!(!v.is_empty());
}

#[test]
fn shared_node_secrets_are_allowed() {
    let mut t = base();
    t.secrets.push(("shared".into(), SecretPath::new("fleet/token").unwrap()));
    let v = invariants::secret_node_reference_exists_or_is_shared(&t);
    assert!(v.is_empty());
}

// ── cross-invariant: canonical registry fails nothing ──────────────────────

#[test]
fn canonical_registry_passes_every_invariant() {
    let t = base();
    let all = t.all_violations();
    assert!(all.is_empty(), "canonical registry violates: {all:?}");
}

#[test]
fn empty_typescape_triggers_expected_set_of_violations() {
    let t = NixTypescape::empty();
    let v = t.all_violations();
    // These "existence" invariants must fire on the empty typescape.
    assert!(has_violation(&v, "nixpkgs_input_present"));
    assert!(has_violation(&v, "blackmatter_aggregator_unique"));
    assert!(has_violation(&v, "foundation_profile_per_platform_exists"));
    // But "uniqueness" invariants don't fire on empty inputs.
    assert!(!has_violation(&v, "node_hostnames_unique"));
    assert!(!has_violation(&v, "vpn_link_names_unique"));
    assert!(!has_violation(&v, "profile_names_unique"));
}

// ── mutation symmetry: adding + removing a violation is idempotent ─────────

#[test]
fn adding_and_reverting_violation_returns_to_consistency() {
    let original = base();
    let original_violations = original.all_violations().len();

    let mut mutated = original.clone();
    mutated.nodes.push(Node::new(
        "dup-plo",
        "plo.quero.lan", // duplicate hostname
        Target::X86_64_LINUX,
        NodeRole::K3sServer,
        "root",
    ));
    let mutated_violations = mutated.all_violations().len();

    assert!(mutated_violations > original_violations);

    // Remove the mutation.
    mutated.nodes.pop();
    let reverted_violations = mutated.all_violations().len();
    assert_eq!(reverted_violations, original_violations);
}

// ── determinism across mutations ───────────────────────────────────────────

#[test]
fn type_hash_changes_with_content() {
    let a = base();
    let mut b = base();
    b.nodes.pop();
    assert_ne!(a.type_hash(), b.type_hash());
}

#[test]
fn type_hash_stable_across_construction() {
    assert_eq!(base().type_hash(), base().type_hash());
}
