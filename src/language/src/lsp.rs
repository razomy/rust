// src/lsp.rs
// =============================================================================
// RAZOMY V5.7 — NATIVE RUST LANGUAGE SERVER PROTOCOL (LSP) ENGINE
// Полноценный LSP-сервер (JSON-RPC 2.0 over stdio) для JetBrains IDEs:
// • Диагностика в реальном времени (Syntax + HIR Laws 1–4 + E01..E07)
// • UFCS Autocomplete (Закон 4: поля, реактивные геттеры, методы, предки, функции)
// • Hover документация (HIR схема форм, Lineage, Union-варианты, сигнатуры)
// • Go to Definition (точный переход к формам, полям, методам, функциям и глобалам)
// • Document Symbols (иерархическое дерево структуры файла для Structure View)
// • Semantic Tokens (семантическая подсветка синтаксиса V5.7 в JetBrains IDE)
// • Inlay Hints, Signature Help, Rename, References & QuickFix Code Actions
// =============================================================================

use pest::Parser;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::analyzer::{AnalysisReport, Severity};
use crate::hir::is_pascal_case;
use crate::parser::{RazomyParser, Rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LspPosition {
    pub line: usize,
    pub character: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

impl LspRange {
    pub fn to_json(self) -> JsonValue {
        json!({
            "start": { "line": self.start.line, "character": self.start.character },
            "end":   { "line": self.end.line,   "character": self.end.character }
        })
    }
}

#[derive(Debug, Clone)]
pub struct DocumentState {
    pub uri: String,
    pub version: i64,
    pub text: String,
    pub line_starts: Vec<usize>,
    pub sentence_spans: Vec<(usize, usize)>,
    pub non_code_spans: Vec<(usize, usize)>,
    pub report: AnalysisReport,
}

impl DocumentState {
    pub fn new(uri: String, version: i64, text: String) -> Self {
        let line_starts = compute_line_starts(&text);
        let sentence_spans = collect_sentence_spans(&text);
        let non_code_spans = collect_non_code_spans(&text);
        let file_path = uri_to_path(&uri);
        let report = crate::analyzer::analyze_source_with_path(&text, file_path.as_deref());

        Self {
            uri,
            version,
            text,
            line_starts,
            sentence_spans,
            non_code_spans,
            report,
        }
    }

    pub fn is_in_non_code(&self, byte_offset: usize) -> bool {
        self.non_code_spans
            .iter()
            .any(|&(start, end)| byte_offset >= start && byte_offset < end)
    }

    pub fn offset_to_position(&self, offset: usize) -> LspPosition {
        let clamped = offset.min(self.text.len());
        let line = match self.line_starts.binary_search(&clamped) {
            Ok(exact) => exact,
            Err(ins) => ins.saturating_sub(1),
        };
        let line_start = self.line_starts.get(line).copied().unwrap_or(0);
        let slice = &self.text[line_start..clamped];
        let character = slice.encode_utf16().count();
        LspPosition { line, character }
    }

    pub fn position_to_offset(&self, pos: LspPosition) -> usize {
        let Some(&line_start) = self.line_starts.get(pos.line) else {
            return self.text.len();
        };
        let line_text = self.text[line_start..]
            .split('\n')
            .next()
            .unwrap_or("");
        let mut utf16_col = 0;
        for (byte_idx, ch) in line_text.char_indices() {
            if utf16_col >= pos.character {
                return line_start + byte_idx;
            }
            utf16_col += ch.len_utf16();
        }
        line_start + line_text.len()
    }

    pub fn span_to_range(&self, start_byte: usize, end_byte: usize) -> LspRange {
        LspRange {
            start: self.offset_to_position(start_byte),
            end: self.offset_to_position(end_byte.max(start_byte)),
        }
    }

    pub fn sentence_range(
        &self,
        sentence_idx: Option<usize>,
        symbol_hint: Option<&str>,
    ) -> LspRange {
        if let Some(idx) = sentence_idx {
            if let Some(&(s_start, s_end)) = self.sentence_spans.get(idx) {
                let sent_slice = &self.text[s_start..s_end];
                if let Some(sym) = symbol_hint {
                    let clean_sym = sym.split('.').last().unwrap_or(sym);
                    if !clean_sym.is_empty() {
                        if let Some(rel) = find_word_in_slice(sent_slice, clean_sym) {
                            let start = s_start + rel;
                            return self.span_to_range(start, start + clean_sym.len());
                        }
                    }
                }
                let first_line_end = sent_slice
                    .find('\n')
                    .map(|nl| s_start + nl)
                    .unwrap_or(s_end);
                return self.span_to_range(s_start, first_line_end);
            }
        }
        LspRange::default()
    }

    pub fn word_at_position(&self, pos: LspPosition) -> (String, LspRange) {
        let offset = self.position_to_offset(pos);
        let bytes = self.text.as_bytes();
        let is_ident_or_dot = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'.';

        let mut start = offset;
        while start > 0 && is_ident_or_dot(bytes[start - 1]) {
            start -= 1;
        }
        let mut end = offset;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            end += 1;
        }
        let word = self.text[start..end].to_string();
        (word, self.span_to_range(start, end))
    }
}

/// Кроссплатформенный декодер `file://` URI в `PathBuf` (поддерживает Windows `file:///C:/...` и `%20`)
fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let raw = uri.strip_prefix("file://")?;
    let decoded = percent_decode(raw);
    let bytes = decoded.as_bytes();
    // На Windows URI имеет вид `/C:/path/to/file.rl`
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
        Some(PathBuf::from(&decoded[1..]))
    } else {
        Some(PathBuf::from(decoded))
    }
}

fn percent_decode(input: &str) -> String {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(hex_str, 16) {
                    out.push(val);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn collect_sentence_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    if let Ok(mut pairs) = RazomyParser::parse(Rule::Program, text) {
        if let Some(root) = pairs.next() {
            for inner in root.into_inner() {
                if inner.as_rule() == Rule::sentence {
                    let sp = inner.as_span();
                    spans.push((sp.start(), sp.end()));
                }
            }
        }
    }
    spans
}

/// Собирает байтовые интервалы всех комментариев (`//`, `# ... #`) и строковых литералов (`"..."`, `'...'`)
/// чтобы исключать их при поиске ссылок (References) и переименовании (Rename).
fn collect_non_code_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            spans.push((start, i));
        } else if bytes[i] == b'#'
            && matches!(bytes.get(i + 1), Some(b' ' | b'\t' | b'\r' | b'\n'))
        {
            let start = i;
            i += 2;
            while i < bytes.len() && bytes[i] != b'#' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            spans.push((start, i));
        } else if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            spans.push((start, i));
        } else {
            i += 1;
        }
    }
    spans
}

fn find_word_in_slice(slice: &str, word: &str) -> Option<usize> {
    let mut search_from = 0;
    let bytes = slice.as_bytes();
    while let Some(pos) = slice[search_from..].find(word) {
        let abs = search_from + pos;
        let before_ok = abs == 0
            || !(bytes[abs - 1].is_ascii_alphanumeric() || bytes[abs - 1] == b'_');
        let after_idx = abs + word.len();
        let after_ok = after_idx >= bytes.len()
            || !(bytes[after_idx].is_ascii_alphanumeric() || bytes[after_idx] == b'_');
        if before_ok && after_ok {
            return Some(abs);
        }
        search_from = abs + 1;
    }
    None
}

// =============================================================================
// СЕМАНТИЧЕСКИЕ ТОКЕНЫ ДЛЯ ПОДСВЕТКИ В JETBRAINS IDE
// =============================================================================

pub const SEMANTIC_TOKEN_TYPES: &[&str] = &[
    "namespace",  // 0: Modules
    "type",       // 1: Forms, Types, Union Variants
    "class",      // 2: Generic Forms / Traits
    "parameter",  // 3: Function / Form parameters
    "variable",   // 4: Local & global variables
    "property",   // 5: Form fields & reactive getters
    "function",   // 6: Functions
    "method",     // 7: Form methods
    "keyword",    // 8: self, Self, _, _0.._9, true, false, Compile, Bypass, Public, Private
    "comment",    // 9: // and # ... #
    "string",     // 10: "...", r"...", 'symbol'
    "number",     // 11: Int, Float, Hex Byte
    "operator",   // 12: ., .>, .&, .?, .|, &, |, /, +, -, *, ==, etc.
];

pub const SEMANTIC_TOKEN_MODIFIERS: &[&str] = &[
    "declaration",   // bit 0 (1)
    "readonly",      // bit 1 (2)
    "static",        // bit 2 (4)
    "modification",  // bit 3 (8)
];

// =============================================================================
// ЯДРО LSP-СЕРВЕРА
// =============================================================================

#[derive(Default)]
pub struct RazomyLanguageServer {
    pub documents: HashMap<String, DocumentState>,
    pub shutdown_requested: bool,
}

impl RazomyLanguageServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_stdio(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut reader = BufReader::new(stdin.lock());
        let mut writer = stdout.lock();

        while let Some(msg) = read_rpc_message(&mut reader)? {
            let should_exit = self.handle_message(&msg, &mut writer)?;
            if should_exit {
                break;
            }
        }
        Ok(())
    }

    pub fn handle_message<W: Write>(
        &mut self,
        msg: &JsonValue,
        writer: &mut W,
    ) -> io::Result<bool> {
        let method = msg.get("method").and_then(JsonValue::as_str).unwrap_or("");
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));

        // Ленивая подгрузка документа с диска, если IDE обратилась к файлу до `didOpen`
        if let Some(uri) = params
            .get("textDocument")
            .and_then(|t| t.get("uri"))
            .and_then(JsonValue::as_str)
        {
            if method != "textDocument/didOpen" && method != "textDocument/didClose" {
                self.ensure_document_loaded(uri);
            }
        }

        match method {
            "initialize" => {
                let result = self.on_initialize(&params);
                if let Some(req_id) = id {
                    write_rpc_response(writer, req_id, result)?;
                }
            }
            "initialized" => {}
            "shutdown" => {
                self.shutdown_requested = true;
                if let Some(req_id) = id {
                    write_rpc_response(writer, req_id, JsonValue::Null)?;
                }
            }
            "exit" => return Ok(true),

            "textDocument/didOpen" => {
                if let Some(doc) = params.get("textDocument") {
                    let uri = doc.get("uri").and_then(JsonValue::as_str).unwrap_or("").to_string();
                    let version = doc.get("version").and_then(JsonValue::as_i64).unwrap_or(1);
                    let text = doc.get("text").and_then(JsonValue::as_str).unwrap_or("").to_string();
                    self.update_document(uri, version, text, writer)?;
                }
            }
            "textDocument/didChange" => {
                if let Some(doc) = params.get("textDocument") {
                    let uri = doc.get("uri").and_then(JsonValue::as_str).unwrap_or("").to_string();
                    let version = doc.get("version").and_then(JsonValue::as_i64).unwrap_or(1);
                    if let Some(changes) = params.get("contentChanges").and_then(JsonValue::as_array) {
                        if let Some(last_change) = changes.last() {
                            if let Some(text) = last_change.get("text").and_then(JsonValue::as_str) {
                                self.update_document(uri, version, text.to_string(), writer)?;
                            }
                        }
                    }
                }
            }
            "textDocument/didSave" => {
                if let Some(doc) = params.get("textDocument") {
                    let uri = doc.get("uri").and_then(JsonValue::as_str).unwrap_or("").to_string();
                    if let Some(text) = params.get("text").and_then(JsonValue::as_str) {
                        self.update_document(uri, 1, text.to_string(), writer)?;
                    } else if let Some(existing) = self.documents.get(&uri).cloned() {
                        self.publish_diagnostics(&existing, writer)?;
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(doc) = params.get("textDocument") {
                    if let Some(uri) = doc.get("uri").and_then(JsonValue::as_str) {
                        self.documents.remove(uri);
                        write_rpc_notification(
                            writer,
                            "textDocument/publishDiagnostics",
                            json!({ "uri": uri, "diagnostics": [] }),
                        )?;
                    }
                }
            }

            "textDocument/hover" => {
                if let Some(req_id) = id {
                    let res = self.on_hover(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/completion" => {
                if let Some(req_id) = id {
                    let res = self.on_completion(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/definition" => {
                if let Some(req_id) = id {
                    let res = self.on_definition(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/documentSymbol" => {
                if let Some(req_id) = id {
                    let res = self.on_document_symbol(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/semanticTokens/full" => {
                if let Some(req_id) = id {
                    let res = self.on_semantic_tokens(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/inlayHint" => {
                if let Some(req_id) = id {
                    let res = self.on_inlay_hints(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/signatureHelp" => {
                if let Some(req_id) = id {
                    let res = self.on_signature_help(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/references" => {
                if let Some(req_id) = id {
                    let res = self.on_references(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/documentHighlight" => {
                if let Some(req_id) = id {
                    let res = self.on_document_highlight(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/prepareRename" => {
                if let Some(req_id) = id {
                    let res = self.on_prepare_rename(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/rename" => {
                if let Some(req_id) = id {
                    let res = self.on_rename(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/formatting" => {
                if let Some(req_id) = id {
                    let res = self.on_formatting(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "textDocument/codeAction" => {
                if let Some(req_id) = id {
                    let res = self.on_code_action(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            "workspace/symbol" => {
                if let Some(req_id) = id {
                    let res = self.on_workspace_symbol(&params);
                    write_rpc_response(writer, req_id, res)?;
                }
            }
            _ => {
                if let Some(req_id) = id {
                    write_rpc_response(writer, req_id, JsonValue::Null)?;
                }
            }
        }

        Ok(false)
    }

    fn ensure_document_loaded(&mut self, uri: &str) {
        if self.documents.contains_key(uri) {
            return;
        }
        if let Some(path) = uri_to_path(uri) {
            if let Ok(text) = std::fs::read_to_string(&path) {
                let state = DocumentState::new(uri.to_string(), 1, text);
                self.documents.insert(uri.to_string(), state);
            }
        }
    }

    /// Находит все точные вхождения идентификатора в коде документа (исключая комментарии и строки)
    fn find_word_occurrences_in_doc(&self, doc: &DocumentState, ident: &str) -> Vec<LspRange> {
        let mut ranges = Vec::new();
        if ident.is_empty() {
            return ranges;
        }
        let mut search_from = 0;
        let bytes = doc.text.as_bytes();
        while let Some(pos) = doc.text[search_from..].find(ident) {
            let abs = search_from + pos;
            let before_ok = abs == 0
                || !(bytes[abs - 1].is_ascii_alphanumeric() || bytes[abs - 1] == b'_');
            let after_idx = abs + ident.len();
            let after_ok = after_idx >= bytes.len()
                || !(bytes[after_idx].is_ascii_alphanumeric() || bytes[after_idx] == b'_');

            if before_ok && after_ok && !doc.is_in_non_code(abs) {
                ranges.push(doc.span_to_range(abs, after_idx));
            }
            search_from = abs + ident.len();
        }
        ranges
    }

    pub fn on_references(&self, params: &JsonValue) -> JsonValue {
        let Some((doc, pos)) = self.get_doc_and_pos(params) else {
            return json!([]);
        };
        let (word, _) = doc.word_at_position(pos);
        let clean = word.trim_matches('.').split('.').last().unwrap_or("");
        if clean.is_empty() {
            return json!([]);
        }

        let mut locations = Vec::new();
        for candidate in self.documents.values() {
            for range in self.find_word_occurrences_in_doc(candidate, clean) {
                locations.push(json!({
                    "uri": candidate.uri,
                    "range": range.to_json()
                }));
            }
        }
        json!(locations)
    }

    pub fn on_document_highlight(&self, params: &JsonValue) -> JsonValue {
        let Some((doc, pos)) = self.get_doc_and_pos(params) else {
            return json!([]);
        };
        let (word, _) = doc.word_at_position(pos);
        let clean = word.trim_matches('.').split('.').last().unwrap_or("");
        if clean.is_empty() {
            return json!([]);
        }

        let highlights: Vec<JsonValue> = self
            .find_word_occurrences_in_doc(doc, clean)
            .into_iter()
            .map(|r| json!({ "range": r.to_json(), "kind": 1 }))
            .collect();
        json!(highlights)
    }

    pub fn on_prepare_rename(&self, params: &JsonValue) -> JsonValue {
        let Some((doc, pos)) = self.get_doc_and_pos(params) else {
            return JsonValue::Null;
        };
        let offset = doc.position_to_offset(pos);
        if doc.is_in_non_code(offset) {
            return JsonValue::Null;
        }
        let (word, range) = doc.word_at_position(pos);
        let clean = word.trim_matches('.').split('.').last().unwrap_or("");
        if clean.is_empty() {
            return JsonValue::Null;
        }
        json!({
            "range": range.to_json(),
            "placeholder": clean
        })
    }

    pub fn on_rename(&self, params: &JsonValue) -> JsonValue {
        let Some((doc, pos)) = self.get_doc_and_pos(params) else {
            return JsonValue::Null;
        };
        let Some(new_name) = params.get("newName").and_then(JsonValue::as_str) else {
            return JsonValue::Null;
        };
        let (word, _) = doc.word_at_position(pos);
        let clean = word.trim_matches('.').split('.').last().unwrap_or("");
        if clean.is_empty() {
            return JsonValue::Null;
        }

        let mut changes = serde_json::Map::new();
        for candidate in self.documents.values() {
            let edits: Vec<JsonValue> = self
                .find_word_occurrences_in_doc(candidate, clean)
                .into_iter()
                .map(|r| json!({ "range": r.to_json(), "newText": new_name }))
                .collect();
            if !edits.is_empty() {
                changes.insert(candidate.uri.clone(), json!(edits));
            }
        }

        json!({ "changes": changes })
    }

    pub fn on_formatting(&self, params: &JsonValue) -> JsonValue {
        let Some(uri) = params
            .get("textDocument")
            .and_then(|t| t.get("uri"))
            .and_then(JsonValue::as_str)
        else {
            return json!([]);
        };
        let Some(doc) = self.documents.get(uri) else {
            return json!([]);
        };

        let formatted = format_razomy_source(&doc.text);
        if formatted == doc.text {
            return json!([]);
        }

        let full_range = doc.span_to_range(0, doc.text.len());
        json!([{
            "range": full_range.to_json(),
            "newText": formatted
        }])
    }

    pub fn on_code_action(&self, params: &JsonValue) -> JsonValue {
        let Some(uri) = params
            .get("textDocument")
            .and_then(|t| t.get("uri"))
            .and_then(JsonValue::as_str)
        else {
            return json!([]);
        };
        let Some(doc) = self.documents.get(uri) else {
            return json!([]);
        };

        let mut actions = Vec::new();

        for d in &doc.report.diagnostics {
            match d.code {
                // E01: `area (Circle { radius 5.0 })` -> Убрать пробел перед `(`
                "E01_AMBIGUOUS_CALL_OR_BIND" => {
                    if let Some(idx) = d.sentence_index {
                        if let Some(&(s_start, s_end)) = doc.sentence_spans.get(idx) {
                            let slice = &doc.text[s_start..s_end];
                            if let Some(paren_pos) = slice.find(" (") {
                                let edit_range =
                                    doc.span_to_range(s_start + paren_pos, s_start + paren_pos + 2);
                                actions.push(json!({
                                    "title": "Сделать плотным вызовом функции без пробела перед '('",
                                    "kind": "quickfix",
                                    "edit": {
                                        "changes": {
                                            doc.uri.clone(): [{
                                                "range": edit_range.to_json(),
                                                "newText": "("
                                            }]
                                        }
                                    }
                                }));
                            }
                        }
                    }
                }
                // W001: Закон 1 — замена `field` на `param.field`
                "W001_LAW1_NAMED_SLOT" | "W001_LAW1_NAMED_SLOT_INTERP" => {
                    if let Some(fix) = d.message.rsplit("Используйте '").next() {
                        let clean_fix = fix.trim_end_matches("'.");
                        let target_field = clean_fix
                            .trim_matches(|c| c == '{' || c == '}')
                            .split('.')
                            .last()
                            .unwrap_or("");
                        if !target_field.is_empty() {
                            let range = doc.sentence_range(d.sentence_index, Some(target_field));
                            let replacement = clean_fix.trim_matches(|c| c == '{' || c == '}');
                            actions.push(json!({
                                "title": format!("Заменить '{target_field}' на '{replacement}' (Закон 1)"),
                                "kind": "quickfix",
                                "edit": {
                                    "changes": {
                                        doc.uri.clone(): [{
                                            "range": range.to_json(),
                                            "newText": replacement
                                        }]
                                    }
                                }
                            }));
                        }
                    }
                }
                // E05: Закон 2 — уточнение пути предка при коллизии Lineage
                "E05_AMBIGUOUS_LINEAGE_FIELD" => {
                    if let Some(paren_hint) = d.message.rsplit('(').next() {
                        let explicit_path = paren_hint.trim_end_matches(").");
                        if explicit_path.contains('.') {
                            let range = doc.sentence_range(d.sentence_index, None);
                            actions.push(json!({
                                "title": format!("Уточнить путь предка: '{explicit_path}' (Закон 2)"),
                                "kind": "quickfix",
                                "edit": {
                                    "changes": {
                                        doc.uri.clone(): [{
                                            "range": range.to_json(),
                                            "newText": explicit_path
                                        }]
                                    }
                                }
                            }));
                        }
                    }
                }
                _ => {}
            }
        }

        json!(actions)
    }

    pub fn on_workspace_symbol(&self, params: &JsonValue) -> JsonValue {
        let query = params
            .get("query")
            .and_then(JsonValue::as_str)
            .unwrap_or("")
            .to_lowercase();

        let mut results = Vec::new();
        for doc in self.documents.values() {
            for (name, hf) in &doc.report.hir.forms {
                if !hf.is_prelude && name.to_lowercase().contains(&query) {
                    let range = doc.sentence_range(Some(hf.origin.sentence_index), Some(name));
                    results.push(json!({
                        "name": name,
                        "kind": 23,
                        "location": { "uri": doc.uri, "range": range.to_json() }
                    }));
                }
            }
            for (name, f) in &doc.report.hir.functions {
                if name.to_lowercase().contains(&query) {
                    let range = doc.sentence_range(Some(f.origin.sentence_index), Some(name));
                    results.push(json!({
                        "name": name,
                        "kind": 12,
                        "location": { "uri": doc.uri, "range": range.to_json() }
                    }));
                }
            }
        }
        json!(results)
    }

    pub fn on_initialize(&self, _params: &JsonValue) -> JsonValue {
        json!({
            "capabilities": {
                "textDocumentSync": {
                    "openClose": true,
                    "change": 1,
                    "save": { "includeText": true }
                },
                "hoverProvider": true,
                "completionProvider": {
                    "resolveProvider": false,
                    "triggerCharacters": [".", "\"", "'", "("]
                },
                "definitionProvider": true,
                "referencesProvider": true,
                "documentHighlightProvider": true,
                "renameProvider": {
                    "prepareProvider": true
                },
                "documentFormattingProvider": true,
                "codeActionProvider": {
                    "codeActionKinds": ["quickfix", "refactor"]
                },
                "documentSymbolProvider": true,
                "workspaceSymbolProvider": true,
                "inlayHintProvider": true,
                "signatureHelpProvider": {
                    "triggerCharacters": ["(", ","]
                },
                "semanticTokensProvider": {
                    "legend": {
                        "tokenTypes": SEMANTIC_TOKEN_TYPES,
                        "tokenModifiers": SEMANTIC_TOKEN_MODIFIERS
                    },
                    "full": true,
                    "range": false
                }
            },
            "serverInfo": {
                "name": "razomy-lsp",
                "version": env!("CARGO_PKG_VERSION")
            }
        })
    }

    pub fn update_document<W: Write>(
        &mut self,
        uri: String,
        version: i64,
        text: String,
        writer: &mut W,
    ) -> io::Result<()> {
        let state = DocumentState::new(uri.clone(), version, text);
        self.publish_diagnostics(&state, writer)?;
        self.documents.insert(uri, state);
        Ok(())
    }

    pub fn publish_diagnostics<W: Write>(
        &self,
        doc: &DocumentState,
        writer: &mut W,
    ) -> io::Result<()> {
        let mut lsp_diags = Vec::new();

        for d in &doc.report.diagnostics {
            let severity_num = match d.severity {
                Severity::Error => 1,
                Severity::Warning => 2,
                Severity::Information => 3,
                Severity::Hint => 4,
            };

            let range = if d.code == "E00_SYNTAX" {
                extract_pest_error_range(doc)
            } else {
                doc.sentence_range(d.sentence_index, d.context_symbol.as_deref())
            };

            lsp_diags.push(json!({
                "range": range.to_json(),
                "severity": severity_num,
                "code": d.code,
                "source": "razomy",
                "message": d.message,
            }));
        }

        write_rpc_notification(
            writer,
            "textDocument/publishDiagnostics",
            json!({
                "uri": doc.uri,
                "version": doc.version,
                "diagnostics": lsp_diags
            }),
        )
    }

    pub fn on_hover(&self, params: &JsonValue) -> JsonValue {
        let Some((doc, pos)) = self.get_doc_and_pos(params) else {
            return JsonValue::Null;
        };
        let (word, range) = doc.word_at_position(pos);
        if word.is_empty() {
            return JsonValue::Null;
        }

        let target = word.trim_matches('.');
        if let Some(md) = doc.report.hover(target) {
            return json!({
                "contents": { "kind": "markdown", "value": md },
                "range": range.to_json()
            });
        }

        // Проверяем составной путь `Form.field`, `var.field` или `Form.method`
        if let Some((base, member)) = target.rsplit_once('.') {
            let root = base.split('.').next().unwrap_or(base);
            let resolved_form = doc.report.hir.forms.get(root).or_else(|| {
                doc.report
                    .hir
                    .globals
                    .get(root)
                    .and_then(|g| doc.report.hir.forms.get(&g.inferred_type))
            });

            if let Some(hf) = resolved_form {
                if let Some(field) = hf.fields.get(member) {
                    let kind_badge = if field.is_reactive {
                        " [Reactive Stream Getter]"
                    } else {
                        ""
                    };
                    let md = format!(
                        "```razomy\n{}.{}: {} (from {}){}\n```",
                        hf.name, field.name, field.inferred_type, field.origin_form, kind_badge
                    );
                    return json!({
                        "contents": { "kind": "markdown", "value": md },
                        "range": range.to_json()
                    });
                }
                if let Some(method) = hf.methods.get(member) {
                    let params_str = method
                        .params
                        .iter()
                        .filter(|p| !p.is_type_param)
                        .map(|p| {
                            if p.type_constraint.is_empty() {
                                p.name.clone()
                            } else {
                                format!("{} {}", p.name, p.type_constraint)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let ret = method.return_type.as_deref().unwrap_or("Any");
                    let md = format!(
                        "```razomy\n{}.{}({}) -> {}\n```",
                        hf.name, method.name, params_str, ret
                    );
                    return json!({
                        "contents": { "kind": "markdown", "value": md },
                        "range": range.to_json()
                    });
                }
            }
        }

        let last_seg = target.split('.').last().unwrap_or(target);
        if let Some(md) = doc.report.hover(last_seg) {
            return json!({
                "contents": { "kind": "markdown", "value": md },
                "range": range.to_json()
            });
        }

        JsonValue::Null
    }

    pub fn on_completion(&self, params: &JsonValue) -> JsonValue {
        let Some((doc, pos)) = self.get_doc_and_pos(params) else {
            return json!({ "isIncomplete": false, "items": [] });
        };

        let (word, _) = doc.word_at_position(pos);
        let query = if word.is_empty() || word == "." {
            find_piped_receiver_before_cursor(doc, pos).unwrap_or(word)
        } else {
            word
        };

        let items = doc.report.completions(&query);
        let lsp_items: Vec<JsonValue> = items
            .into_iter()
            .map(|item| {
                let kind_num = match item.kind {
                    "Method" => 2,
                    "Function" | "UFCSFunction" => 3,
                    "Field" | "ReactiveField" => 5,
                    "Variable" => 6,
                    "Form" | "AncestorSlice" => 7,
                    "TypeAlgebra" => 8,
                    "Stream" => 12,
                    _ => 1,
                };
                json!({
                    "label": item.label,
                    "kind": kind_num,
                    "detail": item.detail,
                    "insertText": item.insert_text,
                })
            })
            .collect();

        json!({
            "isIncomplete": false,
            "items": lsp_items
        })
    }

    pub fn on_definition(&self, params: &JsonValue) -> JsonValue {
        let Some((doc, pos)) = self.get_doc_and_pos(params) else {
            return JsonValue::Null;
        };
        let (word, _) = doc.word_at_position(pos);
        let target = word.trim_matches('.');
        if target.is_empty() {
            return JsonValue::Null;
        }

        // 1. Точный переход к полю или методу `Form.member` или `var.member`
        if let Some((base, member)) = target.rsplit_once('.') {
            let root = base.split('.').next().unwrap_or(base);
            for candidate_doc in std::iter::once(doc).chain(self.documents.values()) {
                let resolved_form = candidate_doc.report.hir.forms.get(root).or_else(|| {
                    candidate_doc
                        .report
                        .hir
                        .globals
                        .get(root)
                        .and_then(|g| candidate_doc.report.hir.forms.get(&g.inferred_type))
                });

                if let Some(hf) = resolved_form {
                    // Если поле унаследовано из другого предка — переходим к форме-источнику (origin_form)
                    let origin_form_name = hf
                        .fields
                        .get(member)
                        .map(|f| f.origin_form.as_str())
                        .unwrap_or(&hf.name);
                    let target_hf = candidate_doc
                        .report
                        .hir
                        .forms
                        .get(origin_form_name)
                        .unwrap_or(hf);
                    if !target_hf.is_prelude {
                        let range = candidate_doc
                            .sentence_range(Some(target_hf.origin.sentence_index), Some(member));
                        return json!({ "uri": candidate_doc.uri, "range": range.to_json() });
                    }
                }
            }
        }

        // 2. Ищем символ (форму, функцию или глобальную переменную) во всех открытых документах
        let clean = target.split('.').next().unwrap_or(target);
        for candidate_doc in std::iter::once(doc).chain(self.documents.values()) {
            if let Some(hf) = candidate_doc.report.hir.forms.get(clean) {
                if !hf.is_prelude {
                    let range =
                        candidate_doc.sentence_range(Some(hf.origin.sentence_index), Some(clean));
                    return json!({ "uri": candidate_doc.uri, "range": range.to_json() });
                }
            }
            if let Some(f) = candidate_doc.report.hir.functions.get(clean) {
                let range =
                    candidate_doc.sentence_range(Some(f.origin.sentence_index), Some(clean));
                return json!({ "uri": candidate_doc.uri, "range": range.to_json() });
            }
            if let Some(g) = candidate_doc.report.hir.globals.get(clean) {
                let range =
                    candidate_doc.sentence_range(Some(g.origin.sentence_index), Some(clean));
                return json!({ "uri": candidate_doc.uri, "range": range.to_json() });
            }
        }

        JsonValue::Null
    }

    pub fn on_document_symbol(&self, params: &JsonValue) -> JsonValue {
        let Some(uri) = params
            .get("textDocument")
            .and_then(|t| t.get("uri"))
            .and_then(JsonValue::as_str)
        else {
            return json!([]);
        };
        let Some(doc) = self.documents.get(uri) else {
            return json!([]);
        };

        let mut symbols = Vec::new();

        for name in &doc.report.hir.form_order {
            if let Some(hf) = doc.report.hir.forms.get(name) {
                if hf.is_prelude {
                    continue;
                }
                let range = doc.sentence_range(Some(hf.origin.sentence_index), Some(name));
                let mut children = Vec::new();

                for fname in &hf.field_order {
                    if let Some(f) = hf.fields.get(fname) {
                        let f_range =
                            doc.sentence_range(Some(hf.origin.sentence_index), Some(fname));
                        children.push(json!({
                            "name": fname,
                            "detail": f.inferred_type,
                            "kind": if f.is_reactive { 7 } else { 8 },
                            "range": f_range.to_json(),
                            "selectionRange": f_range.to_json()
                        }));
                    }
                }
                for mname in &hf.method_order {
                    if let Some(m) = hf.methods.get(mname) {
                        let m_range =
                            doc.sentence_range(Some(hf.origin.sentence_index), Some(mname));
                        children.push(json!({
                            "name": mname,
                            "detail": m.return_type.clone().unwrap_or_else(|| "Method".into()),
                            "kind": 6,
                            "range": m_range.to_json(),
                            "selectionRange": m_range.to_json()
                        }));
                    }
                }

                symbols.push(json!({
                    "name": hf.name,
                    "detail": if hf.is_algebraic_alias { "Type Algebra" } else { "Form" },
                    "kind": if hf.is_algebraic_alias { 11 } else { 23 },
                    "range": range.to_json(),
                    "selectionRange": range.to_json(),
                    "children": children
                }));
            }
        }

        for fn_name in &doc.report.hir.function_order {
            if let Some(f) = doc.report.hir.functions.get(fn_name) {
                let range = doc.sentence_range(Some(f.origin.sentence_index), Some(fn_name));
                symbols.push(json!({
                    "name": f.name,
                    "detail": f.return_type.clone().unwrap_or_else(|| "Stream".into()),
                    "kind": 12,
                    "range": range.to_json(),
                    "selectionRange": range.to_json()
                }));
            }
        }

        for g_name in &doc.report.hir.global_order {
            if doc.report.hir.forms.contains_key(g_name) {
                continue;
            }
            if let Some(g) = doc.report.hir.globals.get(g_name) {
                let range = doc.sentence_range(Some(g.origin.sentence_index), Some(g_name));
                symbols.push(json!({
                    "name": g.key,
                    "detail": g.inferred_type,
                    "kind": 13,
                    "range": range.to_json(),
                    "selectionRange": range.to_json()
                }));
            }
        }

        json!(symbols)
    }

    pub fn on_semantic_tokens(&self, params: &JsonValue) -> JsonValue {
        let Some(uri) = params
            .get("textDocument")
            .and_then(|t| t.get("uri"))
            .and_then(JsonValue::as_str)
        else {
            return json!({ "data": [] });
        };
        let Some(doc) = self.documents.get(uri) else {
            return json!({ "data": [] });
        };

        let data = encode_semantic_tokens(doc);
        json!({ "data": data })
    }

    pub fn on_inlay_hints(&self, params: &JsonValue) -> JsonValue {
        let Some(uri) = params
            .get("textDocument")
            .and_then(|t| t.get("uri"))
            .and_then(JsonValue::as_str)
        else {
            return json!([]);
        };
        let Some(doc) = self.documents.get(uri) else {
            return json!([]);
        };

        let mut hints = Vec::new();
        for g_name in &doc.report.hir.global_order {
            if doc.report.hir.forms.contains_key(g_name) || g_name == "_" {
                continue;
            }
            if let Some(g) = doc.report.hir.globals.get(g_name) {
                if g.inferred_type != "Any" && !is_pascal_case(g_name) {
                    let r = doc.sentence_range(Some(g.origin.sentence_index), Some(g_name));
                    hints.push(json!({
                        "position": { "line": r.end.line, "character": r.end.character },
                        "label": format!(": {}", g.inferred_type),
                        "kind": 1,
                        "paddingLeft": false,
                        "paddingRight": true
                    }));
                }
            }
        }
        json!(hints)
    }

    pub fn on_signature_help(&self, params: &JsonValue) -> JsonValue {
        let Some((doc, pos)) = self.get_doc_and_pos(params) else {
            return JsonValue::Null;
        };
        let offset = doc.position_to_offset(pos);
        let prefix = &doc.text[..offset];
        let Some(open_paren) = prefix.rfind('(') else {
            return JsonValue::Null;
        };
        let before_paren = prefix[..open_paren].trim_end();
        let callee: String = before_paren
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect::<String>()
            .chars()
            .rev()
            .collect();

        if let Some(f) = doc.report.hir.functions.get(&callee) {
            let active_param = prefix[open_paren..].matches(',').count();
            let params_json: Vec<JsonValue> = f
                .params
                .iter()
                .filter(|p| !p.is_type_param)
                .map(|p| {
                    let label = if p.type_constraint.is_empty() {
                        p.name.clone()
                    } else {
                        format!("{} {}", p.name, p.type_constraint)
                    };
                    json!({ "label": label })
                })
                .collect();

            return json!({
                "signatures": [{
                    "label": format!("{}({})", f.name, params_json.iter().filter_map(|p| p.get("label").and_then(JsonValue::as_str)).collect::<Vec<_>>().join(", ")),
                    "parameters": params_json
                }],
                "activeSignature": 0,
                "activeParameter": active_param
            });
        }

        JsonValue::Null
    }

    fn get_doc_and_pos(&self, params: &JsonValue) -> Option<(&DocumentState, LspPosition)> {
        let uri = params
            .get("textDocument")
            .and_then(|t| t.get("uri"))
            .and_then(JsonValue::as_str)?;
        let pos_obj = params.get("position")?;
        let line = pos_obj.get("line").and_then(JsonValue::as_u64)? as usize;
        let character = pos_obj.get("character").and_then(JsonValue::as_u64)? as usize;
        let doc = self.documents.get(uri)?;
        Some((doc, LspPosition { line, character }))
    }
}

fn find_piped_receiver_before_cursor(doc: &DocumentState, pos: LspPosition) -> Option<String> {
    let offset = doc.position_to_offset(pos);
    let before = doc.text[..offset].trim_end();
    let before_dot = before.strip_suffix('.')?.trim_end();
    let token: String = before_dot
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if token.is_empty() {
        None
    } else {
        Some(format!("{token}."))
    }
}

fn extract_pest_error_range(doc: &DocumentState) -> LspRange {
    if let Err(err) = RazomyParser::parse(Rule::Program, &doc.text) {
        let (start, end) = match err.location {
            pest::error::InputLocation::Pos(p) => (p, (p + 1).min(doc.text.len())),
            pest::error::InputLocation::Span((s, e)) => (s, e),
        };
        return doc.span_to_range(start, end);
    }
    LspRange::default()
}

// =============================================================================
// ЛЕКСЕР СЕМАНТИЧЕСКИХ ТОКЕНОВ ДЛЯ LSP (`textDocument/semanticTokens/full`)
// =============================================================================

struct RawSemanticToken {
    line: usize,
    start_char: usize,
    length: usize,
    token_type: u32,
    modifiers: u32,
}

fn encode_semantic_tokens(doc: &DocumentState) -> Vec<u32> {
    let mut raw_tokens = Vec::new();

    if let Ok(mut pairs) = RazomyParser::parse(Rule::Program, &doc.text) {
        if let Some(root) = pairs.next() {
            collect_pair_semantic_tokens(root, doc, &mut raw_tokens);
        }
    }

    collect_comment_semantic_tokens(doc, &mut raw_tokens);

    raw_tokens.sort_by_key(|t| (t.line, t.start_char));
    raw_tokens.dedup_by(|a, b| a.line == b.line && a.start_char == b.start_char);

    let mut data = Vec::with_capacity(raw_tokens.len() * 5);
    let mut prev_line = 0;
    let mut prev_start = 0;

    for tok in raw_tokens {
        if tok.length == 0 {
            continue;
        }
        let delta_line = tok.line.saturating_sub(prev_line) as u32;
        let delta_start = if delta_line == 0 {
            tok.start_char.saturating_sub(prev_start) as u32
        } else {
            tok.start_char as u32
        };
        data.push(delta_line);
        data.push(delta_start);
        data.push(tok.length as u32);
        data.push(tok.token_type);
        data.push(tok.modifiers);

        prev_line = tok.line;
        prev_start = tok.start_char;
    }

    data
}

fn collect_pair_semantic_tokens(
    pair: pest::iterators::Pair<Rule>,
    doc: &DocumentState,
    out: &mut Vec<RawSemanticToken>,
) {
    let rule = pair.as_rule();
    let span = pair.as_span();

    match rule {
        Rule::string_lit | Rule::raw_string_lit | Rule::symbol_lit => {
            push_multiline_token(doc, span.start(), span.end(), 10, 0, out);
            return;
        }
        Rule::tagged_string_lit => {
            for inner in pair.into_inner() {
                collect_pair_semantic_tokens(inner, doc, out);
            }
            return;
        }
        Rule::type_ident => {
            push_single_token(doc, span.start(), span.end(), 1, 0, out);
            return;
        }
        Rule::number_lit | Rule::hex_byte_lit => {
            push_single_token(doc, span.start(), span.end(), 11, 0, out);
            return;
        }
        Rule::operator | Rule::callable_op => {
            push_single_token(doc, span.start(), span.end(), 12, 0, out);
            return;
        }
        Rule::path => {
            let s = pair.as_str();
            let (tok_type, mods) = classify_path_token(s, span.end(), doc);
            push_single_token(doc, span.start(), span.end(), tok_type, mods, out);
            return;
        }
        _ => {}
    }

    for inner in pair.into_inner() {
        collect_pair_semantic_tokens(inner, doc, out);
    }
}

fn classify_path_token(s: &str, end_byte: usize, doc: &DocumentState) -> (u32, u32) {
    if matches!(
        s,
        "true" | "false" | "_" | "self" | "Self" | "Compile" | "Bypass" | "Public" | "Private"
    ) || (s.starts_with('_') && s[1..].chars().all(|c| c.is_ascii_digit()))
    {
        return (8, 0);
    }

    let next_char = doc.text[end_byte..].chars().next();
    if next_char == Some('(') {
        if is_pascal_case(s) {
            return (2, 0);
        }
        return (6, 0);
    }

    let root = s.split('.').next().unwrap_or(s);
    if is_pascal_case(root) {
        if s.contains('.') {
            let last = s.split('.').last().unwrap_or(s);
            if doc
                .report
                .hir
                .forms
                .get(root)
                .map_or(false, |f| f.methods.contains_key(last))
            {
                return (7, 0);
            }
            return (5, 0);
        }
        return (1, 2);
    }

    if doc.report.hir.functions.contains_key(s) {
        return (6, 0);
    }

    if s.contains('.') {
        return (5, 0);
    }

    (4, 0)
}

fn collect_comment_semantic_tokens(doc: &DocumentState, out: &mut Vec<RawSemanticToken>) {
    let bytes = doc.text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
            let start = i;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            push_single_token(doc, start, i, 9, 0, out);
        } else if bytes[i] == b'#'
            && matches!(bytes.get(i + 1), Some(b' ' | b'\t' | b'\r' | b'\n'))
        {
            let start = i;
            i += 2;
            while i < bytes.len() && bytes[i] != b'#' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            push_multiline_token(doc, start, i, 9, 0, out);
        } else if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
        } else {
            i += 1;
        }
    }
}

fn push_single_token(
    doc: &DocumentState,
    start_byte: usize,
    end_byte: usize,
    token_type: u32,
    modifiers: u32,
    out: &mut Vec<RawSemanticToken>,
) {
    let start = doc.offset_to_position(start_byte);
    let end = doc.offset_to_position(end_byte);
    if start.line == end.line && end.character > start.character {
        out.push(RawSemanticToken {
            line: start.line,
            start_char: start.character,
            length: end.character - start.character,
            token_type,
            modifiers,
        });
    }
}

fn push_multiline_token(
    doc: &DocumentState,
    start_byte: usize,
    end_byte: usize,
    token_type: u32,
    modifiers: u32,
    out: &mut Vec<RawSemanticToken>,
) {
    let slice = &doc.text[start_byte..end_byte];
    let mut cur_offset = start_byte;
    for line_str in slice.split('\n') {
        let clean_len = line_str.strip_suffix('\r').unwrap_or(line_str).len();
        if clean_len > 0 {
            push_single_token(
                doc,
                cur_offset,
                cur_offset + clean_len,
                token_type,
                modifiers,
                out,
            );
        }
        cur_offset += line_str.len() + 1;
    }
}

// =============================================================================
// JSON-RPC 2.0 STDIO TRANSPORT FRAMING
// =============================================================================

pub fn read_rpc_message<R: BufRead>(reader: &mut R) -> io::Result<Option<JsonValue>> {
    let mut content_length: Option<usize> = None;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(None);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if content_length.is_some() {
                break;
            }
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            if k.eq_ignore_ascii_case("Content-Length") {
                content_length = v.trim().parse::<usize>().ok();
            }
        }
    }

    let len = content_length.unwrap_or(0);
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;

    let parsed: JsonValue = serde_json::from_slice(&body)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(Some(parsed))
}

pub fn write_rpc_response<W: Write>(
    writer: &mut W,
    id: JsonValue,
    result: JsonValue,
) -> io::Result<()> {
    let payload = json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    });
    send_rpc_json(writer, &payload)
}

pub fn write_rpc_notification<W: Write>(
    writer: &mut W,
    method: &str,
    params: JsonValue,
) -> io::Result<()> {
    let payload = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });
    send_rpc_json(writer, &payload)
}

fn send_rpc_json<W: Write>(writer: &mut W, payload: &JsonValue) -> io::Result<()> {
    let bytes = serde_json::to_vec(payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?;
    writer.write_all(&bytes)?;
    writer.flush()
}

/// Автоформаттер кода Razomy V5.7 (выравнивание блоков `{}`, `[]`, `()` и отступов)
pub fn format_razomy_source(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut indent_level: usize = 0;

    for raw_line in source.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }

        let starts_with_closing = trimmed.starts_with('}')
            || trimmed.starts_with(']')
            || trimmed.starts_with(')');
        if starts_with_closing {
            indent_level = indent_level.saturating_sub(1);
        }

        for _ in 0..indent_level {
            out.push_str("    ");
        }
        out.push_str(trimmed);
        out.push('\n');

        let mut net_open: isize = 0;
        let mut in_str = false;
        let bytes = trimmed.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !in_str && bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'/') {
                break;
            }
            if bytes[i] == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
                in_str = !in_str;
            } else if !in_str {
                match bytes[i] {
                    b'{' | b'[' | b'(' => net_open += 1,
                    b'}' | b']' | b')' => net_open -= 1,
                    _ => {}
                }
            }
            i += 1;
        }

        if starts_with_closing {
            net_open += 1;
        }
        if net_open > 0 {
            indent_level += net_open as usize;
        } else if net_open < 0 {
            indent_level = indent_level.saturating_sub((-net_open) as usize);
        }
    }

    out
}
