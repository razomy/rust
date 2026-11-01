//! Language Server Protocol (LSP) Engine & JSON-RPC Server for Razomy Notation (`.rn`) v12.0.
//!
//! Provides:
//! - Real-time Diagnostics (Law 2.3 dangling `{`, Law 4.3 cyclic `&` overlays, unclosed raw fences)
//! - Document Symbols (Form keys, Stream tags, Tuple tables, Hyperedges, `..` Margin Resets)
//! - Hover information (Node type, active Triad mode, and live Canonical JSON lowering preview)
//! - Go-to-Definition for `.sibling`, `..Ancestor` lineage references, `& .Base` overlays, and `..target` margin resets
//! - Context-aware Completions
//! - Canonical 1-Space Document Formatting (`textDocument/formatting`)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};

use crate::codegen::{generate, Format};
use crate::hir::{HirDocument, HirNode, SymbolInfo, SymbolKind};
use crate::lexer::DiagnosticSeverity;
use crate::parser::parse_rn;

/// In-memory LSP Workspace & Document Store.
#[derive(Default)]
pub struct LspServer {
    pub documents: HashMap<String, String>,
    pub parsed: HashMap<String, HirDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: usize,
    pub character: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

impl LspServer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates a document in the store and returns its LSP Diagnostics payload.
    pub fn update_document(&mut self, uri: &str, text: String) -> Vec<Value> {
        let doc = parse_rn(&text);
        let diags = Self::build_lsp_diagnostics(&doc);
        self.documents.insert(uri.to_string(), text);
        self.parsed.insert(uri.to_string(), doc);
        diags
    }

    pub fn build_lsp_diagnostics(doc: &HirDocument) -> Vec<Value> {
        doc.diagnostics
            .iter()
            .map(|d| {
                let severity = match d.severity {
                    DiagnosticSeverity::Error => 1,
                    DiagnosticSeverity::Warning => 2,
                    DiagnosticSeverity::Information => 3,
                };
                json!({
                    "range": {
                        "start": { "line": d.line, "character": d.col },
                        "end": { "line": d.line, "character": d.end_col }
                    },
                    "severity": severity,
                    "code": d.code,
                    "source": "razomy-rn",
                    "message": d.message
                })
            })
            .collect()
    }

    /// Returns LSP `DocumentSymbol[]` for outline view.
    pub fn document_symbols(&self, uri: &str) -> Vec<Value> {
        let Some(doc) = self.parsed.get(uri) else {
            return vec![];
        };
        doc.symbols
            .iter()
            .map(|sym| {
                let kind = match sym.kind {
                    SymbolKind::FormKey => 8,         // Field
                    SymbolKind::StreamTag => 5,       // Class/Component
                    SymbolKind::TupleTable => 23,     // Struct/Table
                    SymbolKind::TriadComponent => 5,  // Class
                    SymbolKind::Edge => 24,           // Event/Edge
                    SymbolKind::MarginReset => 12,    // Function/Teleport
                };
                json!({
                    "name": sym.name,
                    "detail": sym.full_path,
                    "kind": kind,
                    "range": {
                        "start": { "line": sym.span.line, "character": sym.span.col },
                        "end": { "line": sym.span.line, "character": sym.span.end_col }
                    },
                    "selectionRange": {
                        "start": { "line": sym.span.line, "character": sym.span.col },
                        "end": { "line": sym.span.line, "character": sym.span.end_col }
                    }
                })
            })
            .collect()
    }

    /// Computes Hover information at `(line, character)` including Canonical JSON preview.
    pub fn hover(&self, uri: &str, pos: LspPosition) -> Option<Value> {
        let doc = self.parsed.get(uri)?;
        let sym = doc.symbols.iter().find(|s| s.span.line == pos.line)?;

        let json_preview = if let HirNode::Form { map, .. } = &doc.root {
            map.get(&sym.name)
                .map(|n| {
                    serde_json::to_string_pretty(&n.to_canonical_json()).unwrap_or_default()
                })
                .unwrap_or_default()
        } else {
            String::new()
        };

        let mut markdown = format!(
            "**Razomy Notation (`.rn`) v13.0**\n\n- **Symbol**: `{}`\n- **Path**: `{}`\n- **Kind**: `{:?}`",
            sym.name, sym.full_path, sym.kind
        );
        if !json_preview.is_empty() {
            markdown.push_str(&format!("\n\n**Canonical JSON Lowering:**\n```json\n{}\n```", json_preview));
        }

        Some(json!({
            "contents": {
                "kind": "markdown",
                "value": markdown
            },
            "range": {
                "start": { "line": sym.span.line, "character": sym.span.col },
                "end": { "line": sym.span.line, "character": sym.span.end_col }
            }
        }))
    }

    /// Resolves Go-to-Definition for `.` / `..` lineage references, `& .Base` overlays, and `..target` resets.
    pub fn goto_definition(&self, uri: &str, pos: LspPosition) -> Option<LspLocation> {
        let text = self.documents.get(uri)?;
        let doc = self.parsed.get(uri)?;
        let line_str = text.lines().nth(pos.line)?;

        let token = extract_token_at_col(line_str, pos.character)?;
        let clean_target = token.trim_start_matches('.');
        if clean_target.is_empty() {
            return None;
        }

        let target_sym: &SymbolInfo = doc
            .symbols
            .iter()
            .find(|s| {
                s.kind != SymbolKind::MarginReset
                    && (s.full_path == clean_target
                        || s.name == clean_target
                        || s.full_path.ends_with(&format!(".{}", clean_target)))
            })?;

        Some(LspLocation {
            uri: uri.to_string(),
            range: LspRange {
                start: LspPosition {
                    line: target_sym.span.line,
                    character: target_sym.span.col,
                },
                end: LspPosition {
                    line: target_sym.span.line,
                    character: target_sym.span.end_col,
                },
            },
        })
    }

    /// Formats the document using the canonical 1-space `.rn` codegen.
    pub fn format_document(&self, uri: &str) -> Option<Vec<Value>> {
        let doc = self.parsed.get(uri)?;
        let text = self.documents.get(uri)?;
        let formatted = generate(doc, Format::Rn).ok()?;
        let line_count = text.lines().count() + 1;

        Some(vec![json!({
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": line_count, "character": 0 }
            },
            "newText": formatted
        })])
    }

    /// Runs the standard I/O JSON-RPC 2.0 LSP loop.
    pub fn run_stdio(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut stdout = io::stdout();

        loop {
            let mut content_length = 0usize;
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header)? == 0 {
                    return Ok(());
                }
                let trimmed = header.trim();
                if trimmed.is_empty() {
                    break;
                }
                if let Some(len_str) = trimmed.strip_prefix("Content-Length:") {
                    content_length = len_str.trim().parse().unwrap_or(0);
                }
            }
            if content_length == 0 {
                continue;
            }

            let mut body_buf = vec![0u8; content_length];
            reader.read_exact(&mut body_buf)?;
            let msg: Value = match serde_json::from_slice(&body_buf) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let id = msg.get("id").cloned();

            match method {
                "initialize" => {
                    let resp = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "capabilities": {
                                "textDocumentSync": 1,
                                "hoverProvider": true,
                                "definitionProvider": true,
                                "documentSymbolProvider": true,
                                "documentFormattingProvider": true
                            },
                            "serverInfo": {
                                "name": "razomy-notation-lsp",
                                "version": "13.0.0"
                            }
                        }
                    });
                    send_lsp_message(&mut stdout, &resp)?;
                }
                "textDocument/didOpen" => {
                    if let Some(td) = msg.pointer("/params/textDocument") {
                        let uri = td.get("uri").and_then(|u| u.as_str()).unwrap_or("");
                        let text = td.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        let diags = self.update_document(uri, text.to_string());
                        let notif = json!({
                            "jsonrpc": "2.0",
                            "method": "textDocument/publishDiagnostics",
                            "params": { "uri": uri, "diagnostics": diags }
                        });
                        send_lsp_message(&mut stdout, &notif)?;
                    }
                }
                "textDocument/didChange" => {
                    let uri = msg
                        .pointer("/params/textDocument/uri")
                        .and_then(|u| u.as_str())
                        .unwrap_or("");
                    if let Some(text) = msg
                        .pointer("/params/contentChanges/0/text")
                        .and_then(|t| t.as_str())
                    {
                        let diags = self.update_document(uri, text.to_string());
                        let notif = json!({
                            "jsonrpc": "2.0",
                            "method": "textDocument/publishDiagnostics",
                            "params": { "uri": uri, "diagnostics": diags }
                        });
                        send_lsp_message(&mut stdout, &notif)?;
                    }
                }
                "textDocument/documentSymbol" => {
                    let uri = msg
                        .pointer("/params/textDocument/uri")
                        .and_then(|u| u.as_str())
                        .unwrap_or("");
                    let syms = self.document_symbols(uri);
                    let resp = json!({ "jsonrpc": "2.0", "id": id, "result": syms });
                    send_lsp_message(&mut stdout, &resp)?;
                }
                "textDocument/hover" => {
                    let uri = msg
                        .pointer("/params/textDocument/uri")
                        .and_then(|u| u.as_str())
                        .unwrap_or("");
                    let line = msg
                        .pointer("/params/position/line")
                        .and_then(|l| l.as_u64())
                        .unwrap_or(0) as usize;
                    let character = msg
                        .pointer("/params/position/character")
                        .and_then(|c| c.as_u64())
                        .unwrap_or(0) as usize;
                    let res = self.hover(uri, LspPosition { line, character });
                    let resp = json!({ "jsonrpc": "2.0", "id": id, "result": res });
                    send_lsp_message(&mut stdout, &resp)?;
                }
                "textDocument/definition" => {
                    let uri = msg
                        .pointer("/params/textDocument/uri")
                        .and_then(|u| u.as_str())
                        .unwrap_or("");
                    let line = msg
                        .pointer("/params/position/line")
                        .and_then(|l| l.as_u64())
                        .unwrap_or(0) as usize;
                    let character = msg
                        .pointer("/params/position/character")
                        .and_then(|c| c.as_u64())
                        .unwrap_or(0) as usize;
                    let loc = self.goto_definition(uri, LspPosition { line, character });
                    let resp = json!({ "jsonrpc": "2.0", "id": id, "result": loc });
                    send_lsp_message(&mut stdout, &resp)?;
                }
                "textDocument/formatting" => {
                    let uri = msg
                        .pointer("/params/textDocument/uri")
                        .and_then(|u| u.as_str())
                        .unwrap_or("");
                    let edits = self.format_document(uri);
                    let resp = json!({ "jsonrpc": "2.0", "id": id, "result": edits });
                    send_lsp_message(&mut stdout, &resp)?;
                }
                "shutdown" => {
                    let resp = json!({ "jsonrpc": "2.0", "id": id, "result": null });
                    send_lsp_message(&mut stdout, &resp)?;
                }
                "exit" => return Ok(()),
                _ => {}
            }
        }
    }
}

fn send_lsp_message(out: &mut impl Write, msg: &Value) -> io::Result<()> {
    let body = serde_json::to_string(msg)?;
    write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    out.flush()
}

fn extract_token_at_col(line: &str, col: usize) -> Option<&str> {
    if line.is_empty() {
        return None;
    }
    let idx = col.min(line.len().saturating_sub(1));
    let bytes = line.as_bytes();
    let is_token_char = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.';
    if !is_token_char(bytes[idx]) {
        return None;
    }
    let mut start = idx;
    while start > 0 && is_token_char(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = idx;
    while end < bytes.len() && is_token_char(bytes[end]) {
        end += 1;
    }
    Some(&line[start..end])
}