use pest::Parser;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};

use razomy_language::analyzer::{analyze_source, AnalysisReport, Severity};
use razomy_language::hir::is_pascal_case;
use razomy_language::lsp::RazomyLanguageServer;
use razomy_language::parser::{RazomyParser, Rule};
#[test]
fn test_lsp_lifecycle_and_features() {
    let mut server = RazomyLanguageServer::new();
    let mut out = Vec::new();

    // 1. Initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    server.handle_message(&init_req, &mut out).unwrap();
    assert!(!out.is_empty());

    // 2. Open specification.rl snippet
    out.clear();
    let sample_code = r#"
User { id String, name String, age Int }
greet(User) [ "Welcome, {user.name}!" ]
"#;
    let open_notif = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.rl",
                "version": 1,
                "text": sample_code
            }
        }
    });
    server.handle_message(&open_notif, &mut out).unwrap();

    // 3. UFCS Completion on `User.` (Law 4 should suggest `id`, `name`, `age`, and `greet`!)
    let comp_res = server.on_completion(&json!({
        "textDocument": { "uri": "file:///test.rl" },
        "position": { "line": 1, "character": 4 }
    }));
    let items = comp_res.get("items").and_then(JsonValue::as_array).unwrap();
    assert!(!items.is_empty());

    // 4. Hover on `User`
    let hover_res = server.on_hover(&json!({
        "textDocument": { "uri": "file:///test.rl" },
        "position": { "line": 1, "character": 2 }
    }));
    assert!(hover_res.get("contents").is_some());

    // 5. Document Symbols
    let sym_res = server.on_document_symbol(&json!({
        "textDocument": { "uri": "file:///test.rl" }
    }));
    assert_eq!(sym_res.as_array().unwrap().len(), 2);
}
