//! Node registry types: the fleet machines pleme-io/nix builds configurations for.
//!
//! Every `Node` is a physical or virtual machine that `darwin-rebuild` or
//! `nixos-rebuild` targets. Nodes carry identity, hardware target, role in the
//! fleet, profile stack, and the kernel set of profile/module imports.

use super::platform::{Architecture, Hostname, Platform, Target};

/// A fleet node — a physical or virtual machine managed by `pleme-io/nix`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Node {
    pub short_name: String,
    pub hostname: Hostname,
    pub target: Target,
    pub role: NodeRole,
    pub ssh_user: String,
    pub tags: Vec<String>,
    pub profiles: Vec<String>,
    pub system_user: Option<String>,
    pub managing_node: Option<String>,
}

/// Role enum — drives cross-cutting invariants (k3s servers have kubeconfig,
/// darwin nodes use aarch64, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeRole {
    /// K3s server node (production or staging).
    K3sServer,
    /// K3s agent node (joins a server).
    K3sAgent,
    /// K3s server running inside a local VM (cid-k3s, ryn-k3s).
    K3sVm,
    /// Darwin (macOS) developer workstation.
    DarwinWorkstation,
    /// Dedicated WireGuard VPN gateway.
    VpnGateway,
    /// Legacy machine retained for historical reasons.
    Legacy,
    /// K3s server on a cloud provider (Hetzner, AWS), not VPN-connected in the registry.
    K3sCloudServer,
}

impl NodeRole {
    #[must_use]
    pub fn is_k3s(self) -> bool {
        matches!(self, Self::K3sServer | Self::K3sAgent | Self::K3sVm | Self::K3sCloudServer)
    }

    #[must_use]
    pub fn is_k3s_server(self) -> bool {
        matches!(self, Self::K3sServer | Self::K3sVm | Self::K3sCloudServer)
    }
}

impl Node {
    /// Constructor helper used by the registry builder.
    #[must_use]
    pub fn new(
        short_name: &str,
        hostname_str: &str,
        target: Target,
        role: NodeRole,
        ssh_user: &str,
    ) -> Self {
        Self {
            short_name: short_name.to_string(),
            hostname: Hostname::new(hostname_str)
                .unwrap_or_else(|e| panic!("invalid hostname for {short_name}: {e}")),
            target,
            role,
            ssh_user: ssh_user.to_string(),
            tags: Vec::new(),
            profiles: Vec::new(),
            system_user: None,
            managing_node: None,
        }
    }

    #[must_use]
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.tags = tags.iter().map(|s| (*s).to_string()).collect();
        self
    }

    #[must_use]
    pub fn with_profiles(mut self, profiles: &[&str]) -> Self {
        self.profiles = profiles.iter().map(|s| (*s).to_string()).collect();
        self
    }

    #[must_use]
    pub fn with_system_user(mut self, user: &str) -> Self {
        self.system_user = Some(user.to_string());
        self
    }

    #[must_use]
    pub fn with_managing_node(mut self, managing: &str) -> Self {
        self.managing_node = Some(managing.to_string());
        self
    }

    #[must_use]
    pub fn is_darwin(&self) -> bool {
        self.target.platform == Platform::Darwin
    }

    #[must_use]
    pub fn is_nixos(&self) -> bool {
        self.target.platform == Platform::Linux
    }

    #[must_use]
    pub fn is_aarch64(&self) -> bool {
        self.target.arch == Architecture::Aarch64
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darwin_nodes_are_darwin_and_aarch64() {
        let n = Node::new("cid", "cid.local", Target::AARCH64_DARWIN, NodeRole::DarwinWorkstation, "drzzln");
        assert!(n.is_darwin());
        assert!(!n.is_nixos());
        assert!(n.is_aarch64());
    }

    #[test]
    fn k3s_roles_classify_correctly() {
        assert!(NodeRole::K3sServer.is_k3s());
        assert!(NodeRole::K3sServer.is_k3s_server());
        assert!(NodeRole::K3sAgent.is_k3s());
        assert!(!NodeRole::K3sAgent.is_k3s_server());
        assert!(NodeRole::K3sVm.is_k3s_server());
        assert!(NodeRole::K3sCloudServer.is_k3s_server());
        assert!(!NodeRole::DarwinWorkstation.is_k3s());
        assert!(!NodeRole::Legacy.is_k3s());
        assert!(!NodeRole::VpnGateway.is_k3s());
    }

    #[test]
    fn node_builder_accumulates() {
        let n = Node::new("plo", "plo.quero.lan", Target::X86_64_LINUX, NodeRole::K3sServer, "root")
            .with_tags(&["production", "k3s", "server"])
            .with_profiles(&["nixos-pleme-base", "nixos-k3s-server"]);
        assert_eq!(n.tags.len(), 3);
        assert_eq!(n.profiles.len(), 2);
        assert_eq!(n.short_name, "plo");
    }
}
