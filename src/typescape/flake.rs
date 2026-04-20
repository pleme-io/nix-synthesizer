//! Flake input registry — what the nix repo pulls in and how it follows nixpkgs.

/// A flake input. The pleme-io convention requires `inputs.nixpkgs.follows = "nixpkgs"`
/// for every pleme-io input (and for every sub-input of the aggregator), so the whole
/// tree ends up with one nixpkgs snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlakeInput {
    pub name: String,
    pub url: FlakeInputUrl,
    /// Inputs this input has `inputs.<x>.follows = "<x>"` set on at the root.
    pub follows: Vec<String>,
    pub origin: InputOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FlakeInputUrl {
    GitHub { org: String, repo: String, branch: Option<String> },
    GitLab { org: String, repo: String },
    Tarball(String),
    Indirect(String),
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputOrigin {
    /// Owned by pleme-io (starts with `pleme-io/`).
    PlemeIo,
    /// A blackmatter-* subrepo (starts with `pleme-io/blackmatter-*`).
    Blackmatter,
    /// Nix community / nixpkgs / flake-utils.
    NixCommunity,
    /// Third-party (determinate, sops-nix if external, etc.).
    ThirdParty,
}

impl FlakeInput {
    #[must_use]
    pub fn new(name: &str, url: FlakeInputUrl, origin: InputOrigin) -> Self {
        Self {
            name: name.to_string(),
            url,
            follows: Vec::new(),
            origin,
        }
    }

    #[must_use]
    pub fn follows(mut self, names: &[&str]) -> Self {
        self.follows = names.iter().map(|s| (*s).to_string()).collect();
        self
    }

    #[must_use]
    pub fn follows_nixpkgs(&self) -> bool {
        self.follows.iter().any(|f| f == "nixpkgs")
    }

    #[must_use]
    pub fn is_pleme(&self) -> bool {
        matches!(self.origin, InputOrigin::PlemeIo | InputOrigin::Blackmatter)
    }

    #[must_use]
    pub fn is_blackmatter(&self) -> bool {
        matches!(self.origin, InputOrigin::Blackmatter)
    }
}

impl FlakeInputUrl {
    #[must_use]
    pub fn pleme_gh(repo: &str) -> Self {
        Self::GitHub { org: "pleme-io".to_string(), repo: repo.to_string(), branch: None }
    }

    #[must_use]
    pub fn nix_community_gh(repo: &str) -> Self {
        Self::GitHub { org: "nix-community".to_string(), repo: repo.to_string(), branch: None }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pleme_input_is_pleme() {
        let input = FlakeInput::new(
            "substrate",
            FlakeInputUrl::pleme_gh("substrate"),
            InputOrigin::PlemeIo,
        )
        .follows(&["nixpkgs"]);
        assert!(input.is_pleme());
        assert!(!input.is_blackmatter());
        assert!(input.follows_nixpkgs());
    }

    #[test]
    fn blackmatter_input_detects_origin() {
        let input = FlakeInput::new(
            "blackmatter-shell",
            FlakeInputUrl::pleme_gh("blackmatter-shell"),
            InputOrigin::Blackmatter,
        );
        assert!(input.is_pleme());
        assert!(input.is_blackmatter());
    }

    #[test]
    fn nix_community_input_is_not_pleme() {
        let input = FlakeInput::new(
            "home-manager",
            FlakeInputUrl::nix_community_gh("home-manager"),
            InputOrigin::NixCommunity,
        );
        assert!(!input.is_pleme());
    }
}
