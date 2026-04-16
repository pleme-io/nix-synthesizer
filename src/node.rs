use crate::types::NixType;

/// Every Nix language construct that nix-synthesizer can emit.
///
/// Nodes are pure data — no IO, no side effects. Construction is infallible
/// (invalid combinations are prevented by the type system). `emit()` is
/// deterministic: identical ASTs produce byte-identical Nix source.
#[derive(Debug, Clone, PartialEq)]
pub enum NixNode {
    // ── Pragmas & comments ──────────────────────────────────────────
    /// Single-line comment: `# text`
    Comment(String),
    /// Empty line separator
    Blank,

    // ── Literals ────────────────────────────────────────────────────
    /// String literal: `"text"` (auto-escapes inner quotes)
    Str(String),
    /// Multi-line string: `''text''`
    MultilineStr(String),
    /// Integer literal
    Int(i64),
    /// Boolean literal
    Bool(bool),
    /// Null literal
    Null,
    /// Path literal: `./relative` or `/absolute`
    Path(String),

    // ── Identifiers & references ────────────────────────────────────
    /// Bare identifier: `pkgs`, `lib`, `config`
    Ident(String),
    /// Dotted select: `pkgs.hello` or `lib.mkOption`
    Select {
        expr: Box<NixNode>,
        path: Vec<String>,
    },
    /// Select with default: `expr.attr or default`
    SelectOr {
        expr: Box<NixNode>,
        path: Vec<String>,
        default: Box<NixNode>,
    },

    // ── Collections ─────────────────────────────────────────────────
    /// Attribute set: `{ key = value; ... }`
    AttrSet(Vec<Binding>),
    /// Recursive attribute set: `rec { key = value; ... }`
    RecAttrSet(Vec<Binding>),
    /// List: `[ elem1 elem2 ... ]`
    List(Vec<NixNode>),

    // ── Bindings & declarations ─────────────────────────────────────
    /// Let-in expression: `let bindings in body`
    LetIn {
        bindings: Vec<Binding>,
        body: Box<NixNode>,
    },
    /// With expression: `with expr; body`
    With {
        expr: Box<NixNode>,
        body: Box<NixNode>,
    },
    /// Inherit: `inherit name1 name2;`
    Inherit(Vec<String>),
    /// Inherit from: `inherit (src) name1 name2;`
    InheritFrom {
        src: Box<NixNode>,
        names: Vec<String>,
    },

    // ── Functions ───────────────────────────────────────────────────
    /// Lambda with pattern arg: `{ arg1, arg2, ... }: body`
    Function {
        args: Vec<FnArg>,
        variadic: bool,
        body: Box<NixNode>,
    },
    /// Lambda with simple arg: `arg: body`
    Lambda {
        arg: String,
        body: Box<NixNode>,
    },
    /// Function application: `f arg`
    Apply {
        func: Box<NixNode>,
        arg: Box<NixNode>,
    },

    // ── Control flow ────────────────────────────────────────────────
    /// If-then-else: `if cond then a else b`
    If {
        cond: Box<NixNode>,
        then_body: Box<NixNode>,
        else_body: Box<NixNode>,
    },

    // ── Operators ───────────────────────────────────────────────────
    /// Binary operator: `a OP b`
    BinOp {
        left: Box<NixNode>,
        op: BinOperator,
        right: Box<NixNode>,
    },
    /// String interpolation: `"prefix${expr}suffix"`
    Interpolation {
        parts: Vec<StringPart>,
    },

    // ── Imports ─────────────────────────────────────────────────────
    /// Import expression: `import ./path`
    Import(Box<NixNode>),

    // ── NixOS module domain nodes ───────────────────────────────────
    /// `lib.mkOption { type = ...; default = ...; description = ...; }`
    MkOption {
        option_type: NixType,
        default: Option<Box<NixNode>>,
        description: Option<String>,
    },
    /// `lib.mkEnableOption "description"`
    MkEnableOption(String),
    /// Full NixOS/home-manager module file:
    /// `{ config, lib, pkgs, ... }: { options = ...; config = ...; }`
    ModuleFile {
        extra_args: Vec<String>,
        options: Vec<ModuleOption>,
        config: Vec<Binding>,
    },

    // ── Flake domain nodes ──────────────────────────────────────────
    /// Complete flake.nix structure
    FlakeFile {
        description: String,
        inputs: Vec<FlakeInput>,
        outputs: Box<NixNode>,
    },
    /// Flake input declaration
    FlakeInput {
        name: String,
        url: String,
        follows: Vec<(String, String)>,
    },

    // ── Substrate / nixpkgs domain-specific nodes ───────────────────
    /// `pkgs.writeShellApplication { name; runtimeInputs; text; }` —
    /// the canonical wrapper for emitting a typed shell script as a Nix
    /// package. `runtime_inputs` are package identifiers (emitted as
    /// `pkgs.${ident}`). `text` is opaque shell (shell has no AST here).
    WriteShellApp {
        name: String,
        runtime_inputs: Vec<String>,
        text: String,
    },

    // ── Escape hatch ────────────────────────────────────────────────
    /// Type expression embedded in an AST — typed bridge from NixType.
    TypeExpr(String),
}

/// A single `key = value;` binding in an attribute set or let block.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub key: String,
    pub value: NixNode,
}

/// Function argument with optional default.
#[derive(Debug, Clone, PartialEq)]
pub struct FnArg {
    pub name: String,
    pub default: Option<NixNode>,
}

/// Binary operators.
#[derive(Debug, Clone, PartialEq)]
pub enum BinOperator {
    /// `+` (addition or string concat)
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `++` (list concat)
    Concat,
    /// `//` (attribute set merge)
    Update,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `->`  (logical implication)
    Implies,
}

/// Parts of an interpolated string.
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Literal text
    Literal(String),
    /// `${expr}` interpolation
    Expr(NixNode),
}

/// A module option declaration at a given path.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleOption {
    pub path: Vec<String>,
    pub option: NixNode,
}

/// A flake input with optional follows.
#[derive(Debug, Clone, PartialEq)]
pub struct FlakeInput {
    pub name: String,
    pub url: String,
    pub follows: Vec<(String, String)>,
}

// ── Constructors ────────────────────────────────────────────────────

impl NixNode {
    #[must_use]
    pub fn str(s: &str) -> Self {
        Self::Str(s.to_string())
    }

    #[must_use]
    pub fn ident(s: &str) -> Self {
        Self::Ident(s.to_string())
    }

    #[must_use]
    pub fn path(s: &str) -> Self {
        Self::Path(s.to_string())
    }

    #[must_use]
    pub fn select(expr: Self, path: &[&str]) -> Self {
        Self::Select {
            expr: Box::new(expr),
            path: path.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[must_use]
    pub fn apply(func: Self, arg: Self) -> Self {
        Self::Apply {
            func: Box::new(func),
            arg: Box::new(arg),
        }
    }

    #[must_use]
    pub fn bin_op(left: Self, op: BinOperator, right: Self) -> Self {
        Self::BinOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    #[must_use]
    pub fn attr_set(bindings: Vec<(&str, NixNode)>) -> Self {
        Self::AttrSet(
            bindings
                .into_iter()
                .map(|(k, v)| Binding {
                    key: k.to_string(),
                    value: v,
                })
                .collect(),
        )
    }

    #[must_use]
    pub fn let_in(bindings: Vec<(&str, NixNode)>, body: Self) -> Self {
        Self::LetIn {
            bindings: bindings
                .into_iter()
                .map(|(k, v)| Binding {
                    key: k.to_string(),
                    value: v,
                })
                .collect(),
            body: Box::new(body),
        }
    }

    #[must_use]
    pub fn with(expr: Self, body: Self) -> Self {
        Self::With {
            expr: Box::new(expr),
            body: Box::new(body),
        }
    }

    #[must_use]
    pub fn import(path: Self) -> Self {
        Self::Import(Box::new(path))
    }

    #[must_use]
    pub fn if_then_else(cond: Self, then_body: Self, else_body: Self) -> Self {
        Self::If {
            cond: Box::new(cond),
            then_body: Box::new(then_body),
            else_body: Box::new(else_body),
        }
    }

    #[must_use]
    pub fn interpolation(parts: Vec<StringPart>) -> Self {
        Self::Interpolation { parts }
    }
}

impl Binding {
    #[must_use]
    pub fn new(key: &str, value: NixNode) -> Self {
        Self {
            key: key.to_string(),
            value,
        }
    }
}

impl FnArg {
    #[must_use]
    pub fn required(name: &str) -> Self {
        Self {
            name: name.to_string(),
            default: None,
        }
    }

    #[must_use]
    pub fn with_default(name: &str, default: NixNode) -> Self {
        Self {
            name: name.to_string(),
            default: Some(default),
        }
    }
}

impl BinOperator {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Concat => "++",
            Self::Update => "//",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::And => "&&",
            Self::Or => "||",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::Implies => "->",
        }
    }
}

// ── Emit ────────────────────────────────────────────────────────────

impl NixNode {
    /// Emit this node as Nix source text at the given indentation level.
    /// Each indent level is 2 spaces.
    #[must_use]
    pub fn emit(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        match self {
            // Pragmas & comments
            Self::Comment(text) => format!("{pad}# {text}"),
            Self::Blank => String::new(),

            // Literals
            Self::Str(s) => {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace("${", "\\${");
                format!("{pad}\"{escaped}\"")
            }
            Self::MultilineStr(s) => {
                let escaped = s.replace("''", "'''").replace("${", "''${");
                format!("{pad}''\n{escaped}\n{pad}''")
            }
            Self::Int(n) => format!("{pad}{n}"),
            Self::Bool(b) => format!("{pad}{b}"),
            Self::Null => format!("{pad}null"),
            Self::Path(p) => format!("{pad}{p}"),

            // Identifiers
            Self::Ident(name) => format!("{pad}{name}"),
            Self::Select { expr, path } => {
                let base = expr.emit(0);
                format!("{pad}{base}.{}", path.join("."))
            }
            Self::SelectOr { expr, path, default } => {
                let base = expr.emit(0);
                let def = default.emit(0);
                format!("{pad}{base}.{} or {def}", path.join("."))
            }

            // Collections
            Self::AttrSet(bindings) => emit_attr_set(&pad, indent, bindings, false),
            Self::RecAttrSet(bindings) => emit_attr_set(&pad, indent, bindings, true),
            Self::List(elems) => {
                if elems.is_empty() {
                    return format!("{pad}[ ]");
                }
                if elems.len() == 1 && is_simple(&elems[0]) {
                    return format!("{pad}[ {} ]", elems[0].emit(0));
                }
                let mut out = format!("{pad}[\n");
                for elem in elems {
                    out.push_str(&elem.emit(indent + 1));
                    out.push('\n');
                }
                out.push_str(&format!("{pad}]"));
                out
            }

            // Bindings
            Self::LetIn { bindings, body } => {
                let mut out = format!("{pad}let\n");
                for b in bindings {
                    out.push_str(&emit_binding(indent + 1, b));
                }
                out.push_str(&format!("{pad}in\n"));
                out.push_str(&body.emit(indent));
                out
            }
            Self::With { expr, body } => {
                let e = expr.emit(0);
                let b = body.emit(0);
                format!("{pad}with {e}; {b}")
            }
            Self::Inherit(names) => {
                format!("{pad}inherit {};", names.join(" "))
            }
            Self::InheritFrom { src, names } => {
                let s = src.emit(0);
                format!("{pad}inherit ({s}) {};", names.join(" "))
            }

            // Functions
            Self::Function { args, variadic, body } => {
                let arg_strs: Vec<String> = args
                    .iter()
                    .map(|a| match &a.default {
                        Some(d) => format!("{} ? {}", a.name, d.emit(0)),
                        None => a.name.clone(),
                    })
                    .collect();
                let mut params = arg_strs.join(", ");
                if *variadic {
                    if !params.is_empty() {
                        params.push_str(", ");
                    }
                    params.push_str("...");
                }
                let b = body.emit(0);
                format!("{pad}{{ {params} }}:\n{b}")
            }
            Self::Lambda { arg, body } => {
                let b = body.emit(0);
                format!("{pad}{arg}: {b}")
            }
            Self::Apply { func, arg } => {
                let f = func.emit(0);
                let a = arg.emit(0);
                // Wrap complex args in parens
                if needs_parens(arg) {
                    format!("{pad}{f} ({a})")
                } else {
                    format!("{pad}{f} {a}")
                }
            }

            // Control flow
            Self::If { cond, then_body, else_body } => {
                let c = cond.emit(0);
                let t = then_body.emit(indent + 1);
                let e = else_body.emit(indent + 1);
                format!("{pad}if {c} then\n{t}\n{pad}else\n{e}")
            }

            // Operators
            Self::BinOp { left, op, right } => {
                let l = left.emit(0);
                let r = right.emit(0);
                format!("{pad}{l} {} {r}", op.as_str())
            }
            Self::Interpolation { parts } => {
                let mut out = format!("{pad}\"");
                for part in parts {
                    match part {
                        StringPart::Literal(s) => {
                            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                            out.push_str(&escaped);
                        }
                        StringPart::Expr(e) => {
                            out.push_str("${");
                            out.push_str(&e.emit(0));
                            out.push('}');
                        }
                    }
                }
                out.push('"');
                out
            }

            // Imports
            Self::Import(path) => {
                let p = path.emit(0);
                format!("{pad}import {p}")
            }

            // NixOS module domain nodes
            Self::MkOption { option_type, default, description } => {
                let mut bindings = vec![Binding::new("type", option_type.to_node())];
                if let Some(d) = default {
                    bindings.push(Binding::new("default", *d.clone()));
                }
                if let Some(desc) = description {
                    bindings.push(Binding::new("description", NixNode::Str(desc.clone())));
                }
                let inner = NixNode::AttrSet(bindings);
                let mk = NixNode::apply(
                    NixNode::select(NixNode::ident("lib"), &["mkOption"]),
                    inner,
                );
                mk.emit(indent)
            }
            Self::MkEnableOption(desc) => {
                let mk = NixNode::apply(
                    NixNode::select(NixNode::ident("lib"), &["mkEnableOption"]),
                    NixNode::Str(desc.clone()),
                );
                mk.emit(indent)
            }
            Self::ModuleFile { extra_args, options, config } => {
                let mut args = vec![
                    FnArg::required("config"),
                    FnArg::required("lib"),
                    FnArg::required("pkgs"),
                ];
                for a in extra_args {
                    args.push(FnArg::required(a));
                }
                let mut body_bindings = Vec::new();

                // options
                if !options.is_empty() {
                    let mut opt_bindings: Vec<Binding> = Vec::new();
                    for mo in options {
                        let nested = build_nested_attrs(&mo.path, mo.option.clone());
                        opt_bindings.push(nested);
                    }
                    body_bindings.push(Binding::new("options", NixNode::AttrSet(opt_bindings)));
                }

                // config
                if !config.is_empty() {
                    body_bindings.push(Binding::new("config", NixNode::AttrSet(config.clone())));
                }

                let func = NixNode::Function {
                    args,
                    variadic: true,
                    body: Box::new(NixNode::AttrSet(body_bindings)),
                };
                func.emit(indent)
            }

            // Flake domain nodes
            Self::FlakeFile { description, inputs, outputs } => {
                let mut top_bindings = vec![
                    Binding::new("description", NixNode::Str(description.clone())),
                ];

                // inputs
                let mut input_bindings = Vec::new();
                for input in inputs {
                    let mut ib = vec![Binding::new("url", NixNode::Str(input.url.clone()))];
                    if !input.follows.is_empty() {
                        let mut follows_bindings = Vec::new();
                        for (name, target) in &input.follows {
                            // nixpkgs.follows = "nixpkgs" (not nixpkgs = "nixpkgs")
                            follows_bindings.push(Binding::new(name,
                                NixNode::AttrSet(vec![Binding::new("follows", NixNode::Str(target.clone()))])
                            ));
                        }
                        ib.push(Binding::new("inputs", NixNode::AttrSet(follows_bindings)));
                    }
                    input_bindings.push(Binding::new(&input.name, NixNode::AttrSet(ib)));
                }
                top_bindings.push(Binding::new("inputs", NixNode::AttrSet(input_bindings)));

                // outputs
                top_bindings.push(Binding::new("outputs", *outputs.clone()));

                NixNode::AttrSet(top_bindings).emit(indent)
            }
            Self::FlakeInput { name, url, follows } => {
                let mut bindings = vec![Binding::new("url", NixNode::Str(url.clone()))];
                if !follows.is_empty() {
                    let mut fb = Vec::new();
                    for (n, t) in follows {
                        fb.push(Binding::new(n, NixNode::Str(t.clone())));
                    }
                    bindings.push(Binding::new("inputs", NixNode::AttrSet(fb)));
                }
                let set = NixNode::AttrSet(bindings);
                let b = Binding::new(name, set);
                emit_binding(indent, &b)
            }

            // pkgs.writeShellApplication — typed shell wrapper.
            //
            // Nix `''...''` multi-line strings interpret `${...}` as interpolation,
            // so shell uses of `${VAR}` (which are common) must be escaped as
            // `''${VAR}`. We escape that at emit time so callers pass natural
            // shell syntax.
            Self::WriteShellApp { name, runtime_inputs, text } => {
                let inner_pad = "  ".repeat(indent + 1);
                let inputs_list = if runtime_inputs.is_empty() {
                    "[ ]".to_string()
                } else {
                    let items = runtime_inputs
                        .iter()
                        .map(|i| format!("pkgs.{i}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!("[ {items} ]")
                };
                // Escape shell-style ${...} for Nix multi-line string context.
                // (Shell `$VAR` stays as-is; only `${...}` needs the `''` prefix.)
                let escaped_text = text.replace("${", "''${");
                let text_inner_pad = "  ".repeat(indent + 2);
                let text_body = escaped_text
                    .lines()
                    .map(|l| {
                        if l.is_empty() {
                            String::new()
                        } else {
                            format!("{text_inner_pad}{l}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "{pad}pkgs.writeShellApplication {{\n\
                     {inner_pad}name = \"{name}\";\n\
                     {inner_pad}runtimeInputs = {inputs_list};\n\
                     {inner_pad}text = ''\n\
                     {text_body}\n\
                     {inner_pad}'';\n\
                     {pad}}}"
                )
            }

            // Escape hatch
            Self::TypeExpr(s) => format!("{pad}{s}"),
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn emit_attr_set(pad: &str, indent: usize, bindings: &[Binding], rec: bool) -> String {
    if bindings.is_empty() {
        return if rec {
            format!("{pad}rec {{ }}")
        } else {
            format!("{pad}{{ }}")
        };
    }
    let prefix = if rec { "rec " } else { "" };
    let mut out = format!("{pad}{prefix}{{\n");
    for b in bindings {
        out.push_str(&emit_binding(indent + 1, b));
    }
    out.push_str(&format!("{pad}}}"));
    out
}

fn emit_binding(indent: usize, binding: &Binding) -> String {
    let pad = "  ".repeat(indent);
    let value = binding.value.emit(0);

    // For multi-line values, format the value on the next line
    if value.contains('\n') {
        let indented_value = indent_multiline(&value, indent + 1);
        format!("{pad}{} =\n{};\n", binding.key, indented_value)
    } else {
        format!("{pad}{} = {};\n", binding.key, value)
    }
}

fn indent_multiline(s: &str, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    s.lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("{pad}{}", line.trim())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_simple(node: &NixNode) -> bool {
    matches!(
        node,
        NixNode::Str(_)
            | NixNode::Int(_)
            | NixNode::Bool(_)
            | NixNode::Null
            | NixNode::Path(_)
            | NixNode::Ident(_)
    )
}

fn needs_parens(node: &NixNode) -> bool {
    matches!(
        node,
        NixNode::Apply { .. }
            | NixNode::BinOp { .. }
            | NixNode::If { .. }
            | NixNode::LetIn { .. }
            | NixNode::With { .. }
            | NixNode::Lambda { .. }
            | NixNode::Function { .. }
    )
}

/// Build a nested attribute set from a dotted path.
/// `["a", "b", "c"]` with value `v` becomes `a = { b = { c = v; }; };`
fn build_nested_attrs(path: &[String], value: NixNode) -> Binding {
    assert!(!path.is_empty(), "module option path must not be empty");
    if path.len() == 1 {
        Binding::new(&path[0], value)
    } else {
        let inner = build_nested_attrs(&path[1..], value);
        Binding::new(&path[0], NixNode::AttrSet(vec![inner]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_emits_hash_prefix() {
        assert_eq!(NixNode::Comment("hello".into()).emit(0), "# hello");
    }

    #[test]
    fn blank_emits_empty_string() {
        assert_eq!(NixNode::Blank.emit(0), "");
    }

    #[test]
    fn string_escapes_quotes() {
        assert_eq!(NixNode::str("say \"hi\"").emit(0), r#""say \"hi\"""#);
    }

    #[test]
    fn string_escapes_interpolation() {
        assert_eq!(NixNode::str("${x}").emit(0), r#""\${x}""#);
    }

    #[test]
    fn int_emits_number() {
        assert_eq!(NixNode::Int(42).emit(0), "42");
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
        assert_eq!(NixNode::path("./foo/bar.nix").emit(0), "./foo/bar.nix");
    }

    #[test]
    fn ident_emits_name() {
        assert_eq!(NixNode::ident("pkgs").emit(0), "pkgs");
    }

    #[test]
    fn select_emits_dotted() {
        let node = NixNode::select(NixNode::ident("pkgs"), &["lib", "mkOption"]);
        assert_eq!(node.emit(0), "pkgs.lib.mkOption");
    }

    #[test]
    fn empty_attr_set_emits_braces() {
        assert_eq!(NixNode::AttrSet(vec![]).emit(0), "{ }");
    }

    #[test]
    fn attr_set_emits_bindings() {
        let node = NixNode::attr_set(vec![("x", NixNode::Int(1))]);
        let out = node.emit(0);
        assert!(out.contains("x = 1;"));
        assert!(out.starts_with('{'));
        assert!(out.ends_with('}'));
    }

    #[test]
    fn list_emits_brackets() {
        let node = NixNode::List(vec![NixNode::Int(1), NixNode::Int(2)]);
        let out = node.emit(0);
        assert!(out.starts_with('['));
        assert!(out.contains('1'));
        assert!(out.contains('2'));
    }

    #[test]
    fn singleton_list_inline() {
        let node = NixNode::List(vec![NixNode::str("hello")]);
        assert_eq!(node.emit(0), r#"[ "hello" ]"#);
    }

    #[test]
    fn inherit_emits_names() {
        let node = NixNode::Inherit(vec!["a".into(), "b".into()]);
        assert_eq!(node.emit(0), "inherit a b;");
    }

    #[test]
    fn inherit_from_emits_src() {
        let node = NixNode::InheritFrom {
            src: Box::new(NixNode::ident("pkgs")),
            names: vec!["hello".into()],
        };
        assert_eq!(node.emit(0), "inherit (pkgs) hello;");
    }

    #[test]
    fn with_emits_inline() {
        let node = NixNode::with(NixNode::ident("lib"), NixNode::ident("x"));
        assert_eq!(node.emit(0), "with lib; x");
    }

    #[test]
    fn import_emits_import() {
        let node = NixNode::import(NixNode::path("./module.nix"));
        assert_eq!(node.emit(0), "import ./module.nix");
    }

    #[test]
    fn bin_op_emits_operator() {
        let node = NixNode::bin_op(NixNode::ident("a"), BinOperator::Update, NixNode::ident("b"));
        assert_eq!(node.emit(0), "a // b");
    }

    #[test]
    fn interpolation_emits_dollar_brace() {
        let node = NixNode::interpolation(vec![
            StringPart::Literal("hello ".into()),
            StringPart::Expr(NixNode::ident("name")),
        ]);
        assert_eq!(node.emit(0), "\"hello ${name}\"");
    }

    #[test]
    fn function_emits_pattern_args() {
        let node = NixNode::Function {
            args: vec![FnArg::required("pkgs"), FnArg::required("lib")],
            variadic: true,
            body: Box::new(NixNode::ident("pkgs")),
        };
        let out = node.emit(0);
        assert!(out.starts_with("{ pkgs, lib, ... }:"));
    }

    #[test]
    fn function_arg_with_default() {
        let node = NixNode::Function {
            args: vec![FnArg::with_default("x", NixNode::Int(42))],
            variadic: false,
            body: Box::new(NixNode::ident("x")),
        };
        let out = node.emit(0);
        assert!(out.starts_with("{ x ? 42 }:"));
    }

    #[test]
    fn lambda_emits_simple() {
        let node = NixNode::Lambda {
            arg: "x".into(),
            body: Box::new(NixNode::ident("x")),
        };
        assert_eq!(node.emit(0), "x: x");
    }

    #[test]
    fn apply_emits_juxtaposition() {
        let node = NixNode::apply(NixNode::ident("f"), NixNode::Int(1));
        assert_eq!(node.emit(0), "f 1");
    }

    #[test]
    fn apply_wraps_complex_args() {
        let inner = NixNode::apply(NixNode::ident("g"), NixNode::Int(1));
        let node = NixNode::apply(NixNode::ident("f"), inner);
        assert_eq!(node.emit(0), "f (g 1)");
    }

    #[test]
    fn if_then_else_emits_branches() {
        let node = NixNode::if_then_else(
            NixNode::Bool(true),
            NixNode::Int(1),
            NixNode::Int(2),
        );
        let out = node.emit(0);
        assert!(out.contains("if true then"));
        assert!(out.contains("else"));
    }

    #[test]
    fn indent_propagates() {
        let node = NixNode::Comment("indented".into());
        assert_eq!(node.emit(2), "    # indented");
    }

    #[test]
    fn nested_attrs_build_correctly() {
        let b = build_nested_attrs(
            &["a".into(), "b".into(), "c".into()],
            NixNode::Int(1),
        );
        assert_eq!(b.key, "a");
        // Inner structure is nested AttrSets
        if let NixNode::AttrSet(inner) = &b.value {
            assert_eq!(inner[0].key, "b");
        } else {
            panic!("expected AttrSet");
        }
    }

    // ── Regression: Lambda/Function parenthesization ─────────

    #[test]
    fn lambda_as_apply_arg_gets_parens() {
        // flake-utils.lib.eachDefaultSystem (system: ...)
        // Without parens: `eachDefaultSystem system: ...` is a syntax error
        let node = NixNode::Apply {
            func: Box::new(NixNode::select(NixNode::ident("flake-utils"), &["lib", "eachDefaultSystem"])),
            arg: Box::new(NixNode::Lambda {
                arg: "system".into(),
                body: Box::new(NixNode::attr_set(vec![("x", NixNode::Int(1))])),
            }),
        };
        let out = node.emit(0);
        assert!(out.contains("(system:"), "lambda arg must be parenthesized: {out}");
    }

    #[test]
    fn function_as_apply_arg_gets_parens() {
        // builtins.foldl' ({ acc, ws, ... }: acc // ws)
        let node = NixNode::Apply {
            func: Box::new(NixNode::ident("f")),
            arg: Box::new(NixNode::Function {
                args: vec![FnArg::required("x")],
                variadic: false,
                body: Box::new(NixNode::ident("x")),
            }),
        };
        let out = node.emit(0);
        assert!(out.contains("({ x }:"), "function arg must be parenthesized: {out}");
    }

    #[test]
    fn simple_apply_no_extra_parens() {
        // import ./path — no parens needed for simple args
        let node = NixNode::Apply {
            func: Box::new(NixNode::ident("f")),
            arg: Box::new(NixNode::Int(42)),
        };
        assert_eq!(node.emit(0), "f 42");
    }

    #[test]
    fn flake_follows_produces_nested_attr() {
        // substrate.inputs.nixpkgs.follows = "nixpkgs" (NOT nixpkgs = "nixpkgs")
        let flake = crate::builders::FlakeBuilder::new("test")
            .input_with_follows("substrate", "github:pleme-io/substrate", vec![("nixpkgs", "nixpkgs")])
            .outputs(NixNode::ident("{}"))
            .emit();
        assert!(flake.contains("follows = \"nixpkgs\""), "follows must be nested: {flake}");
        assert!(!flake.contains("nixpkgs = \"nixpkgs\""), "must NOT be flat string assignment");
    }

    #[test]
    fn let_in_as_apply_arg_gets_parens() {
        let node = NixNode::Apply {
            func: Box::new(NixNode::ident("f")),
            arg: Box::new(NixNode::let_in(
                vec![("x", NixNode::Int(1))],
                NixNode::ident("x"),
            )),
        };
        let out = node.emit(0);
        assert!(out.contains("(let"), "let-in arg must be parenthesized: {out}");
    }

    // ── WriteShellApp — pkgs.writeShellApplication wrapper ──────────────

    #[test]
    fn write_shell_app_emits_canonical_shape() {
        let node = NixNode::WriteShellApp {
            name: "foo".into(),
            runtime_inputs: vec!["bash".into(), "jq".into()],
            text: "echo hi".into(),
        };
        let out = node.emit(0);
        assert!(out.contains("pkgs.writeShellApplication {"));
        assert!(out.contains("name = \"foo\";"));
        assert!(out.contains("runtimeInputs = [ pkgs.bash pkgs.jq ];"));
        assert!(out.contains("text = ''"));
        assert!(out.contains("echo hi"));
        assert!(out.contains("'';"));
    }

    #[test]
    fn write_shell_app_empty_runtime_inputs() {
        let node = NixNode::WriteShellApp {
            name: "bare".into(),
            runtime_inputs: vec![],
            text: "true".into(),
        };
        let out = node.emit(0);
        assert!(out.contains("runtimeInputs = [ ];"));
    }

    #[test]
    fn write_shell_app_escapes_dollar_brace_for_nix() {
        // Shell-style `${VAR}` must be escaped to `''${VAR}` inside `''..''`
        // so Nix doesn't treat it as a Nix interpolation.
        let node = NixNode::WriteShellApp {
            name: "needs-escape".into(),
            runtime_inputs: vec![],
            text: "echo \"${HOME}\"".into(),
        };
        let out = node.emit(0);
        assert!(
            out.contains("''${HOME}"),
            "shell ${{VAR}} must be escaped as ''${{VAR}} — got: {out}"
        );
    }

    #[test]
    fn write_shell_app_preserves_bare_dollar_vars() {
        // `$VAR` (no braces) is fine as-is; no escape needed.
        let node = NixNode::WriteShellApp {
            name: "bare-dollar".into(),
            runtime_inputs: vec![],
            text: "echo \"$HOME\"".into(),
        };
        let out = node.emit(0);
        assert!(out.contains("echo \"$HOME\""));
    }

    #[test]
    fn write_shell_app_multiline_body_indented() {
        let node = NixNode::WriteShellApp {
            name: "multi".into(),
            runtime_inputs: vec![],
            text: "line1\nline2\nline3".into(),
        };
        let out = node.emit(0);
        // Each non-empty shell line is indented within the text block
        assert!(out.contains("line1"));
        assert!(out.contains("line2"));
        assert!(out.contains("line3"));
    }

    #[test]
    fn write_shell_app_emit_is_deterministic() {
        let node = NixNode::WriteShellApp {
            name: "det".into(),
            runtime_inputs: vec!["ruby".into(), "bundler".into()],
            text: "echo deterministic".into(),
        };
        assert_eq!(node.emit(0), node.emit(0));
    }

    #[test]
    fn write_shell_app_nests_in_attr_set() {
        let node = NixNode::AttrSet(vec![Binding::new(
            "program",
            NixNode::WriteShellApp {
                name: "nested".into(),
                runtime_inputs: vec!["coreutils".into()],
                text: "echo nested".into(),
            },
        )]);
        let out = node.emit(0);
        assert!(out.contains("program ="));
        assert!(out.contains("pkgs.writeShellApplication"));
    }
}
