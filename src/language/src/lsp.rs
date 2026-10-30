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
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;

use crate::analyzer::{AnalysisReport, CompletionItem, Severity};
use crate::hir::{is_pascal_case, SourceOrigin};
use crate::parser::{
    classify_sentence, extract_import, has_pipe_op, normalize_path, parse_code,
    recover_source_for_ide, split_pipeline, ImportDirective, ImportSource, Literal, PipeOp,
    RazomyParser, Rule, SemanticKind, Sentence, Token,
};

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
    pub recovered_text: String,
    pub line_starts: Vec<usize>,
    pub sentence_spans: Vec<(usize, usize)>,
    pub non_code_spans: Vec<(usize, usize)>,
    pub report: AnalysisReport,
}

impl DocumentState {
    pub fn new(uri: String, version: i64, text: String) -> Self {
        Self::new_with_memory(uri, version, text, &HashMap::new())
    }

    pub fn new_with_memory(
        uri: String,
        version: i64,
        text: String,
        in_memory_files: &HashMap<PathBuf, String>,
    ) -> Self {
        let line_starts = compute_line_starts(&text);
        let recovered_text = if RazomyParser::parse(Rule::Program, &text).is_ok() {
            text.clone()
        } else {
            recover_source_for_ide(&text)
        };
        let sentence_spans = collect_sentence_spans(&recovered_text);
        let non_code_spans = collect_non_code_spans(&text);
        let file_path = uri_to_path(&uri);
        let report = crate::analyzer::analyze_source_with_path_and_memory(
            &text,
            file_path.as_deref(),
            in_memory_files,
        );

        Self {
            uri,
            version,
            text,
            recovered_text,
            line_starts,
            sentence_spans,
            non_code_spans,
            report,
        }
    }

    pub fn sentence_index_at_offset(&self, offset: usize) -> Option<usize> {
        self.sentence_spans
            .iter()
            .position(|&(s, e)| offset >= s && offset <= e)
    }

    pub fn sentence_index_at_pos(&self, pos: LspPosition) -> Option<usize> {
        let offset = self.position_to_offset(pos);
        self.sentence_index_at_offset(offset)
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
                let sent_slice = &self.text[s_start..s_end.min(self.text.len())];
                if let Some(sym) = symbol_hint {
                    if !sym.is_empty() {
                        if let Some(rel) = find_word_in_slice(sent_slice, sym) {
                            let start = s_start + rel;
                            return self.span_to_range(start, start + sym.len());
                        }
                        let clean_sym = sym.split('.').last().unwrap_or(sym);
                        if !clean_sym.is_empty() {
                            if let Some(rel) = find_word_in_slice(sent_slice, clean_sym) {
                                let start = s_start + rel;
                                return self.span_to_range(start, start + clean_sym.len());
                            }
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
        // Fallback: если индекс смещен из-за импортов, ищем точное слово symbol_hint в коде документа
        if let Some(sym) = symbol_hint {
            let clean_sym = sym.split('.').last().unwrap_or(sym);
            if !clean_sym.is_empty() {
                if let Some(rel) = find_word_in_slice(&self.text, clean_sym) {
                    if !self.is_in_non_code(rel) {
                        return self.span_to_range(rel, rel + clean_sym.len());
                    }
                }
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

/// Если курсор находится внутри инициализации формы `User { id "1", name "Алексей" }` или `u User { name "Алексей" }`,
/// возвращает имя этой формы (`"User"`), чтобы работали Go to Definition и Hover при клике на `name`!
fn find_enclosing_struct_init_form(
    doc: &DocumentState,
    pos: LspPosition,
    sent_idx: Option<usize>,
) -> Option<String> {
    let offset = doc.position_to_offset(pos);
    let before = &doc.text[..offset];
    let mut depth = 0i32;
    let mut open_brace_idx = None;
    for (i, ch) in before.char_indices().rev() {
        match ch {
            '}' => depth += 1,
            '{' => {
                if depth == 0 {
                    open_brace_idx = Some(i);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    let open_brace = open_brace_idx?;
    let before_brace = before[..open_brace].trim_end();
    let before_amp = before_brace
        .strip_suffix('&')
        .map(str::trim_end)
        .unwrap_or(before_brace);
    let target_ident: String = before_amp
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if target_ident.is_empty() {
        return None;
    }
    let scope = doc.report.scope_for_sentence(sent_idx);
    let (hf, _) = doc.report.resolve_target_form_in_scope(&target_ident, scope)?;
    Some(hf.name)
}

pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let raw = uri.strip_prefix("file://")?;
    let decoded = percent_decode(raw);
    let bytes = decoded.as_bytes();
    // На Windows URI имеет вид `/C:/path/to/file.rl`
    let pb = if bytes.len() >= 3
        && bytes[0] == b'/'
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b':'
    {
        PathBuf::from(&decoded[1..])
    } else {
        PathBuf::from(decoded)
    };
    Some(normalize_path(&pb))
}

/// Кроссплатформенный конвертер `Path` в `file://` URI (без Windows `\\?\` префиксов)
pub fn path_to_uri(path: &std::path::Path) -> String {
    let norm = normalize_path(path);
    let raw = norm.to_string_lossy().replace('\\', "/");
    let s = raw.strip_prefix("//?/").unwrap_or(&raw);
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

fn paths_match(a: &std::path::Path, b: &std::path::Path) -> bool {
    if normalize_path(a) == normalize_path(b) {
        return true;
    }
    if let (Ok(ca), Ok(cb)) = (a.canonicalize(), b.canonicalize()) {
        return ca == cb;
    }
    false
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
            "razomy/inspect" => {
                if let Some(req_id) = id {
                    let res = self.on_razomy_inspect(&params);
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

    pub fn in_memory_files(&self) -> HashMap<PathBuf, String> {
        let mut map = HashMap::new();
        for (uri, doc) in &self.documents {
            if let Some(path) = uri_to_path(uri) {
                map.insert(path, doc.text.clone());
            }
        }
        map
    }

    pub fn find_doc_by_path(&self, target_path: &std::path::Path) -> Option<&DocumentState> {
        let direct_uri = path_to_uri(target_path);
        if let Some(doc) = self.documents.get(&direct_uri) {
            return Some(doc);
        }
        self.documents.values().find(|doc| {
            uri_to_path(&doc.uri).map_or(false, |doc_path| paths_match(&doc_path, target_path))
        })
    }

    /// Преобразует `SourceOrigin` любого символа (включая символы из импортированных файлов!)
    /// в точный LSP Location `{ "uri": ..., "range": ... }`.
    pub fn location_for_origin(
        &self,
        current_doc: &DocumentState,
        origin: &SourceOrigin,
        symbol_name: &str,
    ) -> Option<JsonValue> {
        if let Some(ref target_path) = origin.file_path {
            if let Some(target_doc) = self.find_doc_by_path(target_path) {
                return Some(build_location_in_doc(
                    target_doc,
                    origin.sentence_index,
                    symbol_name,
                ));
            }
            // Если импортированный файл еще не был открыт в кэше, читаем его с диска на лету
            if let Ok(text) = std::fs::read_to_string(target_path) {
                let temp_doc = DocumentState::new_with_memory(
                    path_to_uri(target_path),
                    1,
                    text,
                    &self.in_memory_files(),
                );
                return Some(build_location_in_doc(
                    &temp_doc,
                    origin.sentence_index,
                    symbol_name,
                ));
            }
            return None;
        }
        Some(build_location_in_doc(
            current_doc,
            origin.sentence_index,
            symbol_name,
        ))
    }

    fn ensure_document_loaded(&mut self, uri: &str) {
        if self.documents.contains_key(uri) {
            return;
        }
        if let Some(path) = uri_to_path(uri) {
            self.ensure_path_loaded(&path);
        }
    }

    fn ensure_path_loaded(&mut self, path: &std::path::Path) {
        if self.find_doc_by_path(path).is_some() {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(path) {
            let uri = path_to_uri(path);
            let mem = self.in_memory_files();
            let state = DocumentState::new_with_memory(uri.clone(), 1, text, &mem);
            self.documents.insert(uri, state);
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
                            let range = doc.sentence_range(
                                d.sentence_index,
                                d.highlight_token.as_deref(),
                            );
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
                // E08: Неизвестное поле — замена на ближайшее похожее поле формы
                "E08_UNKNOWN_MEMBER" => {
                    if let Some(suggested) = d
                        .message
                        .split("Возможно, вы имели в виду '")
                        .nth(1)
                        .and_then(|s| s.strip_suffix("'?"))
                    {
                        if let Some(tok) = &d.highlight_token {
                            let replacement = if let Some((obj, _)) = tok.rsplit_once('.') {
                                format!("{obj}.{suggested}")
                            } else {
                                suggested.to_string()
                            };
                            let range = doc.sentence_range(d.sentence_index, Some(tok));
                            actions.push(json!({
                                "title": format!("Исправить опечатку на '{replacement}'"),
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
                // E11: Неизвестный тип — замена на ближайшую существующую форму (например, User -> User2)
                "E11_UNKNOWN_TYPE" => {
                    if let Some(suggested) = d
                        .message
                        .split("Возможно, вы имели в виду '")
                        .nth(1)
                        .and_then(|s| s.strip_suffix("'?"))
                    {
                        if let Some(tok) = &d.highlight_token {
                            let range = doc.sentence_range(d.sentence_index, Some(tok));
                            actions.push(json!({
                                "title": format!("Заменить тип '{tok}' на '{suggested}'"),
                                "kind": "quickfix",
                                "edit": {
                                    "changes": {
                                        doc.uri.clone(): [{
                                            "range": range.to_json(),
                                            "newText": suggested
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


        // --- РЕФАКТОРИНГ: ЭКСПОРТ ОБЪЕКТА ПОД КУРСОРОМ В ОТДЕЛЬНЫЙ ФАЙЛ `<Name>.rl` ---
        if let Some(range_obj) = params.get("range").and_then(|r| r.get("start")) {
            let line = range_obj.get("line").and_then(JsonValue::as_u64).unwrap_or(0) as usize;
            let character = range_obj.get("character").and_then(JsonValue::as_u64).unwrap_or(0) as usize;
            let cursor_pos = LspPosition { line, character };
            let (word, _) = doc.word_at_position(cursor_pos);
            let clean_ident = word.trim_matches('.').split('.').next().unwrap_or("");

            if !clean_ident.is_empty() {
                // Ищем sentence_index для формы, функции или глобального объекта с таким именем
                let target_sent_idx = doc
                    .report
                    .hir
                    .forms
                    .get(clean_ident)
                    .filter(|f| !f.is_prelude && f.origin.file_path.is_none())
                    .map(|f| f.origin.sentence_index)
                    .or_else(|| {
                        doc.report
                            .hir
                            .functions
                            .get(clean_ident)
                            .filter(|f| !f.is_prelude && f.origin.file_path.is_none())
                            .map(|f| f.origin.sentence_index)
                    })
                    .or_else(|| {
                        doc.report
                            .hir
                            .globals
                            .get(clean_ident)
                            .filter(|g| g.origin.file_path.is_none())
                            .map(|g| g.origin.sentence_index)
                    });

                if let Some(sent_idx) = target_sent_idx {
                    if let Some(&(s_start, s_end)) = doc.sentence_spans.get(sent_idx) {
                        if let Some(cur_path) = uri_to_path(&doc.uri) {
                            if let Some(parent_dir) = cur_path.parent() {
                                let new_file_path = parent_dir.join(format!("{clean_ident}.rl"));
                                let new_file_uri = path_to_uri(&new_file_path);
                                let object_code = doc.text[s_start..s_end.min(doc.text.len())].trim();
                                let formatted_content = format_razomy_source(object_code);
                                let sentence_range = doc.span_to_range(s_start, s_end);

                                // Заменяем определение в текущем файле на директиву импорта `_ <Name>`
                                let import_stmt = format!("_ {clean_ident}");

                                actions.push(json!({
                                    "title": format!("Экспортировать '{clean_ident}' в файл '{clean_ident}.rl'"),
                                    "kind": "refactor.extract",
                                    "edit": {
                                        "documentChanges": [
                                            {
                                                "kind": "create",
                                                "uri": new_file_uri,
                                                "options": { "overwrite": true, "ignoreIfExists": false }
                                            },
                                            {
                                                "textDocument": { "uri": new_file_uri, "version": null },
                                                "edits": [{
                                                    "range": LspRange::default().to_json(),
                                                    "newText": formatted_content
                                                }]
                                            },
                                            {
                                                "textDocument": { "uri": doc.uri, "version": doc.version },
                                                "edits": [{
                                                    "range": sentence_range.to_json(),
                                                    "newText": import_stmt
                                                }]
                                            }
                                        ]
                                    }
                                }));
                            }
                        }
                    }
                }
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
                if !hf.is_prelude
                    && hf.origin.file_path.is_none()
                    && is_pascal_case(name)
                    && name.to_lowercase().contains(&query)
                {
                    let range = doc.sentence_range(Some(hf.origin.sentence_index), Some(name));
                    results.push(json!({
                        "name": name,
                        "kind": 23,
                        "location": { "uri": doc.uri, "range": range.to_json() }
                    }));
                }
            }
            for (name, f) in &doc.report.hir.functions {
                if !f.is_prelude
                    && f.origin.file_path.is_none()
                    && name.to_lowercase().contains(&query)
                {
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
        let mut mem = self.in_memory_files();
        let current_path = uri_to_path(&uri);
        if let Some(ref p) = current_path {
            mem.insert(p.clone(), text.clone());
        }

        let state = DocumentState::new_with_memory(uri.clone(), version, text, &mem);

        // 1. Подгружаем все явно импортированные файлы (`_ models`, `_ Path"./models.rl"`, и т.д.)
        let imported_paths: Vec<PathBuf> = state
            .report
            .hir
            .imports
            .iter()
            .filter_map(|imp| imp.resolved_path.clone())
            .collect();
        for target_path in imported_paths {
            self.ensure_path_loaded(&target_path);
        }

        // 2. Подгружаем соседние `.rl` файлы из той же директории (для Find References / Rename по проекту)
        if let Some(base_dir) = current_path.as_deref().and_then(|p| p.parent()) {
            if let Ok(entries) = std::fs::read_dir(base_dir) {
                for entry in entries.flatten().take(32) {
                    let ep = entry.path();
                    if ep.is_file() && ep.extension().and_then(|e| e.to_str()) == Some("rl") {
                        self.ensure_path_loaded(&ep);
                    }
                }
            }
        }

        self.publish_diagnostics(&state, writer)?;
        self.documents.insert(uri.clone(), state);

        // 3. Если обновился файл, который импортируется другими открытыми документами в IDE,
        // пересчитываем диагностику и типы для зависимых открытых файлов!
        if let Some(ref updated_path) = current_path {
            let full_mem = self.in_memory_files();
            let dependent_uris: Vec<(String, i64, String)> = self
                .documents
                .values()
                .filter(|doc| {
                    doc.uri != uri
                        && doc.report.hir.imports.iter().any(|imp| {
                            imp.resolved_path
                                .as_deref()
                                .map_or(false, |rp| paths_match(rp, updated_path))
                        })
                })
                .map(|doc| (doc.uri.clone(), doc.version, doc.text.clone()))
                .collect();

            for (dep_uri, dep_ver, dep_text) in dependent_uris {
                let refreshed =
                    DocumentState::new_with_memory(dep_uri.clone(), dep_ver, dep_text, &full_mem);
                self.publish_diagnostics(&refreshed, writer)?;
                self.documents.insert(dep_uri, refreshed);
            }
        }

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
                let hint = d
                    .highlight_token
                    .as_deref()
                    .or(d.context_symbol.as_deref());
                doc.sentence_range(d.sentence_index, hint)
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
        let sent_idx = doc.sentence_index_at_pos(pos);

        let target = word.trim_matches('.');
        if let Some(md) = doc.report.hover_at(target, sent_idx) {
            return json!({
                "contents": { "kind": "markdown", "value": md },
                "range": range.to_json()
            });
        }

        if let Some(enclosing_form) = find_enclosing_struct_init_form(doc, pos, sent_idx) {
            let member = target.split('.').last().unwrap_or(target);
            if let Some(md) = doc
                .report
                .hover_at(&format!("{enclosing_form}.{member}"), sent_idx)
            {
                return json!({
                    "contents": { "kind": "markdown", "value": md },
                    "range": range.to_json()
                });
            }
        }

        let last_seg = target.split('.').last().unwrap_or(target);
        if let Some(md) = doc.report.hover_at(last_seg, sent_idx) {
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

        let sent_idx = doc.sentence_index_at_pos(pos);
        let query = extract_completion_query(doc, pos);

        let mut items = Vec::new();
        // Если курсор внутри блока инициализации формы `FormName { <cursor> }` или роутера `expr . { <cursor> }`
        if !query.contains('.') {
            items.extend(complete_brace_context(doc, pos, &query, sent_idx));
        }

        items.extend(doc.report.completions_at(&query, sent_idx));

        let mut seen = std::collections::HashSet::new();
        let lsp_items: Vec<JsonValue> = items
            .into_iter()
            .filter(|it| seen.insert(it.label.clone()))
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
        let sent_idx = doc.sentence_index_at_pos(pos);
        let scope = doc.report.scope_for_sentence(sent_idx);

        // 0. Если кликнули на строку импорта (`_ models`, `Base root.base`, `_ Path"./models.rl"`)
        if let Some(idx) = sent_idx {
            if let Some(imp) = doc
                .report
                .hir
                .imports
                .iter()
                .find(|i| i.origin.file_path.is_none() && i.origin.sentence_index == idx)
            {
                let clean_word = target.split('.').next().unwrap_or(target);
                let is_destructured_symbol =
                    imp.targets.iter().any(|t| t != "_" && t == clean_word)
                        && (doc.report.hir.forms.contains_key(clean_word)
                            || doc.report.hir.functions.contains_key(clean_word));
                if !is_destructured_symbol {
                    if let Some(ref target_path) = imp.resolved_path {
                        let uri = self
                            .find_doc_by_path(target_path)
                            .map(|d| d.uri.clone())
                            .unwrap_or_else(|| path_to_uri(target_path));
                        return json!({
                            "uri": uri,
                            "range": LspRange::default().to_json()
                        });
                    }
                }
            }
        }

        if target.is_empty() {
            return JsonValue::Null;
        }

        // Определяем цель перехода: учитываем пайп `cat . greet` и поля внутри `User { id "1", name "Alex" }`
        let effective_target = if !target.contains('.') {
            if let Some(piped) = find_piped_receiver_before_cursor(doc, pos, target) {
                piped
            } else if let Some(enclosing_form) = find_enclosing_struct_init_form(doc, pos, sent_idx)
            {
                if doc
                    .report
                    .resolve_type_to_form(&enclosing_form)
                    .map_or(false, |hf| hf.fields.contains_key(target))
                {
                    format!("{enclosing_form}.{target}")
                } else {
                    target.to_string()
                }
            } else {
                target.to_string()
            }
        } else {
            target.to_string()
        };

        // 1. Переход к полю, методу, UFCS-функции или предку (`my_car.brand`, `cat . greet`, `User { name "Alex" }`)
        if let Some((base, member)) = effective_target.rsplit_once('.') {
            if let Some((hf, _)) = doc.report.resolve_target_form_in_scope(base, scope) {
                if let Some(field) = hf.fields.get(member) {
                    let is_prelude = doc
                        .report
                        .hir
                        .forms
                        .get(&field.origin_form)
                        .map_or(false, |f| f.is_prelude);
                    if !is_prelude {
                        if let Some(loc) = self.location_for_origin(doc, &field.origin, member) {
                            return loc;
                        }
                    }
                }
                if let Some(method) = hf.methods.get(member) {
                    if !method.is_prelude {
                        if let Some(loc) = self.location_for_origin(doc, &method.origin, member) {
                            return loc;
                        }
                    }
                }
                if let Some(ufcs_fn) = doc.report.hir.functions.get(member) {
                    if !ufcs_fn.is_prelude {
                        if let Some(loc) = self.location_for_origin(doc, &ufcs_fn.origin, member) {
                            return loc;
                        }
                    }
                }
                if let Some(anc_hf) = doc.report.hir.forms.get(member) {
                    if !anc_hf.is_prelude {
                        if let Some(loc) = self.location_for_origin(doc, &anc_hf.origin, member) {
                            return loc;
                        }
                    }
                }
            }
        }

        // 2. Переход к форме, функции или глобальной переменной (в текущем или импортированном файле)
        let clean = target.split('.').next().unwrap_or(target);
        if let Some(hf) = doc.report.hir.forms.get(clean) {
            if !hf.is_prelude && (is_pascal_case(clean) || !doc.report.hir.globals.contains_key(clean))
            {
                if let Some(loc) = self.location_for_origin(doc, &hf.origin, clean) {
                    return loc;
                }
            }
        }
        if let Some(f) = doc.report.hir.functions.get(clean) {
            if !f.is_prelude {
                if let Some(loc) = self.location_for_origin(doc, &f.origin, clean) {
                    return loc;
                }
            }
        }
        if let Some(g) = doc.report.hir.globals.get(clean) {
            if let Some(loc) = self.location_for_origin(doc, &g.origin, clean) {
                return loc;
            }
        }

        // Fallback: поиск по всем открытым документам воркспейса (даже если файл ещё не импортирован)
        for cand in self.documents.values() {
            if let Some(hf) = cand.report.hir.forms.get(clean) {
                if !hf.is_prelude && hf.origin.file_path.is_none() && is_pascal_case(clean) {
                    if let Some(loc) = self.location_for_origin(cand, &hf.origin, clean) {
                        return loc;
                    }
                }
            }
            if let Some(f) = cand.report.hir.functions.get(clean) {
                if !f.is_prelude && f.origin.file_path.is_none() {
                    if let Some(loc) = self.location_for_origin(cand, &f.origin, clean) {
                        return loc;
                    }
                }
            }
        }

        // 3. Переход к локальной переменной или параметру внутри текущей функции/формы
        if let Some(sc) = scope {
            if let Some(var_ty) = sc.var_types.get(clean) {
                let origin = SourceOrigin {
                    sentence_index: sc.sentence_index,
                    file_path: None,
                    symbol_context: sc.context_name.clone(),
                    raw_ast: vec![],
                };
                if let Some(loc) = self.location_for_origin(doc, &origin, clean) {
                    return loc;
                }
                if let Some(loc) = self.location_for_origin(doc, &origin, var_ty) {
                    return loc;
                }
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
                if hf.is_prelude || hf.origin.file_path.is_some() || !is_pascal_case(name) {
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
                if f.is_prelude || f.origin.file_path.is_some() {
                    continue;
                }
                let range = doc.sentence_range(Some(f.origin.sentence_index), Some(fn_name));
                symbols.push(json!({
                    "name": f.name,
                    "detail": f.effective_return_type().unwrap_or("Stream"),
                    "kind": 12,
                    "range": range.to_json(),
                    "selectionRange": range.to_json()
                }));
            }
        }

        for g_name in &doc.report.hir.global_order {
            if is_pascal_case(g_name) && doc.report.hir.forms.contains_key(g_name) {
                continue;
            }
            if let Some(g) = doc.report.hir.globals.get(g_name) {
                if g.origin.file_path.is_some() {
                    continue;
                }
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
        // 1. Inlay hints для глобальных привязок
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
        // 2. Inlay hints для выведенных возвращаемых типов функций и локальных переменных
        for fn_name in &doc.report.hir.function_order {
            let Some(f) = doc.report.hir.functions.get(fn_name) else {
                continue;
            };
            if f.is_prelude {
                continue;
            }
            if f.return_type.is_none() {
                if let Some(inf_ret) = &f.inferred_return_type {
                    if let Some(&(s_start, s_end)) = doc.sentence_spans.get(f.origin.sentence_index)
                    {
                        let slice = &doc.text[s_start..s_end.min(doc.text.len())];
                        if let Some(open_bracket) = slice.find('[') {
                            let pos_before = doc.offset_to_position(s_start + open_bracket);
                            hints.push(json!({
                                "position": { "line": pos_before.line, "character": pos_before.character },
                                "label": format!("-> {inf_ret} "),
                                "kind": 1,
                                "paddingLeft": true,
                                "paddingRight": true
                            }));
                        }
                    }
                }
            }
            if let Some(sc) = doc.report.scopes.get(fn_name) {
                let param_names: std::collections::HashSet<&str> =
                    f.params.iter().map(|p| p.name.as_str()).collect();
                for (var_name, var_ty) in &sc.var_types {
                    if matches!(
                        var_name.as_str(),
                        "self" | "Self" | "_" | "_0" | "_1" | "_2" | "_3"
                    ) || param_names.contains(var_name.as_str())
                        || is_pascal_case(var_name)
                        || var_ty == "Any"
                    {
                        continue;
                    }
                    let r = doc.sentence_range(Some(f.origin.sentence_index), Some(var_name));
                    hints.push(json!({
                        "position": { "line": r.end.line, "character": r.end.character },
                        "label": format!(": {var_ty}"),
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
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
            .collect::<String>()
            .chars()
            .rev()
            .collect();

        let active_param = prefix[open_paren..].matches(',').count();
        let sent_idx = doc.sentence_index_at_pos(pos);
        let scope = doc.report.scope_for_sentence(sent_idx);

        // 1. Глобальная функция
        if let Some(f) = doc.report.hir.functions.get(&callee) {
            return build_signature_help_json(&f.name, &f.params, f.effective_return_type(), active_param);
        }

        // 2. Метод формы `Db.find_user(` или `my_car.method(`
        if let Some((base, method_name)) = callee.rsplit_once('.') {
            if let Some((hf, _)) = doc.report.resolve_target_form_in_scope(base, scope) {
                if let Some(m) = hf.methods.get(method_name) {
                    let full_name = format!("{}.{}", hf.name, m.name);
                    return build_signature_help_json(
                        &full_name,
                        &m.params,
                        m.effective_return_type(),
                        active_param,
                    );
                }
            }
        }

        // 3. Конструктор параметризованной формы `MathOps(10, 2)` или `Money("199.99")`
        if let Some(hf) = doc.report.hir.forms.get(&callee) {
            if !hf.value_params.is_empty() {
                return build_signature_help_json(
                    &hf.name,
                    &hf.value_params,
                    Some(&hf.name),
                    active_param,
                );
            }
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

fn build_signature_help_json(
    name: &str,
    params: &[crate::hir::HirParam],
    ret_ty: Option<&str>,
    active_param: usize,
) -> JsonValue {
    let params_json: Vec<JsonValue> = params
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
    let param_labels: Vec<&str> = params_json
        .iter()
        .filter_map(|p| p.get("label").and_then(JsonValue::as_str))
        .collect();
    let ret_suffix = ret_ty.map(|r| format!(" -> {r}")).unwrap_or_default();
    json!({
        "signatures": [{
            "label": format!("{}({}){}", name, param_labels.join(", "), ret_suffix),
            "parameters": params_json
        }],
        "activeSignature": 0,
        "activeParameter": active_param
    })
}

fn build_location_in_doc(
    target_doc: &DocumentState,
    sentence_idx: usize,
    symbol_name: &str,
) -> JsonValue {
    let range = target_doc.sentence_range(Some(sentence_idx), Some(symbol_name));
    json!({
        "uri": target_doc.uri,
        "range": range.to_json()
    })
}

/// Единый экстрактор запроса автодополнения: одинаково чисто разбирает
/// слитную точку (`cat.`, `cat.gr`, `state.err.`) и пробельный пайп (`cat .`, `cat . `, `cat .gr`, `cat . gr`).
fn extract_completion_query(doc: &DocumentState, pos: LspPosition) -> String {
    let offset = doc.position_to_offset(pos).min(doc.text.len());
    let before = &doc.text[..offset];

    let prefix: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    let before_prefix = &before[..before.len() - prefix.len()];
    let before_trimmed = before_prefix.trim_end_matches([' ', '\t', '\r', '\n']);

    let before_op = before_trimmed
        .strip_suffix(".>")
        .or_else(|| before_trimmed.strip_suffix(".&"))
        .or_else(|| before_trimmed.strip_suffix(".?"))
        .or_else(|| {
            if before_trimmed.ends_with("..") {
                None
            } else {
                before_trimmed.strip_suffix('.')
            }
        });

    if let Some(recv_part) = before_op {
        let recv_trimmed = recv_part.trim_end_matches([' ', '\t', '\r', '\n']);
        let receiver: String = recv_trimmed
            .chars()
            .rev()
            .take_while(|c| {
                c.is_ascii_alphanumeric() || matches!(*c, '_' | '.' | '(' | ')' | '-')
            })
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let clean_recv = receiver.trim_matches('.');
        if !clean_recv.is_empty() {
            return format!("{clean_recv}.{prefix}");
        }
    }

    prefix
}

fn find_piped_receiver_before_cursor(
    doc: &DocumentState,
    pos: LspPosition,
    current_word: &str,
) -> Option<String> {
    let offset = doc.position_to_offset(pos).min(doc.text.len());
    let clean_word = current_word.trim_start_matches('.');
    let word_start = doc.text[..offset]
        .rfind(clean_word)
        .unwrap_or_else(|| offset.saturating_sub(clean_word.len()));
    let before = doc.text[..word_start].trim_end();
    let before_op = before
        .strip_suffix(".>")
        .or_else(|| before.strip_suffix(".&"))
        .or_else(|| before.strip_suffix(".?"))
        .or_else(|| before.strip_suffix('.'))?
        .trim_end();
    let token: String = before_op
        .chars()
        .rev()
        .take_while(|c| {
            c.is_ascii_alphanumeric() || *c == '_' || *c == '.' || *c == '(' || *c == ')'
        })
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if token.is_empty() {
        None
    } else {
        Some(format!("{token}.{clean_word}"))
    }
}

/// Автодополнение полей при инициализации формы `User { <cursor> }` и веток роутера `g . { <cursor> }`
fn complete_brace_context(
    doc: &DocumentState,
    pos: LspPosition,
    prefix: &str,
    sent_idx: Option<usize>,
) -> Vec<CompletionItem> {
    let mut out = Vec::new();
    let offset = doc.position_to_offset(pos);
    let before = &doc.text[..offset];
    let Some(open_brace) = before.rfind('{') else {
        return out;
    };
    if before[open_brace..].contains('}') {
        return out;
    }

    let before_brace = before[..open_brace].trim_end();
    let scope = doc.report.scope_for_sentence(sent_idx);

    // 1. Роутер по объединению типов: `g . { <cursor> }`
    if let Some(before_dot) = before_brace.strip_suffix('.') {
        let recv: String = before_dot
            .trim_end()
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.' || *c == '(' || *c == ')')
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Some(ty) = doc.report.resolve_expr_type(&recv, scope) {
            let variants: Vec<String> = if let Some(hf) = doc.report.hir.forms.get(&ty) {
                hf.union_variants.clone()
            } else if ty.contains('|') {
                ty.split('|').map(|s| s.trim().to_string()).collect()
            } else {
                vec![]
            };
            for v in variants {
                let v_clean = v.split('(').next().unwrap_or(&v).trim();
                if v_clean.starts_with(prefix) {
                    out.push(CompletionItem {
                        label: v_clean.to_string(),
                        kind: "Form",
                        detail: format!("Router branch for {v}"),
                        insert_text: format!("{v_clean} [ ]"),
                    });
                }
            }
        }
        return out;
    }

    // 2. Инициализация или обновление формы: `User { <cursor> }` или `my_car & { <cursor> }`
    let before_amp = before_brace
        .strip_suffix('&')
        .map(str::trim_end)
        .unwrap_or(before_brace);
    let target_ident: String = before_amp
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if target_ident.is_empty() {
        return out;
    }

    if let Some((hf, _)) = doc.report.resolve_target_form_in_scope(&target_ident, scope) {
        for fname in &hf.field_order {
            if fname.starts_with(prefix) {
                if let Some(f) = hf.fields.get(fname) {
                    if !f.is_reactive {
                        out.push(CompletionItem {
                            label: fname.clone(),
                            kind: "Field",
                            detail: format!("{}: {} (in {})", fname, f.inferred_type, hf.name),
                            insert_text: format!("{fname} "),
                        });
                    }
                }
            }
        }
    }

    out
}

// =============================================================================
// ИНСПЕКТОР ПАРСЕРА, AST, HIR И ДИАГНОСТИКИ ДЛЯ PARSER.HTML (`razomy/inspect`)
// =============================================================================

impl RazomyLanguageServer {
    pub fn on_razomy_inspect(&mut self, params: &JsonValue) -> JsonValue {
        let uri = params
            .get("textDocument")
            .and_then(|t| t.get("uri"))
            .or_else(|| params.get("uri"))
            .and_then(JsonValue::as_str)
            .unwrap_or("file:///workspace/playground.rl")
            .to_string();

        if let Some(text) = params.get("text").and_then(JsonValue::as_str) {
            let version = params.get("version").and_then(JsonValue::as_i64).unwrap_or(1);
            let mut sink = Vec::new();
            let _ = self.update_document(uri.clone(), version, text.to_string(), &mut sink);
        } else {
            self.ensure_document_loaded(&uri);
        }

        let Some(doc) = self.documents.get(&uri) else {
            return JsonValue::Null;
        };

        let sentences_ast = parse_code(&doc.text)
            .or_else(|_| parse_code(&doc.recovered_text))
            .unwrap_or_default();

        let mut sentences_json = Vec::new();
        for (idx, sent) in sentences_ast.iter().enumerate() {
            let (s_byte, e_byte) = doc.sentence_spans.get(idx).copied().unwrap_or((0, 0));
            let range = doc.span_to_range(s_byte, e_byte);
            let raw_slice = doc
                .text
                .get(s_byte..e_byte.min(doc.text.len()))
                .unwrap_or("")
                .trim();
            sentences_json.push(serialize_sentence_inspection(
                idx, s_byte, e_byte, range, raw_slice, sent,
            ));
        }

        let mut forms_json = Vec::new();
        for name in &doc.report.hir.form_order {
            if let Some(hf) = doc.report.hir.forms.get(name) {
                if hf.is_prelude {
                    continue;
                }
                let range = doc.sentence_range(Some(hf.origin.sentence_index), Some(name));
                let fields: Vec<JsonValue> = hf
                    .field_order
                    .iter()
                    .filter_map(|fname| hf.fields.get(fname))
                    .map(|f| {
                        json!({
                            "name": f.name,
                            "type": f.inferred_type,
                            "origin": f.origin_form,
                            "defaultExpr": format_token_list(&f.default_expr),
                            "reactive": f.is_reactive,
                            "ambiguous": f.is_ambiguous,
                            "conflicts": f.conflicting_origins,
                            "visibility": format!("{:?}", f.visibility),
                            "sentenceIndex": f.origin.sentence_index,
                        })
                    })
                    .collect();

                let methods: Vec<JsonValue> = hf
                    .method_order
                    .iter()
                    .filter_map(|mname| hf.methods.get(mname))
                    .map(serialize_hir_function)
                    .collect();

                let value_params: Vec<JsonValue> =
                    hf.value_params.iter().map(serialize_hir_param).collect();

                forms_json.push(json!({
                    "name": hf.name,
                    "typeParams": hf.type_params,
                    "valueParams": value_params,
                    "typeArgs": hf.type_args,
                    "ancestors": hf.ancestors,
                    "unionVariants": hf.union_variants,
                    "refinementPredicates": hf.refinement_predicates.iter().map(|t| format_token_list(std::slice::from_ref(t))).collect::<Vec<_>>(),
                    "isBypass": hf.is_bypass,
                    "isAlgebraicAlias": hf.is_algebraic_alias,
                    "fields": fields,
                    "methods": methods,
                    "sentenceIndex": hf.origin.sentence_index,
                    "range": range.to_json(),
                }));
            }
        }

        let mut functions_json = Vec::new();
        for fn_name in &doc.report.hir.function_order {
            if let Some(f) = doc.report.hir.functions.get(fn_name) {
                if f.is_prelude {
                    continue;
                }
                let mut f_json = serialize_hir_function(f);
                let range = doc.sentence_range(Some(f.origin.sentence_index), Some(fn_name));
                if let Some(obj) = f_json.as_object_mut() {
                    obj.insert("range".into(), range.to_json());
                    if let Some(sc) = doc.report.scopes.get(fn_name) {
                        obj.insert("scopeVars".into(), json!(sc.var_types));
                    }
                }
                functions_json.push(f_json);
            }
        }

        let mut globals_json = Vec::new();
        for g_name in &doc.report.hir.global_order {
            if let Some(g) = doc.report.hir.globals.get(g_name) {
                let range = doc.sentence_range(Some(g.origin.sentence_index), Some(g_name));
                globals_json.push(json!({
                    "key": g.key,
                    "type": g.inferred_type,
                    "valueExpr": format_token_list(&g.value_expr),
                    "sentenceIndex": g.origin.sentence_index,
                    "range": range.to_json(),
                }));
            }
        }

        let imports_json: Vec<JsonValue> = doc
            .report
            .hir
            .imports
            .iter()
            .map(|imp| {
                json!({
                    "targets": imp.targets,
                    "sourcePath": imp.source_path,
                    "sentenceIndex": imp.origin.sentence_index,
                })
            })
            .collect();

        let diags_json: Vec<JsonValue> = doc
            .report
            .diagnostics
            .iter()
            .map(|d| {
                let severity_num = match d.severity {
                    Severity::Error => 1,
                    Severity::Warning => 2,
                    Severity::Information => 3,
                    Severity::Hint => 4,
                };
                let range = if d.code == "E00_SYNTAX" {
                    extract_pest_error_range(doc)
                } else {
                    let hint = d
                        .highlight_token
                        .as_deref()
                        .or(d.context_symbol.as_deref());
                    doc.sentence_range(d.sentence_index, hint)
                };
                json!({
                    "severity": severity_num,
                    "severityLabel": format!("{:?}", d.severity),
                    "code": d.code,
                    "message": d.message,
                    "symbol": d.context_symbol,
                    "highlightToken": d.highlight_token,
                    "sentenceIndex": d.sentence_index,
                    "range": range.to_json(),
                })
            })
            .collect();

        let doc_param = json!({ "textDocument": { "uri": uri } });
        let inlay_hints = self.on_inlay_hints(&doc_param);
        let doc_symbols = self.on_document_symbol(&doc_param);
        let code_actions = self.on_code_action(&doc_param);

        json!({
            "uri": doc.uri,
            "version": doc.version,
            "sentences": sentences_json,
            "hir": {
                "forms": forms_json,
                "functions": functions_json,
                "globals": globals_json,
                "imports": imports_json,
            },
            "diagnostics": diags_json,
            "inlayHints": inlay_hints,
            "documentSymbols": doc_symbols,
            "codeActions": code_actions,
        })
    }

    /// Запускает HTTP JSON-RPC 2.0 сервер для работы `parser.html` с нативным LSP-сервером Razomy
    pub fn run_http(&mut self, bind_addr: &str) -> io::Result<()> {
        let listener = std::net::TcpListener::bind(bind_addr)?;
        eprintln!("[razomy-lsp] HTTP Studio & LSP Server listening on http://{bind_addr}");
        eprintln!("[razomy-lsp] Open http://{bind_addr} in your browser to inspect Razomy syntax!");

        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = self.handle_http_connection(&mut stream);
        }
        Ok(())
    }

    fn handle_http_connection(&mut self, stream: &mut std::net::TcpStream) -> io::Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line)? == 0 {
            return Ok(());
        }

        let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
        let method = parts.first().copied().unwrap_or("GET");
        let path = parts.get(1).copied().unwrap_or("/");

        let mut content_length = 0usize;
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                if k.trim().eq_ignore_ascii_case("Content-Length") {
                    content_length = v.trim().parse().unwrap_or(0);
                }
            }
        }

        if method == "OPTIONS" {
            write!(
                stream,
                "HTTP/1.1 204 No Content\r\n\
                 Access-Control-Allow-Origin: *\r\n\
                 Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                 Access-Control-Allow-Headers: Content-Type\r\n\
                 Content-Length: 0\r\n\r\n"
            )?;
            return stream.flush();
        }

        if method == "GET" && (path == "/" || path.starts_with("/parser.html")) {
            let html = std::fs::read_to_string("parser.html")
                .or_else(|_| std::fs::read_to_string("src/parser.html"))
                .unwrap_or_else(|_| "<h1>parser.html not found in working directory</h1>".into());
            let bytes = html.as_bytes();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/html; charset=utf-8\r\n\
                 Access-Control-Allow-Origin: *\r\n\
                 Content-Length: {}\r\n\r\n",
                bytes.len()
            )?;
            stream.write_all(bytes)?;
            return stream.flush();
        }

        if method == "POST" {
            let mut body = vec![0u8; content_length];
            reader.read_exact(&mut body)?;

            let incoming: JsonValue = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
            let mut rpc_out = Vec::new();
            let _ = self.handle_message(&incoming, &mut rpc_out);

            let mut out_reader = BufReader::new(rpc_out.as_slice());
            let mut messages = Vec::new();
            while let Ok(Some(msg)) = read_rpc_message(&mut out_reader) {
                messages.push(msg);
            }

            let response_json = match messages.len() {
                0 => json!({ "jsonrpc": "2.0", "id": incoming.get("id").cloned().unwrap_or(JsonValue::Null), "result": null }),
                1 => messages.remove(0),
                _ => json!({
                    "jsonrpc": "2.0",
                    "id": incoming.get("id").cloned().unwrap_or(JsonValue::Null),
                    "result": messages.last().and_then(|m| m.get("result")).cloned().unwrap_or(JsonValue::Null),
                    "notifications": messages
                }),
            };

            let resp_bytes = serde_json::to_vec(&response_json).unwrap_or_default();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: application/json; charset=utf-8\r\n\
                 Access-Control-Allow-Origin: *\r\n\
                 Content-Length: {}\r\n\r\n",
                resp_bytes.len()
            )?;
            stream.write_all(&resp_bytes)?;
            return stream.flush();
        }

        write!(
            stream,
            "HTTP/1.1 404 Not Found\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: 0\r\n\r\n"
        )?;
        stream.flush()
    }
}

fn serialize_hir_param(p: &crate::hir::HirParam) -> JsonValue {
    json!({
        "name": p.name,
        "typeConstraint": p.type_constraint,
        "defaultExpr": format_token_list(&p.default_expr),
        "isAnonymousTypeSlot": p.is_anonymous_type_slot,
        "isTypeParam": p.is_type_param,
        "destructuredFields": p.destructured_fields,
    })
}

fn serialize_hir_function(f: &crate::hir::HirFunction) -> JsonValue {
    json!({
        "name": f.name,
        "signature": crate::analyzer::format_fn_signature(f),
        "params": f.params.iter().map(serialize_hir_param).collect::<Vec<_>>(),
        "autoUnpacks": f.auto_unpacks,
        "returnType": f.return_type,
        "inferredReturnType": f.inferred_return_type,
        "effectiveReturnType": f.effective_return_type(),
        "isContract": f.is_contract,
        "isCompileTime": f.is_compile_time,
        "visibility": format!("{:?}", f.visibility),
        "sentenceIndex": f.origin.sentence_index,
    })
}

fn serialize_sentence_inspection(
    idx: usize,
    start_byte: usize,
    end_byte: usize,
    range: LspRange,
    raw_text: &str,
    sent: &Sentence,
) -> JsonValue {
    let import_json = extract_import(sent).map(|imp| match imp {
        ImportDirective::Wildcard { source } => json!({
            "kind": "Wildcard",
            "targets": ["_"],
            "source": format_import_source(&source),
        }),
        ImportDirective::Destructured { names, source } => json!({
            "kind": "Destructured",
            "targets": names,
            "source": format_import_source(&source),
        }),
        ImportDirective::Namespaced { alias, source } => json!({
            "kind": "Namespaced",
            "targets": [alias],
            "source": format_import_source(&source),
        }),
    });

    let sem = classify_sentence(sent);
    let (kind_name, summary, details) = match &sem {
        SemanticKind::FormDef { name, body, tail } => (
            "FormDef",
            format!("Form {name} {{ {} sentences }}", body.len()),
            json!({ "name": name, "bodyCount": body.len(), "tail": format_token_list(tail) }),
        ),
        SemanticKind::GenericDef { name, params, body } => (
            "GenericDef",
            format!("{name}({} slots) {}", params.len(), format_token_list(body)),
            json!({ "name": name, "paramsCount": params.len(), "body": format_token_list(body) }),
        ),
        SemanticKind::AlgebraicDef { name, expr } => (
            "AlgebraicDef",
            format!("{name} = {}", format_token_list(expr)),
            json!({ "name": name, "expr": format_token_list(expr) }),
        ),
        SemanticKind::FunctionDef {
            name,
            args,
            body,
            tail,
        } => (
            "FunctionDef",
            format!(
                "fn {name}({} args) {} [ {} steps ]",
                args.len(),
                format_token_list(tail),
                body.len()
            ),
            json!({ "name": name, "argsCount": args.len(), "bodyCount": body.len(), "tail": format_token_list(tail) }),
        ),
        SemanticKind::StreamOrBranch { name, stream } => (
            "StreamOrBranch",
            format!("{name} [ {} steps ]", stream.len()),
            json!({ "name": name, "stepsCount": stream.len() }),
        ),
        SemanticKind::LambdaOrTupleBranch { args, body } => (
            "LambdaOrTupleBranch",
            format!("({} args) [ {} steps ]", args.len(), body.len()),
            json!({ "argsCount": args.len(), "stepsCount": body.len() }),
        ),
        SemanticKind::PatternBranch { pattern, body } => (
            "PatternBranch",
            format!(
                "{} [ {} steps ]",
                format_token_list(std::slice::from_ref(pattern)),
                body.len()
            ),
            json!({ "pattern": format_token_list(std::slice::from_ref(pattern)), "stepsCount": body.len() }),
        ),
        SemanticKind::Binding { key, value } => (
            "Binding",
            format!("{key} ⟵ {}", format_token_list(value)),
            json!({ "key": key, "value": format_token_list(value) }),
        ),
        SemanticKind::TupleBinding { tuple, value } => (
            "TupleBinding",
            format!("({} items) ⟵ {}", tuple.len(), format_token_list(value)),
            json!({ "tupleItems": tuple.len(), "value": format_token_list(value) }),
        ),
        SemanticKind::CallExpr { name, args } => (
            "CallExpr",
            format!("{name}({} args) [tight call ')']", args.len()),
            json!({ "callee": name, "argsCount": args.len() }),
        ),
        SemanticKind::Pipeline(tokens) => (
            "Pipeline",
            format_token_list(tokens),
            json!({ "expr": format_token_list(tokens) }),
        ),
        SemanticKind::Expression(tokens) => (
            "Expression",
            format_token_list(tokens),
            json!({ "expr": format_token_list(tokens) }),
        ),
    };

    let pipeline_json = if has_pipe_op(sent) {
        let (head, steps) = split_pipeline(sent);
        let steps_list: Vec<JsonValue> = steps
            .iter()
            .map(|st| {
                let (op_sym, op_name) = match st.op {
                    PipeOp::Dot => (".", "Dot (Standard Pipe)"),
                    PipeOp::Tap => (".>", "Tap (Side-Effect)"),
                    PipeOp::Enrich => (".&", "Enrich (Merge &)"),
                    PipeOp::Filter => (".?", "Filter (Predicate)"),
                    PipeOp::Parallel => (".|", "Parallel (Fan-Out)"),
                };
                json!({
                    "op": op_sym,
                    "opName": op_name,
                    "expr": format_token_list(&st.expr),
                    "tokens": st.expr.iter().map(serialize_ast_token).collect::<Vec<_>>(),
                })
            })
            .collect();
        Some(json!({
            "head": format_token_list(&head),
            "headTokens": head.iter().map(serialize_ast_token).collect::<Vec<_>>(),
            "steps": steps_list,
        }))
    } else {
        None
    };

    json!({
        "index": idx,
        "startByte": start_byte,
        "endByte": end_byte,
        "range": range.to_json(),
        "rawText": raw_text,
        "semanticKind": if import_json.is_some() { "ImportDirective" } else { kind_name },
        "summary": summary,
        "details": details,
        "import": import_json,
        "pipeline": pipeline_json,
        "tokens": sent.iter().map(serialize_ast_token).collect::<Vec<_>>(),
    })
}

fn format_import_source(src: &ImportSource) -> String {
    match src {
        ImportSource::ModulePath(m) => m.clone(),
        ImportSource::FilePath(f) => format!("Path\"{f}\""),
    }
}

fn serialize_ast_token(tok: &Token) -> JsonValue {
    match tok {
        Token::Path(p) => json!({
            "kind": "Path",
            "value": p,
            "isPascalCase": is_pascal_case(p.split('.').next().unwrap_or(p)),
            "hasIndex": p.split('.').skip(1).any(|s| s.parse::<i32>().is_ok()),
        }),
        Token::Lit(lit) => match lit {
            Literal::Int(i) => json!({ "kind": "Lit", "subKind": "Int", "value": i.to_string() }),
            Literal::Float(f) => json!({ "kind": "Lit", "subKind": "Float", "value": f.to_string() }),
            Literal::Bool(b) => json!({ "kind": "Lit", "subKind": "Bool", "value": b.to_string() }),
            Literal::Byte(b) => json!({ "kind": "Lit", "subKind": "Byte", "value": format!("0x{b:02X}") }),
            Literal::String(s) => json!({ "kind": "Lit", "subKind": "String", "value": format!("\"{s}\"") }),
            Literal::RawString(s) => json!({ "kind": "Lit", "subKind": "RawString", "value": format!("r\"{s}\"") }),
            Literal::Symbol(s) => json!({ "kind": "Lit", "subKind": "Symbol", "value": format!("'{s}'") }),
        },
        Token::Op(op) => json!({
            "kind": "Op",
            "value": op,
            "isPipe": matches!(op.as_str(), "." | ".>" | ".&" | ".?" | ".|"),
            "isAlgebra": matches!(op.as_str(), "&" | "|" | "/"),
        }),
        Token::Range(a, b) => json!({
            "kind": "Range",
            "value": format!("{}..{}", format_token_list(std::slice::from_ref(a)), format_token_list(std::slice::from_ref(b))),
            "start": serialize_ast_token(a),
            "end": serialize_ast_token(b),
        }),
        Token::Group(delim, sents) => {
            let (group_type, bracket_label) = match delim {
                '{' => ("FormGroup", "{}"),
                '[' => ("StreamGroup", "[]"),
                '(' => ("SpacedTupleGroup", "() [spaced]"),
                ')' => ("TightCallGroup", "() [tight ')']"),
                _ => ("Group", "()"),
            };
            json!({
                "kind": "Group",
                "delim": delim.to_string(),
                "groupType": group_type,
                "bracketLabel": bracket_label,
                "sentencesCount": sents.len(),
                "sentences": sents.iter().enumerate().map(|(i, s)| {
                    json!({
                        "index": i,
                        "semanticKind": format!("{:?}", classify_sentence(s)).split('{').next().unwrap_or("Expr").split('(').next().unwrap_or("Expr").trim(),
                        "text": format_token_list(s),
                        "tokens": s.iter().map(serialize_ast_token).collect::<Vec<_>>(),
                    })
                }).collect::<Vec<_>>(),
            })
        }
    }
}

fn format_token_list(tokens: &[Token]) -> String {
    let mut out = Vec::new();
    for tok in tokens {
        match tok {
            Token::Path(p) => out.push(p.clone()),
            Token::Lit(Literal::Int(i)) => out.push(i.to_string()),
            Token::Lit(Literal::Float(f)) => out.push(f.to_string()),
            Token::Lit(Literal::Bool(b)) => out.push(b.to_string()),
            Token::Lit(Literal::Byte(b)) => out.push(format!("0x{b:02X}")),
            Token::Lit(Literal::String(s)) => out.push(format!("\"{s}\"")),
            Token::Lit(Literal::RawString(s)) => out.push(format!("r\"{s}\"")),
            Token::Lit(Literal::Symbol(s)) => out.push(format!("'{s}'")),
            Token::Op(op) => out.push(op.clone()),
            Token::Range(a, b) => out.push(format!(
                "{}..{}",
                format_token_list(std::slice::from_ref(a)),
                format_token_list(std::slice::from_ref(b))
            )),
            Token::Group(')', inner) => {
                let inner_s = inner
                    .iter()
                    .map(|s| format_token_list(s))
                    .collect::<Vec<_>>()
                    .join(", ");
                if let Some(last) = out.last_mut() {
                    last.push_str(&format!("({inner_s})"));
                } else {
                    out.push(format!("({inner_s})"));
                }
            }
            Token::Group(d, inner) => {
                let (open, close) = match d {
                    '{' => ('{', '}'),
                    '[' => ('[', ']'),
                    _ => ('(', ')'),
                };
                let inner_s = inner
                    .iter()
                    .map(|s| format_token_list(s))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push(format!("{open} {inner_s} {close}"));
            }
        }
    }
    out.join(" ")
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

    let parse_res = RazomyParser::parse(Rule::Program, &doc.text)
        .or_else(|_| RazomyParser::parse(Rule::Program, &doc.recovered_text));
    if let Ok(mut pairs) = parse_res {
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

    let next_char = doc
        .text
        .get(end_byte..)
        .and_then(|s| s.chars().next());
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

/// Автоформаттер кода Razomy V5.7:
/// 1. Выравнивает блоки `{}`, `[]`, `()` по уровням отступов
/// 2. Строки продолжения (`| Active`, `& Role`, `. map`) сдвигает на +1 отступ (4 пробела)
///    и форматирует с одиночным пробелом после ведущего оператора (`    | Active`)
/// 3. Выстраивает поля форм и привязки (Bindings) в ровные колонки!
/// 4. Выравнивает хвостовые комментарии `// ...` в отдельную колонку
pub fn format_razomy_source(source: &str) -> String {
    #[derive(Clone)]
    struct LineInfo {
        indent: usize,
        is_empty: bool,
        is_comment_only: bool,
        is_continuation: bool,
        code: String,
        comment: Option<String>,
        tokens: Vec<String>,
    }

    let mut lines_info: Vec<LineInfo> = Vec::new();
    let mut indent_level: usize = 0;
    let mut prev_ended_with_op = false;

    for raw_line in source.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            prev_ended_with_op = false;
            lines_info.push(LineInfo {
                indent: indent_level,
                is_empty: true,
                is_comment_only: false,
                is_continuation: false,
                code: String::new(),
                comment: None,
                tokens: Vec::new(),
            });
            continue;
        }

        let starts_with_closing =
            trimmed.starts_with('}') || trimmed.starts_with(']') || trimmed.starts_with(')');
        if starts_with_closing {
            indent_level = indent_level.saturating_sub(1);
            prev_ended_with_op = false;
        }
        let line_indent = indent_level;

        if trimmed.starts_with("//") || trimmed.starts_with('#') {
            lines_info.push(LineInfo {
                indent: line_indent,
                is_empty: false,
                is_comment_only: true,
                is_continuation: false,
                code: trimmed.to_string(),
                comment: None,
                tokens: Vec::new(),
            });
            continue;
        }

        let (code_part, comment_part) = split_code_and_comment(trimmed);
        let tokens = tokenize_line_for_format(code_part.trim());

        let starts_with_continuation_op = tokens
            .first()
            .map(|t| matches!(t.as_str(), "|" | "&" | "." | ".>" | ".&" | ".?" | ".|" | "->" | "/"))
            .unwrap_or(false);

        let is_continuation =
            !starts_with_closing && (starts_with_continuation_op || prev_ended_with_op);
        let effective_indent = if is_continuation {
            line_indent + 1
        } else {
            line_indent
        };

        let mut net_open: isize = 0;
        for tok in &tokens {
            for ch in tok.chars() {
                if !tok.starts_with('"') && !tok.starts_with('\'') && !tok.starts_with("r\"") {
                    match ch {
                        '{' | '[' | '(' => net_open += 1,
                        '}' | ']' | ')' => net_open -= 1,
                        _ => {}
                    }
                }
            }
        }

        if starts_with_closing {
            net_open += 1;
        }
        if net_open > 0 {
            indent_level += net_open as usize;
        } else if net_open < 0 {
            indent_level = indent_level.saturating_sub((-net_open) as usize);
        }

        // Если текущая строка заканчивается на `|` или `&` (и не открывает `{`/`[`),
        // следующая строка также считается продолжением (continuation)
        prev_ended_with_op = tokens
            .last()
            .map(|t| matches!(t.as_str(), "|" | "&" | "." | ".>" | ".&" | ".?" | ".|"))
            .unwrap_or(false);

        lines_info.push(LineInfo {
            indent: effective_indent,
            is_empty: false,
            is_comment_only: false,
            is_continuation,
            code: tokens.join(" "),
            comment: comment_part.map(|s| s.to_string()),
            tokens,
        });
    }

    // Проход 2: Группировка соседних строк одного уровня и выравнивание по колонкам
    let mut out = String::with_capacity(source.len() + 256);
    let mut i = 0;
    while i < lines_info.len() {
        if lines_info[i].is_empty || lines_info[i].is_comment_only {
            let info = &lines_info[i];
            if !info.is_empty {
                out.push_str(&"    ".repeat(info.indent));
                out.push_str(&info.code);
            }
            out.push('\n');
            i += 1;
            continue;
        }

        let base_indent = lines_info[i].indent;
        let base_cont = lines_info[i].is_continuation;
        let mut group_end = i;
        while group_end < lines_info.len() {
            let cur = &lines_info[group_end];
            if cur.is_empty
                || cur.is_comment_only
                || cur.indent != base_indent
                || cur.is_continuation != base_cont
                || is_structural_boundary(&cur.tokens)
            {
                break;
            }
            group_end += 1;
        }

        if group_end - i >= 2 {
            let slice = &lines_info[i..group_end];
            let aligned_codes = align_group_columns(
                slice.iter().map(|l| l.tokens.clone()).collect(),
                base_cont,
            );
            let max_code_width = aligned_codes
                .iter()
                .map(|s| s.chars().count())
                .max()
                .unwrap_or(0);

            for (idx, line) in slice.iter().enumerate() {
                out.push_str(&"    ".repeat(line.indent));
                let code_str = &aligned_codes[idx];
                out.push_str(code_str);
                if let Some(ref comm) = line.comment {
                    let pad = max_code_width.saturating_sub(code_str.chars().count()) + 2;
                    out.push_str(&" ".repeat(pad));
                    out.push_str(comm);
                }
                out.push('\n');
            }
            i = group_end;
        } else {
            let line = &lines_info[i];
            out.push_str(&"    ".repeat(line.indent));
            out.push_str(&line.code);
            if let Some(ref comm) = line.comment {
                if !line.code.is_empty() {
                    out.push_str("  ");
                }
                out.push_str(comm);
            }
            out.push('\n');
            i += 1;
        }
    }

    out
}

/// Отделяет код от строкового комментария `// ...`, игнорируя `//` внутри кавычек
fn split_code_and_comment(line: &str) -> (&str, Option<&str>) {
    let bytes = line.as_bytes();
    let mut in_quote: Option<u8> = None;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_quote {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                in_quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            in_quote = Some(b);
        } else if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            return (&line[..i], Some(line[i..].trim()));
        }
        i += 1;
    }
    (line, None)
}

/// Разбивает строку кода на логические токены, сохраняя строковые литералы и плотные вызовы `fn(a, b)`
fn tokenize_line_for_format(code: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    let mut paren_depth: usize = 0;

    while i < chars.len() {
        let c = chars[i];

        // Строковые литералы
        if c == '"' || c == '\'' {
            let quote = c;
            cur.push(c);
            i += 1;
            while i < chars.len() {
                let sc = chars[i];
                cur.push(sc);
                i += 1;
                if sc == '\\' && i < chars.len() {
                    cur.push(chars[i]);
                    i += 1;
                } else if sc == quote {
                    break;
                }
            }
            continue;
        }

        if c == '(' {
            paren_depth += 1;
            cur.push(c);
            i += 1;
            continue;
        }
        if c == ')' && paren_depth > 0 {
            paren_depth -= 1;
            cur.push(c);
            i += 1;
            continue;
        }

        // Внутри круглых скобок не бьем по пробелам, чтобы `Point(10, 20)` оставался единой ячейкой
        if paren_depth > 0 {
            cur.push(c);
            i += 1;
            continue;
        }

        if c.is_whitespace() {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
            i += 1;
            continue;
        }

        // Двухсимвольные пайп-операторы и сравнения
        if i + 1 < chars.len() {
            let two: String = [c, chars[i + 1]].iter().collect();
            if matches!(
                two.as_str(),
                ".>" | ".&" | ".?" | ".|" | "==" | "!=" | ">=" | "<=" | "&&" | "||" | "->"
            ) {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
                tokens.push(two);
                i += 2;
                continue;
            }
        }

        // Одиночные операторы и скобки блоков
        if matches!(c, '{' | '}' | '[' | ']' | '|' | '&') {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
            tokens.push(c.to_string());
            i += 1;
            continue;
        }

        cur.push(c);
        i += 1;
    }

    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}


/// Строки, которые открывают или закрывают многострочный блок (`User {`, `}`, `[`, `]`),
/// не должны сливаться в таблицу с обычными полями
fn is_structural_boundary(tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return true;
    }
    let first = tokens.first().map(String::as_str).unwrap_or("");
    let last = tokens.last().map(String::as_str).unwrap_or("");
    matches!(first, "}" | "]" | ")")
        || (matches!(last, "{" | "[") && !tokens.iter().any(|t| t == "}" || t == "]"))
}

fn is_alignment_operator(tok: &str) -> bool {
    matches!(
        tok,
        "." | ".>" | ".&" | ".?" | ".|" | "|" | "&" | "/" | "==" | "!=" | ">=" | "<=" | "+" | "-" | "*"
    )
}

/// Выстраивает группу строк в ровные колонки:
/// • Для строк продолжения (`| Active`, `| Suspended`) — выравнивает ведущий оператор с 1 пробелом (`| Active`).
/// • Для полей форм (`id Int 1`) и привязок — выравнивает Имя | Тип | Значение.
fn align_group_columns(rows: Vec<Vec<String>>, is_continuation_group: bool) -> Vec<String> {
    // Если это группа строк-продолжений (например, `| Active` и `| Suspended`),
    // где нет дополнительных инфиксных операторов внутри строки — держим ровно 1 пробел после `|`
    if is_continuation_group {
        let all_simple_continuations = rows
            .iter()
            .all(|r| r.len() <= 2 || !r.iter().skip(1).any(|t| is_alignment_operator(t)));
        if all_simple_continuations {
            return rows.into_iter().map(|r| r.join(" ")).collect();
        }
    }

    let has_explicit_ops = rows
        .iter()
        .filter(|r| r.iter().any(|t| is_alignment_operator(t)))
        .count()
        >= 2;

    let mut table: Vec<Vec<String>> = Vec::with_capacity(rows.len());

    if has_explicit_ops {
        for row in &rows {
            let mut cells = Vec::new();
            let mut current_cell = Vec::new();
            for tok in row {
                if is_alignment_operator(tok) {
                    if !current_cell.is_empty() {
                        cells.push(current_cell.join(" "));
                        current_cell.clear();
                    }
                    cells.push(tok.clone());
                } else {
                    current_cell.push(tok.clone());
                }
            }
            if !current_cell.is_empty() {
                cells.push(current_cell.join(" "));
            }
            table.push(cells);
        }
    } else {
        for row in &rows {
            let cells = match row.len() {
                0 => vec![],
                1 => vec![row[0].clone()],
                2 => vec![row[0].clone(), row[1].clone()],
                _ => {
                    let second_is_type =
                        is_pascal_case(row[1].split('(').next().unwrap_or(&row[1]));
                    if second_is_type {
                        vec![row[0].clone(), row[1].clone(), row[2..].join(" ")]
                    } else {
                        vec![row[0].clone(), row[1..].join(" ")]
                    }
                }
            };
            table.push(cells);
        }
    }

    let max_cols = table.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_widths = vec![0usize; max_cols];
    for row in &table {
        for (c_idx, cell) in row.iter().enumerate() {
            let w = cell.chars().count();
            if w > col_widths[c_idx] {
                col_widths[c_idx] = w;
            }
        }
    }

    table
        .into_iter()
        .map(|row| {
            let mut line = String::new();
            let last_idx = row.len().saturating_sub(1);
            for (c_idx, cell) in row.into_iter().enumerate() {
                line.push_str(&cell);
                if c_idx < last_idx {
                    let pad = col_widths[c_idx].saturating_sub(cell.chars().count()) + 1;
                    line.push_str(&" ".repeat(pad));
                }
            }
            line
        })
        .collect()
}