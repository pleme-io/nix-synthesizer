use nix_synthesizer::*;

// ── Every NixNode variant emits non-empty output ────────────────────

#[test]
fn comment_emits_nonempty() {
    let out = NixNode::Comment("test".into()).emit(0);
    assert!(!out.is_empty());
    assert!(out.starts_with('#'));
}

#[test]
fn blank_emits_empty_string() {
    assert_eq!(NixNode::Blank.emit(0), "");
}

#[test]
fn str_emits_quoted() {
    let out = NixNode::Str("hello".into()).emit(0);
    assert!(out.starts_with('"'));
    assert!(out.ends_with('"'));
}

#[test]
fn multiline_str_emits_double_tick() {
    let out = NixNode::MultilineStr("line1\nline2".into()).emit(0);
    assert!(out.contains("''"));
}

#[test]
fn int_emits_number() {
    assert_eq!(NixNode::Int(42).emit(0), "42");
    assert_eq!(NixNode::Int(-1).emit(0), "-1");
    assert_eq!(NixNode::Int(0).emit(0), "0");
}

#[test]
fn bool_emits_lowercase() {
    assert_eq!(NixNode::Bool(true).emit(0), "true");
    assert_eq!(NixNode::Bool(false).emit(0), "false");
}

#[test]
fn null_emits_null() {
    assert_eq!(NixNode::Null.emit(0), "null");
}

#[test]
fn path_emits_verbatim() {
    assert_eq!(NixNode::Path("./foo".into()).emit(0), "./foo");
    assert_eq!(NixNode::Path("/absolute".into()).emit(0), "/absolute");
}

#[test]
fn ident_emits_name() {
    assert_eq!(NixNode::Ident("pkgs".into()).emit(0), "pkgs");
}

#[test]
fn select_emits_dotted_path() {
    let node = NixNode::Select {
        expr: Box::new(NixNode::ident("pkgs")),
        path: vec!["lib".into(), "mkOption".into()],
    };
    assert_eq!(node.emit(0), "pkgs.lib.mkOption");
}

#[test]
fn select_or_emits_with_default() {
    let node = NixNode::SelectOr {
        expr: Box::new(NixNode::ident("config")),
        path: vec!["services".into(), "x".into()],
        default: Box::new(NixNode::Null),
    };
    let out = node.emit(0);
    assert!(out.contains("or"));
    assert!(out.contains("null"));
}

#[test]
fn attr_set_empty_emits_braces() {
    assert_eq!(NixNode::AttrSet(vec![]).emit(0), "{ }");
}

#[test]
fn attr_set_with_bindings() {
    let node = NixNode::AttrSet(vec![
        Binding::new("x", NixNode::Int(1)),
        Binding::new("y", NixNode::str("hello")),
    ]);
    let out = node.emit(0);
    assert!(out.contains("x = 1;"));
    assert!(out.contains("y = \"hello\";"));
}

#[test]
fn rec_attr_set_has_rec_prefix() {
    let node = NixNode::RecAttrSet(vec![
        Binding::new("x", NixNode::Int(1)),
    ]);
    let out = node.emit(0);
    assert!(out.starts_with("rec {"));
}

#[test]
fn rec_attr_set_empty() {
    assert_eq!(NixNode::RecAttrSet(vec![]).emit(0), "rec { }");
}

#[test]
fn list_empty() {
    assert_eq!(NixNode::List(vec![]).emit(0), "[ ]");
}

#[test]
fn list_singleton_inline() {
    let node = NixNode::List(vec![NixNode::Int(1)]);
    assert_eq!(node.emit(0), "[ 1 ]");
}

#[test]
fn list_multi_element() {
    let node = NixNode::List(vec![NixNode::Int(1), NixNode::Int(2), NixNode::Int(3)]);
    let out = node.emit(0);
    assert!(out.starts_with('['));
    assert!(out.ends_with(']'));
    assert!(out.contains('1'));
    assert!(out.contains('2'));
    assert!(out.contains('3'));
}

#[test]
fn let_in_emits_let_and_in() {
    let node = NixNode::LetIn {
        bindings: vec![Binding::new("x", NixNode::Int(1))],
        body: Box::new(NixNode::ident("x")),
    };
    let out = node.emit(0);
    assert!(out.contains("let"));
    assert!(out.contains("in"));
    assert!(out.contains("x = 1;"));
}

#[test]
fn with_emits_inline() {
    let node = NixNode::With {
        expr: Box::new(NixNode::ident("lib")),
        body: Box::new(NixNode::ident("x")),
    };
    assert_eq!(node.emit(0), "with lib; x");
}

#[test]
fn inherit_emits_names() {
    let node = NixNode::Inherit(vec!["a".into(), "b".into(), "c".into()]);
    assert_eq!(node.emit(0), "inherit a b c;");
}

#[test]
fn inherit_from_emits_src_and_names() {
    let node = NixNode::InheritFrom {
        src: Box::new(NixNode::ident("pkgs")),
        names: vec!["hello".into(), "world".into()],
    };
    assert_eq!(node.emit(0), "inherit (pkgs) hello world;");
}

#[test]
fn function_emits_pattern_args() {
    let node = NixNode::Function {
        args: vec![
            FnArg::required("config"),
            FnArg::required("lib"),
            FnArg::required("pkgs"),
        ],
        variadic: true,
        body: Box::new(NixNode::AttrSet(vec![])),
    };
    let out = node.emit(0);
    assert!(out.contains("{ config, lib, pkgs, ... }:"));
}

#[test]
fn function_without_variadic() {
    let node = NixNode::Function {
        args: vec![FnArg::required("x")],
        variadic: false,
        body: Box::new(NixNode::ident("x")),
    };
    let out = node.emit(0);
    assert!(out.contains("{ x }:"));
    assert!(!out.contains("..."));
}

#[test]
fn function_arg_with_default() {
    let node = NixNode::Function {
        args: vec![FnArg::with_default("port", NixNode::Int(8080))],
        variadic: false,
        body: Box::new(NixNode::ident("port")),
    };
    let out = node.emit(0);
    assert!(out.contains("port ? 8080"));
}

#[test]
fn lambda_emits_simple_arg() {
    let node = NixNode::Lambda {
        arg: "x".into(),
        body: Box::new(NixNode::ident("x")),
    };
    assert_eq!(node.emit(0), "x: x");
}

#[test]
fn apply_emits_juxtaposition() {
    let node = NixNode::Apply {
        func: Box::new(NixNode::ident("f")),
        arg: Box::new(NixNode::Int(1)),
    };
    assert_eq!(node.emit(0), "f 1");
}

#[test]
fn apply_wraps_complex_args_in_parens() {
    let inner_apply = NixNode::Apply {
        func: Box::new(NixNode::ident("g")),
        arg: Box::new(NixNode::Int(1)),
    };
    let node = NixNode::Apply {
        func: Box::new(NixNode::ident("f")),
        arg: Box::new(inner_apply),
    };
    assert_eq!(node.emit(0), "f (g 1)");
}

#[test]
fn if_then_else_emits_branches() {
    let node = NixNode::If {
        cond: Box::new(NixNode::Bool(true)),
        then_body: Box::new(NixNode::Int(1)),
        else_body: Box::new(NixNode::Int(2)),
    };
    let out = node.emit(0);
    assert!(out.contains("if true then"));
    assert!(out.contains("else"));
}

#[test]
fn bin_op_all_operators() {
    let operators = vec![
        (BinOperator::Add, "+"),
        (BinOperator::Sub, "-"),
        (BinOperator::Mul, "*"),
        (BinOperator::Div, "/"),
        (BinOperator::Concat, "++"),
        (BinOperator::Update, "//"),
        (BinOperator::Eq, "=="),
        (BinOperator::Ne, "!="),
        (BinOperator::And, "&&"),
        (BinOperator::Or, "||"),
        (BinOperator::Lt, "<"),
        (BinOperator::Gt, ">"),
        (BinOperator::Le, "<="),
        (BinOperator::Ge, ">="),
        (BinOperator::Implies, "->"),
    ];
    for (op, sym) in operators {
        let node = NixNode::BinOp {
            left: Box::new(NixNode::ident("a")),
            op,
            right: Box::new(NixNode::ident("b")),
        };
        let out = node.emit(0);
        assert!(out.contains(sym), "operator {sym} not found in output: {out}");
    }
}

#[test]
fn interpolation_emits_dollar_brace() {
    let node = NixNode::Interpolation {
        parts: vec![
            StringPart::Literal("hello ".into()),
            StringPart::Expr(NixNode::ident("name")),
            StringPart::Literal("!".into()),
        ],
    };
    let out = node.emit(0);
    assert_eq!(out, "\"hello ${name}!\"");
}

#[test]
fn import_emits_import_keyword() {
    let node = NixNode::Import(Box::new(NixNode::Path("./module.nix".into())));
    assert_eq!(node.emit(0), "import ./module.nix");
}

#[test]
fn mk_option_emits_lib_mk_option() {
    let node = NixNode::MkOption {
        option_type: NixType::Str,
        default: Some(Box::new(NixNode::str(""))),
        description: Some("A string option".into()),
    };
    let out = node.emit(0);
    assert!(out.contains("lib.mkOption"));
    assert!(out.contains("types.str"));
    assert!(out.contains("A string option"));
}

#[test]
fn mk_enable_option_emits() {
    let node = NixNode::MkEnableOption("Enable myapp".into());
    let out = node.emit(0);
    assert!(out.contains("lib.mkEnableOption"));
    assert!(out.contains("Enable myapp"));
}

#[test]
fn module_file_emits_function_with_standard_args() {
    let node = NixNode::ModuleFile {
        extra_args: vec![],
        options: vec![ModuleOption {
            path: vec!["services".into(), "myapp".into(), "enable".into()],
            option: NixNode::MkEnableOption("Enable myapp".into()),
        }],
        config: vec![],
    };
    let out = node.emit(0);
    assert!(out.contains("config"));
    assert!(out.contains("lib"));
    assert!(out.contains("pkgs"));
    assert!(out.contains("..."));
    assert!(out.contains("options"));
    assert!(out.contains("services"));
}

#[test]
fn module_file_with_extra_args() {
    let node = NixNode::ModuleFile {
        extra_args: vec!["inputs".into()],
        options: vec![],
        config: vec![Binding::new("x", NixNode::Int(1))],
    };
    let out = node.emit(0);
    assert!(out.contains("inputs"));
}

#[test]
fn flake_file_emits_description_inputs_outputs() {
    let node = NixNode::FlakeFile {
        description: "My flake".into(),
        inputs: vec![FlakeInput {
            name: "nixpkgs".into(),
            url: "github:NixOS/nixpkgs".into(),
            follows: vec![],
        }],
        outputs: Box::new(NixNode::Function {
            args: vec![FnArg::required("self"), FnArg::required("nixpkgs")],
            variadic: true,
            body: Box::new(NixNode::AttrSet(vec![])),
        }),
    };
    let out = node.emit(0);
    assert!(out.contains("My flake"));
    assert!(out.contains("nixpkgs"));
    assert!(out.contains("github:NixOS/nixpkgs"));
    assert!(out.contains("outputs"));
}

#[test]
fn flake_input_with_follows() {
    let node = NixNode::FlakeFile {
        description: "test".into(),
        inputs: vec![FlakeInput {
            name: "substrate".into(),
            url: "github:pleme-io/substrate".into(),
            follows: vec![("nixpkgs".into(), "nixpkgs".into())],
        }],
        outputs: Box::new(NixNode::AttrSet(vec![])),
    };
    let out = node.emit(0);
    assert!(out.contains("substrate"));
    assert!(out.contains("github:pleme-io/substrate"));
}

#[test]
fn raw_emits_verbatim() {
    let out = NixNode::Raw("builtins.fetchTarball { }".into()).emit(0);
    assert_eq!(out, "builtins.fetchTarball { }");
}

// ── String escaping proofs ──────────────────────────────────────────

#[test]
fn str_escapes_double_quotes() {
    let out = NixNode::str(r#"say "hi""#).emit(0);
    assert!(out.contains(r#"\""#));
}

#[test]
fn str_escapes_backslashes() {
    let out = NixNode::str(r"path\to\file").emit(0);
    assert!(out.contains(r"\\"));
}

#[test]
fn str_escapes_interpolation_syntax() {
    let out = NixNode::str("${x}").emit(0);
    assert!(out.contains(r"\${"));
}

#[test]
fn multiline_str_escapes_double_tick() {
    let out = NixNode::MultilineStr("don't use '' here".into()).emit(0);
    assert!(out.contains("'''"));
}

#[test]
fn multiline_str_escapes_interpolation() {
    let out = NixNode::MultilineStr("${x}".into()).emit(0);
    assert!(out.contains("''${"));
}

// ── Indentation proofs ──────────────────────────────────────────────

#[test]
fn indent_level_0_no_prefix() {
    let out = NixNode::Comment("test".into()).emit(0);
    assert!(out.starts_with("# "));
}

#[test]
fn indent_level_1_two_spaces() {
    let out = NixNode::Comment("test".into()).emit(1);
    assert!(out.starts_with("  # "));
}

#[test]
fn indent_level_2_four_spaces() {
    let out = NixNode::Comment("test".into()).emit(2);
    assert!(out.starts_with("    # "));
}

#[test]
fn indent_level_3_six_spaces() {
    let out = NixNode::Comment("test".into()).emit(3);
    assert!(out.starts_with("      # "));
}

// ── NixType variant proofs ──────────────────────────────────────────

#[test]
fn type_str() {
    assert_eq!(NixType::Str.emit(), "types.str");
}

#[test]
fn type_int() {
    assert_eq!(NixType::Int.emit(), "types.int");
}

#[test]
fn type_float() {
    assert_eq!(NixType::Float.emit(), "types.float");
}

#[test]
fn type_bool() {
    assert_eq!(NixType::Bool.emit(), "types.bool");
}

#[test]
fn type_path() {
    assert_eq!(NixType::Path.emit(), "types.path");
}

#[test]
fn type_package() {
    assert_eq!(NixType::Package.emit(), "types.package");
}

#[test]
fn type_attrs() {
    assert_eq!(NixType::Attrs.emit(), "types.attrs");
}

#[test]
fn type_anything() {
    assert_eq!(NixType::Anything.emit(), "types.anything");
}

#[test]
fn type_list_of_simple() {
    assert_eq!(NixType::list_of(NixType::Str).emit(), "types.listOf types.str");
}

#[test]
fn type_list_of_compound_gets_parens() {
    let ty = NixType::list_of(NixType::list_of(NixType::Int));
    assert_eq!(ty.emit(), "types.listOf (types.listOf types.int)");
}

#[test]
fn type_attrs_of() {
    assert_eq!(NixType::attrs_of(NixType::Str).emit(), "types.attrsOf types.str");
}

#[test]
fn type_enum() {
    assert_eq!(
        NixType::enum_of(vec!["tcp", "udp"]).emit(),
        r#"types.enum [ "tcp" "udp" ]"#
    );
}

#[test]
fn type_null_or() {
    assert_eq!(NixType::null_or(NixType::Str).emit(), "types.nullOr types.str");
}

#[test]
fn type_null_or_idempotent() {
    let once = NixType::null_or(NixType::Str);
    let twice = NixType::null_or(once.clone());
    assert_eq!(once, twice);
    assert_eq!(once.emit(), twice.emit());
}

#[test]
fn type_either() {
    let ty = NixType::one_of(vec![NixType::Str, NixType::Int]);
    assert_eq!(ty.emit(), "types.either types.str types.int");
}

#[test]
fn type_one_of_single_degenerates() {
    let ty = NixType::one_of(vec![NixType::Str]);
    assert_eq!(ty, NixType::Str);
}

#[test]
fn type_one_of_three_plus() {
    let ty = NixType::one_of(vec![NixType::Str, NixType::Int, NixType::Bool]);
    assert_eq!(ty.emit(), "types.oneOf [ types.str types.int types.bool ]");
}

#[test]
#[should_panic(expected = "0 variants")]
fn type_one_of_zero_panics() {
    let _ = NixType::one_of(vec![]);
}

#[test]
fn type_submodule_with_options() {
    let ty = NixType::Submodule(vec![SubmoduleOption {
        name: "port".into(),
        option_type: NixType::Int,
        default: Some(NixNode::Int(8080)),
        description: Some("Port number".into()),
    }]);
    let out = ty.emit();
    assert!(out.contains("types.submodule"));
    assert!(out.contains("port"));
    assert!(out.contains("types.int"));
    assert!(out.contains("8080"));
}

#[test]
fn type_submodule_empty() {
    let ty = NixType::Submodule(vec![]);
    assert_eq!(ty.emit(), "types.submodule { }");
}

#[test]
fn type_raw() {
    let ty = NixType::Raw("types.functionTo types.str".into());
    assert_eq!(ty.emit(), "types.functionTo types.str");
}
