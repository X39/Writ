//! writ-lsp: Language server for the Writ scripting language.
//!
//! Speaks LSP over stdio. Launched by the VS Code extension (writ-vscode).

use tower_lsp::{LspService, Server};
use writ_lsp::backend::Backend;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);

    Server::new(stdin, stdout, socket).serve(service).await;
}
