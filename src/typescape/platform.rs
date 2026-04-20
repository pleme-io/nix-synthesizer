//! Platform primitives: architecture, target triples, hostnames, CIDRs, WireGuard interfaces.
//!
//! These are the atomic types every higher-level dimension (nodes, profiles, VPN, clusters)
//! composes out of. All validation happens here so that downstream code can trust the types.

use std::fmt;
use std::hash::{Hash, Hasher};

// ── Architecture + Platform + Target ────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Architecture {
    Aarch64,
    X86_64,
}

impl Architecture {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::X86_64 => "x86_64",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Darwin,
    Linux,
}

impl Platform {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Darwin => "darwin",
            Self::Linux => "linux",
        }
    }
}

/// A target triple (`aarch64-darwin`, `x86_64-linux`, etc.). Represents a build system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target {
    pub arch: Architecture,
    pub platform: Platform,
}

impl Target {
    pub const AARCH64_DARWIN: Self = Self { arch: Architecture::Aarch64, platform: Platform::Darwin };
    pub const X86_64_DARWIN: Self = Self { arch: Architecture::X86_64, platform: Platform::Darwin };
    pub const X86_64_LINUX: Self = Self { arch: Architecture::X86_64, platform: Platform::Linux };
    pub const AARCH64_LINUX: Self = Self { arch: Architecture::Aarch64, platform: Platform::Linux };

    #[must_use]
    pub fn as_nix_system(self) -> String {
        format!("{}-{}", self.arch.as_str(), self.platform.as_str())
    }

    #[must_use]
    pub fn all_canonical() -> [Self; 4] {
        [Self::AARCH64_DARWIN, Self::X86_64_DARWIN, Self::X86_64_LINUX, Self::AARCH64_LINUX]
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_nix_system())
    }
}

// ── Hostname ────────────────────────────────────────────────────────────────

/// A validated hostname. Lowercase ASCII letters/digits/hyphens/periods, ≤ 253 chars total,
/// each label ≤ 63 chars (RFC 1123). IP addresses are also accepted since some fleet
/// entries (rai, cid-k3s, ryn-k3s) use raw IPs as their ssh hostname.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hostname(String);

impl Hostname {
    pub const MAX_TOTAL: usize = 253;
    pub const MAX_LABEL: usize = 63;

    /// Validate and construct. Returns Err if the hostname is malformed.
    pub fn new(s: impl Into<String>) -> Result<Self, HostnameError> {
        let s = s.into();
        if s.is_empty() {
            return Err(HostnameError::Empty);
        }
        if s.len() > Self::MAX_TOTAL {
            return Err(HostnameError::TooLong(s.len()));
        }
        // Allow IPv4 dotted-quad.
        if s.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Ok(Self(s));
        }
        for label in s.split('.') {
            if label.is_empty() || label.len() > Self::MAX_LABEL {
                return Err(HostnameError::InvalidLabel(label.to_string()));
            }
            for (i, c) in label.chars().enumerate() {
                let ok = c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || (c == '-' && i != 0 && i != label.len() - 1);
                if !ok {
                    return Err(HostnameError::InvalidChar(c));
                }
            }
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Short name — the first label before the first period.
    #[must_use]
    pub fn short(&self) -> &str {
        self.0.split('.').next().unwrap_or(&self.0)
    }
}

impl fmt::Display for Hostname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostnameError {
    Empty,
    TooLong(usize),
    InvalidLabel(String),
    InvalidChar(char),
}

impl fmt::Display for HostnameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("hostname cannot be empty"),
            Self::TooLong(n) => write!(f, "hostname too long ({n} > 253 chars)"),
            Self::InvalidLabel(l) => write!(f, "invalid hostname label: {l:?}"),
            Self::InvalidChar(c) => write!(f, "invalid hostname character: {c:?}"),
        }
    }
}

impl std::error::Error for HostnameError {}

// ── IPv4 + CIDR ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IpV4Address(pub [u8; 4]);

impl IpV4Address {
    pub fn parse(s: &str) -> Result<Self, CidrError> {
        let mut octets = [0u8; 4];
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return Err(CidrError::InvalidAddress(s.to_string()));
        }
        for (i, p) in parts.iter().enumerate() {
            octets[i] = p.parse().map_err(|_| CidrError::InvalidAddress(s.to_string()))?;
        }
        Ok(Self(octets))
    }

    #[must_use]
    pub fn as_u32(self) -> u32 {
        u32::from_be_bytes(self.0)
    }
}

impl fmt::Display for IpV4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

/// An IPv4 CIDR block (e.g. `10.100.1.0/24`). The stored `addr` is the masked network base;
/// the original address bits beyond the prefix are zeroed on construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IpV4Cidr {
    addr: IpV4Address,
    prefix: u8,
}

impl IpV4Cidr {
    pub fn parse(s: &str) -> Result<Self, CidrError> {
        let (a, p) = s
            .split_once('/')
            .ok_or_else(|| CidrError::InvalidFormat(s.to_string()))?;
        let prefix: u8 = p.parse().map_err(|_| CidrError::InvalidPrefix(p.to_string()))?;
        if prefix > 32 {
            return Err(CidrError::InvalidPrefix(p.to_string()));
        }
        let addr = IpV4Address::parse(a)?;
        // Mask off host bits to normalize.
        let mask = if prefix == 0 { 0 } else { !0u32 << (32 - prefix) };
        let base = addr.as_u32() & mask;
        let normalized = IpV4Address(base.to_be_bytes());
        Ok(Self { addr: normalized, prefix })
    }

    #[must_use]
    pub fn prefix(self) -> u8 {
        self.prefix
    }

    #[must_use]
    pub fn network(self) -> IpV4Address {
        self.addr
    }

    /// Highest address in the block (the broadcast address for /24 etc).
    #[must_use]
    pub fn broadcast(self) -> IpV4Address {
        let mask = if self.prefix == 0 { 0 } else { !0u32 << (32 - self.prefix) };
        let high = self.addr.as_u32() | !mask;
        IpV4Address(high.to_be_bytes())
    }

    /// True if `other` overlaps with this CIDR (either contains the other, or shares any addresses).
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        let a_lo = self.addr.as_u32();
        let a_hi = self.broadcast().as_u32();
        let b_lo = other.addr.as_u32();
        let b_hi = other.broadcast().as_u32();
        a_lo <= b_hi && b_lo <= a_hi
    }

    /// True if this CIDR contains the given address.
    #[must_use]
    pub fn contains(self, addr: IpV4Address) -> bool {
        let a = addr.as_u32();
        a >= self.addr.as_u32() && a <= self.broadcast().as_u32()
    }
}

impl fmt::Display for IpV4Cidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.addr, self.prefix)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CidrError {
    InvalidAddress(String),
    InvalidFormat(String),
    InvalidPrefix(String),
}

impl fmt::Display for CidrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAddress(a) => write!(f, "invalid IPv4 address: {a:?}"),
            Self::InvalidFormat(s) => write!(f, "invalid CIDR format: {s:?}"),
            Self::InvalidPrefix(p) => write!(f, "invalid CIDR prefix: {p:?}"),
        }
    }
}

impl std::error::Error for CidrError {}

// ── WireGuard interface name ────────────────────────────────────────────────

/// A WireGuard interface name. Linux kernel limits interface names to 15 chars (IFNAMSIZ-1).
/// pleme-io convention prefixes with `wg-`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WireguardInterface(String);

impl WireguardInterface {
    pub const MAX_LEN: usize = 15;
    pub const PREFIX: &'static str = "wg-";

    pub fn new(s: impl Into<String>) -> Result<Self, WireguardError> {
        let s = s.into();
        if s.is_empty() {
            return Err(WireguardError::Empty);
        }
        if s.len() > Self::MAX_LEN {
            return Err(WireguardError::TooLong(s.len()));
        }
        if !s.starts_with(Self::PREFIX) {
            return Err(WireguardError::MissingPrefix);
        }
        for c in s.chars() {
            if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err(WireguardError::InvalidChar(c));
            }
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WireguardInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireguardError {
    Empty,
    TooLong(usize),
    MissingPrefix,
    InvalidChar(char),
}

impl fmt::Display for WireguardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("interface name cannot be empty"),
            Self::TooLong(n) => write!(f, "interface name too long ({n} > 15 chars)"),
            Self::MissingPrefix => f.write_str("interface name must start with 'wg-'"),
            Self::InvalidChar(c) => write!(f, "invalid interface character: {c:?}"),
        }
    }
}

impl std::error::Error for WireguardError {}

// ── Content-addressable identity ────────────────────────────────────────────

/// Stable hash for the identity of any value. Uses the default std hasher —
/// deterministic within a single binary run, stable across runs when the inputs
/// are stable. Not cryptographic; upgrade to BLAKE3 if adversarial resistance
/// is required.
#[must_use]
pub fn identity_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_canonical_triples_render() {
        assert_eq!(Target::AARCH64_DARWIN.as_nix_system(), "aarch64-darwin");
        assert_eq!(Target::X86_64_LINUX.as_nix_system(), "x86_64-linux");
        assert_eq!(Target::AARCH64_LINUX.as_nix_system(), "aarch64-linux");
        assert_eq!(Target::X86_64_DARWIN.as_nix_system(), "x86_64-darwin");
    }

    #[test]
    fn target_has_four_canonical_systems() {
        assert_eq!(Target::all_canonical().len(), 4);
    }

    #[test]
    fn hostname_accepts_valid_names() {
        for name in ["plo", "ryn-k3s", "plo.quero.lan", "orion.lilitu.io"] {
            Hostname::new(name).expect(name);
        }
    }

    #[test]
    fn hostname_accepts_ipv4() {
        Hostname::new("192.168.64.2").unwrap();
        Hostname::new("10.0.0.1").unwrap();
    }

    #[test]
    fn hostname_rejects_uppercase() {
        assert!(matches!(Hostname::new("PLO"), Err(HostnameError::InvalidChar(_))));
    }

    #[test]
    fn hostname_rejects_empty() {
        assert_eq!(Hostname::new(""), Err(HostnameError::Empty));
    }

    #[test]
    fn hostname_rejects_leading_hyphen() {
        assert!(matches!(Hostname::new("-bad"), Err(HostnameError::InvalidChar(_))));
    }

    #[test]
    fn hostname_short_truncates_fqdn() {
        assert_eq!(Hostname::new("plo.quero.lan").unwrap().short(), "plo");
    }

    #[test]
    fn cidr_parses_24() {
        let cidr = IpV4Cidr::parse("10.100.1.0/24").unwrap();
        assert_eq!(cidr.prefix(), 24);
        assert_eq!(cidr.network().to_string(), "10.100.1.0");
        assert_eq!(cidr.broadcast().to_string(), "10.100.1.255");
    }

    #[test]
    fn cidr_normalizes_host_bits() {
        // 10.100.1.42/24 should mask to 10.100.1.0/24
        let cidr = IpV4Cidr::parse("10.100.1.42/24").unwrap();
        assert_eq!(cidr.network().to_string(), "10.100.1.0");
    }

    #[test]
    fn cidr_overlaps_disjoint() {
        let a = IpV4Cidr::parse("10.100.1.0/24").unwrap();
        let b = IpV4Cidr::parse("10.100.2.0/24").unwrap();
        assert!(!a.overlaps(b));
    }

    #[test]
    fn cidr_overlaps_self() {
        let a = IpV4Cidr::parse("10.100.1.0/24").unwrap();
        assert!(a.overlaps(a));
    }

    #[test]
    fn cidr_overlaps_superset() {
        let a = IpV4Cidr::parse("10.0.0.0/8").unwrap();
        let b = IpV4Cidr::parse("10.100.1.0/24").unwrap();
        assert!(a.overlaps(b));
        assert!(b.overlaps(a));
    }

    #[test]
    fn cidr_contains_address() {
        let cidr = IpV4Cidr::parse("10.100.1.0/24").unwrap();
        assert!(cidr.contains(IpV4Address::parse("10.100.1.1").unwrap()));
        assert!(cidr.contains(IpV4Address::parse("10.100.1.255").unwrap()));
        assert!(!cidr.contains(IpV4Address::parse("10.100.2.0").unwrap()));
    }

    #[test]
    fn wg_interface_accepts_valid() {
        let iface = WireguardInterface::new("wg-ryn-k3s").unwrap();
        assert_eq!(iface.as_str(), "wg-ryn-k3s");
    }

    #[test]
    fn wg_interface_rejects_no_prefix() {
        assert_eq!(WireguardInterface::new("ryn-k3s"), Err(WireguardError::MissingPrefix));
    }

    #[test]
    fn wg_interface_rejects_too_long() {
        // 16 chars
        let n = "wg-0123456789012";
        assert_eq!(n.len(), 16);
        assert!(matches!(WireguardInterface::new(n), Err(WireguardError::TooLong(_))));
    }

    #[test]
    fn wg_interface_accepts_15_chars() {
        let n = "wg-012345678901"; // 15 chars
        assert_eq!(n.len(), 15);
        WireguardInterface::new(n).unwrap();
    }

    #[test]
    fn identity_hash_deterministic() {
        let h1 = identity_hash(&"quero.lol");
        let h2 = identity_hash(&"quero.lol");
        assert_eq!(h1, h2);
    }

    #[test]
    fn identity_hash_differs() {
        let h1 = identity_hash(&"quero.lol");
        let h2 = identity_hash(&"pleme.io");
        assert_ne!(h1, h2);
    }
}
