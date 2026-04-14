# nix-synthesizer

Typed AST for structurally correct Nix expression generation. Flake.nix, NixOS modules, home-manager modules, devShells, substrate builder invocations.

## Tests: 241 (with iac-bridge) | Status: Proven, Zero Raw in Production

## Core API

| Type | Purpose |
|------|---------|
| `NixNode` | 26+ variants including TypeExpr for embedding NixType in ASTs |
| `NixType` | 16 variants: Str, Int, Float, Bool, Path, Package, Attrs, Anything, ListOf, AttrsOf, Enum, NullOr, Submodule, OneOf, Either, Raw (deprecated) |
| `emit_file(&[NixNode])` | Emit nodes as complete Nix file |

`Raw` is **deprecated** on both NixNode and NixType. Use `TypeExpr` for type embeddings.

## Builders

- `FlakeBuilder` — `.input("nixpkgs", url).outputs(fn_node).emit()`
- `ModuleBuilder` — `.option(path, type, default, desc).config(key, value).emit()`
- `DevShellBuilder` — `.package(pkg).env("KEY", value).shell_hook("...").build()`
- `SubstrateBuilder` — `.new("rust-tool-release", desc).arg("name", value).emit()`

## IaC Bridge (feature: iac-bridge)

`iac_type_to_nix(ty: &IacType) -> NixType` — proven total, injective, deterministic.

## No-Raw Invariant

Test scans production source for NixNode::Raw and NixType::Raw constructors → assert zero.
