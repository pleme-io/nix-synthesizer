# nix-synthesizer

Typed AST for structurally correct Nix expression generation. Flake.nix, NixOS modules, home-manager modules, devShells, substrate builder invocations.

## Status: Proven, Structurally No-Raw (Wave 3)

## Core API

| Type | Purpose |
|------|---------|
| `NixNode` | Typed Nix AST variants including TypeExpr for embedding NixType in ASTs |
| `NixType` | 15 variants: Str, Int, Float, Bool, Path, Package, Attrs, Anything, ListOf, AttrsOf, Enum, NullOr, Submodule, OneOf, Either |
| `emit_file(&[NixNode])` | Emit nodes as complete Nix file |

Use `TypeExpr` for type embeddings. Raw variants were removed in Wave 3 of the compound-knowledge refactor — invalid states are unrepresentable at the type level.

## Builders

- `FlakeBuilder` — `.input("nixpkgs", url).outputs(fn_node).emit()`
- `ModuleBuilder` — `.option(path, type, default, desc).config(key, value).emit()`
- `DevShellBuilder` — `.package(pkg).env("KEY", value).shell_hook("...").build()`
- `SubstrateBuilder` — `.new("rust-tool-release", desc).arg("name", value).emit()`

## IaC Bridge (feature: iac-bridge)

`iac_type_to_nix(ty: &IacType) -> NixType` — proven total, injective, deterministic.

## No-Raw Invariant

Structural: Raw variants do not exist on NixNode or NixType. The source-scan test in `tests/synthesizer_core_conformance.rs` is retained as a defensive guard against reintroduction.
