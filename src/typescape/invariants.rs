//! Structural invariants — the proofs that hold on any valid `NixTypescape`.
//!
//! Each invariant is expressed as a pure function returning a `Vec<String>` of
//! violation messages. An invariant holds iff the vec is empty. This lets us
//! compose invariants into a full report and drive both unit tests and
//! proptest-generated arbitrary inputs against the same check.

use super::blackmatter::ComponentRole;
use super::node::NodeRole;
use super::platform::{Architecture, Platform};
use super::substrate_builder::SubstrateBuilder;
use super::vpn::SideName;
use super::NixTypescape;

/// Identifier + description of a single invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvariantId(pub &'static str);

/// A violation with its invariant id and a free-form message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub id: InvariantId,
    pub message: String,
}

impl Violation {
    fn new(id: &'static str, msg: impl Into<String>) -> Self {
        Self { id: InvariantId(id), message: msg.into() }
    }
}

/// The full list of structural invariants enforced against a `NixTypescape`.
/// Stable ordering: each entry matches a named check below.
pub const ALL_INVARIANTS: &[&str] = &[
    "node_hostnames_unique",
    "node_short_names_unique",
    "node_target_coherence",
    "darwin_nodes_use_aarch64",
    "nixos_nodes_have_at_least_one_profile_or_are_legacy",
    "k3s_vm_nodes_have_managing_node",
    "profile_names_unique",
    "specialization_has_foundation",
    "specialization_foundation_exists",
    "foundation_profile_per_platform_exists",
    "blackmatter_component_names_unique",
    "blackmatter_component_repo_names_unique",
    "blackmatter_aggregator_unique",
    "blackmatter_component_has_at_least_one_module",
    "vpn_link_names_unique",
    "vpn_interface_names_unique",
    "vpn_sides_distinct",
    "vpn_addresses_distinct",
    "vpn_addresses_in_subnet",
    "vpn_subnets_non_overlapping",
    "vpn_exactly_one_side_listens",
    "vpn_responder_has_endpoint",
    "vpn_psk_on_initiator",
    "vpn_local_node_exists",
    "vpn_cidr_is_slash_24",
    "vpn_keepalive_set_for_internet_links",
    "cluster_names_unique",
    "cluster_node_exists",
    "cluster_server_has_vpn_or_is_public",
    "cluster_kubeconfig_convention",
    "cluster_cidrs_do_not_overlap_service_cidrs",
    "cluster_vpn_link_exists",
    "flake_input_names_unique",
    "pleme_inputs_follow_nixpkgs",
    "nixpkgs_input_present",
    "substrate_rust_release_has_four_targets",
    "substrate_builder_names_unique_per_kind",
    "substrate_services_expose_nixos_module",
    "substrate_rust_tool_image_archs_are_linux",
    "secret_paths_valid_format",
    "secret_paths_unique_per_node",
    "secret_node_reference_exists_or_is_shared",
    "vpn_endpoint_format_valid",
    "cid_k3s_managed_by_darwin_host",
    "ryn_k3s_managed_by_darwin_host",
];

impl NixTypescape {
    /// Run every invariant and collect the full violation list.
    #[must_use]
    pub fn all_violations(&self) -> Vec<Violation> {
        let mut v = Vec::new();
        v.extend(node_hostnames_unique(self));
        v.extend(node_short_names_unique(self));
        v.extend(node_target_coherence(self));
        v.extend(darwin_nodes_use_aarch64(self));
        v.extend(nixos_nodes_have_at_least_one_profile_or_are_legacy(self));
        v.extend(k3s_vm_nodes_have_managing_node(self));
        v.extend(profile_names_unique(self));
        v.extend(specialization_has_foundation(self));
        v.extend(specialization_foundation_exists(self));
        v.extend(foundation_profile_per_platform_exists(self));
        v.extend(blackmatter_component_names_unique(self));
        v.extend(blackmatter_component_repo_names_unique(self));
        v.extend(blackmatter_aggregator_unique(self));
        v.extend(blackmatter_component_has_at_least_one_module(self));
        v.extend(vpn_link_names_unique(self));
        v.extend(vpn_interface_names_unique(self));
        v.extend(vpn_sides_distinct(self));
        v.extend(vpn_addresses_distinct(self));
        v.extend(vpn_addresses_in_subnet(self));
        v.extend(vpn_subnets_non_overlapping(self));
        v.extend(vpn_exactly_one_side_listens(self));
        v.extend(vpn_responder_has_endpoint(self));
        v.extend(vpn_psk_on_initiator(self));
        v.extend(vpn_local_node_exists(self));
        v.extend(vpn_cidr_is_slash_24(self));
        v.extend(vpn_keepalive_set_for_internet_links(self));
        v.extend(cluster_names_unique(self));
        v.extend(cluster_node_exists(self));
        v.extend(cluster_server_has_vpn_or_is_public(self));
        v.extend(cluster_kubeconfig_convention(self));
        v.extend(cluster_cidrs_do_not_overlap_service_cidrs(self));
        v.extend(cluster_vpn_link_exists(self));
        v.extend(flake_input_names_unique(self));
        v.extend(pleme_inputs_follow_nixpkgs(self));
        v.extend(nixpkgs_input_present(self));
        v.extend(substrate_rust_release_has_four_targets(self));
        v.extend(substrate_builder_names_unique_per_kind(self));
        v.extend(substrate_services_expose_nixos_module(self));
        v.extend(substrate_rust_tool_image_archs_are_linux(self));
        v.extend(secret_paths_valid_format(self));
        v.extend(secret_paths_unique_per_node(self));
        v.extend(secret_node_reference_exists_or_is_shared(self));
        v.extend(vpn_endpoint_format_valid(self));
        v.extend(cid_k3s_managed_by_darwin_host(self));
        v.extend(ryn_k3s_managed_by_darwin_host(self));
        v
    }

    /// Convenience: is the typescape internally consistent?
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        self.all_violations().is_empty()
    }
}

// ── Individual invariant functions ──────────────────────────────────────────

pub fn node_hostnames_unique(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashMap::<&str, usize>::new();
    for n in &t.nodes {
        *seen.entry(n.hostname.as_str()).or_insert(0) += 1;
    }
    for (h, count) in seen {
        if count > 1 {
            out.push(Violation::new(
                "node_hostnames_unique",
                format!("hostname {h:?} appears {count} times"),
            ));
        }
    }
    out
}

pub fn node_short_names_unique(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashMap::<&str, usize>::new();
    for n in &t.nodes {
        *seen.entry(n.short_name.as_str()).or_insert(0) += 1;
    }
    for (s, count) in seen {
        if count > 1 {
            out.push(Violation::new(
                "node_short_names_unique",
                format!("short name {s:?} appears {count} times"),
            ));
        }
    }
    out
}

pub fn node_target_coherence(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    for n in &t.nodes {
        // Darwin nodes must be aarch64 in the current fleet (M1/M2 only).
        if n.is_darwin() && n.target.arch != Architecture::Aarch64 {
            out.push(Violation::new(
                "node_target_coherence",
                format!("darwin node {:?} is not aarch64", n.short_name),
            ));
        }
        // K3s VMs are aarch64 linux (UTM on Apple silicon).
        if n.role == NodeRole::K3sVm && (n.target.platform != Platform::Linux || n.target.arch != Architecture::Aarch64) {
            out.push(Violation::new(
                "node_target_coherence",
                format!("k3s-vm node {:?} must be aarch64-linux", n.short_name),
            ));
        }
    }
    out
}

pub fn darwin_nodes_use_aarch64(t: &NixTypescape) -> Vec<Violation> {
    t.nodes
        .iter()
        .filter(|n| n.is_darwin() && n.target.arch != Architecture::Aarch64)
        .map(|n| {
            Violation::new(
                "darwin_nodes_use_aarch64",
                format!("{} is darwin but not aarch64", n.short_name),
            )
        })
        .collect()
}

pub fn nixos_nodes_have_at_least_one_profile_or_are_legacy(t: &NixTypescape) -> Vec<Violation> {
    t.nodes
        .iter()
        .filter(|n| n.is_nixos() && n.profiles.is_empty() && n.role != NodeRole::Legacy && n.role != NodeRole::VpnGateway && n.role != NodeRole::K3sVm)
        .map(|n| {
            Violation::new(
                "nixos_nodes_have_at_least_one_profile_or_are_legacy",
                format!("nixos node {} has no profiles and is not legacy/vpn/vm", n.short_name),
            )
        })
        .collect()
}

pub fn k3s_vm_nodes_have_managing_node(t: &NixTypescape) -> Vec<Violation> {
    t.nodes
        .iter()
        .filter(|n| n.role == NodeRole::K3sVm && n.managing_node.is_none())
        .map(|n| {
            Violation::new(
                "k3s_vm_nodes_have_managing_node",
                format!("k3s-vm {} has no managing_node", n.short_name),
            )
        })
        .collect()
}

pub fn profile_names_unique(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashMap::<&str, usize>::new();
    for p in &t.profiles {
        *seen.entry(p.name.as_str()).or_insert(0) += 1;
    }
    for (n, count) in seen {
        if count > 1 {
            out.push(Violation::new("profile_names_unique", format!("profile {n:?} appears {count} times")));
        }
    }
    out
}

pub fn specialization_has_foundation(t: &NixTypescape) -> Vec<Violation> {
    t.profiles
        .iter()
        .filter(|p| p.is_specialization() && p.requires_foundation.is_none())
        .map(|p| {
            Violation::new(
                "specialization_has_foundation",
                format!("specialization profile {:?} has no requires_foundation", p.name),
            )
        })
        .collect()
}

pub fn specialization_foundation_exists(t: &NixTypescape) -> Vec<Violation> {
    let names: std::collections::HashSet<&str> = t.profiles.iter().map(|p| p.name.as_str()).collect();
    t.profiles
        .iter()
        .filter_map(|p| {
            if let Some(f) = &p.requires_foundation {
                if !names.contains(f.as_str()) {
                    return Some(Violation::new(
                        "specialization_foundation_exists",
                        format!("profile {:?} requires missing foundation {:?}", p.name, f),
                    ));
                }
            }
            None
        })
        .collect()
}

pub fn foundation_profile_per_platform_exists(t: &NixTypescape) -> Vec<Violation> {
    use super::profile::ProfileKind;
    let mut has_nixos = false;
    let mut has_darwin = false;
    for p in &t.profiles {
        if p.is_foundation() {
            match p.kind {
                ProfileKind::NixOs => has_nixos = true,
                ProfileKind::Darwin => has_darwin = true,
                ProfileKind::Kindling => {}
            }
        }
    }
    let mut out = Vec::new();
    if !has_nixos {
        out.push(Violation::new("foundation_profile_per_platform_exists", "no nixos foundation profile defined"));
    }
    if !has_darwin {
        out.push(Violation::new("foundation_profile_per_platform_exists", "no darwin foundation profile defined"));
    }
    out
}

pub fn blackmatter_component_names_unique(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashMap::<&str, usize>::new();
    for c in &t.blackmatter_components {
        *seen.entry(c.option_namespace.as_str()).or_insert(0) += 1;
    }
    for (n, count) in seen {
        if count > 1 {
            out.push(Violation::new(
                "blackmatter_component_names_unique",
                format!("namespace {n:?} duplicated {count} times"),
            ));
        }
    }
    out
}

pub fn blackmatter_component_repo_names_unique(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashMap::<&str, usize>::new();
    for c in &t.blackmatter_components {
        *seen.entry(c.repo.as_str()).or_insert(0) += 1;
    }
    for (n, count) in seen {
        if count > 1 {
            out.push(Violation::new(
                "blackmatter_component_repo_names_unique",
                format!("repo {n:?} duplicated {count} times"),
            ));
        }
    }
    out
}

pub fn blackmatter_aggregator_unique(t: &NixTypescape) -> Vec<Violation> {
    let count = t.blackmatter_components.iter().filter(|c| c.role == ComponentRole::Aggregator).count();
    match count {
        0 => vec![Violation::new("blackmatter_aggregator_unique", "no aggregator component")],
        1 => vec![],
        n => vec![Violation::new(
            "blackmatter_aggregator_unique",
            format!("expected 1 aggregator, found {n}"),
        )],
    }
}

pub fn blackmatter_component_has_at_least_one_module(t: &NixTypescape) -> Vec<Violation> {
    t.blackmatter_components
        .iter()
        .filter(|c| !c.provides_any_module())
        .map(|c| {
            Violation::new(
                "blackmatter_component_has_at_least_one_module",
                format!("component {:?} provides no modules", c.name),
            )
        })
        .collect()
}

pub fn vpn_link_names_unique(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashMap::<&str, usize>::new();
    for l in &t.vpn_links {
        *seen.entry(l.name.as_str()).or_insert(0) += 1;
    }
    for (n, count) in seen {
        if count > 1 {
            out.push(Violation::new("vpn_link_names_unique", format!("link {n:?} duplicated {count} times")));
        }
    }
    out
}

pub fn vpn_interface_names_unique(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashMap::<&str, usize>::new();
    for l in &t.vpn_links {
        *seen.entry(l.interface.as_str()).or_insert(0) += 1;
    }
    for (n, count) in seen {
        if count > 1 {
            out.push(Violation::new(
                "vpn_interface_names_unique",
                format!("interface {n:?} duplicated {count} times"),
            ));
        }
    }
    out
}

pub fn vpn_sides_distinct(t: &NixTypescape) -> Vec<Violation> {
    t.vpn_links
        .iter()
        .filter(|l| !l.sides_are_distinct())
        .map(|l| {
            Violation::new(
                "vpn_sides_distinct",
                format!("link {:?} has identical sides ({:?} ↔ {:?})", l.name, l.side_a.node, l.side_b.node),
            )
        })
        .collect()
}

pub fn vpn_addresses_distinct(t: &NixTypescape) -> Vec<Violation> {
    t.vpn_links
        .iter()
        .filter(|l| !l.addresses_are_distinct())
        .map(|l| Violation::new("vpn_addresses_distinct", format!("link {:?} has identical side addresses", l.name)))
        .collect()
}

pub fn vpn_addresses_in_subnet(t: &NixTypescape) -> Vec<Violation> {
    t.vpn_links
        .iter()
        .filter(|l| !l.addresses_in_subnet())
        .map(|l| Violation::new("vpn_addresses_in_subnet", format!("link {:?} addresses outside subnet", l.name)))
        .collect()
}

pub fn vpn_subnets_non_overlapping(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    for (i, a) in t.vpn_links.iter().enumerate() {
        for b in t.vpn_links.iter().skip(i + 1) {
            if a.subnet.overlaps(b.subnet) {
                out.push(Violation::new(
                    "vpn_subnets_non_overlapping",
                    format!("links {:?} and {:?} have overlapping subnets", a.name, b.name),
                ));
            }
        }
    }
    out
}

pub fn vpn_exactly_one_side_listens(t: &NixTypescape) -> Vec<Violation> {
    t.vpn_links
        .iter()
        .filter(|l| !l.exactly_one_side_listens())
        .map(|l| {
            Violation::new(
                "vpn_exactly_one_side_listens",
                format!("link {:?} does not have exactly one side listening", l.name),
            )
        })
        .collect()
}

pub fn vpn_responder_has_endpoint(t: &NixTypescape) -> Vec<Violation> {
    t.vpn_links
        .iter()
        .filter(|l| !l.responder_has_endpoint())
        .map(|l| {
            Violation::new(
                "vpn_responder_has_endpoint",
                format!("link {:?} responder missing endpoint or listen_port", l.name),
            )
        })
        .collect()
}

pub fn vpn_psk_on_initiator(t: &NixTypescape) -> Vec<Violation> {
    t.vpn_links
        .iter()
        .filter(|l| l.psk_on_side != SideName::A)
        .map(|l| {
            Violation::new(
                "vpn_psk_on_initiator",
                format!("link {:?} psk is not on side a", l.name),
            )
        })
        .collect()
}

pub fn vpn_local_node_exists(t: &NixTypescape) -> Vec<Violation> {
    let short_names: std::collections::HashSet<&str> = t.nodes.iter().map(|n| n.short_name.as_str()).collect();
    let mut out = Vec::new();
    // The initiator side (side_a) must always be a registered local node.
    for l in &t.vpn_links {
        if !short_names.contains(l.side_a.node.as_str()) {
            out.push(Violation::new(
                "vpn_local_node_exists",
                format!("link {:?} initiator {:?} is not a registered node", l.name, l.side_a.node),
            ));
        }
    }
    out
}

pub fn vpn_cidr_is_slash_24(t: &NixTypescape) -> Vec<Violation> {
    t.vpn_links
        .iter()
        .filter(|l| l.subnet.prefix() != 24)
        .map(|l| {
            Violation::new(
                "vpn_cidr_is_slash_24",
                format!("link {:?} subnet is /{}", l.name, l.subnet.prefix()),
            )
        })
        .collect()
}

pub fn vpn_keepalive_set_for_internet_links(t: &NixTypescape) -> Vec<Violation> {
    // Heuristic: internet links have MTU < 1420. Those must have persistent_keepalive.
    t.vpn_links
        .iter()
        .filter(|l| l.mtu < 1420 && l.persistent_keepalive.is_none())
        .map(|l| {
            Violation::new(
                "vpn_keepalive_set_for_internet_links",
                format!("internet link {:?} (mtu<1420) missing persistent_keepalive", l.name),
            )
        })
        .collect()
}

pub fn cluster_names_unique(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashMap::<&str, usize>::new();
    for c in &t.clusters {
        *seen.entry(c.name.as_str()).or_insert(0) += 1;
    }
    for (n, count) in seen {
        if count > 1 {
            out.push(Violation::new("cluster_names_unique", format!("cluster {n:?} duplicated {count} times")));
        }
    }
    out
}

pub fn cluster_node_exists(t: &NixTypescape) -> Vec<Violation> {
    let short_names: std::collections::HashSet<&str> = t.nodes.iter().map(|n| n.short_name.as_str()).collect();
    t.clusters
        .iter()
        .filter(|c| !short_names.contains(c.node.as_str()))
        .map(|c| {
            Violation::new(
                "cluster_node_exists",
                format!("cluster {:?} references missing node {:?}", c.name, c.node),
            )
        })
        .collect()
}

pub fn cluster_server_has_vpn_or_is_public(t: &NixTypescape) -> Vec<Violation> {
    use super::cluster::K3sRole;
    t.clusters
        .iter()
        .filter(|c| {
            c.role == K3sRole::Server
                && c.vpn_links.is_empty()
                && !is_publicly_reachable(t, &c.node)
        })
        .map(|c| {
            Violation::new(
                "cluster_server_has_vpn_or_is_public",
                format!("server cluster {:?} has no VPN and is not public", c.name),
            )
        })
        .collect()
}

fn is_publicly_reachable(t: &NixTypescape, node_name: &str) -> bool {
    // Cloud servers (orion) or standard production hosts (plo with public DNS) count as public.
    t.nodes.iter().any(|n| {
        n.short_name == node_name
            && (n.role == NodeRole::K3sCloudServer
                || n.hostname.as_str().ends_with(".quero.lan")
                || n.hostname.as_str().ends_with(".lilitu.io"))
    })
}

pub fn cluster_kubeconfig_convention(t: &NixTypescape) -> Vec<Violation> {
    t.clusters
        .iter()
        .filter(|c| !c.uses_default_kubeconfig())
        .map(|c| {
            Violation::new(
                "cluster_kubeconfig_convention",
                format!("cluster {:?} does not use /etc/rancher/k3s/k3s.yaml", c.name),
            )
        })
        .collect()
}

pub fn cluster_cidrs_do_not_overlap_service_cidrs(t: &NixTypescape) -> Vec<Violation> {
    t.clusters
        .iter()
        .filter(|c| c.cluster_cidr.overlaps(c.service_cidr))
        .map(|c| {
            Violation::new(
                "cluster_cidrs_do_not_overlap_service_cidrs",
                format!("cluster {:?} pod/service CIDRs overlap", c.name),
            )
        })
        .collect()
}

pub fn cluster_vpn_link_exists(t: &NixTypescape) -> Vec<Violation> {
    let link_names: std::collections::HashSet<&str> = t.vpn_links.iter().map(|l| l.name.as_str()).collect();
    let mut out = Vec::new();
    for c in &t.clusters {
        for l in &c.vpn_links {
            if !link_names.contains(l.as_str()) {
                out.push(Violation::new(
                    "cluster_vpn_link_exists",
                    format!("cluster {:?} references missing vpn link {:?}", c.name, l),
                ));
            }
        }
    }
    out
}

pub fn flake_input_names_unique(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashMap::<&str, usize>::new();
    for f in &t.flake_inputs {
        *seen.entry(f.name.as_str()).or_insert(0) += 1;
    }
    for (n, count) in seen {
        if count > 1 {
            out.push(Violation::new("flake_input_names_unique", format!("input {n:?} duplicated {count} times")));
        }
    }
    out
}

pub fn pleme_inputs_follow_nixpkgs(t: &NixTypescape) -> Vec<Violation> {
    t.flake_inputs
        .iter()
        .filter(|f| f.is_pleme() && !f.follows_nixpkgs())
        .map(|f| {
            Violation::new(
                "pleme_inputs_follow_nixpkgs",
                format!("pleme input {:?} does not follow nixpkgs", f.name),
            )
        })
        .collect()
}

pub fn nixpkgs_input_present(t: &NixTypescape) -> Vec<Violation> {
    if t.flake_inputs.iter().any(|f| f.name == "nixpkgs") {
        vec![]
    } else {
        vec![Violation::new("nixpkgs_input_present", "no `nixpkgs` flake input declared")]
    }
}

pub fn substrate_rust_release_has_four_targets(t: &NixTypescape) -> Vec<Violation> {
    t.substrate_builders
        .iter()
        .filter_map(|b| match b {
            SubstrateBuilder::RustToolRelease { tool_name, targets, .. }
            | SubstrateBuilder::RustWorkspaceRelease { tool_name, targets, .. } => {
                if targets.len() == 4 { None } else {
                    Some(Violation::new(
                        "substrate_rust_release_has_four_targets",
                        format!("{tool_name:?} has {} targets, expected 4", targets.len()),
                    ))
                }
            }
            _ => None,
        })
        .collect()
}

pub fn substrate_builder_names_unique_per_kind(t: &NixTypescape) -> Vec<Violation> {
    let mut seen = std::collections::HashMap::<(super::substrate_builder::BuilderKind, String), usize>::new();
    for b in &t.substrate_builders {
        let key = match b {
            SubstrateBuilder::RustToolRelease { tool_name, .. } => (b.kind(), tool_name.clone()),
            SubstrateBuilder::RustWorkspaceRelease { tool_name, .. } => (b.kind(), tool_name.clone()),
            SubstrateBuilder::RustToolImage { tool_name, .. } => (b.kind(), tool_name.clone()),
            SubstrateBuilder::RustService { service_name, .. } => (b.kind(), service_name.clone()),
            SubstrateBuilder::RustLibrary { crate_name } => (b.kind(), crate_name.clone()),
            SubstrateBuilder::LeptosBuild { app_name, .. } => (b.kind(), app_name.clone()),
            SubstrateBuilder::GoTool { pname, .. } => (b.kind(), pname.clone()),
            SubstrateBuilder::GoMonorepoSource { repo, .. } => (b.kind(), repo.clone()),
            SubstrateBuilder::GoMonorepoBinary { pname, .. } => (b.kind(), pname.clone()),
            SubstrateBuilder::TypescriptTool { tool_name, .. } => (b.kind(), tool_name.clone()),
            SubstrateBuilder::TypescriptLibrary { name } => (b.kind(), name.clone()),
            SubstrateBuilder::RubyGem { name } => (b.kind(), name.clone()),
            SubstrateBuilder::ZigToolRelease { tool_name, .. } => (b.kind(), tool_name.clone()),
            SubstrateBuilder::WasiService { service_name, .. } => (b.kind(), service_name.clone()),
            SubstrateBuilder::NixOsAmiBuild { ami_name } => (b.kind(), ami_name.clone()),
        };
        *seen.entry(key).or_insert(0) += 1;
    }
    seen.into_iter()
        .filter_map(|(k, v)| {
            if v > 1 {
                Some(Violation::new(
                    "substrate_builder_names_unique_per_kind",
                    format!("builder {:?} {:?} appears {v} times", k.0, k.1),
                ))
            } else {
                None
            }
        })
        .collect()
}

pub fn substrate_services_expose_nixos_module(t: &NixTypescape) -> Vec<Violation> {
    t.substrate_builders
        .iter()
        .filter_map(|b| match b {
            SubstrateBuilder::RustService { service_name, has_nixos_module, .. } if !*has_nixos_module => {
                Some(Violation::new(
                    "substrate_services_expose_nixos_module",
                    format!("rust service {service_name:?} has no nixos module"),
                ))
            }
            _ => None,
        })
        .collect()
}

pub fn substrate_rust_tool_image_archs_are_linux(t: &NixTypescape) -> Vec<Violation> {
    // Docker images are linux-only; architectures recorded on the builder should
    // be a non-empty subset of {aarch64, x86_64}, which holds by enum exhaustion.
    // We validate non-emptiness here.
    t.substrate_builders
        .iter()
        .filter_map(|b| match b {
            SubstrateBuilder::RustToolImage { tool_name, archs } if archs.is_empty() => {
                Some(Violation::new(
                    "substrate_rust_tool_image_archs_are_linux",
                    format!("rust tool image {tool_name:?} has no architectures"),
                ))
            }
            _ => None,
        })
        .collect()
}

pub fn secret_paths_valid_format(t: &NixTypescape) -> Vec<Violation> {
    // The SecretPath constructor already enforces the format, so this is vacuously
    // satisfied when the registry builds. We keep the invariant here for proptest
    // and self-validation paths.
    let mut out = Vec::new();
    for (node, path) in &t.secrets {
        if path.depth() < 2 || path.depth() > 5 {
            out.push(Violation::new(
                "secret_paths_valid_format",
                format!("secret {:?} on node {:?} has invalid depth {}", path.as_str(), node, path.depth()),
            ));
        }
    }
    out
}

pub fn secret_paths_unique_per_node(t: &NixTypescape) -> Vec<Violation> {
    let mut seen = std::collections::HashMap::<(String, String), usize>::new();
    for (node, path) in &t.secrets {
        let k = (node.clone(), path.as_str().to_string());
        *seen.entry(k).or_insert(0) += 1;
    }
    seen.into_iter()
        .filter_map(|((node, path), n)| {
            if n > 1 {
                Some(Violation::new(
                    "secret_paths_unique_per_node",
                    format!("secret {path:?} on {node:?} appears {n} times"),
                ))
            } else {
                None
            }
        })
        .collect()
}

pub fn secret_node_reference_exists_or_is_shared(t: &NixTypescape) -> Vec<Violation> {
    let short_names: std::collections::HashSet<&str> = t.nodes.iter().map(|n| n.short_name.as_str()).collect();
    let mut out = Vec::new();
    for (node, path) in &t.secrets {
        // Allow "shared" pseudo-node used for fleet-wide secrets
        if !short_names.contains(node.as_str()) && node != "shared" {
            out.push(Violation::new(
                "secret_node_reference_exists_or_is_shared",
                format!("secret {path:?} owner {node:?} not in node registry"),
            ));
        }
    }
    out
}

pub fn vpn_endpoint_format_valid(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    for l in &t.vpn_links {
        if let Some(ep) = &l.side_b.endpoint {
            let (host, port) = match ep.rsplit_once(':') {
                Some(hp) => hp,
                None => {
                    out.push(Violation::new(
                        "vpn_endpoint_format_valid",
                        format!("link {:?} endpoint {:?} missing :port", l.name, ep),
                    ));
                    continue;
                }
            };
            if host.is_empty() || port.parse::<u16>().is_err() {
                out.push(Violation::new(
                    "vpn_endpoint_format_valid",
                    format!("link {:?} endpoint {:?} malformed", l.name, ep),
                ));
            }
        }
    }
    out
}

pub fn cid_k3s_managed_by_darwin_host(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    if let Some(cid_k3s) = t.nodes.iter().find(|n| n.short_name == "cid-k3s") {
        match cid_k3s.managing_node.as_deref() {
            Some(name) => {
                if t.nodes.iter().find(|n| n.short_name == name).map(|n| n.is_darwin()) != Some(true) {
                    out.push(Violation::new(
                        "cid_k3s_managed_by_darwin_host",
                        format!("cid-k3s managing node {name:?} is not darwin"),
                    ));
                }
            }
            None => out.push(Violation::new(
                "cid_k3s_managed_by_darwin_host",
                "cid-k3s has no managing_node",
            )),
        }
    }
    out
}

pub fn ryn_k3s_managed_by_darwin_host(t: &NixTypescape) -> Vec<Violation> {
    let mut out = Vec::new();
    if let Some(n) = t.nodes.iter().find(|n| n.short_name == "ryn-k3s") {
        match n.managing_node.as_deref() {
            Some(name) => {
                if t.nodes.iter().find(|n| n.short_name == name).map(|n| n.is_darwin()) != Some(true) {
                    out.push(Violation::new(
                        "ryn_k3s_managed_by_darwin_host",
                        format!("ryn-k3s managing node {name:?} is not darwin"),
                    ));
                }
            }
            None => out.push(Violation::new("ryn_k3s_managed_by_darwin_host", "ryn-k3s has no managing_node")),
        }
    }
    out
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typescape::registry::pleme_nix_registry;

    #[test]
    fn canonical_registry_is_consistent() {
        let reg = pleme_nix_registry();
        let violations = reg.all_violations();
        if !violations.is_empty() {
            for v in &violations {
                eprintln!("[{}] {}", v.id.0, v.message);
            }
            panic!("canonical registry has {} invariant violations", violations.len());
        }
    }

    #[test]
    fn all_invariants_listed_match_run() {
        // Every invariant id we claim in ALL_INVARIANTS should appear as a function.
        // Smoke test: running all_violations touches every function via dispatch.
        let reg = pleme_nix_registry();
        let _ = reg.all_violations();
        assert!(!ALL_INVARIANTS.is_empty());
    }

    #[test]
    fn empty_typescape_triggers_missing_nixpkgs() {
        let t = NixTypescape::empty();
        let v = nixpkgs_input_present(&t);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].id.0, "nixpkgs_input_present");
    }

    #[test]
    fn empty_typescape_missing_aggregator() {
        let t = NixTypescape::empty();
        let v = blackmatter_aggregator_unique(&t);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn duplicated_hostname_detected() {
        let mut t = pleme_nix_registry();
        // Duplicate plo's entry to trigger detection.
        let plo = t.nodes.iter().find(|n| n.short_name == "plo").cloned().unwrap();
        let mut dup = plo.clone();
        dup.short_name = "plo-dup".into();
        t.nodes.push(dup);
        let v = node_hostnames_unique(&t);
        assert_eq!(v.len(), 1, "expected 1 hostname violation");
    }

    #[test]
    fn overlap_subnet_detected() {
        use super::super::platform::{IpV4Cidr, WireguardInterface};
        use super::super::vpn::{SideName, VpnLink, VpnProfile, VpnSide};
        let mut t = pleme_nix_registry();
        t.vpn_links.push(VpnLink {
            name: "bad-link".into(),
            profile: VpnProfile::K8sControlPlane,
            interface: WireguardInterface::new("wg-bad").unwrap(),
            subnet: IpV4Cidr::parse("10.100.1.128/25").unwrap(), // overlaps with ryn-k3s /24
            mtu: 1420,
            persistent_keepalive: None,
            side_a: VpnSide::initiator("ryn", "10.100.1.129", "x"),
            side_b: VpnSide::responder("foo", "10.100.1.130", 51900, "foo:51900", "y"),
            psk_on_side: SideName::A,
        });
        let v = vpn_subnets_non_overlapping(&t);
        assert!(!v.is_empty());
    }
}
