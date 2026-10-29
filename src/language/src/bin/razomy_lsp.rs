// src/bin/razomy_lsp.rs
use razomy_language::lsp::RazomyLanguageServer;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut server = RazomyLanguageServer::new();

    if matches!(args.get(1).map(String::as_str), Some("--http" | "http" | "serve")) {
        let port = args.get(2).map(String::as_str).unwrap_or("8765");
        let addr = if port.contains(':') {
            port.to_string()
        } else {
            format!("127.0.0.1:{port}")
        };
        if let Err(err) = server.run_http(&addr) {
            eprintln!("[razomy-lsp] HTTP error: {err}");
            std::process::exit(1);
        }
        return;
    }

    if let Err(err) = server.run_stdio() {
        eprintln!("[razomy-lsp] I/O error: {err}");
        std::process::exit(1);
    }
}