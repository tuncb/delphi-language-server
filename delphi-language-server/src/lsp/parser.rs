use tower_lsp::lsp_types::*;
use tree_sitter::{Language, Parser};

extern "C" {
    fn tree_sitter_pascal() -> Language;
}

pub struct DelphiParser {
    parser: Parser,
}

impl DelphiParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        unsafe {
            parser
                .set_language(tree_sitter_pascal())
                .expect("Error loading Pascal grammar");
        }
        Self { parser }
    }

    pub fn parse(&mut self, text: &str) -> Option<tree_sitter::Tree> {
        self.parser.parse(text, None)
    }

    pub fn get_diagnostics(&mut self, text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(tree) = self.parse(text) {
            if tree.root_node().has_error() {
                // Walk the tree to find syntax errors
                let mut cursor = tree.walk();
                self.collect_error_nodes(&mut cursor, text, &mut diagnostics);
            }
        }

        diagnostics
    }

    fn collect_error_nodes(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        source: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if cursor.node().is_error() || cursor.node().is_missing() {
            let range = self.node_range(cursor.node());
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                message: "Syntax error".to_string(),
                source: Some("dls".to_string()),
                ..Diagnostic::default()
            });
        }

        if cursor.goto_first_child() {
            loop {
                self.collect_error_nodes(cursor, source, diagnostics);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn node_range(&self, node: tree_sitter::Node) -> Range {
        Range {
            start: Position {
                line: node.start_position().row as u32,
                character: node.start_position().column as u32,
            },
            end: Position {
                line: node.end_position().row as u32,
                character: node.end_position().column as u32,
            },
        }
    }
}
