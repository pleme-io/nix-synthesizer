//! Self-validation — the typed registry is pinned to the real `pleme-io/nix`
//! repository on disk. These tests parse the actual nix source files (with
//! simple regex-based scanning, not a full nix parser) and verify the types
//! stay in sync.
//!
//! If the nix repo changes, these tests tell you exactly which entries in
//! `typescape/registry.rs` drifted.

use nix_synthesizer::typescape::registry::pleme_nix_registry;
use std::path::{Path, PathBuf};

const NIX_REPO: &str = "/Users/drzzln/code/github/pleme-io/nix";

fn nix_repo_path() -> PathBuf {
    PathBuf::from(NIX_REPO)
}

fn read_file(relative: &str) -> Option<String> {
    let p = nix_repo_path().join(relative);
    std::fs::read_to_string(&p).ok()
}

/// Skip the test body if the nix repo isn't present (e.g. running from CI outside this host).
macro_rules! skip_if_no_repo {
    () => {
        if !Path::new(NIX_REPO).exists() {
            eprintln!("skipping — {NIX_REPO} not present");
            return;
        }
    };
}

// ── nodes ──────────────────────────────────────────────────────────────────

#[test]
fn real_nodes_nix_defines_expected_nixos_nodes() {
    skip_if_no_repo!();
    let src = read_file("lib/nodes.nix").expect("lib/nodes.nix should exist");
    // Each node is declared as `<name> = { system = ...; hostname = ...; ... };`
    // at the top level of the returned attrset. Detect with a simple regex.
    let re = regex::Regex::new(r#"^\s{2}([a-z][a-z0-9-]*)\s*=\s*\{\s*$"#).unwrap();
    let mut found = Vec::<String>::new();
    for line in src.lines() {
        if let Some(cap) = re.captures(line) {
            found.push(cap[1].to_string());
        }
    }
    // The registry claims these NixOS nodes exist.
    for expected in ["plo", "zek", "orion", "rai", "wireguard", "cid-k3s", "ryn-k3s"] {
        assert!(
            found.contains(&expected.to_string()),
            "lib/nodes.nix does not define {expected:?}; found: {found:?}"
        );
    }
}

#[test]
fn real_nodes_nix_does_not_add_unknown_nixos_nodes() {
    skip_if_no_repo!();
    let src = read_file("lib/nodes.nix").expect("lib/nodes.nix should exist");
    let re = regex::Regex::new(r#"^\s{2}([a-z][a-z0-9-]*)\s*=\s*\{\s*$"#).unwrap();
    let mut found = Vec::<String>::new();
    for line in src.lines() {
        if let Some(cap) = re.captures(line) {
            found.push(cap[1].to_string());
        }
    }
    let reg = pleme_nix_registry();
    let registered: std::collections::HashSet<&str> = reg
        .nodes
        .iter()
        .filter(|n| n.is_nixos())
        .map(|n| n.short_name.as_str())
        .collect();

    for name in found {
        assert!(
            registered.contains(name.as_str()),
            "lib/nodes.nix declares {name:?} but the registry does not know about it"
        );
    }
}

#[test]
fn real_darwin_configurations_declare_expected_nodes() {
    skip_if_no_repo!();
    let src = read_file("darwinConfigurations/default.nix").expect("darwin configs exist");
    // Scan for `cid = mkDarwin { ... }` / `ryn = mkDarwin { ... }`.
    let re = regex::Regex::new(r#"^\s{2}([a-z][a-z0-9-]*)\s*=\s*mkDarwin"#).unwrap();
    let mut found = Vec::<String>::new();
    for line in src.lines() {
        if let Some(cap) = re.captures(line) {
            found.push(cap[1].to_string());
        }
    }
    for expected in ["cid", "ryn"] {
        assert!(found.contains(&expected.to_string()), "darwin configs do not include {expected:?}");
    }
    let reg = pleme_nix_registry();
    let registered_darwin: std::collections::HashSet<&str> = reg
        .nodes
        .iter()
        .filter(|n| n.is_darwin())
        .map(|n| n.short_name.as_str())
        .collect();
    for name in &found {
        assert!(
            registered_darwin.contains(name.as_str()),
            "darwin configs reference {name:?}, not in registry"
        );
    }
}

// ── vpn ────────────────────────────────────────────────────────────────────

#[test]
fn real_vpn_links_nix_declares_expected_links() {
    skip_if_no_repo!();
    let src = read_file("lib/vpn-links.nix").expect("lib/vpn-links.nix should exist");
    // Each link is `<name> = { interface = "wg-..."; ... };` — match on the
    // line with `interface = "wg-"`.
    let link_re = regex::Regex::new(r#"^\s{2}([a-z][a-z0-9-]*)\s*=\s*\{\s*$"#).unwrap();
    let iface_re = regex::Regex::new(r#"^\s+interface\s*=\s*"(wg-[a-z0-9-]+)""#).unwrap();

    let mut current_link: Option<String> = None;
    let mut links: Vec<(String, String)> = Vec::new();

    for line in src.lines() {
        if let Some(cap) = link_re.captures(line) {
            current_link = Some(cap[1].to_string());
        }
        if let Some(cap) = iface_re.captures(line) {
            if let Some(name) = current_link.take() {
                links.push((name, cap[1].to_string()));
            }
        }
    }

    assert!(!links.is_empty(), "no VPN links found in lib/vpn-links.nix");

    let reg = pleme_nix_registry();
    for (link_name, iface_name) in &links {
        let link = reg
            .vpn_links
            .iter()
            .find(|l| &l.name == link_name)
            .unwrap_or_else(|| panic!("registry missing link {link_name:?}"));
        assert_eq!(
            link.interface.as_str(),
            iface_name,
            "link {link_name:?} interface drift: registry={} repo={}",
            link.interface,
            iface_name
        );
    }
}

#[test]
fn real_vpn_links_use_slash_24_subnets() {
    skip_if_no_repo!();
    let src = read_file("lib/vpn-links.nix").expect("lib/vpn-links.nix should exist");
    let subnet_re = regex::Regex::new(r#"subnet\s*=\s*"10\.100\.\d+\.0/24""#).unwrap();
    let count = subnet_re.find_iter(&src).count();
    assert!(count >= 3, "expected >= 3 /24 subnets in lib/vpn-links.nix, found {count}");
}

// ── profiles ───────────────────────────────────────────────────────────────

#[test]
fn real_profiles_directory_matches_registered_profiles() {
    skip_if_no_repo!();
    let profiles_dir = nix_repo_path().join("profiles");
    assert!(profiles_dir.is_dir(), "profiles/ directory should exist");

    let mut found_dirs = Vec::new();
    for entry in std::fs::read_dir(&profiles_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            found_dirs.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    let reg = pleme_nix_registry();
    let registered: std::collections::HashSet<&str> =
        reg.profiles.iter().map(|p| p.name.as_str()).collect();

    // Every directory under profiles/ must be represented in the registry.
    for dir in &found_dirs {
        assert!(
            registered.contains(dir.as_str()),
            "profiles/{dir} exists on disk but no registry entry of that name"
        );
    }
}

// ── darwin darwinModules sanity ────────────────────────────────────────────

#[test]
fn real_darwin_modules_imports_blackmatter() {
    skip_if_no_repo!();
    let src = read_file("darwinConfigurations/default.nix").unwrap();
    assert!(
        src.contains("inputs.blackmatter.homeManagerModules.blackmatter"),
        "darwin configs should import blackmatter.homeManagerModules.blackmatter"
    );
}

#[test]
fn real_darwin_modules_imports_zoekt_and_codesearch_atomically() {
    skip_if_no_repo!();
    let src = read_file("darwinConfigurations/default.nix").unwrap();
    let has_zoekt = src.contains("zoekt-mcp.homeManagerModules.default");
    let has_codesearch = src.contains("codesearch.homeManagerModules.default");
    let has_indexing = src.contains("indexing.enable = true");
    assert!(has_zoekt, "darwin configs missing zoekt-mcp HM module");
    assert!(has_codesearch, "darwin configs missing codesearch HM module");
    assert!(has_indexing, "darwin configs missing indexing enable");
}

// ── nodes.nix imports blackmatter-secrets ──────────────────────────────────

#[test]
fn real_nodes_use_blackmatter_secrets() {
    skip_if_no_repo!();
    let src = read_file("lib/nodes.nix").unwrap();
    assert!(
        src.contains("blackmatter-secrets.homeManagerModules.default")
            || src.contains("blackmatter-secrets.nixosModules.default"),
        "lib/nodes.nix should import blackmatter-secrets"
    );
}

// ── substrate patterns ─────────────────────────────────────────────────────

fn sample_repo_flake(name: &str) -> Option<String> {
    let path = format!("/Users/drzzln/code/github/pleme-io/{name}/flake.nix");
    std::fs::read_to_string(path).ok()
}

#[test]
fn real_tobira_flake_uses_rust_tool_release() {
    let Some(src) = sample_repo_flake("tobira") else {
        eprintln!("skipping — tobira not on disk");
        return;
    };
    assert!(
        src.contains("rust-tool-release-flake.nix"),
        "tobira/flake.nix should import rust-tool-release-flake.nix"
    );
}

#[test]
fn real_mamorigami_flake_uses_workspace_release() {
    let Some(src) = sample_repo_flake("mamorigami") else {
        eprintln!("skipping — mamorigami not on disk");
        return;
    };
    assert!(
        src.contains("rust-workspace-release-flake.nix") || src.contains("workspace-release"),
        "mamorigami/flake.nix should import rust-workspace-release-flake.nix"
    );
}

#[test]
fn real_irodori_flake_uses_rust_library() {
    let Some(src) = sample_repo_flake("irodori") else {
        eprintln!("skipping — irodori not on disk");
        return;
    };
    assert!(
        src.contains("rust-library.nix") || src.contains("rustLibrary"),
        "irodori/flake.nix should use rust-library.nix"
    );
}

// ── Blackmatter repo structure ─────────────────────────────────────────────

#[test]
fn real_blackmatter_repos_exist() {
    let reg = pleme_nix_registry();
    let mut missing = Vec::new();
    for c in &reg.blackmatter_components {
        let path = format!("/Users/drzzln/code/github/pleme-io/{}", c.repo);
        if !Path::new(&path).exists() {
            missing.push(c.repo.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "registered blackmatter repos missing from disk: {missing:?}"
    );
}

// ── Canonical registry still consistent ────────────────────────────────────

#[test]
fn canonical_registry_has_no_invariant_violations() {
    let reg = pleme_nix_registry();
    let v = reg.all_violations();
    if !v.is_empty() {
        for violation in &v {
            eprintln!("  [{}] {}", violation.id.0, violation.message);
        }
        panic!("registry has {} invariant violations", v.len());
    }
}

#[test]
fn canonical_registry_summary_metrics_match_floor() {
    let reg = pleme_nix_registry();
    let s = reg.summary();
    assert_eq!(s.node_count, 10, "expected 10 nodes (8 nixos + 2 darwin)");
    assert_eq!(s.darwin_node_count, 2);
    assert_eq!(s.nixos_node_count, 8);
    assert_eq!(s.vpn_link_count, 3);
    assert!(s.cluster_count >= 3);
    assert!(s.blackmatter_component_count >= 20);
    assert!(s.substrate_builder_count >= 15);
    assert!(s.is_consistent);
    assert_eq!(s.violations_count, 0);
}
