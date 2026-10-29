// src/bin/razomy_lsp.rs
use razomy_language::lsp::RazomyLanguageServer;

fn main() {
    let mut server = RazomyLanguageServer::new();
    if let Err(err) = server.run_stdio() {
        eprintln!("[razomy-lsp] I/O error: {err}");
        std::process::exit(1);
    }
}