use nix_synthesizer::*;
use nix_synthesizer::builders::*;

// ── FlakeBuilder structural proofs ──────────────────────────────────

#[test]
fn flake_structure_description_inputs_outputs() {
    let out = FlakeBuilder::new("test flake")
        .input("nixpkgs", "github:NixOS/nixpkgs")
        .input("flake-utils", "github:numtide/flake-utils")
        .outputs(NixNode::AttrSet(vec![]))
        .emit();

    // Must contain all three top-level keys
    assert!(out.contains("description"), "missing description");
    assert!(out.contains("inputs"), "missing inputs");
    assert!(out.contains("outputs"), "missing outputs");
}

#[test]
fn flake_input_url_present() {
    let out = FlakeBuilder::new("test")
        .input("nixpkgs", "github:NixOS/nixpkgs/nixos-unstable")
        .outputs(NixNode::AttrSet(vec![]))
        .emit();
    assert!(out.contains("github:NixOS/nixpkgs/nixos-unstable"));
}

#[test]
fn flake_follows_present() {
    let out = FlakeBuilder::new("test")
        .input_with_follows(
            "substrate",
            "github:pleme-io/substrate",
            vec![("nixpkgs", "nixpkgs")],
        )
        .outputs(NixNode::AttrSet(vec![]))
        .emit();
    assert!(out.contains("substrate"));
}

#[test]
fn flake_multiple_inputs() {
    let out = FlakeBuilder::new("test")
        .input("a", "github:a/a")
        .input("b", "github:b/b")
        .input("c", "github:c/c")
        .outputs(NixNode::AttrSet(vec![]))
        .emit();
    assert!(out.contains("github:a/a"));
    assert!(out.contains("github:b/b"));
    assert!(out.contains("github:c/c"));
}

// ── ModuleBuilder structural proofs ─────────────────────────────────

#[test]
fn module_standard_args_always_present() {
    let out = ModuleBuilder::new().emit();
    assert!(out.contains("config"));
    assert!(out.contains("lib"));
    assert!(out.contains("pkgs"));
}

#[test]
fn module_variadic_always() {
    let out = ModuleBuilder::new().emit();
    assert!(out.contains("..."));
}

#[test]
fn module_nested_options_path() {
    let out = ModuleBuilder::new()
        .option(
            vec!["services", "myapp", "port"],
            NixType::Int,
            Some(NixNode::Int(8080)),
            Some("TCP port"),
        )
        .emit();
    // All path segments present
    assert!(out.contains("services"));
    assert!(out.contains("myapp"));
    assert!(out.contains("port"));
    // Type and default present
    assert!(out.contains("types.int"));
    assert!(out.contains("8080"));
}

#[test]
fn module_enable_option() {
    let out = ModuleBuilder::new()
        .enable_option(vec!["services", "myapp", "enable"], "Enable my application")
        .emit();
    assert!(out.contains("mkEnableOption"));
    assert!(out.contains("Enable my application"));
}

#[test]
fn module_config_section() {
    let out = ModuleBuilder::new()
        .config("environment", NixNode::attr_set(vec![
            ("RUST_LOG", NixNode::str("info")),
        ]))
        .emit();
    assert!(out.contains("config"));
    assert!(out.contains("environment"));
    assert!(out.contains("RUST_LOG"));
}

#[test]
fn module_both_options_and_config() {
    let out = ModuleBuilder::new()
        .option(vec!["x"], NixType::Str, None, None)
        .config("y", NixNode::Bool(true))
        .emit();
    assert!(out.contains("options"));
    assert!(out.contains("config"));
}

#[test]
fn module_let_bindings() {
    let out = ModuleBuilder::new()
        .let_bind("cfg", NixNode::select(NixNode::ident("config"), &["services", "myapp"]))
        .config("x", NixNode::ident("cfg"))
        .emit();
    assert!(out.contains("let"));
    assert!(out.contains("cfg"));
}

#[test]
fn module_extra_args() {
    let out = ModuleBuilder::new()
        .arg("inputs")
        .arg("self")
        .emit();
    assert!(out.contains("inputs"));
    assert!(out.contains("self"));
}

// ── SubstrateBuilder structural proofs ──────────────────────────────

#[test]
fn substrate_has_substrate_input() {
    let out = SubstrateBuilder::new("rust-tool-release", "tool").emit();
    assert!(out.contains("substrate"));
    assert!(out.contains("github:pleme-io/substrate"));
}

#[test]
fn substrate_has_nixpkgs_input() {
    let out = SubstrateBuilder::new("rust-tool-release", "tool").emit();
    assert!(out.contains("nixpkgs"));
}

#[test]
fn substrate_calls_builder_by_name() {
    let out = SubstrateBuilder::new("rust-workspace-release", "ws").emit();
    assert!(out.contains("rust-workspace-release"));
}

#[test]
fn substrate_custom_url() {
    let out = SubstrateBuilder::new("ruby-gem", "gem")
        .substrate_url("github:pleme-io/substrate/dev")
        .emit();
    assert!(out.contains("github:pleme-io/substrate/dev"));
}

#[test]
fn substrate_extra_inputs() {
    let out = SubstrateBuilder::new("rust-tool-release", "tool")
        .extra_input("crate2nix", "github:nix-community/crate2nix")
        .emit();
    assert!(out.contains("crate2nix"));
}

#[test]
fn substrate_builder_args() {
    let out = SubstrateBuilder::new("rust-tool-release", "tool")
        .arg("toolName", NixNode::str("my-tool"))
        .arg("src", NixNode::ident("self"))
        .emit();
    assert!(out.contains("toolName"));
    assert!(out.contains("my-tool"));
}

// ── DevShellBuilder structural proofs ───────────────────────────────

#[test]
fn devshell_mk_shell() {
    let out = DevShellBuilder::new().build().emit(0);
    assert!(out.contains("pkgs.mkShell"));
}

#[test]
fn devshell_packages() {
    let out = DevShellBuilder::new()
        .package(NixNode::select(NixNode::ident("pkgs"), &["rustc"]))
        .package(NixNode::select(NixNode::ident("pkgs"), &["cargo"]))
        .build()
        .emit(0);
    assert!(out.contains("packages"));
    assert!(out.contains("rustc"));
    assert!(out.contains("cargo"));
}

#[test]
fn devshell_env_vars() {
    let out = DevShellBuilder::new()
        .env("RUST_LOG", NixNode::str("debug"))
        .env("CARGO_TARGET_DIR", NixNode::str("./target"))
        .build()
        .emit(0);
    assert!(out.contains("RUST_LOG"));
    assert!(out.contains("CARGO_TARGET_DIR"));
}

#[test]
fn devshell_shell_hook() {
    let out = DevShellBuilder::new()
        .shell_hook("echo hello")
        .build()
        .emit(0);
    assert!(out.contains("shellHook"));
    assert!(out.contains("echo hello"));
}

// ── Real-world generation patterns ──────────────────────────────────

#[test]
fn realistic_nixos_module() {
    let out = ModuleBuilder::new()
        .enable_option(vec!["services", "vector", "enable"], "Enable Vector log aggregator")
        .option(
            vec!["services", "vector", "configFile"],
            NixType::Path,
            None,
            Some("Path to Vector config"),
        )
        .option(
            vec!["services", "vector", "extraArgs"],
            NixType::list_of(NixType::Str),
            Some(NixNode::List(vec![])),
            Some("Extra command-line arguments"),
        )
        .config("systemd", NixNode::attr_set(vec![
            ("services", NixNode::attr_set(vec![
                ("vector", NixNode::attr_set(vec![
                    ("enable", NixNode::Bool(true)),
                ])),
            ])),
        ]))
        .emit();

    // All options present
    assert!(out.contains("enable"));
    assert!(out.contains("configFile"));
    assert!(out.contains("extraArgs"));
    assert!(out.contains("types.path"));
    assert!(out.contains("types.listOf types.str"));
    // Config section present
    assert!(out.contains("systemd"));
}

#[test]
fn realistic_flake_nix() {
    let out = FlakeBuilder::new("Vector log aggregator for pleme infrastructure")
        .input("nixpkgs", "github:NixOS/nixpkgs/nixos-unstable")
        .input_with_follows(
            "substrate",
            "github:pleme-io/substrate",
            vec![("nixpkgs", "nixpkgs")],
        )
        .outputs(NixNode::Function {
            args: vec![
                FnArg::required("self"),
                FnArg::required("nixpkgs"),
                FnArg::required("substrate"),
            ],
            variadic: true,
            body: Box::new(NixNode::attr_set(vec![
                ("packages", NixNode::attr_set(vec![
                    ("x86_64-linux", NixNode::attr_set(vec![
                        ("default", NixNode::ident("vector")),
                    ])),
                ])),
            ])),
        })
        .emit();

    assert!(out.contains("Vector log aggregator"));
    assert!(out.contains("nixpkgs"));
    assert!(out.contains("substrate"));
    assert!(out.contains("packages"));
    assert!(out.contains("x86_64-linux"));
}
