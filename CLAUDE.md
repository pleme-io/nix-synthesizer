# nix-synthesizer

Typed AST for structurally correct Nix expression generation. Generates flake.nix, NixOS modules, home-manager modules, devShells, substrate builder invocations.

## Tests: 240 | Status: Proven

## Core API

| Type | Purpose |
|------|---------|
| `NixNode` | 25+ variants: Comment, Blank, Str, MultilineStr, Int, Bool, Null, Path, Ident, Select, SelectOr, AttrSet, RecAttrSet, List, LetIn, With, Inherit, InheritFrom, Function, Lambda, Apply, If, BinOp, Interpolation, Import, MkOption, MkEnableOption, ModuleFile, FlakeFile, FlakeInput, Raw |
| `NixType` | 16 variants: Str, Int, Float, Bool, Path, Package, Attrs, Anything, ListOf, AttrsOf, Enum, NullOr, Submodule, OneOf, Either, Raw |
| `emit_file(&[NixNode])` | Emit nodes as complete Nix file |

## Builders

- `FlakeBuilder` — `.input("nixpkgs", url).outputs(fn_node).emit()`
- `ModuleBuilder` — `.option(path, type, default, desc).config(key, value).emit()`
- `DevShellBuilder` — `.package(pkg).env("KEY", value).shell_hook("...").build()`
- `SubstrateBuilder` — `.new("rust-tool-release", desc).arg("name", value).emit()`

## IaC Bridge (feature: iac-bridge)

`iac_type_to_nix(ty: &IacType) -> NixType` — proven total, injective, deterministic.

## Type Algebra

- `NixType::null_or()` — idempotent
- `NixType::one_of()` — 1 variant degenerates, 2 → Either, 3+ → OneOf
- All base types emit distinctly (injectivity proven)

## Dependencies

- iac-forge (optional, feature: iac-bridge)
- proptest, regex (dev)
