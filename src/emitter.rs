use crate::node::NixNode;

/// Emit a sequence of top-level nodes as a complete Nix file.
///
/// Deterministic: identical ASTs produce byte-identical output.
/// Always ends with exactly one trailing newline.
#[must_use]
pub fn emit_file(nodes: &[NixNode]) -> String {
    let mut lines: Vec<String> = Vec::with_capacity(nodes.len());
    for node in nodes {
        lines.push(node.emit(0));
    }
    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_emits_newline() {
        assert_eq!(emit_file(&[]), "\n");
    }

    #[test]
    fn single_node_has_trailing_newline() {
        let out = emit_file(&[NixNode::Comment("test".into())]);
        assert!(out.ends_with('\n'));
        assert_eq!(out, "# test\n");
    }

    #[test]
    fn multiple_nodes_joined_by_newlines() {
        let out = emit_file(&[
            NixNode::Comment("line 1".into()),
            NixNode::Blank,
            NixNode::Comment("line 2".into()),
        ]);
        assert_eq!(out, "# line 1\n\n# line 2\n");
    }

    #[test]
    fn deterministic_output() {
        let nodes = vec![
            NixNode::Comment("test".into()),
            NixNode::attr_set(vec![("x", NixNode::Int(1))]),
        ];
        let a = emit_file(&nodes);
        let b = emit_file(&nodes);
        assert_eq!(a, b);
    }
}
