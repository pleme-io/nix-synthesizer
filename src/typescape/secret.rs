//! Secret registry — SOPS/Akeyless path convention and backend metadata.
//!
//! The path convention across the fleet is `category/subcategory?/name`,
//! e.g. `github/ghcr-token`, `cid/kubernetes/plo/token`, `ryn/wireguard/ryn-k3s/psk`.
//! Both SOPS (committed to git, encrypted with age) and Akeyless (cloud API) use
//! the same string format — the backend is a parameter.

use std::fmt;

/// A secret path like `"ryn/wireguard/ryn-k3s/private-key"`. Validated on
/// construction — 2..=5 slash-separated lowercase/digit/hyphen labels.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretPath(String);

impl SecretPath {
    pub const MIN_DEPTH: usize = 2;
    pub const MAX_DEPTH: usize = 5;

    pub fn new(s: impl Into<String>) -> Result<Self, SecretPathError> {
        let s = s.into();
        if s.is_empty() {
            return Err(SecretPathError::Empty);
        }
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() < Self::MIN_DEPTH {
            return Err(SecretPathError::TooShallow(parts.len()));
        }
        if parts.len() > Self::MAX_DEPTH {
            return Err(SecretPathError::TooDeep(parts.len()));
        }
        for part in &parts {
            if part.is_empty() {
                return Err(SecretPathError::EmptyComponent);
            }
            for (i, c) in part.chars().enumerate() {
                let ok = c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || (c == '-' && i != 0 && i != part.len() - 1)
                    || c == '.';
                if !ok {
                    return Err(SecretPathError::InvalidChar(c));
                }
            }
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn category(&self) -> &str {
        self.0.split('/').next().unwrap_or(&self.0)
    }

    #[must_use]
    pub fn depth(&self) -> usize {
        self.0.split('/').count()
    }
}

impl fmt::Display for SecretPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretPathError {
    Empty,
    EmptyComponent,
    TooShallow(usize),
    TooDeep(usize),
    InvalidChar(char),
}

impl fmt::Display for SecretPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("secret path cannot be empty"),
            Self::EmptyComponent => f.write_str("secret path has empty component"),
            Self::TooShallow(n) => write!(f, "secret path too shallow ({n} < 2 components)"),
            Self::TooDeep(n) => write!(f, "secret path too deep ({n} > 5 components)"),
            Self::InvalidChar(c) => write!(f, "invalid secret path character: {c:?}"),
        }
    }
}

impl std::error::Error for SecretPathError {}

/// Secret backend selection — the unified blackmatter-secrets interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretBackend {
    /// sops-nix with age — committed to git, decrypted on activation.
    Sops,
    /// Akeyless cloud vault — fetched at activation, offline-cacheable.
    Akeyless,
}

impl SecretBackend {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sops => "sops",
            Self::Akeyless => "akeyless",
        }
    }

    /// Akeyless convention: every secret path is prefixed with `/pleme` when
    /// stored in the vault.
    #[must_use]
    pub fn akeyless_path_prefix() -> &'static str {
        "/pleme"
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_two_component_path() {
        let p = SecretPath::new("github/token").unwrap();
        assert_eq!(p.depth(), 2);
        assert_eq!(p.category(), "github");
    }

    #[test]
    fn accepts_nested_path() {
        let p = SecretPath::new("ryn/wireguard/ryn-k3s/private-key").unwrap();
        assert_eq!(p.depth(), 4);
        assert_eq!(p.category(), "ryn");
    }

    #[test]
    fn accepts_max_depth() {
        let p = SecretPath::new("a/b/c/d/e").unwrap();
        assert_eq!(p.depth(), 5);
    }

    #[test]
    fn rejects_too_shallow() {
        assert_eq!(SecretPath::new("github"), Err(SecretPathError::TooShallow(1)));
    }

    #[test]
    fn rejects_too_deep() {
        assert_eq!(SecretPath::new("a/b/c/d/e/f"), Err(SecretPathError::TooDeep(6)));
    }

    #[test]
    fn rejects_uppercase() {
        assert!(matches!(SecretPath::new("GitHub/Token"), Err(SecretPathError::InvalidChar(_))));
    }

    #[test]
    fn rejects_empty_component() {
        assert_eq!(SecretPath::new("a//b"), Err(SecretPathError::EmptyComponent));
    }

    #[test]
    fn backend_names_are_stable() {
        assert_eq!(SecretBackend::Sops.as_str(), "sops");
        assert_eq!(SecretBackend::Akeyless.as_str(), "akeyless");
    }

    #[test]
    fn akeyless_prefix_is_pleme() {
        assert_eq!(SecretBackend::akeyless_path_prefix(), "/pleme");
    }
}
