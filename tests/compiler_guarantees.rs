use nix_synthesizer::*;
use nix_synthesizer::builders::*;

// ── Determinism: same AST → byte-identical output ───────────────────

#[test]
fn emit_file_deterministic() {
    let nodes = vec![
        NixNode::Comment("test".into()),
        NixNode::Blank,
        NixNode::attr_set(vec![
            ("x", NixNode::Int(1)),
            ("y", NixNode::str("hello")),
        ]),
    ];
    let a = emit_file(&nodes);
    let b = emit_file(&nodes);
    assert_eq!(a, b, "emit_file must be deterministic");
}

#[test]
fn node_emit_deterministic() {
    let node = NixNode::LetIn {
        bindings: vec![
            Binding::new("x", NixNode::Int(1)),
            Binding::new("y", NixNode::str("hello")),
        ],
        body: Box::new(NixNode::ident("x")),
    };
    let a = node.emit(0);
    let b = node.emit(0);
    assert_eq!(a, b, "node.emit must be deterministic");
}

#[test]
fn type_emit_deterministic() {
    let ty = NixType::list_of(NixType::null_or(NixType::Str));
    let a = ty.emit();
    let b = ty.emit();
    assert_eq!(a, b, "NixType.emit must be deterministic");
}

// ── Trailing newline ────────────────────────────────────────────────

#[test]
fn emit_file_always_trailing_newline() {
    assert!(emit_file(&[]).ends_with('\n'));
    assert!(emit_file(&[NixNode::Comment("x".into())]).ends_with('\n'));
    assert!(emit_file(&[NixNode::Int(42)]).ends_with('\n'));
}

#[test]
fn flake_builder_trailing_newline() {
    let out = FlakeBuilder::new("test")
        .outputs(NixNode::AttrSet(vec![]))
        .emit();
    assert!(out.ends_with('\n'));
}

#[test]
fn module_builder_trailing_newline() {
    let out = ModuleBuilder::new().emit();
    assert!(out.ends_with('\n'));
}

// ── Balanced delimiters ─────────────────────────────────────────────

fn count_char(s: &str, c: char) -> usize {
    s.chars().filter(|&ch| ch == c).count()
}

#[test]
fn attr_set_balanced_braces() {
    let node = NixNode::attr_set(vec![
        ("a", NixNode::attr_set(vec![("b", NixNode::Int(1))])),
        ("c", NixNode::List(vec![NixNode::Int(2)])),
    ]);
    let out = node.emit(0);
    assert_eq!(
        count_char(&out, '{'),
        count_char(&out, '}'),
        "braces must be balanced: {out}"
    );
}

#[test]
fn list_balanced_brackets() {
    let node = NixNode::List(vec![
        NixNode::List(vec![NixNode::Int(1)]),
        NixNode::List(vec![NixNode::Int(2), NixNode::Int(3)]),
    ]);
    let out = node.emit(0);
    assert_eq!(
        count_char(&out, '['),
        count_char(&out, ']'),
        "brackets must be balanced: {out}"
    );
}

#[test]
fn interpolation_balanced_quotes() {
    let node = NixNode::Interpolation {
        parts: vec![
            StringPart::Literal("a".into()),
            StringPart::Expr(NixNode::ident("x")),
            StringPart::Literal("b".into()),
        ],
    };
    let out = node.emit(0);
    // Outer quotes should be balanced (2 total — opening and closing)
    assert_eq!(count_char(&out, '"'), 2, "quotes must be balanced: {out}");
}

#[test]
fn let_in_has_both_keywords() {
    let node = NixNode::LetIn {
        bindings: vec![Binding::new("x", NixNode::Int(1))],
        body: Box::new(NixNode::ident("x")),
    };
    let out = node.emit(0);
    assert!(out.contains("let"), "must contain 'let'");
    assert!(out.contains("in"), "must contain 'in'");
}

#[test]
fn if_then_else_has_all_keywords() {
    let node = NixNode::If {
        cond: Box::new(NixNode::Bool(true)),
        then_body: Box::new(NixNode::Int(1)),
        else_body: Box::new(NixNode::Int(2)),
    };
    let out = node.emit(0);
    assert!(out.contains("if"), "must contain 'if'");
    assert!(out.contains("then"), "must contain 'then'");
    assert!(out.contains("else"), "must contain 'else'");
}

// ── Indentation consistency ─────────────────────────────────────────

#[test]
fn two_space_indentation() {
    let node = NixNode::attr_set(vec![("x", NixNode::Int(1))]);
    let out = node.emit(0);
    // Inner binding at indent=1 should have exactly 2 spaces
    for line in out.lines() {
        if line.starts_with(' ') {
            let spaces = line.len() - line.trim_start().len();
            assert_eq!(spaces % 2, 0, "indentation must be multiples of 2: '{line}'");
        }
    }
}

#[test]
fn nested_indentation_increases() {
    let node = NixNode::attr_set(vec![
        ("outer", NixNode::attr_set(vec![("inner", NixNode::Int(1))])),
    ]);
    let out = node.emit(0);
    let lines: Vec<&str> = out.lines().collect();
    // Find the line with "inner"
    let inner_line = lines.iter().find(|l| l.contains("inner")).unwrap();
    let outer_line = lines.iter().find(|l| l.contains("outer")).unwrap();
    let inner_indent = inner_line.len() - inner_line.trim_start().len();
    let outer_indent = outer_line.len() - outer_line.trim_start().len();
    assert!(
        inner_indent > outer_indent,
        "inner must be more indented than outer"
    );
}

// ── Clean ASCII ─────────────────────────────────────────────────────

#[test]
fn no_control_characters_in_output() {
    let nodes = vec![
        NixNode::Comment("test".into()),
        NixNode::attr_set(vec![("x", NixNode::str("hello"))]),
        NixNode::List(vec![NixNode::Int(1)]),
    ];
    let out = emit_file(&nodes);
    for ch in out.chars() {
        if ch.is_control() {
            assert!(
                ch == '\n' || ch == '\t',
                "unexpected control character: {:?}",
                ch
            );
        }
    }
}

#[test]
fn no_trailing_whitespace_on_lines() {
    let nodes = vec![
        NixNode::Comment("test".into()),
        NixNode::Blank,
        NixNode::attr_set(vec![("x", NixNode::Int(1))]),
    ];
    let out = emit_file(&nodes);
    for (i, line) in out.lines().enumerate() {
        if !line.is_empty() {
            assert!(
                !line.ends_with(' ') && !line.ends_with('\t'),
                "line {} has trailing whitespace: '{}'",
                i + 1,
                line
            );
        }
    }
}

// ── Module structure guarantees ─────────────────────────────────────

#[test]
fn module_always_has_standard_args() {
    let out = ModuleBuilder::new()
        .option(vec!["x"], NixType::Str, None, None)
        .emit();
    assert!(out.contains("config"), "module must have 'config' arg");
    assert!(out.contains("lib"), "module must have 'lib' arg");
    assert!(out.contains("pkgs"), "module must have 'pkgs' arg");
    assert!(out.contains("..."), "module must be variadic");
}

#[test]
fn module_options_under_options_key() {
    let out = ModuleBuilder::new()
        .option(vec!["services", "x"], NixType::Bool, None, None)
        .emit();
    assert!(out.contains("options"), "module must have 'options' key");
}

#[test]
fn module_config_under_config_key() {
    let out = ModuleBuilder::new()
        .config("x", NixNode::Bool(true))
        .emit();
    assert!(out.contains("config"), "module must have 'config' key");
}

// ── Flake structure guarantees ──────────────────────────────────────

#[test]
fn flake_has_description() {
    let out = FlakeBuilder::new("My flake description")
        .outputs(NixNode::AttrSet(vec![]))
        .emit();
    assert!(out.contains("My flake description"));
}

#[test]
fn flake_has_inputs_section() {
    let out = FlakeBuilder::new("test")
        .input("nixpkgs", "github:NixOS/nixpkgs")
        .outputs(NixNode::AttrSet(vec![]))
        .emit();
    assert!(out.contains("inputs"));
    assert!(out.contains("nixpkgs"));
}

#[test]
fn flake_has_outputs_section() {
    let out = FlakeBuilder::new("test")
        .outputs(NixNode::AttrSet(vec![]))
        .emit();
    assert!(out.contains("outputs"));
}

// ── Substrate builder guarantees ────────────────────────────────────

#[test]
fn substrate_builder_includes_substrate_input() {
    let out = SubstrateBuilder::new("rust-tool-release", "tool").emit();
    assert!(out.contains("substrate"));
    assert!(out.contains("github:pleme-io/substrate"));
}

#[test]
fn substrate_builder_includes_nixpkgs() {
    let out = SubstrateBuilder::new("ruby-gem", "gem").emit();
    assert!(out.contains("nixpkgs"));
}

#[test]
fn substrate_builder_calls_correct_builder() {
    let out = SubstrateBuilder::new("rust-workspace-release", "ws tool").emit();
    assert!(out.contains("rust-workspace-release"));
}

#[test]
fn substrate_builder_passes_args() {
    let out = SubstrateBuilder::new("rust-tool-release", "tool")
        .arg("name", NixNode::str("my-tool"))
        .arg("version", NixNode::str("0.1.0"))
        .emit();
    assert!(out.contains("my-tool"));
    assert!(out.contains("0.1.0"));
}

// ── DevShell guarantees ─────────────────────────────────────────────

#[test]
fn devshell_uses_mkshell() {
    let out = DevShellBuilder::new()
        .package(NixNode::select(NixNode::ident("pkgs"), &["rustc"]))
        .build()
        .emit(0);
    assert!(out.contains("pkgs.mkShell"));
}

#[test]
fn devshell_has_packages_list() {
    let out = DevShellBuilder::new()
        .package(NixNode::ident("cargo"))
        .build()
        .emit(0);
    assert!(out.contains("packages"));
}
