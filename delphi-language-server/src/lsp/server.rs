use crate::lsp::analyzer::SymbolAnalyzer;
use crate::lsp::parser::DelphiParser;
use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

pub struct DelphiLanguageServer {
    client: Client,
    document_map: Mutex<HashMap<String, String>>,
    parser: Mutex<DelphiParser>,
    analyzer: Mutex<SymbolAnalyzer>,
}

impl DelphiLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            document_map: Mutex::new(HashMap::new()),
            parser: Mutex::new(DelphiParser::new()),
            analyzer: Mutex::new(SymbolAnalyzer::new()),
        }
    }

    async fn validate_document(&self, uri: &str, text: &str) {
        let diagnostics = {
            let mut parser = self.parser.lock().unwrap();
            let diagnostics = parser.get_diagnostics(text);
            if let Some(tree) = parser.parse(text) {
                let mut analyzer = self.analyzer.lock().unwrap();
                analyzer.set_content(tree, text.to_string(), Url::parse(uri).unwrap());
            }
            diagnostics
        };

        self.client
            .publish_diagnostics(Url::parse(uri).unwrap(), diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for DelphiLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                    completion_item: None,
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "Delphi Language Server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Delphi language server initialized!")
            .await;
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        if let Some(text) = self.document_map.lock().unwrap().get(&uri.to_string()) {
            let mut parser = self.parser.lock().unwrap();
            if let Some(tree) = parser.parse(text) {
                let mut analyzer = self.analyzer.lock().unwrap();
                analyzer.set_content(tree, text.to_string(), uri);
                if let Some(symbols) = analyzer.get_document_symbols() {
                    return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
                }
            }
        }
        Ok(None)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = params.text_document.text;
        {
            let mut document_map = self.document_map.lock().unwrap();
            document_map.insert(uri.clone(), text.clone());
        }
        self.validate_document(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = {
            let mut document_map = self.document_map.lock().unwrap();
            if let Some(content) = document_map.get_mut(&uri) {
                for change in params.content_changes {
                    if change.range.is_none() {
                        *content = change.text;
                    } else {
                        // Handle incremental updates if needed
                        // For now, just replace the entire content
                        *content = change.text;
                    }
                }
                content.clone()
            } else {
                return;
            }
        };
        self.validate_document(&uri, &text).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.document_map.lock().unwrap().remove(&uri);

        self.client
            .log_message(MessageType::INFO, &format!("File closed: {}", uri))
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        if let Some(text) = self.document_map.lock().unwrap().get(&uri.to_string()) {
            let mut parser = self.parser.lock().unwrap();
            if let Some(tree) = parser.parse(text) {
                let mut analyzer = self.analyzer.lock().unwrap();
                analyzer.set_content(tree, text.to_string(), uri);
                return Ok(analyzer.get_hover_info(position));
            }
        }
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        if let Some(text) = self.document_map.lock().unwrap().get(&uri.to_string()) {
            let mut parser = self.parser.lock().unwrap();
            if let Some(tree) = parser.parse(text) {
                let mut analyzer = self.analyzer.lock().unwrap();
                analyzer.set_content(tree, text.to_string(), uri);
                if let Some(location) = analyzer.find_definition(position) {
                    return Ok(Some(GotoDefinitionResponse::Scalar(location)));
                }
            }
        }
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        if let Some(text) = self.document_map.lock().unwrap().get(&uri.to_string()) {
            let mut parser = self.parser.lock().unwrap();
            if let Some(tree) = parser.parse(text) {
                let mut analyzer = self.analyzer.lock().unwrap();
                analyzer.set_content(tree, text.to_string(), uri);
                if let Some(items) = analyzer.get_completion_items(
                    position,
                    params.context.and_then(|ctx| ctx.trigger_character),
                ) {
                    return Ok(Some(CompletionResponse::Array(items)));
                }
            }
        }
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        if let Some(text) = self.document_map.lock().unwrap().get(&uri.to_string()) {
            let mut parser = self.parser.lock().unwrap();
            if let Some(tree) = parser.parse(text) {
                let mut analyzer = self.analyzer.lock().unwrap();
                analyzer.set_content(tree, text.to_string(), uri);
                return Ok(analyzer.find_references(position));
            }
        }
        Ok(None)
    }
}
