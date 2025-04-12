use clap::Parser as ClapParser;
use std::fs;
use std::path::PathBuf;
use tree_sitter::Parser;

mod lsp;

extern "C" {
    fn tree_sitter_pascal() -> tree_sitter::Language;
}

#[derive(ClapParser)]
#[command(name = "delphi-parser")]
#[command(about = "Parse and analyze Pascal/Delphi files")]
struct Cli {
    /// Run in LSP server mode
    #[arg(long, short)]
    lsp: bool,

    /// The path to the Pascal file to parse (only in CLI mode)
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Cli::parse();

    if args.lsp {
        // LSP server mode
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) =
            tower_lsp::LspService::new(|client| lsp::server::DelphiLanguageServer::new(client));
        tower_lsp::Server::new(stdin, stdout, socket)
            .serve(service)
            .await;
    } else {
        // CLI parsing mode
        let file = match args.file {
            Some(f) => f,
            None => {
                eprintln!("Error: File path is required in CLI mode");
                return;
            }
        };

        // Read the file content
        let source_code = match fs::read_to_string(&file) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file: {}", e);
                return;
            }
        };

        // Create a parser
        let mut parser = Parser::new();
        parser
            .set_language(unsafe { tree_sitter_pascal() })
            .expect("Error loading Pascal grammar");

        // Parse the source code
        let tree = parser
            .parse(&source_code, None)
            .expect("Error parsing file");

        // Print the syntax tree
        println!("Syntax tree:\n{}", tree.root_node().to_sexp());
    }
}
