use std::collections::HashMap;
use tower_lsp::lsp_types::*;
use tree_sitter::Node;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub selection_range: Range,
    pub children: Vec<Symbol>,
    pub detail: Option<String>,
}

pub struct SymbolAnalyzer {
    tree: Option<tree_sitter::Tree>,
    source: String,
    symbol_map: HashMap<String, Vec<Symbol>>,
    document_uri: Option<Url>,
}

impl SymbolAnalyzer {
    pub fn new() -> Self {
        Self {
            tree: None,
            source: String::new(),
            symbol_map: HashMap::new(),
            document_uri: None,
        }
    }

    pub fn set_content(&mut self, tree: tree_sitter::Tree, source: String, uri: Url) {
        self.tree = Some(tree);
        self.source = source;
        self.document_uri = Some(uri);
        self.update_symbol_map();
    }

    fn update_symbol_map(&mut self) {
        self.symbol_map.clear();
        if let Some(tree) = &self.tree {
            let symbols = self.collect_symbols(tree.root_node());
            for symbol in symbols {
                self.symbol_map
                    .entry(symbol.name.clone())
                    .or_insert_with(Vec::new)
                    .push(symbol);
            }
        }
    }

    pub fn get_document_symbols(&self) -> Option<Vec<DocumentSymbol>> {
        let tree = self.tree.as_ref()?;
        let root_node = tree.root_node();

        let symbols = self.collect_symbols(root_node);
        Some(
            symbols
                .into_iter()
                .map(|s| self.to_document_symbol(s))
                .collect(),
        )
    }

    fn collect_symbols(&self, node: Node) -> Vec<Symbol> {
        let mut symbols = Vec::new();

        match node.kind() {
            "program" | "unit" => {
                // Handle program/unit declarations
                if let Some(name_node) = self.find_identifier(node) {
                    symbols.push(Symbol {
                        name: self.get_node_text(name_node),
                        kind: SymbolKind::MODULE,
                        range: self.node_to_range(node),
                        selection_range: self.node_to_range(name_node),
                        children: self.collect_children_symbols(node),
                        detail: None,
                    });
                }
            }
            "type_declaration" => {
                // Handle type declarations
                if let Some(name_node) = self.find_identifier(node) {
                    symbols.push(Symbol {
                        name: self.get_node_text(name_node),
                        kind: SymbolKind::CLASS,
                        range: self.node_to_range(node),
                        selection_range: self.node_to_range(name_node),
                        children: self.collect_children_symbols(node),
                        detail: None,
                    });
                }
            }
            "procedure_declaration" | "function_declaration" => {
                // Handle procedure and function declarations
                if let Some(name_node) = self.find_identifier(node) {
                    symbols.push(Symbol {
                        name: self.get_node_text(name_node),
                        kind: SymbolKind::FUNCTION,
                        range: self.node_to_range(node),
                        selection_range: self.node_to_range(name_node),
                        children: Vec::new(),
                        detail: Some(self.get_declaration_detail(node)),
                    });
                }
            }
            "var_declaration" => {
                // Handle variable declarations
                if let Some(name_node) = self.find_identifier(node) {
                    symbols.push(Symbol {
                        name: self.get_node_text(name_node),
                        kind: SymbolKind::VARIABLE,
                        range: self.node_to_range(node),
                        selection_range: self.node_to_range(name_node),
                        children: Vec::new(),
                        detail: None,
                    });
                }
            }
            _ => {}
        }

        symbols
    }

    fn collect_children_symbols(&self, node: Node) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let mut cursor = node.walk();

        if cursor.goto_first_child() {
            loop {
                symbols.extend(self.collect_symbols(cursor.node()));
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        symbols
    }

    fn find_identifier<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                if cursor.node().kind() == "identifier" {
                    return Some(cursor.node());
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        None
    }

    fn get_node_text(&self, node: Node) -> String {
        self.source[node.byte_range()].to_string()
    }

    fn get_declaration_detail(&self, node: Node) -> String {
        // Get the full declaration text for hover info
        self.source[node.byte_range()].to_string()
    }

    fn node_to_range(&self, node: Node) -> Range {
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

    fn to_document_symbol(&self, symbol: Symbol) -> DocumentSymbol {
        DocumentSymbol {
            name: symbol.name,
            detail: symbol.detail,
            kind: symbol.kind,
            tags: None,
            deprecated: None,
            range: symbol.range,
            selection_range: symbol.selection_range,
            children: Some(
                symbol
                    .children
                    .into_iter()
                    .map(|s| self.to_document_symbol(s))
                    .collect(),
            ),
        }
    }

    pub fn get_hover_info(&self, position: Position) -> Option<Hover> {
        let tree = self.tree.as_ref()?;
        let point = tree_sitter::Point {
            row: position.line as usize,
            column: position.character as usize,
        };

        let node = tree.root_node().descendant_for_point_range(point, point)?;

        // Try to find the closest meaningful parent node
        let hover_node = self.find_hover_node(node);

        match hover_node.kind() {
            "identifier" => {
                let parent = hover_node.parent()?;
                match parent.kind() {
                    "procedure_declaration" | "function_declaration" => Some(self.create_hover(
                        self.get_node_text(parent),
                        Some("function".to_string()),
                        self.node_to_range(hover_node),
                    )),
                    "type_declaration" => Some(self.create_hover(
                        self.get_node_text(parent),
                        Some("type".to_string()),
                        self.node_to_range(hover_node),
                    )),
                    "var_declaration" => Some(self.create_hover(
                        self.get_node_text(parent),
                        Some("variable".to_string()),
                        self.node_to_range(hover_node),
                    )),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn find_definition(&self, position: Position) -> Option<Location> {
        let tree = self.tree.as_ref()?;
        let point = tree_sitter::Point {
            row: position.line as usize,
            column: position.character as usize,
        };

        let node = tree.root_node().descendant_for_point_range(point, point)?;
        let hover_node = self.find_hover_node(node);

        if hover_node.kind() == "identifier" {
            let name = self.get_node_text(hover_node);
            if let Some(symbols) = self.symbol_map.get(&name) {
                return Some(Location {
                    uri: self.document_uri.clone()?,
                    range: symbols[0].range,
                });
            }
        }
        None
    }

    pub fn find_references(&self, position: Position) -> Option<Vec<Location>> {
        let tree = self.tree.as_ref()?;
        let point = tree_sitter::Point {
            row: position.line as usize,
            column: position.character as usize,
        };

        let node = tree.root_node().descendant_for_point_range(point, point)?;
        let hover_node = self.find_hover_node(node);

        if hover_node.kind() == "identifier" {
            let name = self.get_node_text(hover_node);
            if let Some(symbols) = self.symbol_map.get(&name) {
                // Get document URI once before the map
                let uri = self.document_uri.clone()?;
                let locations: Vec<Location> = symbols
                    .iter()
                    .map(|symbol| Location {
                        uri: uri.clone(),
                        range: symbol.range,
                    })
                    .collect();
                return Some(locations);
            }
        }
        None
    }

    fn find_hover_node<'a>(&self, mut node: Node<'a>) -> Node<'a> {
        while node.kind() == "ERROR" || node.is_extra() {
            if let Some(parent) = node.parent() {
                node = parent;
            } else {
                break;
            }
        }
        node
    }

    fn create_hover(&self, content: String, kind: Option<String>, range: Range) -> Hover {
        let mut value = String::new();
        if let Some(k) = kind {
            value.push_str(&format!("```pascal\n{}\n```\n", k));
        }
        value.push_str(&content);

        Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(range),
        }
    }

    pub fn get_completion_items(
        &self,
        position: Position,
        trigger_char: Option<String>,
    ) -> Option<Vec<CompletionItem>> {
        let tree = self.tree.as_ref()?;
        let point = tree_sitter::Point {
            row: position.line as usize,
            column: position.character as usize,
        };

        let node = tree.root_node().descendant_for_point_range(point, point)?;
        let mut items = Vec::new();

        if let Some(trigger) = trigger_char {
            if trigger == "." {
                // Handle member completion after dot
                if let Some(scope) = self.find_completion_scope(node) {
                    items.extend(self.get_scope_members(scope));
                }
            }
        } else {
            // Handle general identifier completion
            items.extend(self.get_visible_symbols(node));
        }

        Some(items)
    }

    fn find_completion_scope(&self, node: Node) -> Option<String> {
        // For now, just return the type name if we can find it
        // TODO: Implement proper scope resolution
        let mut current = node;
        while let Some(parent) = current.parent() {
            if parent.kind() == "type_declaration" {
                if let Some(name_node) = self.find_identifier(parent) {
                    return Some(self.get_node_text(name_node));
                }
            }
            current = parent;
        }
        None
    }

    fn get_scope_members(&self, scope: String) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        // TODO: Implement proper member lookup based on type/scope
        if let Some(symbols) = self.symbol_map.get(&scope) {
            for symbol in symbols {
                items.push(CompletionItem {
                    label: symbol.name.clone(),
                    kind: Some(self.symbol_kind_to_completion_kind(symbol.kind)),
                    detail: symbol.detail.clone(),
                    documentation: None,
                    ..CompletionItem::default()
                });
            }
        }
        items
    }

    fn get_visible_symbols(&self, node: Node) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Add all symbols in the current scope
        for symbols in self.symbol_map.values() {
            for symbol in symbols {
                items.push(CompletionItem {
                    label: symbol.name.clone(),
                    kind: Some(self.symbol_kind_to_completion_kind(symbol.kind)),
                    detail: symbol.detail.clone(),
                    documentation: None,
                    ..CompletionItem::default()
                });
            }
        }

        items
    }

    fn symbol_kind_to_completion_kind(&self, kind: SymbolKind) -> CompletionItemKind {
        match kind {
            SymbolKind::FUNCTION => CompletionItemKind::FUNCTION,
            SymbolKind::CLASS => CompletionItemKind::CLASS,
            SymbolKind::VARIABLE => CompletionItemKind::VARIABLE,
            SymbolKind::MODULE => CompletionItemKind::MODULE,
            _ => CompletionItemKind::TEXT,
        }
    }
}
