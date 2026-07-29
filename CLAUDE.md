# nix-synthesizer

> **★★★ CSE / Knowable Construction.** This repo operates under **Constructive Substrate Engineering** — canonical specification at [`pleme-io/theory/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md`](https://github.com/pleme-io/theory/blob/main/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md). The Compounding Directive (operational rules: solve once, load-bearing fixes only, idiom-first, models stay current, direction beats velocity) is in the org-level pleme-io/CLAUDE.md ★★★ section. Read both before non-trivial changes.


Typed AST for structurally correct Nix expression generation. Flake.nix, NixOS modules, home-manager modules, devShells, substrate builder invocations.

## Status: Proven, Structurally No-Raw (Wave 3)

## Core API

| Type | Purpose |
|------|---------|
| `NixNode` | Typed Nix AST variants including TypeExpr for embedding NixType in ASTs |
| `NixType` | 15 variants: Str, Int, Float, Bool, Path, Package, Attrs, Anything, ListOf, AttrsOf, Enum, NullOr, Submodule, OneOf, Either |
| `emit_file(&[NixNode])` | Emit nodes as complete Nix file |
| `Binding::inherit(&["a"])` / `Binding::inherit_from(src, &["a"])` | `inherit a;` inside an `AttrSet` or `LetIn` |

### `inherit` is a binding form, not a `key = value;` pair

It has no left-hand side and carries its own `;`, so `NixNode::Inherit` could be
emitted standalone but never *placed* — `AttrSet` and `LetIn` both take
`Binding`s, which left `{ inherit system; }` with no representation at all.
`Binding::inherit` fills that gap; `emit_binding` dispatches on the **value**
being an inherit node, so the (unused, empty) key cannot fall out of sync.

Desugaring to `{ system = system; }` was considered and rejected. The two mean
the same thing, but `nix-instantiate --parse` prints them as different trees, so
desugaring would forfeit AST-level equality against hand-written Nix — which is
precisely the oracle a conversion from hand-written source to this emitter needs.

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
