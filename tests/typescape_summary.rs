//! Single test that prints the canonical typescape summary to stderr. Useful
//! for `cargo test typescape_summary -- --nocapture` to see the digest.

use nix_synthesizer::typescape::{invariants::ALL_INVARIANTS, registry::pleme_nix_registry};

#[test]
fn print_canonical_summary() {
    let reg = pleme_nix_registry();
    let s = reg.summary();
    eprintln!("\n{s}");
    eprintln!(
        "  invariants declared: {} ({} checked against registry)",
        s.invariants_total,
        ALL_INVARIANTS.len()
    );
    assert!(s.is_consistent);
}
