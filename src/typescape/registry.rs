//! The canonical pleme-io fleet registry, encoded as typed Rust data.
//!
//! This is the single source of truth in Rust for the nix architecture. The
//! `pleme_nix_registry()` function returns a fully-populated `NixTypescape`
//! reflecting `~/code/github/pleme-io/nix` as of the absorption date.
//!
//! Keep in sync with the actual nix repo by running the self-validation tests.

use super::blackmatter::{BlackmatterComponent, ComponentRole};
use super::cluster::{Cluster, FluxAuth, K3sRole};
use super::flake::{FlakeInput, FlakeInputUrl, InputOrigin};
use super::node::{Node, NodeRole};
use super::platform::{IpV4Cidr, Platform, Target, WireguardInterface};
use super::profile::{Profile, ProfileKind, ProfileLayer};
use super::secret::{SecretBackend, SecretPath};
use super::substrate_builder::SubstrateBuilder;
use super::vpn::{SideName, VpnLink, VpnProfile, VpnSide};
use super::NixTypescape;

/// The full pleme-io fleet typescape — built from source declarations in the
/// `pleme-io/nix` repo. Deterministic: same inputs → same output.
#[must_use]
pub fn pleme_nix_registry() -> NixTypescape {
    NixTypescape {
        nodes: nodes(),
        profiles: profiles(),
        blackmatter_components: blackmatter_components(),
        vpn_links: vpn_links(),
        clusters: clusters(),
        flake_inputs: flake_inputs(),
        substrate_builders: substrate_builders(),
        secrets: secrets(),
        default_secret_backend: SecretBackend::Sops,
    }
}

// ── Nodes ───────────────────────────────────────────────────────────────────

fn nodes() -> Vec<Node> {
    vec![
        // ── NixOS fleet (from lib/nodes.nix) ────────────────────────────
        Node::new("plo", "plo.quero.lan", Target::X86_64_LINUX, NodeRole::K3sServer, "root")
            .with_tags(&["production", "k3s", "server"])
            .with_profiles(&["nixos-pleme-base", "nixos-k3s-server"])
            .with_system_user("luis"),

        Node::new("zek", "zek.quero.lan", Target::X86_64_LINUX, NodeRole::K3sAgent, "root")
            .with_tags(&["staging", "k3s", "agent"])
            .with_profiles(&["nixos-pleme-base", "nixos-security-hardened", "nixos-laptop-server"])
            .with_system_user("luis"),

        Node::new("orion", "orion.lilitu.io", Target::X86_64_LINUX, NodeRole::K3sCloudServer, "root")
            .with_tags(&["production", "k3s", "server", "hetzner"])
            .with_profiles(&["nixos-pleme-base"])
            .with_system_user("luis"),

        Node::new("rai", "192.168.50.2", Target::X86_64_LINUX, NodeRole::Legacy, "root")
            .with_tags(&["legacy"]),

        Node::new("wireguard", "wireguard.example.com", Target::AARCH64_LINUX, NodeRole::VpnGateway, "root")
            .with_tags(&["infrastructure", "vpn"]),

        Node::new("cid-k3s", "192.168.64.2", Target::AARCH64_LINUX, NodeRole::K3sVm, "root")
            .with_tags(&["local", "k3s", "server", "vm", "dev"])
            .with_profiles(&["nixos-k3s-vm"])
            .with_managing_node("cid"),

        Node::new("ryn-k3s", "192.168.64.3", Target::AARCH64_LINUX, NodeRole::K3sVm, "root")
            .with_tags(&["local", "k3s", "server", "vm", "dev"])
            .with_profiles(&["nixos-k3s-vm"])
            .with_managing_node("ryn"),

        // ── Darwin fleet (from darwinConfigurations/default.nix) ───────
        Node::new("cid", "cid.local", Target::AARCH64_DARWIN, NodeRole::DarwinWorkstation, "drzzln")
            .with_tags(&["workstation", "darwin", "akeyless-org"])
            .with_profiles(&["darwin-developer", "darwin"])
            .with_system_user("drzzln"),

        Node::new("ryn", "ryn.local", Target::AARCH64_DARWIN, NodeRole::DarwinWorkstation, "drzzln")
            .with_tags(&["workstation", "darwin", "pangea-builder"])
            .with_profiles(&["darwin-developer", "darwin"])
            .with_system_user("drzzln"),
    ]
}

// ── Profiles ────────────────────────────────────────────────────────────────

fn profiles() -> Vec<Profile> {
    vec![
        // NixOS foundation
        Profile::new("nixos-pleme-base", ProfileKind::NixOs, ProfileLayer::Foundation)
            .enabling(&["blackmatter", "blackmatter-secrets"]),

        // NixOS specializations
        Profile::new("nixos-k3s-server", ProfileKind::NixOs, ProfileLayer::Specialization)
            .requiring("nixos-pleme-base")
            .enabling(&["blackmatter-kubernetes"])
            .with_variant("server"),
        Profile::new("nixos-k3s-vm", ProfileKind::NixOs, ProfileLayer::Specialization)
            .requiring("nixos-pleme-base")
            .enabling(&["blackmatter-kubernetes"])
            .with_variant("vm"),

        // NixOS standalones
        Profile::new("nixos-security-hardened", ProfileKind::NixOs, ProfileLayer::Standalone)
            .enabling(&["blackmatter-security"]),
        Profile::new("nixos-laptop-server", ProfileKind::NixOs, ProfileLayer::Standalone),

        // Darwin foundation
        Profile::new("darwin-developer", ProfileKind::Darwin, ProfileLayer::Foundation)
            .enabling(&[
                "blackmatter", "blackmatter-secrets", "blackmatter-claude",
                "blackmatter-kubernetes", "blackmatter-akeyless",
                "blackmatter-cursor", "blackmatter-anvil",
            ]),

        // Darwin sub-profile aggregator
        Profile::new("darwin", ProfileKind::Darwin, ProfileLayer::Standalone),

        // Kindling shared profiles
        Profile::new("k3s-agent", ProfileKind::Kindling, ProfileLayer::Specialization)
            .requiring("nixos-pleme-base")
            .enabling(&["blackmatter-kubernetes"])
            .with_variant("agent"),
        Profile::new("k3s-cloud-server", ProfileKind::Kindling, ProfileLayer::Specialization)
            .requiring("nixos-pleme-base")
            .enabling(&["blackmatter-kubernetes"])
            .with_variant("cloud-server"),
    ]
}

// ── Blackmatter components ──────────────────────────────────────────────────

fn blackmatter_components() -> Vec<BlackmatterComponent> {
    use ComponentRole::*;
    vec![
        BlackmatterComponent::new("blackmatter", "blackmatter", Aggregator)
            .with_modules(true, true, true)
            .with_namespace("blackmatter.profiles"),
        BlackmatterComponent::new("shell", "blackmatter-shell", Capability),
        BlackmatterComponent::new("neovim", "blackmatter-nvim", Capability),
        BlackmatterComponent::new("desktop", "blackmatter-desktop", Capability)
            .with_platforms(&[Platform::Linux, Platform::Darwin]),
        BlackmatterComponent::new("ghostty", "blackmatter-ghostty", Capability)
            .with_platforms(&[Platform::Darwin, Platform::Linux]),
        BlackmatterComponent::new("claude", "blackmatter-claude", DevTool),
        BlackmatterComponent::new("cursor", "blackmatter-cursor", DevTool),
        BlackmatterComponent::new("opencode", "blackmatter-opencode", DevTool),
        BlackmatterComponent::new("anvil", "blackmatter-anvil", DevTool),
        BlackmatterComponent::new("movie", "blackmatter-movie", DevTool),
        BlackmatterComponent::new("kubernetes", "blackmatter-kubernetes", Infrastructure)
            .with_modules(true, true, false),
        BlackmatterComponent::new("secrets", "blackmatter-secrets", Infrastructure)
            .with_modules(true, true, true),
        BlackmatterComponent::new("services", "blackmatter-services", Infrastructure)
            .with_modules(true, true, true),
        BlackmatterComponent::new("tailscale", "blackmatter-tailscale", Infrastructure)
            .with_modules(true, true, true),
        BlackmatterComponent::new("vpn", "blackmatter-vpn", Infrastructure)
            .with_modules(true, true, true),
        BlackmatterComponent::new("android", "blackmatter-android", Capability)
            .with_modules(true, true, false)
            .with_platforms(&[Platform::Linux]),
        BlackmatterComponent::new("macos", "blackmatter-macos", Capability)
            .with_modules(true, false, true)
            .with_platforms(&[Platform::Darwin]),
        BlackmatterComponent::new("security", "blackmatter-security", Security)
            .with_modules(true, true, false),
        BlackmatterComponent::new("home", "blackmatter-home", Infrastructure),
        BlackmatterComponent::new("tend", "blackmatter-tend", DevTool),
        BlackmatterComponent::new("ayatsuri", "blackmatter-ayatsuri", Capability),
        BlackmatterComponent::new("pleme", "blackmatter-pleme", Org),
        BlackmatterComponent::new("direnv", "blackmatter-direnv", DevTool),
        BlackmatterComponent::new("atlassian", "blackmatter-atlassian", DevTool),
        BlackmatterComponent::new("code", "blackmatter-code", Org),
        BlackmatterComponent::new("github", "blackmatter-github", DevTool),
        BlackmatterComponent::new("akeyless", "blackmatter-akeyless", Infrastructure),
        BlackmatterComponent::new("go", "blackmatter-go", Infrastructure),
        BlackmatterComponent::new("zig", "blackmatter-zig", Infrastructure),
        BlackmatterComponent::new("profiles", "blackmatter-profiles", Infrastructure)
            .with_overlay(false),
    ]
}

// ── VPN links (from lib/vpn-links.nix) ─────────────────────────────────────

fn vpn_links() -> Vec<VpnLink> {
    vec![
        VpnLink {
            name: "ryn-k3s".to_string(),
            profile: VpnProfile::K8sControlPlane,
            interface: WireguardInterface::new("wg-ryn-k3s").expect("static"),
            subnet: IpV4Cidr::parse("10.100.1.0/24").expect("static"),
            mtu: 1420,
            persistent_keepalive: None,
            side_a: VpnSide::initiator("ryn", "10.100.1.1", "ryn/wireguard/ryn-k3s/private-key"),
            side_b: VpnSide::responder(
                "ryn-k3s",
                "10.100.1.2",
                51821,
                "192.168.64.3:51821",
                "clusters/ryn-k3s/wireguard/private-key",
            ),
            psk_on_side: SideName::A,
        },
        VpnLink {
            name: "ryn-akeyless-dev".to_string(),
            profile: VpnProfile::K8sControlPlane,
            interface: WireguardInterface::new("wg-ryn-ak").expect("static"),
            subnet: IpV4Cidr::parse("10.100.3.0/24").expect("static"),
            mtu: 1380,
            persistent_keepalive: Some(25),
            side_a: VpnSide::initiator("ryn", "10.100.3.1", "ryn/wireguard/ryn-akeyless-dev/private-key"),
            side_b: VpnSide::responder(
                "akeyless-dev",
                "10.100.3.2",
                51822,
                "vpn.akeyless-dev.quero.lol:51822",
                "clusters/akeyless-dev/wireguard/private-key",
            ),
            psk_on_side: SideName::A,
        },
        VpnLink {
            name: "ryn-seph".to_string(),
            profile: VpnProfile::K8sControlPlane,
            interface: WireguardInterface::new("wg-ryn-seph").expect("static"),
            subnet: IpV4Cidr::parse("10.100.4.0/24").expect("static"),
            mtu: 1380,
            persistent_keepalive: Some(25),
            side_a: VpnSide::initiator("ryn", "10.100.4.1", "ryn/wireguard/ryn-seph/private-key"),
            side_b: VpnSide::responder(
                "seph",
                "10.100.4.2",
                51822,
                "vpn.seph.1.k8s.quero.lol:51822",
                "clusters/seph/wireguard/private-key",
            ),
            psk_on_side: SideName::A,
        },
    ]
}

// ── Clusters ────────────────────────────────────────────────────────────────

fn clusters() -> Vec<Cluster> {
    vec![
        Cluster::new("plo", "plo", K3sRole::Server)
            .with_flux_auth(FluxAuth::SshKey),
        Cluster::new("zek", "zek", K3sRole::Agent),
        Cluster::new("orion", "orion", K3sRole::Server)
            .with_flux_auth(FluxAuth::HttpsToken),
        Cluster::new("cid-k3s", "cid-k3s", K3sRole::Server)
            .with_vpn_links(&["ryn-k3s"])
            .managed_by("cid"),
        Cluster::new("ryn-k3s", "ryn-k3s", K3sRole::Server)
            .with_vpn_links(&["ryn-k3s"])
            .managed_by("ryn"),
    ]
}

// ── Flake inputs (selected — not exhaustive) ───────────────────────────────

fn flake_inputs() -> Vec<FlakeInput> {
    use InputOrigin::*;
    vec![
        FlakeInput::new("nixpkgs", FlakeInputUrl::GitHub {
            org: "nixos".to_string(), repo: "nixpkgs".to_string(), branch: Some("nixos-unstable".to_string()),
        }, NixCommunity),
        FlakeInput::new("home-manager", FlakeInputUrl::nix_community_gh("home-manager"), NixCommunity)
            .follows(&["nixpkgs"]),
        FlakeInput::new("nix-darwin", FlakeInputUrl::GitHub {
            org: "lnl7".to_string(), repo: "nix-darwin".to_string(), branch: None,
        }, ThirdParty).follows(&["nixpkgs"]),
        FlakeInput::new("sops-nix", FlakeInputUrl::nix_community_gh("sops-nix"), NixCommunity)
            .follows(&["nixpkgs"]),
        FlakeInput::new("flake-parts", FlakeInputUrl::GitHub {
            org: "hercules-ci".to_string(), repo: "flake-parts".to_string(), branch: None,
        }, NixCommunity).follows(&["nixpkgs"]),

        // pleme-io core
        FlakeInput::new("substrate", FlakeInputUrl::pleme_gh("substrate"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("kindling", FlakeInputUrl::pleme_gh("kindling"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("kindling-profiles", FlakeInputUrl::pleme_gh("kindling-profiles"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("akeyless-nix", FlakeInputUrl::pleme_gh("akeyless-nix"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("toride", FlakeInputUrl::pleme_gh("bifrost"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("amimori", FlakeInputUrl::pleme_gh("amimori"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("zoekt-mcp", FlakeInputUrl::pleme_gh("zoekt-mcp"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("codesearch", FlakeInputUrl::pleme_gh("codesearch"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("kurage", FlakeInputUrl::pleme_gh("kurage"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("umbra", FlakeInputUrl::pleme_gh("umbra"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("mado", FlakeInputUrl::pleme_gh("mado"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("tobira", FlakeInputUrl::pleme_gh("tobira"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("escriba", FlakeInputUrl::pleme_gh("escriba"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("shinryu-mcp", FlakeInputUrl::pleme_gh("shinryu-mcp"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("slack-forge", FlakeInputUrl::pleme_gh("slack-forge"), PlemeIo)
            .follows(&["nixpkgs"]),
        FlakeInput::new("teiki", FlakeInputUrl::pleme_gh("teiki"), PlemeIo)
            .follows(&["nixpkgs"]),

        // blackmatter subrepos
        FlakeInput::new("blackmatter", FlakeInputUrl::pleme_gh("blackmatter"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-shell", FlakeInputUrl::pleme_gh("blackmatter-shell"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-nvim", FlakeInputUrl::pleme_gh("blackmatter-nvim"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-claude", FlakeInputUrl::pleme_gh("blackmatter-claude"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-desktop", FlakeInputUrl::pleme_gh("blackmatter-desktop"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-ghostty", FlakeInputUrl::pleme_gh("blackmatter-ghostty"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-kubernetes", FlakeInputUrl::pleme_gh("blackmatter-kubernetes"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-secrets", FlakeInputUrl::pleme_gh("blackmatter-secrets"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-akeyless", FlakeInputUrl::pleme_gh("blackmatter-akeyless"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-atlassian", FlakeInputUrl::pleme_gh("blackmatter-atlassian"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-anvil", FlakeInputUrl::pleme_gh("blackmatter-anvil"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-cursor", FlakeInputUrl::pleme_gh("blackmatter-cursor"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-movie", FlakeInputUrl::pleme_gh("blackmatter-movie"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-security", FlakeInputUrl::pleme_gh("blackmatter-security"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-direnv", FlakeInputUrl::pleme_gh("blackmatter-direnv"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-code", FlakeInputUrl::pleme_gh("blackmatter-code"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-github", FlakeInputUrl::pleme_gh("blackmatter-github"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-tailscale", FlakeInputUrl::pleme_gh("blackmatter-tailscale"), Blackmatter)
            .follows(&["nixpkgs"]),
        FlakeInput::new("blackmatter-vpn", FlakeInputUrl::pleme_gh("blackmatter-vpn"), Blackmatter)
            .follows(&["nixpkgs"]),
    ]
}

// ── substrate builders (representative sample) ──────────────────────────────

fn substrate_builders() -> Vec<SubstrateBuilder> {
    use SubstrateBuilder::*;
    let four = Target::all_canonical().to_vec();
    vec![
        // Rust tool releases
        RustToolRelease { tool_name: "tobira".into(), targets: four.clone(), has_hm_module: true },
        RustToolRelease { tool_name: "mado".into(), targets: four.clone(), has_hm_module: true },
        RustToolRelease { tool_name: "hibiki".into(), targets: four.clone(), has_hm_module: true },
        RustToolRelease { tool_name: "kurage".into(), targets: four.clone(), has_hm_module: true },
        RustToolRelease { tool_name: "tend".into(), targets: four.clone(), has_hm_module: false },
        RustToolRelease { tool_name: "kindling".into(), targets: four.clone(), has_hm_module: true },
        RustToolRelease { tool_name: "codesearch".into(), targets: four.clone(), has_hm_module: true },
        RustToolRelease { tool_name: "zoekt-mcp".into(), targets: four.clone(), has_hm_module: true },

        // Rust workspaces
        RustWorkspaceRelease {
            tool_name: "mamorigami".into(),
            package_name: "mamorigami-cli".into(),
            targets: four.clone(),
            has_hm_module: true,
        },

        // Rust libraries
        RustLibrary { crate_name: "irodori".into() },
        RustLibrary { crate_name: "garasu".into() },
        RustLibrary { crate_name: "egaku".into() },
        RustLibrary { crate_name: "shikumi".into() },
        RustLibrary { crate_name: "kaname".into() },
        RustLibrary { crate_name: "hayai".into() },
        RustLibrary { crate_name: "meimei".into() },
        RustLibrary { crate_name: "sekkei".into() },
        RustLibrary { crate_name: "takumi".into() },

        // Rust services
        RustService { service_name: "hanabi".into(), has_hm_module: false, has_nixos_module: true },
        RustService { service_name: "kenshi".into(), has_hm_module: false, has_nixos_module: true },
        RustService { service_name: "shinka".into(), has_hm_module: false, has_nixos_module: true },
    ]
}

// ── Representative secrets (node → paths) ──────────────────────────────────

fn secrets() -> Vec<(String, SecretPath)> {
    let paths_plo = [
        "plo/cloudflare/tunnel-secret",
        "plo/cloudflare/tunnel-token",
        "wifi/psk",
        "attic/jwt/token",
        "crates/publish-token",
        "fluxcd/kube-clusters/pat",
        "openai/drzzln/token",
        "tailscale/auth-key",
        "kubernetes/ca",
        "kubernetes/crt",
        "kubernetes/key",
    ];
    let paths_zek = [
        "discord/zek/webhook-url",
        "k3s/agent-token",
        "wifi/psk",
        "zek/kubernetes/plo/token",
        "attic/jwt/token",
        "tailscale/auth-key",
    ];
    let paths_ryn = [
        "ryn/wireguard/ryn-k3s/private-key",
        "ryn/wireguard/ryn-k3s/psk",
        "ryn/wireguard/ryn-akeyless-dev/private-key",
        "ryn/wireguard/ryn-akeyless-dev/psk",
        "ryn/wireguard/ryn-seph/private-key",
        "ryn/wireguard/ryn-seph/psk",
        "ryn/pangea/builder-ssh-key",
    ];
    let mut out = Vec::new();
    for p in paths_plo { out.push(("plo".to_string(), SecretPath::new(p).expect(p))); }
    for p in paths_zek { out.push(("zek".to_string(), SecretPath::new(p).expect(p))); }
    for p in paths_ryn { out.push(("ryn".to_string(), SecretPath::new(p).expect(p))); }
    out
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_builds() {
        let reg = pleme_nix_registry();
        assert!(!reg.nodes.is_empty());
        assert!(!reg.profiles.is_empty());
        assert!(!reg.blackmatter_components.is_empty());
        assert!(!reg.vpn_links.is_empty());
        assert!(!reg.clusters.is_empty());
        assert!(!reg.flake_inputs.is_empty());
        assert!(!reg.substrate_builders.is_empty());
        assert!(!reg.secrets.is_empty());
    }

    #[test]
    fn registry_is_deterministic() {
        let a = pleme_nix_registry();
        let b = pleme_nix_registry();
        assert_eq!(a.nodes.len(), b.nodes.len());
        assert_eq!(a.profiles.len(), b.profiles.len());
        assert_eq!(a.vpn_links.len(), b.vpn_links.len());
    }

    #[test]
    fn has_darwin_and_nixos_nodes() {
        let reg = pleme_nix_registry();
        assert!(reg.nodes.iter().any(|n| n.is_darwin()));
        assert!(reg.nodes.iter().any(|n| n.is_nixos()));
    }

    #[test]
    fn has_expected_fleet_nodes() {
        let reg = pleme_nix_registry();
        for expected in ["plo", "zek", "orion", "rai", "wireguard", "cid-k3s", "ryn-k3s", "cid", "ryn"] {
            assert!(
                reg.nodes.iter().any(|n| n.short_name == expected),
                "missing node: {expected}"
            );
        }
    }

    #[test]
    fn has_expected_vpn_links() {
        let reg = pleme_nix_registry();
        for expected in ["ryn-k3s", "ryn-akeyless-dev", "ryn-seph"] {
            assert!(
                reg.vpn_links.iter().any(|l| l.name == expected),
                "missing link: {expected}"
            );
        }
    }

    #[test]
    fn has_aggregator_component() {
        let reg = pleme_nix_registry();
        assert!(reg.blackmatter_components.iter().any(|c| c.role == ComponentRole::Aggregator));
    }

    #[test]
    fn every_builder_has_nonempty_name_or_crate() {
        let reg = pleme_nix_registry();
        for b in &reg.substrate_builders {
            match b {
                SubstrateBuilder::RustToolRelease { tool_name, .. }
                | SubstrateBuilder::RustWorkspaceRelease { tool_name, .. }
                | SubstrateBuilder::RustToolImage { tool_name, .. }
                | SubstrateBuilder::ZigToolRelease { tool_name, .. } => assert!(!tool_name.is_empty()),
                SubstrateBuilder::RustLibrary { crate_name } => assert!(!crate_name.is_empty()),
                SubstrateBuilder::RustService { service_name, .. } => assert!(!service_name.is_empty()),
                _ => {}
            }
        }
    }
}
