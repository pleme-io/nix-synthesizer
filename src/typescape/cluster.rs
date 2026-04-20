//! Kubernetes cluster registry — maps to physical nodes and ties into VPN links.

use super::platform::IpV4Cidr;

/// A Kubernetes cluster managed by the fleet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Cluster {
    pub name: String,
    pub node: String,
    pub role: K3sRole,
    pub kubeconfig_path: String,
    pub flux_auth: FluxAuth,
    /// VPN link names (by name, referencing the VPN registry).
    pub vpn_links: Vec<String>,
    pub service_cidr: IpV4Cidr,
    pub cluster_cidr: IpV4Cidr,
    /// Name of the node that *manages* this cluster from outside (for VM clusters).
    pub managed_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum K3sRole {
    Server,
    Agent,
}

impl K3sRole {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::Agent => "agent",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FluxAuth {
    /// SSH with user key (typical for production, e.g. plo).
    SshKey,
    /// HTTPS + GitHub PAT (typical for remote cloud clusters).
    HttpsToken,
}

impl Cluster {
    #[must_use]
    pub fn new(name: &str, node: &str, role: K3sRole) -> Self {
        Self {
            name: name.to_string(),
            node: node.to_string(),
            role,
            kubeconfig_path: "/etc/rancher/k3s/k3s.yaml".to_string(),
            flux_auth: FluxAuth::SshKey,
            vpn_links: Vec::new(),
            service_cidr: IpV4Cidr::parse("10.43.0.0/16").expect("static"),
            cluster_cidr: IpV4Cidr::parse("10.42.0.0/16").expect("static"),
            managed_by: None,
        }
    }

    #[must_use]
    pub fn with_vpn_links(mut self, links: &[&str]) -> Self {
        self.vpn_links = links.iter().map(|s| (*s).to_string()).collect();
        self
    }

    #[must_use]
    pub fn with_flux_auth(mut self, auth: FluxAuth) -> Self {
        self.flux_auth = auth;
        self
    }

    #[must_use]
    pub fn managed_by(mut self, node: &str) -> Self {
        self.managed_by = Some(node.to_string());
        self
    }

    /// True iff this cluster uses the default k3s kubeconfig path convention.
    #[must_use]
    pub fn uses_default_kubeconfig(&self) -> bool {
        self.kubeconfig_path == "/etc/rancher/k3s/k3s.yaml"
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_k3s_cluster_uses_standard_cidrs() {
        let c = Cluster::new("plo", "plo", K3sRole::Server);
        assert_eq!(c.service_cidr.to_string(), "10.43.0.0/16");
        assert_eq!(c.cluster_cidr.to_string(), "10.42.0.0/16");
        assert!(c.uses_default_kubeconfig());
    }

    #[test]
    fn cluster_with_vpn_links_records_them() {
        let c = Cluster::new("ryn-k3s", "ryn-k3s", K3sRole::Server)
            .with_vpn_links(&["ryn-k3s"])
            .managed_by("ryn");
        assert_eq!(c.vpn_links, vec!["ryn-k3s".to_string()]);
        assert_eq!(c.managed_by.as_deref(), Some("ryn"));
    }

    #[test]
    fn cidrs_do_not_overlap_each_other() {
        let c = Cluster::new("plo", "plo", K3sRole::Server);
        assert!(!c.service_cidr.overlaps(c.cluster_cidr));
    }

    #[test]
    fn k3s_role_renders() {
        assert_eq!(K3sRole::Server.as_str(), "server");
        assert_eq!(K3sRole::Agent.as_str(), "agent");
    }
}
