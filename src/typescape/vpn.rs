//! VPN link registry: WireGuard tunnels between fleet nodes.
//!
//! Every link has two sides (`a` = initiator, `b` = responder). The structural
//! invariants enforced here — bidirectionality, interface-length, CIDR shape,
//! address-within-subnet — are validated in the `invariants` module against
//! the full registry.

use super::platform::{IpV4Address, IpV4Cidr, WireguardInterface};

/// A single WireGuard point-to-point link.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VpnLink {
    pub name: String,
    pub profile: VpnProfile,
    pub interface: WireguardInterface,
    pub subnet: IpV4Cidr,
    pub mtu: u16,
    pub persistent_keepalive: Option<u16>,
    pub side_a: VpnSide,
    pub side_b: VpnSide,
    /// Which side stores the PSK. pleme-io convention: always side `a`.
    pub psk_on_side: SideName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideName {
    A,
    B,
}

/// One end of a VPN link.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VpnSide {
    /// Short node name (not the full hostname). May reference a node in the
    /// registry or an external endpoint (e.g. `akeyless-dev`, `seph`).
    pub node: String,
    pub address: IpV4Address,
    pub listen_port: Option<u16>,
    pub endpoint: Option<String>,
    pub private_key_secret: String,
}

/// The *profile* of a VPN link — determines firewall rules, not WireGuard config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VpnProfile {
    /// Minimal — kubectl API access only (TCP 6443).
    K8sControlPlane,
    /// Full K8s — kubelet / controller / scheduler access (TCP 6443, 10250, 10257, 10259).
    K8sFull,
    /// Site-to-site LAN extension.
    SiteToSite,
    /// Mesh connectivity between many nodes.
    Mesh,
}

impl VpnProfile {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::K8sControlPlane => "k8s-control-plane",
            Self::K8sFull => "k8s-full",
            Self::SiteToSite => "site-to-site",
            Self::Mesh => "mesh",
        }
    }
}

impl VpnLink {
    /// True iff the address of each side lies within the link's subnet.
    #[must_use]
    pub fn addresses_in_subnet(&self) -> bool {
        self.subnet.contains(self.side_a.address) && self.subnet.contains(self.side_b.address)
    }

    /// True iff side `a` and side `b` refer to different nodes.
    #[must_use]
    pub fn sides_are_distinct(&self) -> bool {
        self.side_a.node != self.side_b.node
    }

    /// True iff the addresses of each side are distinct.
    #[must_use]
    pub fn addresses_are_distinct(&self) -> bool {
        self.side_a.address != self.side_b.address
    }

    /// True iff exactly one side has a listen port (convention: the responder `b`).
    #[must_use]
    pub fn exactly_one_side_listens(&self) -> bool {
        self.side_a.listen_port.is_some() ^ self.side_b.listen_port.is_some()
    }

    /// True iff the responder (b) has an endpoint and a listen port (since `a` dials `b`).
    #[must_use]
    pub fn responder_has_endpoint(&self) -> bool {
        self.side_b.endpoint.is_some() && self.side_b.listen_port.is_some()
    }
}

impl VpnSide {
    /// An initiator side (no listen port, no endpoint — dials out).
    #[must_use]
    pub fn initiator(node: &str, address: &str, private_key_secret: &str) -> Self {
        Self {
            node: node.to_string(),
            address: IpV4Address::parse(address).expect("valid initiator address"),
            listen_port: None,
            endpoint: None,
            private_key_secret: private_key_secret.to_string(),
        }
    }

    /// A responder side (has listen port + endpoint — accepts inbound).
    #[must_use]
    pub fn responder(
        node: &str,
        address: &str,
        listen_port: u16,
        endpoint: &str,
        private_key_secret: &str,
    ) -> Self {
        Self {
            node: node.to_string(),
            address: IpV4Address::parse(address).expect("valid responder address"),
            listen_port: Some(listen_port),
            endpoint: Some(endpoint.to_string()),
            private_key_secret: private_key_secret.to_string(),
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typescape::platform::{IpV4Cidr, WireguardInterface};

    fn example_link() -> VpnLink {
        VpnLink {
            name: "ryn-k3s".to_string(),
            profile: VpnProfile::K8sControlPlane,
            interface: WireguardInterface::new("wg-ryn-k3s").unwrap(),
            subnet: IpV4Cidr::parse("10.100.1.0/24").unwrap(),
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
        }
    }

    #[test]
    fn sides_are_distinct() {
        assert!(example_link().sides_are_distinct());
    }

    #[test]
    fn addresses_are_distinct() {
        assert!(example_link().addresses_are_distinct());
    }

    #[test]
    fn addresses_lie_within_subnet() {
        assert!(example_link().addresses_in_subnet());
    }

    #[test]
    fn exactly_one_side_listens() {
        assert!(example_link().exactly_one_side_listens());
    }

    #[test]
    fn responder_has_endpoint() {
        assert!(example_link().responder_has_endpoint());
    }

    #[test]
    fn vpn_profiles_have_stable_names() {
        assert_eq!(VpnProfile::K8sControlPlane.as_str(), "k8s-control-plane");
        assert_eq!(VpnProfile::K8sFull.as_str(), "k8s-full");
        assert_eq!(VpnProfile::SiteToSite.as_str(), "site-to-site");
        assert_eq!(VpnProfile::Mesh.as_str(), "mesh");
    }
}
