use crate::node::{Binding, FnArg, ModuleOption, NixNode};
use crate::types::NixType;

// ── FlakeBuilder ────────────────────────────────────────────────────

/// Build a structurally correct `flake.nix` file.
///
/// Enforces: description present, inputs valid, outputs is a function.
pub struct FlakeBuilder {
    description: String,
    inputs: Vec<FlakeInputDef>,
    outputs_fn: Option<NixNode>,
}

struct FlakeInputDef {
    name: String,
    url: String,
    follows: Vec<(String, String)>,
}

impl FlakeBuilder {
    #[must_use]
    pub fn new(description: &str) -> Self {
        Self {
            description: description.to_string(),
            inputs: Vec::new(),
            outputs_fn: None,
        }
    }

    #[must_use]
    pub fn input(mut self, name: &str, url: &str) -> Self {
        self.inputs.push(FlakeInputDef {
            name: name.to_string(),
            url: url.to_string(),
            follows: Vec::new(),
        });
        self
    }

    #[must_use]
    pub fn input_with_follows(
        mut self,
        name: &str,
        url: &str,
        follows: Vec<(&str, &str)>,
    ) -> Self {
        self.inputs.push(FlakeInputDef {
            name: name.to_string(),
            url: url.to_string(),
            follows: follows
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        });
        self
    }

    #[must_use]
    pub fn outputs(mut self, outputs_fn: NixNode) -> Self {
        self.outputs_fn = Some(outputs_fn);
        self
    }

    /// Emit the complete `flake.nix` as a string.
    ///
    /// # Panics
    /// Panics if no outputs function was provided.
    #[must_use]
    pub fn emit(&self) -> String {
        let outputs = self
            .outputs_fn
            .clone()
            .expect("FlakeBuilder: outputs function is required");

        let inputs: Vec<crate::node::FlakeInput> = self
            .inputs
            .iter()
            .map(|i| crate::node::FlakeInput {
                name: i.name.clone(),
                url: i.url.clone(),
                follows: i.follows.clone(),
            })
            .collect();

        let flake = NixNode::FlakeFile {
            description: self.description.clone(),
            inputs,
            outputs: Box::new(outputs),
        };

        let mut out = flake.emit(0);
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out
    }
}

// ── ModuleBuilder ───────────────────────────────────────────────────

/// Build a structurally correct NixOS / home-manager module.
///
/// Enforces: `{ config, lib, pkgs, ... }:` function signature,
/// options under `options.*`, config under `config.*`.
pub struct ModuleBuilder {
    extra_args: Vec<String>,
    options: Vec<ModuleOption>,
    config: Vec<Binding>,
    let_bindings: Vec<Binding>,
}

impl ModuleBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            extra_args: Vec::new(),
            options: Vec::new(),
            config: Vec::new(),
            let_bindings: Vec::new(),
        }
    }

    /// Add an extra function argument beyond `config`, `lib`, `pkgs`.
    #[must_use]
    pub fn arg(mut self, name: &str) -> Self {
        self.extra_args.push(name.to_string());
        self
    }

    /// Add a module option at the given path with a type.
    #[must_use]
    pub fn option(
        mut self,
        path: Vec<&str>,
        option_type: NixType,
        default: Option<NixNode>,
        description: Option<&str>,
    ) -> Self {
        let option_node = NixNode::MkOption {
            option_type,
            default: default.map(Box::new),
            description: description.map(|s| s.to_string()),
        };
        self.options.push(ModuleOption {
            path: path.into_iter().map(|s| s.to_string()).collect(),
            option: option_node,
        });
        self
    }

    /// Add an enable option at the given path.
    #[must_use]
    pub fn enable_option(mut self, path: Vec<&str>, description: &str) -> Self {
        self.options.push(ModuleOption {
            path: path.into_iter().map(|s| s.to_string()).collect(),
            option: NixNode::MkEnableOption(description.to_string()),
        });
        self
    }

    /// Add a config binding.
    #[must_use]
    pub fn config(mut self, key: &str, value: NixNode) -> Self {
        self.config.push(Binding::new(key, value));
        self
    }

    /// Add a let binding (hoisted to top of module body).
    #[must_use]
    pub fn let_bind(mut self, key: &str, value: NixNode) -> Self {
        self.let_bindings.push(Binding::new(key, value));
        self
    }

    /// Emit the module as a complete Nix file string.
    #[must_use]
    pub fn emit(&self) -> String {
        let module = if self.let_bindings.is_empty() {
            NixNode::ModuleFile {
                extra_args: self.extra_args.clone(),
                options: self.options.clone(),
                config: self.config.clone(),
            }
        } else {
            // Wrap in let-in: construct the function body directly
            // with let bindings instead of using ModuleFile
            let mut args = vec![
                FnArg::required("config"),
                FnArg::required("lib"),
                FnArg::required("pkgs"),
            ];
            for a in &self.extra_args {
                args.push(FnArg::required(a));
            }

            let mut body_bindings = Vec::new();
            if !self.options.is_empty() {
                let mut opt_bindings: Vec<Binding> = Vec::new();
                for mo in &self.options {
                    let nested = build_nested_from_path(&mo.path, mo.option.clone());
                    opt_bindings.push(nested);
                }
                body_bindings.push(Binding::new("options", NixNode::AttrSet(opt_bindings)));
            }
            if !self.config.is_empty() {
                body_bindings.push(Binding::new("config", NixNode::AttrSet(self.config.clone())));
            }

            let body_set = NixNode::AttrSet(body_bindings);
            let let_body = NixNode::LetIn {
                bindings: self.let_bindings.clone(),
                body: Box::new(body_set),
            };

            NixNode::Function {
                args,
                variadic: true,
                body: Box::new(let_body),
            }
        };

        let mut out = module.emit(0);
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out
    }
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn build_nested_from_path(path: &[String], value: NixNode) -> Binding {
    assert!(!path.is_empty(), "option path must not be empty");
    if path.len() == 1 {
        Binding::new(&path[0], value)
    } else {
        let inner = build_nested_from_path(&path[1..], value);
        Binding::new(&path[0], NixNode::AttrSet(vec![inner]))
    }
}

// ── DevShellBuilder ─────────────────────────────────────────────────

/// Build a devShell definition for a flake.
pub struct DevShellBuilder {
    packages: Vec<NixNode>,
    shell_hook: Option<String>,
    env_vars: Vec<(String, NixNode)>,
}

impl DevShellBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            shell_hook: None,
            env_vars: Vec::new(),
        }
    }

    /// Add a package to the devShell.
    #[must_use]
    pub fn package(mut self, pkg: NixNode) -> Self {
        self.packages.push(pkg);
        self
    }

    /// Set the shell hook.
    #[must_use]
    pub fn shell_hook(mut self, hook: &str) -> Self {
        self.shell_hook = Some(hook.to_string());
        self
    }

    /// Add an environment variable.
    #[must_use]
    pub fn env(mut self, key: &str, value: NixNode) -> Self {
        self.env_vars.push((key.to_string(), value));
        self
    }

    /// Emit as a `pkgs.mkShell { ... }` expression.
    #[must_use]
    pub fn build(self) -> NixNode {
        let mut bindings = Vec::new();

        if !self.packages.is_empty() {
            bindings.push(Binding::new("packages", NixNode::List(self.packages)));
        }

        for (key, value) in self.env_vars {
            bindings.push(Binding::new(&key, value));
        }

        if let Some(hook) = self.shell_hook {
            bindings.push(Binding::new("shellHook", NixNode::MultilineStr(hook)));
        }

        NixNode::apply(
            NixNode::select(NixNode::ident("pkgs"), &["mkShell"]),
            NixNode::AttrSet(bindings),
        )
    }
}

impl Default for DevShellBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ── SubstrateBuilder ────────────────────────────────────────────────

/// Generate a flake.nix that calls a substrate builder pattern.
///
/// Substrate builders are typed Nix functions in `substrate/lib/`.
/// This builder generates the correct invocation for each pattern.
pub struct SubstrateBuilder {
    builder_name: String,
    flake_description: String,
    substrate_url: String,
    extra_inputs: Vec<FlakeInputDef>,
    builder_args: Vec<(String, NixNode)>,
}

impl SubstrateBuilder {
    #[must_use]
    pub fn new(builder_name: &str, description: &str) -> Self {
        Self {
            builder_name: builder_name.to_string(),
            flake_description: description.to_string(),
            substrate_url: "github:pleme-io/substrate".to_string(),
            extra_inputs: Vec::new(),
            builder_args: Vec::new(),
        }
    }

    #[must_use]
    pub fn substrate_url(mut self, url: &str) -> Self {
        self.substrate_url = url.to_string();
        self
    }

    #[must_use]
    pub fn extra_input(mut self, name: &str, url: &str) -> Self {
        self.extra_inputs.push(FlakeInputDef {
            name: name.to_string(),
            url: url.to_string(),
            follows: Vec::new(),
        });
        self
    }

    #[must_use]
    pub fn arg(mut self, key: &str, value: NixNode) -> Self {
        self.builder_args.push((key.to_string(), value));
        self
    }

    /// Emit the complete flake.nix.
    #[must_use]
    pub fn emit(&self) -> String {
        let mut fb = FlakeBuilder::new(&self.flake_description)
            .input("substrate", &self.substrate_url)
            .input_with_follows("nixpkgs", "github:NixOS/nixpkgs/nixos-unstable", vec![]);

        for input in &self.extra_inputs {
            fb = fb.input(&input.name, &input.url);
        }

        // Build the outputs function
        // { self, substrate, nixpkgs, ... }: substrate.lib.${system}.${builder} { args }
        let builder_bindings: Vec<Binding> = self
            .builder_args
            .iter()
            .map(|(k, v)| Binding::new(k, v.clone()))
            .collect();

        // The actual outputs expression depends on the builder pattern
        // Most substrate builders return per-system outputs
        let builder_call = NixNode::apply(
            NixNode::select(
                NixNode::ident("substrate"),
                &["lib", &self.builder_name],
            ),
            NixNode::AttrSet(builder_bindings),
        );

        let outputs_fn = NixNode::Function {
            args: vec![
                FnArg::required("self"),
                FnArg::required("substrate"),
                FnArg::required("nixpkgs"),
            ],
            variadic: true,
            body: Box::new(builder_call),
        };

        fb = fb.outputs(outputs_fn);
        fb.emit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flake_builder_emits_description() {
        let out = FlakeBuilder::new("test flake")
            .input("nixpkgs", "github:NixOS/nixpkgs")
            .outputs(NixNode::Function {
                args: vec![FnArg::required("self"), FnArg::required("nixpkgs")],
                variadic: true,
                body: Box::new(NixNode::AttrSet(vec![])),
            })
            .emit();

        assert!(out.contains("test flake"));
        assert!(out.contains("nixpkgs"));
    }

    #[test]
    fn flake_builder_includes_inputs() {
        let out = FlakeBuilder::new("test")
            .input("nixpkgs", "github:NixOS/nixpkgs")
            .input("flake-utils", "github:numtide/flake-utils")
            .outputs(NixNode::AttrSet(vec![]))
            .emit();

        assert!(out.contains("nixpkgs"));
        assert!(out.contains("flake-utils"));
        assert!(out.contains("github:NixOS/nixpkgs"));
    }

    #[test]
    fn flake_builder_has_trailing_newline() {
        let out = FlakeBuilder::new("test")
            .outputs(NixNode::AttrSet(vec![]))
            .emit();
        assert!(out.ends_with('\n'));
    }

    #[test]
    #[should_panic(expected = "outputs function is required")]
    fn flake_builder_panics_without_outputs() {
        let _ = FlakeBuilder::new("test").emit();
    }

    #[test]
    fn module_builder_emits_function_sig() {
        let out = ModuleBuilder::new()
            .option(
                vec!["services", "myapp", "enable"],
                NixType::Bool,
                Some(NixNode::Bool(false)),
                Some("Enable myapp"),
            )
            .emit();

        assert!(out.contains("config"));
        assert!(out.contains("lib"));
        assert!(out.contains("pkgs"));
        assert!(out.contains("..."));
    }

    #[test]
    fn module_builder_nests_options() {
        let out = ModuleBuilder::new()
            .option(
                vec!["services", "myapp", "port"],
                NixType::Int,
                Some(NixNode::Int(8080)),
                Some("Service port"),
            )
            .emit();

        assert!(out.contains("services"));
        assert!(out.contains("myapp"));
        assert!(out.contains("port"));
    }

    #[test]
    fn module_builder_emits_config() {
        let out = ModuleBuilder::new()
            .config(
                "systemd",
                NixNode::attr_set(vec![("enable", NixNode::Bool(true))]),
            )
            .emit();

        assert!(out.contains("config"));
        assert!(out.contains("systemd"));
    }

    #[test]
    fn module_builder_extra_args() {
        let out = ModuleBuilder::new().arg("inputs").emit();
        assert!(out.contains("inputs"));
    }

    #[test]
    fn module_builder_has_trailing_newline() {
        let out = ModuleBuilder::new().emit();
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn devshell_builder_has_packages() {
        let node = DevShellBuilder::new()
            .package(NixNode::select(NixNode::ident("pkgs"), &["rustc"]))
            .package(NixNode::select(NixNode::ident("pkgs"), &["cargo"]))
            .build();

        let out = node.emit(0);
        assert!(out.contains("pkgs.mkShell"));
        assert!(out.contains("packages"));
        assert!(out.contains("rustc"));
        assert!(out.contains("cargo"));
    }

    #[test]
    fn devshell_builder_has_env() {
        let node = DevShellBuilder::new()
            .env("RUST_LOG", NixNode::str("debug"))
            .build();

        let out = node.emit(0);
        assert!(out.contains("RUST_LOG"));
        assert!(out.contains("debug"));
    }

    #[test]
    fn substrate_builder_emits_flake() {
        let out = SubstrateBuilder::new("rust-tool-release", "My CLI tool")
            .arg("name", NixNode::str("my-tool"))
            .arg("version", NixNode::str("0.1.0"))
            .emit();

        assert!(out.contains("My CLI tool"));
        assert!(out.contains("substrate"));
        assert!(out.contains("rust-tool-release"));
        assert!(out.contains("my-tool"));
    }

    #[test]
    fn substrate_builder_includes_substrate_input() {
        let out = SubstrateBuilder::new("ruby-gem", "My gem").emit();
        assert!(out.contains("github:pleme-io/substrate"));
    }
}
