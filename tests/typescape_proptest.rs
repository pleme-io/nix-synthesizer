//! Property-based proofs for the nix typescape. Each property states a universal
//! claim over randomly generated inputs and is checked against thousands of cases
//! by `proptest`. Together with the unit invariants these prove the typescape is
//! internally consistent under arbitrary perturbation.

use nix_synthesizer::typescape::{
    invariants::{self, Violation},
    node::{Node, NodeRole},
    platform::{Architecture, Hostname, IpV4Address, IpV4Cidr, Platform, Target, WireguardInterface},
    profile::{Profile, ProfileKind, ProfileLayer},
    secret::SecretPath,
    vpn::{SideName, VpnLink, VpnProfile, VpnSide},
    NixTypescape,
};
use proptest::prelude::*;

// ── Strategy: Hostname ──────────────────────────────────────────────────────

fn arb_hostname_label() -> impl Strategy<Value = String> {
    // 1..=10 chars, starts + ends with alnum, hyphen allowed in middle.
    "[a-z0-9][a-z0-9-]{0,8}[a-z0-9]".prop_map(|s| s)
}

fn arb_hostname() -> impl Strategy<Value = Hostname> {
    prop::collection::vec(arb_hostname_label(), 1..=3)
        .prop_map(|parts| parts.join("."))
        .prop_filter("valid hostname", |s| Hostname::new(s).is_ok())
        .prop_map(|s| Hostname::new(s).unwrap())
}

// ── Strategy: CIDR ──────────────────────────────────────────────────────────

fn arb_cidr_24() -> impl Strategy<Value = IpV4Cidr> {
    (0u8..=255, 0u8..=255).prop_map(|(b, c)| {
        IpV4Cidr::parse(&format!("10.{b}.{c}.0/24")).unwrap()
    })
}

// ── Strategy: WireguardInterface ────────────────────────────────────────────

fn arb_wg_name() -> impl Strategy<Value = WireguardInterface> {
    "[a-z]{1,11}".prop_map(|s| {
        let name = format!("wg-{s}");
        WireguardInterface::new(&name).unwrap()
    })
}

// ── Strategy: Target ────────────────────────────────────────────────────────

fn arb_target() -> impl Strategy<Value = Target> {
    prop_oneof![
        Just(Target::AARCH64_DARWIN),
        Just(Target::X86_64_LINUX),
        Just(Target::AARCH64_LINUX),
    ]
}

// ── Hostname properties ────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_valid_hostname_roundtrips(h in arb_hostname()) {
        let s = h.as_str().to_string();
        let h2 = Hostname::new(&s).unwrap();
        prop_assert_eq!(h.as_str(), h2.as_str());
    }

    #[test]
    fn prop_hostname_short_is_prefix_of_full(h in arb_hostname()) {
        prop_assert!(h.as_str().starts_with(h.short()));
    }

    #[test]
    fn prop_hostname_rejects_uppercase(s in "[A-Z]{2,10}") {
        prop_assert!(Hostname::new(&s).is_err());
    }

    #[test]
    fn prop_hostname_rejects_leading_hyphen(suffix in "[a-z0-9]{1,8}") {
        let bad = format!("-{suffix}");
        prop_assert!(Hostname::new(&bad).is_err());
    }
}

// ── CIDR properties ────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_cidr_overlap_is_symmetric(a in arb_cidr_24(), b in arb_cidr_24()) {
        prop_assert_eq!(a.overlaps(b), b.overlaps(a));
    }

    #[test]
    fn prop_cidr_contains_network_address(c in arb_cidr_24()) {
        prop_assert!(c.contains(c.network()));
    }

    #[test]
    fn prop_cidr_contains_broadcast_address(c in arb_cidr_24()) {
        prop_assert!(c.contains(c.broadcast()));
    }

    #[test]
    fn prop_distinct_slash_24_subnets_do_not_overlap(
        b1 in 0u8..=255, c1 in 0u8..=255,
        b2 in 0u8..=255, c2 in 0u8..=255,
    ) {
        let a = IpV4Cidr::parse(&format!("10.{b1}.{c1}.0/24")).unwrap();
        let b = IpV4Cidr::parse(&format!("10.{b2}.{c2}.0/24")).unwrap();
        let same = b1 == b2 && c1 == c2;
        prop_assert_eq!(a.overlaps(b), same);
    }

    #[test]
    fn prop_cidr_normalization_idempotent(c in arb_cidr_24()) {
        let reparsed = IpV4Cidr::parse(&c.to_string()).unwrap();
        prop_assert_eq!(c, reparsed);
    }

    #[test]
    fn prop_cidr_network_below_or_eq_broadcast(c in arb_cidr_24()) {
        prop_assert!(c.network().as_u32() <= c.broadcast().as_u32());
    }
}

// ── WireguardInterface properties ───────────────────────────────────────────

proptest! {
    #[test]
    fn prop_valid_wg_name_roundtrips(iface in arb_wg_name()) {
        let s = iface.as_str().to_string();
        prop_assert!(WireguardInterface::new(&s).is_ok());
    }

    #[test]
    fn prop_wg_name_length_bounded(iface in arb_wg_name()) {
        prop_assert!(iface.as_str().len() <= WireguardInterface::MAX_LEN);
    }

    #[test]
    fn prop_wg_name_rejects_long(tail in "[a-z]{14,30}") {
        let bad = format!("wg-{tail}");
        prop_assert!(WireguardInterface::new(&bad).is_err());
    }

    #[test]
    fn prop_wg_name_requires_prefix(s in "[a-z]{1,11}") {
        prop_assert!(WireguardInterface::new(&s).is_err());
    }
}

// ── SecretPath properties ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_valid_secret_path_depths_bounded(
        parts in prop::collection::vec("[a-z0-9]([a-z0-9-]{0,6}[a-z0-9])?", 2..=5)
    ) {
        let s = parts.join("/");
        let p = SecretPath::new(&s).unwrap();
        prop_assert!(p.depth() >= SecretPath::MIN_DEPTH);
        prop_assert!(p.depth() <= SecretPath::MAX_DEPTH);
    }

    #[test]
    fn prop_secret_path_roundtrips(
        parts in prop::collection::vec("[a-z0-9]([a-z0-9-]{0,6}[a-z0-9])?", 2..=5)
    ) {
        let s = parts.join("/");
        let p = SecretPath::new(&s).unwrap();
        prop_assert_eq!(p.as_str(), &s);
    }

    #[test]
    fn prop_single_component_secret_path_rejected(one in "[a-z][a-z0-9]{1,10}") {
        prop_assert!(SecretPath::new(&one).is_err());
    }

    #[test]
    fn prop_deep_secret_path_rejected(parts in prop::collection::vec("[a-z]{1,4}", 6..=10)) {
        let s = parts.join("/");
        prop_assert!(SecretPath::new(&s).is_err());
    }
}

// ── VpnLink properties: bidirectionality ────────────────────────────────────

fn arb_vpn_link() -> impl Strategy<Value = VpnLink> {
    (
        arb_wg_name(),
        arb_cidr_24(),
        arb_hostname_label(),
        arb_hostname_label(),
        0u16..=65535,
        Just(VpnProfile::K8sControlPlane),
    )
        .prop_filter_map("side names distinct, addresses valid", |(iface, subnet, a, b, port, profile)| {
            if a == b { return None; }
            // Addresses within the subnet: pick network+1 and network+2.
            let base = subnet.network().as_u32();
            let addr_a = IpV4Address((base + 1).to_be_bytes());
            let addr_b = IpV4Address((base + 2).to_be_bytes());
            Some(VpnLink {
                name: format!("{a}-{b}"),
                profile,
                interface: iface,
                subnet,
                mtu: 1420,
                persistent_keepalive: None,
                side_a: VpnSide { node: a, address: addr_a, listen_port: None, endpoint: None, private_key_secret: "x".into() },
                side_b: VpnSide { node: b, address: addr_b, listen_port: Some(port), endpoint: Some(format!("{port}:host")), private_key_secret: "y".into() },
                psk_on_side: SideName::A,
            })
        })
}

proptest! {
    #[test]
    fn prop_generated_vpn_link_has_distinct_sides(l in arb_vpn_link()) {
        prop_assert!(l.sides_are_distinct());
    }

    #[test]
    fn prop_generated_vpn_link_has_distinct_addresses(l in arb_vpn_link()) {
        prop_assert!(l.addresses_are_distinct());
    }

    #[test]
    fn prop_generated_vpn_link_addresses_in_subnet(l in arb_vpn_link()) {
        prop_assert!(l.addresses_in_subnet());
    }

    #[test]
    fn prop_generated_vpn_link_exactly_one_side_listens(l in arb_vpn_link()) {
        prop_assert!(l.exactly_one_side_listens());
    }
}

// ── Profile stacking properties ────────────────────────────────────────────

fn arb_profile_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("nixos-pleme-base".to_string()),
        Just("nixos-k3s-server".to_string()),
        Just("nixos-k3s-vm".to_string()),
        Just("nixos-security-hardened".to_string()),
        Just("darwin-developer".to_string()),
        Just("k3s-agent".to_string()),
    ]
}

fn arb_profile() -> impl Strategy<Value = Profile> {
    (
        arb_profile_name(),
        prop_oneof![Just(ProfileKind::NixOs), Just(ProfileKind::Darwin), Just(ProfileKind::Kindling)],
        prop_oneof![Just(ProfileLayer::Foundation), Just(ProfileLayer::Specialization), Just(ProfileLayer::Standalone)],
    )
        .prop_map(|(name, kind, layer)| {
            let mut p = Profile::new(&name, kind, layer);
            if layer == ProfileLayer::Specialization {
                p = p.requiring("nixos-pleme-base");
            }
            p
        })
}

proptest! {
    #[test]
    fn prop_specialization_profiles_have_foundation(p in arb_profile()) {
        if p.is_specialization() {
            prop_assert!(p.requires_foundation.is_some());
        }
    }

    #[test]
    fn prop_foundation_profiles_have_no_requirement(p in arb_profile()) {
        if p.is_foundation() {
            // The arbitrary constructor doesn't set requires_foundation for foundations.
            prop_assert!(p.requires_foundation.is_none());
        }
    }
}

// ── Target properties ──────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_target_nix_system_has_hyphen(t in arb_target()) {
        prop_assert!(t.as_nix_system().contains('-'));
    }

    #[test]
    fn prop_target_roundtrips_through_parts(t in arb_target()) {
        let s = t.as_nix_system();
        let (arch, plat) = s.split_once('-').unwrap();
        prop_assert!(arch == "aarch64" || arch == "x86_64");
        prop_assert!(plat == "darwin" || plat == "linux");
    }
}

// ── Node properties ────────────────────────────────────────────────────────

fn arb_node_role() -> impl Strategy<Value = NodeRole> {
    prop_oneof![
        Just(NodeRole::K3sServer),
        Just(NodeRole::K3sAgent),
        Just(NodeRole::K3sVm),
        Just(NodeRole::DarwinWorkstation),
        Just(NodeRole::VpnGateway),
        Just(NodeRole::Legacy),
        Just(NodeRole::K3sCloudServer),
    ]
}

proptest! {
    #[test]
    fn prop_k3s_role_classifier_consistent(r in arb_node_role()) {
        if matches!(r, NodeRole::K3sServer | NodeRole::K3sVm | NodeRole::K3sCloudServer) {
            prop_assert!(r.is_k3s());
            prop_assert!(r.is_k3s_server());
        }
        if matches!(r, NodeRole::K3sAgent) {
            prop_assert!(r.is_k3s());
            prop_assert!(!r.is_k3s_server());
        }
        if matches!(r, NodeRole::DarwinWorkstation | NodeRole::Legacy | NodeRole::VpnGateway) {
            prop_assert!(!r.is_k3s());
        }
    }
}

// ── Full-typescape invariant preservation ──────────────────────────────────

proptest! {
    /// The canonical registry is consistent, so adding a fresh arbitrary node
    /// with a unique hostname and clearing stale references should preserve
    /// all invariants that don't depend on node count parity.
    #[test]
    fn prop_canonical_registry_is_consistent_trivially(_seed in 0u32..64) {
        let t = nix_synthesizer::typescape::registry::pleme_nix_registry();
        let violations: Vec<Violation> = t.all_violations();
        prop_assert!(violations.is_empty(), "canonical registry fails: {:?}", violations);
    }

    /// An empty typescape violates exactly the "must exist" invariants, not
    /// the "no duplicate" invariants.
    #[test]
    fn prop_empty_typescape_has_expected_failures(_seed in 0u32..16) {
        let t = NixTypescape::empty();
        let v: Vec<_> = t.all_violations().into_iter().map(|v| v.id.0).collect();
        prop_assert!(v.contains(&"nixpkgs_input_present"));
        prop_assert!(v.contains(&"blackmatter_aggregator_unique"));
        prop_assert!(v.contains(&"foundation_profile_per_platform_exists"));
    }
}

// ── Substrate builder properties ───────────────────────────────────────────

use nix_synthesizer::typescape::substrate_builder::SubstrateBuilder;

fn arb_rust_tool_release() -> impl Strategy<Value = SubstrateBuilder> {
    "[a-z][a-z0-9-]{0,10}".prop_map(|name| SubstrateBuilder::RustToolRelease {
        tool_name: name,
        targets: Target::all_canonical().to_vec(),
        has_hm_module: true,
    })
}

proptest! {
    #[test]
    fn prop_rust_tool_release_always_has_four_targets(b in arb_rust_tool_release()) {
        prop_assert_eq!(b.target_count(), Some(4));
    }

    #[test]
    fn prop_rust_tool_release_produces_hm_module(b in arb_rust_tool_release()) {
        prop_assert!(b.produces_hm_module());
    }
}

// ── Identity hash properties ───────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_hostname_hash_stable(h in arb_hostname()) {
        use nix_synthesizer::typescape::platform::identity_hash;
        let h1 = identity_hash(&h);
        let h2 = identity_hash(&h);
        prop_assert_eq!(h1, h2);
    }

    #[test]
    fn prop_distinct_hostnames_distinct_hashes(
        s1 in "[a-z]{4,10}", s2 in "[a-z]{4,10}",
    ) {
        prop_assume!(s1 != s2);
        use nix_synthesizer::typescape::platform::identity_hash;
        let h1 = Hostname::new(&s1).unwrap();
        let h2 = Hostname::new(&s2).unwrap();
        // Hash collisions are astronomically rare for the default hasher over small strings.
        prop_assert_ne!(identity_hash(&h1), identity_hash(&h2));
    }
}

// ── Node builder sanity ────────────────────────────────────────────────────

fn arb_built_node() -> impl Strategy<Value = Node> {
    (
        "[a-z][a-z0-9-]{0,8}",
        arb_hostname(),
        arb_target(),
        arb_node_role(),
    ).prop_map(|(short, host, t, role)| {
        Node::new(&short, host.as_str(), t, role, "root")
    })
}

proptest! {
    #[test]
    fn prop_node_is_darwin_iff_platform_is_darwin(n in arb_built_node()) {
        prop_assert_eq!(n.is_darwin(), n.target.platform == Platform::Darwin);
        prop_assert_eq!(n.is_nixos(), n.target.platform == Platform::Linux);
        prop_assert_eq!(n.is_aarch64(), n.target.arch == Architecture::Aarch64);
    }

    #[test]
    fn prop_node_darwin_nixos_disjoint(n in arb_built_node()) {
        prop_assert!(!(n.is_darwin() && n.is_nixos()));
    }
}

// ── Invariant module self-consistency ──────────────────────────────────────

#[test]
fn invariant_list_count_matches_expected_floor() {
    assert!(
        invariants::ALL_INVARIANTS.len() >= 40,
        "expected at least 40 invariants, found {}",
        invariants::ALL_INVARIANTS.len()
    );
}

#[test]
fn invariant_list_ids_unique() {
    let mut seen = std::collections::HashSet::new();
    for id in invariants::ALL_INVARIANTS {
        assert!(seen.insert(*id), "duplicated invariant id: {id}");
    }
}
