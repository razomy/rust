//! Phase 2 Universal Node Parser & HIR Lowering Engine for Razomy Notation (`.rn`) v0.1.0.
//!
//! Implements the Layer 0 Axiomatic Specification:
//! - Universal Stream Root `[]` & Unified Triad Grammar:
//!   `Node ::= Key ({Props})? ((Slots))? (Algebra)* (Schema)? ( " " Payload )?`
//! - 1-Space Topology, Dot-Path Flattening (`a.b.c`), Outliner Trees
//! - 1D Lists vs 2D Relational Tables (`(` Tuple Mode)
//! - Schema Portals (`Key Schema {`, `(`, `[`)
//! - Algebraic Composition (`&` Deep Merge, `^` Symmetric Difference / XOR, `/` Key Strip)
//! - Scalar Auto-Typing (`true`/`false`, `null`/`None`, Int/Float/Hex/Sci, `'symbol`, `Type"..."`)

use indexmap::IndexMap;
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashSet;

use crate::hir::*;
use crate::lexer::{
    pre_lex, ContextMode, Diagnostic, DiagnosticSeverity, IndentActivation, LexedLine,
};

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct RnPestParser;

/// Parsed universal node header:
/// `Key ({Props})? ((Slots))? (Algebra)* (Schema)? ( " " Payload )?`
#[derive(Debug, Clone, Default)]
pub struct ParsedHeader {
    pub raw_key: String,
    pub clean_key: String,
    pub props: Option<IndexMap<String, HirNode>>,
    pub slots: Option<Vec<HirNode>>,
    pub overlays: Vec<OverlayOp>,
    pub schema: Option<String>,
    pub payload_str: Option<String>,
}

const FLAT_DOC_TAGS: &[&str] = &[
    "part", "section", "topic", "title", "subtitle", "quote",
];

/// Parses a `.rn` source string into a resolved `HirDocument`.
pub fn parse_rn(source: &str) -> HirDocument {
    let pre = pre_lex(source);
    let mut diagnostics = pre.diagnostics;
    let mut symbols = Vec::new();

    if pre.lines.is_empty() {
        return HirDocument {
            root_mode: ContextMode::Stream,
            root: HirNode::Stream {
                schema: None,
                items: vec![],
                span: Span::default(),
            },
            diagnostics,
            symbols,
        };
    }

    let first_non_comment = pre.lines.iter().find(|l| !l.is_comment);
    let mut root_mode = ContextMode::Stream;

    if let Some(first) = first_non_comment {
        if first.head_text.is_empty() && first.activation == IndentActivation::IndentForm {
            root_mode = ContextMode::Form;
        } else if first.head_text.is_empty() && first.activation == IndentActivation::IndentTuple {
            root_mode = ContextMode::Tuple;
        } else if is_likely_form_root(&pre.lines) {
            root_mode = ContextMode::Form;
        }
    }

    let mut cursor = 0;
    let root = if root_mode == ContextMode::Form {
        if let Some(first) = first_non_comment {
            if first.head_text.is_empty() && first.activation == IndentActivation::IndentForm {
                let first_idx = pre.lines.iter().position(|l| !l.is_comment).unwrap_or(0);
                cursor = first_idx + 1;
            }
        }
        parse_form_block(
            &pre.lines,
            &mut cursor,
            0,
            None,
            &mut diagnostics,
            &mut symbols,
            "",
        )
    } else if root_mode == ContextMode::Tuple {
        if let Some(first) = first_non_comment {
            if first.head_text.is_empty() && first.activation == IndentActivation::IndentTuple {
                let first_idx = pre.lines.iter().position(|l| !l.is_comment).unwrap_or(0);
                cursor = first_idx + 1;
            }
        }
        parse_tuple_block(&pre.lines, &mut cursor, 0, None, &mut diagnostics)
    } else {
        parse_stream_block(
            &pre.lines,
            &mut cursor,
            0,
            None,
            &mut diagnostics,
            &mut symbols,
            "",
        )
    };

    let mut resolved_root = root;
    resolve_overlays_in_tree(&mut resolved_root, &mut diagnostics);

    HirDocument {
        root_mode,
        root: resolved_root,
        diagnostics,
        symbols,
    }
}

/// Determines whether a root document is a pure data Form `{` or a Universal Stream `[]`.
/// By the Root Axiom, plain text lines, outliner trees (`IndentInherit`), flat document tags (`part`, `title`),
/// and top-level markup tags (`article{...} [`, `h1 [`) stay in Stream Root mode.
/// Pure data/config files (`cluster.primary.database {`, `users_schema (`, `".btn" {`) lower to Form Root.
fn is_likely_form_root(lines: &[LexedLine]) -> bool {
    let top_lines: Vec<&LexedLine> = lines
        .iter()
        .filter(|l| !l.is_comment && l.indent == 0)
        .collect();
    if top_lines.is_empty() {
        return false;
    }

    // If any top-level line is an Outliner parent (`IndentInherit`), Stream opener (`IndentStream`),
    // flat document tag (`part`, `section`, `title`...), or plain prose sentence, it is a Stream root!
    for l in &top_lines {
        if l.activation == IndentActivation::IndentInherit
            || l.activation == IndentActivation::IndentStream
        {
            return false;
        }
        let first_word = l.head_text.split_whitespace().next().unwrap_or("");
        if FLAT_DOC_TAGS.contains(&first_word) {
            return false;
        }
        if l.head_text.starts_with("..") {
            return true;
        }
        // Check if single-line stream tag like `h1 [ ... ]` or inline markup `b[...]`
        if l.original_trimmed.ends_with(']') {
            let hdr = parse_node_header(&l.head_text, true);
            if let Some(p) = &hdr.payload_str {
                if p.starts_with('[') && p.ends_with(']') {
                    return false;
                }
            }
        }
        if l.activation == IndentActivation::Leaf && has_unspaced_inline_tag(&l.head_text) {
            return false;
        }
    }

    // Check if top-level lines are structured Form/Tuple blocks (`config {`, `users (`)
    if top_lines.iter().all(|l| {
        matches!(
            l.activation,
            IndentActivation::IndentForm
                | IndentActivation::IndentTuple
                | IndentActivation::IndentRaw(_)
        )
    }) {
        return true;
    }

    // Check if top-level lines are all Form key-value pairs or blocks
    top_lines.iter().all(|l| {
        !l.head_text.is_empty()
            && !l.head_text.starts_with('[')
            && (l.activation == IndentActivation::IndentForm
                || l.activation == IndentActivation::IndentTuple
                || is_single_identifier(&l.head_text)
                || looks_like_key_value_line(&l.head_text))
    })
}

fn looks_like_key_value_line(s: &str) -> bool {
    let (key, rest) = extract_first_key_token(s);
    if key.is_empty() || rest.trim().is_empty() {
        return false;
    }
    // If key ends with punctuation like `?` or `!`, it's prose (`how my day?`), not a Form key
    if key.ends_with('?') || key.ends_with('!') || key.ends_with(':') {
        return false;
    }
    true
}

fn has_unspaced_inline_tag(s: &str) -> bool {
    let segs = parse_stream_segments(s);
    segs.iter()
        .any(|seg| matches!(seg, StreamSegment::InlineTag { .. } | StreamSegment::Code(_)))
}

fn is_single_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// Parses universal Node header:
/// `Node ::= Key ({Props})? ((Slots))? (Algebra)* (Schema)? ( " " Payload )?`
pub fn parse_node_header(head: &str, expect_block_opener: bool) -> ParsedHeader {
    let trimmed = head.trim();
    if trimmed.is_empty() {
        return ParsedHeader::default();
    }

    let (raw_key, mut rest) = extract_first_key_token(trimmed);
    let clean_key = unquote_key(&raw_key);

    let mut props = None;
    let mut slots = None;

    // Attached `{Props}` and `(Slots)` WITHOUT a leading space
    loop {
        if rest.starts_with('{') {
            if let Some((inside_brace, after_brace)) = extract_balanced_delim(rest, '{', '}') {
                props = extract_inline_form_props(&format!("{{{}}}", inside_brace));
                rest = after_brace;
                continue;
            }
        }
        if rest.starts_with('(') {
            if let Some((inside_parens, after_parens)) = extract_balanced_delim(rest, '(', ')') {
                slots = Some(parse_tuple_cells(inside_parens, Span::default()));
                rest = after_parens;
                continue;
            }
        }
        break;
    }

    rest = rest.trim_start();

    // Parse optional space-separated `(Algebra)*` (`& Source`, `^ Source`, `/ Key`)
    let mut overlays = Vec::new();
    loop {
        if let Some(after_op) = rest.strip_prefix("& ") {
            let after_op = after_op.trim_start();
            let (target, rem) = next_whitespace_token(after_op);
            if !target.is_empty() {
                overlays.push(OverlayOp::Merge(target.to_string()));
                rest = rem.trim_start();
                continue;
            }
        } else if let Some(after_op) = rest.strip_prefix("^ ") {
            let after_op = after_op.trim_start();
            let (target, rem) = next_whitespace_token(after_op);
            if !target.is_empty() {
                overlays.push(OverlayOp::Xor(target.to_string()));
                rest = rem.trim_start();
                continue;
            }
        } else if let Some(after_op) = rest.strip_prefix("/ ") {
            let after_op = after_op.trim_start();
            let (target, rem) = next_whitespace_token(after_op);
            if !target.is_empty() {
                let clean_strip = target.strip_prefix('\'').unwrap_or(target);
                overlays.push(OverlayOp::Omit(clean_strip.to_string()));
                rest = rem.trim_start();
                continue;
            }
        }
        break;
    }

    // Legacy support: if `{Props}` was placed after a space on a block opener line (`article#post { lang en } [`)
    if expect_block_opener && props.is_none() && rest.starts_with('{') {
        if let Some((inside_brace, after_brace)) = extract_balanced_delim(rest, '{', '}') {
            if after_brace.trim().is_empty() {
                props = extract_inline_form_props(&format!("{{{}}}", inside_brace));
                rest = "";
            }
        }
    }

    // Check optional `Schema` (`PascalCase` adapter placed AFTER `Key`/`Algebra` and BEFORE `{`, `(`, `[`)
    // Axiom V: "An optional `PascalCase` token placed AFTER `Key` (and `Algebra`) and BEFORE `{`, `(`, `[`
    // binds a schema adapter to that subtree (`styles Scss {`, `query Sql (`, `view Html [`).
    // Without a trailing bracket (`id Uuid`, `name Razomy Notation`), a `PascalCase` token is a scalar value in `.rn`."
    let mut schema = None;
    let mut payload_str = None;

    if !rest.is_empty() {
        let (first_tok, after_first) = next_whitespace_token(rest);
        let after_first_trimmed = after_first.trim_start();

        if is_pascal_case_schema(first_tok) {
            if expect_block_opener && after_first_trimmed.is_empty() {
                // Block mode portal: `query Sql (`, `styles Scss {`, `view Html [`
                schema = Some(first_tok.to_string());
            } else if !expect_block_opener
                && (after_first_trimmed.starts_with('{')
                    || after_first_trimmed.starts_with('(')
                    || after_first_trimmed.starts_with('['))
            {
                // Inline mode portal: `styles Scss { ... }`, `view Html [ ... ]`
                schema = Some(first_tok.to_string());
                payload_str = Some(after_first_trimmed.to_string());
            } else {
                // Scalar value in `.rn` (`id Uuid`, `name Razomy Notation`)
                payload_str = Some(rest.to_string());
            }
        } else {
            payload_str = Some(rest.to_string());
        }
    }

    ParsedHeader {
        raw_key,
        clean_key,
        props,
        slots,
        overlays,
        schema,
        payload_str,
    }
}

pub fn is_pascal_case_schema(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn next_whitespace_token(s: &str) -> (&str, &str) {
    for (idx, ch) in s.char_indices() {
        if ch.is_whitespace() {
            return (&s[..idx], &s[idx..]);
        }
    }
    (s, "")
}

/// Parses a block of lines in `{` Form Mode.
fn parse_form_block(
    lines: &[LexedLine],
    cursor: &mut usize,
    min_indent: usize,
    schema: Option<String>,
    diagnostics: &mut Vec<Diagnostic>,
    symbols: &mut Vec<SymbolInfo>,
    parent_path: &str,
) -> HirNode {
    let start_line = lines.get(*cursor).map(|l| l.line_idx).unwrap_or(0);
    let mut map: IndexMap<String, HirNode> = IndexMap::new();
    let mut edges: Vec<HirEdge> = Vec::new();
    let mut overlays: Vec<OverlayOp> = Vec::new();

    while *cursor < lines.len() {
        let line = &lines[*cursor];
        if line.indent < min_indent {
            break;
        }
        let cur_indent = line.indent;
        let span = Span {
            line: line.line_idx,
            col: line.indent,
            end_col: line.indent + line.original_trimmed.len(),
        };

        if line.is_comment {
            *cursor += 1;
            continue;
        }

        // Legacy `..target` margin reset support
        if line.indent == 0 && line.head_text.starts_with("..") {
            let reset_str = &line.head_text[2..];
            let hdr = parse_node_header(reset_str, true);
            symbols.push(SymbolInfo {
                name: hdr.clean_key.clone(),
                full_path: hdr.clean_key.clone(),
                kind: SymbolKind::MarginReset,
                span,
            });
            *cursor += 1;

            let child_node = build_activated_child(
                &line.activation,
                &line.raw_payload,
                hdr.schema.clone(),
                hdr.props.clone(),
                lines,
                cursor,
                cur_indent + 1,
                ContextMode::Form,
                diagnostics,
                symbols,
                &hdr.clean_key,
                span,
            );
            let child_with_overlays = attach_overlays(child_node, hdr.overlays);
            merge_into_path(&mut map, &hdr.clean_key, child_with_overlays);
            continue;
        }

        // Standalone Hypergraph Edge (`Ingress -> (Auth, Api) { ... }`)
        if is_standalone_edge_line(&line.head_text) {
            if let Some(edge) = parse_edge_str(&line.head_text, span) {
                symbols.push(SymbolInfo {
                    name: line.head_text.clone(),
                    full_path: join_path(parent_path, "->"),
                    kind: SymbolKind::Edge,
                    span,
                });
                edges.push(edge);
                *cursor += 1;
                continue;
            }
        }

        // Column-0 Multiline & Raw String (`welcome_msg 1"` or `run sh1"`)
        if let Some(raw) = &line.raw_payload {
            let hdr = parse_node_header(&line.head_text, true);
            let key = if hdr.clean_key.is_empty() {
                "\"".to_string()
            } else {
                hdr.clean_key
            };
            let raw_node = HirNode::Raw {
                lang: raw.lang.clone(),
                content: raw.content.clone(),
                rank: raw.rank,
                span,
            };
            insert_folded_key(&mut map, &key, hdr.slots, hdr.overlays, raw_node, span);
            *cursor += 1;
            continue;
        }

        // Block opener (`{`, `(`, `[` or inherited indentation)
        if line.activation != IndentActivation::Leaf {
            let hdr = parse_node_header(&line.head_text, true);
            let full_sym_path = join_path(parent_path, &hdr.clean_key);
            symbols.push(SymbolInfo {
                name: hdr.clean_key.clone(),
                full_path: full_sym_path.clone(),
                kind: if line.activation == IndentActivation::IndentTuple {
                    SymbolKind::TupleTable
                } else {
                    SymbolKind::FormKey
                },
                span,
            });
            *cursor += 1;

            let child_node = build_activated_child(
                &line.activation,
                &line.raw_payload,
                hdr.schema.clone(),
                hdr.props.clone(),
                lines,
                cursor,
                cur_indent + 1,
                ContextMode::Form,
                diagnostics,
                symbols,
                &full_sym_path,
                span,
            );

            let child_with_overlays = attach_overlays(child_node, hdr.overlays);
            insert_folded_key(
                &mut map,
                &hdr.raw_key,
                hdr.slots,
                vec![],
                child_with_overlays,
                span,
            );
        } else {
            parse_and_insert_form_leaf(
                &line.head_text,
                &mut map,
                &mut edges,
                &mut overlays,
                span,
                symbols,
                parent_path,
            );
            *cursor += 1;
        }
    }

    HirNode::Form {
        schema,
        map,
        edges,
        overlays,
        span: Span {
            line: start_line,
            col: min_indent,
            end_col: min_indent,
        },
    }
}

/// Parses a block of lines in `[` Stream Mode (Universal Stream Root, Outliner, Markup & Data Blocks).
fn parse_stream_block(
    lines: &[LexedLine],
    cursor: &mut usize,
    min_indent: usize,
    schema: Option<String>,
    diagnostics: &mut Vec<Diagnostic>,
    symbols: &mut Vec<SymbolInfo>,
    parent_path: &str,
) -> HirNode {
    let start_line = lines.get(*cursor).map(|l| l.line_idx).unwrap_or(0);
    let mut items: Vec<HirNode> = Vec::new();

    while *cursor < lines.len() {
        let line = &lines[*cursor];
        if line.indent < min_indent {
            break;
        }
        let cur_indent = line.indent;
        let span = Span {
            line: line.line_idx,
            col: line.indent,
            end_col: line.indent + line.original_trimmed.len(),
        };

        if line.is_comment {
            *cursor += 1;
            continue;
        }

        // Column-0 Multiline String inside Stream (`code ts"`, `input rn2"`)
        if let Some(raw) = &line.raw_payload {
            let hdr = parse_node_header(&line.head_text, true);
            let raw_node = HirNode::Raw {
                lang: raw.lang.clone(),
                content: raw.content.clone(),
                rank: raw.rank,
                span,
            };
            if hdr.clean_key.is_empty() {
                items.push(raw_node);
            } else {
                let mut m = IndexMap::new();
                m.insert(hdr.clean_key, raw_node);
                items.push(HirNode::Form {
                    schema: hdr.schema,
                    map: m,
                    edges: vec![],
                    overlays: hdr.overlays,
                    span,
                });
            }
            *cursor += 1;
            continue;
        }

        // Outliner: Indenting (`+1 space`) beneath a plain text line implicitly nests a child Stream under that line!
        if line.activation == IndentActivation::IndentInherit {
            let text_node = parse_stream_text_or_richtext(&line.head_text, span);
            *cursor += 1;
            let child_stream = parse_stream_block(
                lines,
                cursor,
                cur_indent + 1,
                None,
                diagnostics,
                symbols,
                parent_path,
            );
            let children = match child_stream {
                HirNode::Stream { items: c, .. } => c,
                other => vec![other],
            };
            items.push(HirNode::Outliner {
                text: Box::new(text_node),
                children,
                span,
            });
            continue;
        }

        // Structured Node opening a block (`[`, `{`, `(` at the end of the line)
        if line.activation != IndentActivation::Leaf {
            let hdr = parse_node_header(&line.head_text, true);
            let (clean_tag, id_opt, classes) = split_css_shorthand(&hdr.clean_key);
            let full_sym_path = join_path(parent_path, &clean_tag);
            symbols.push(SymbolInfo {
                name: clean_tag.clone(),
                full_path: full_sym_path.clone(),
                kind: SymbolKind::StreamTag,
                span,
            });
            *cursor += 1;

            let child_node = build_activated_child(
                &line.activation,
                &line.raw_payload,
                hdr.schema.clone(),
                None,
                lines,
                cursor,
                cur_indent + 1,
                ContextMode::Stream,
                diagnostics,
                symbols,
                &full_sym_path,
                span,
            );

            if id_opt.is_some()
                || !classes.is_empty()
                || hdr.props.is_some()
                || hdr.slots.is_some()
            {
                let body_vec = match child_node {
                    HirNode::Stream { items: b, .. } => Some(b),
                    other => Some(vec![other]),
                };
                items.push(HirNode::Triad {
                    name: clean_tag,
                    id: id_opt,
                    classes,
                    params: hdr.slots,
                    overlays: hdr.overlays,
                    schema: hdr.schema,
                    props: hdr.props,
                    body: body_vec,
                    span,
                });
            } else {
                let mut m = IndexMap::new();
                let child_with_overlays = attach_overlays(child_node, hdr.overlays);
                insert_folded_key(&mut m, &hdr.raw_key, None, vec![], child_with_overlays, span);
                items.push(HirNode::Form {
                    schema: None,
                    map: m,
                    edges: vec![],
                    overlays: vec![],
                    span,
                });
            }
            continue;
        }

        // Flat document tags in Stream (`title Razomy Guide`, `part ...`, `section ...`, `topic ...`, `subtitle ...`, `quote ...`)
        if let Some(flat_node) = try_parse_flat_doc_tag(&line.head_text, span, symbols, parent_path)
        {
            items.push(flat_node);
            *cursor += 1;
            continue;
        }

        // Single-line structured Node in Stream (`h1 [ Why Razomy? ]`, `p [ Read ... ]`, `env { A 1, B 2 }`)
        if let Some(triad_node) =
            try_parse_single_line_stream_tag(&line.head_text, span, symbols, parent_path)
        {
            items.push(triad_node);
            *cursor += 1;
            continue;
        }

        // Plain text / inline markup line in Stream
        let node = parse_stream_text_or_richtext(&line.head_text, span);
        items.push(node);
        *cursor += 1;
    }

    HirNode::Stream {
        schema,
        items,
        span: Span {
            line: start_line,
            col: min_indent,
            end_col: min_indent,
        },
    }
}

fn parse_stream_text_or_richtext(text: &str, span: Span) -> HirNode {
    let segs = parse_stream_segments(text);
    if segs.len() == 1 {
        if let StreamSegment::Text(t) = &segs[0] {
            return HirNode::String(t.clone(), span);
        }
    }
    HirNode::RichText {
        segments: segs,
        span,
    }
}

fn try_parse_flat_doc_tag(
    line_text: &str,
    span: Span,
    symbols: &mut Vec<SymbolInfo>,
    parent_path: &str,
) -> Option<HirNode> {
    let (first_word, rest) = next_whitespace_token(line_text.trim());
    if !FLAT_DOC_TAGS.contains(&first_word) {
        return None;
    }
    let rest_trimmed = rest.trim();
    if rest_trimmed.is_empty() {
        return None;
    }
    // If it already uses `[ ... ]`, let `try_parse_single_line_stream_tag` handle it
    if rest_trimmed.starts_with('[') && rest_trimmed.ends_with(']') {
        return None;
    }

    symbols.push(SymbolInfo {
        name: first_word.to_string(),
        full_path: join_path(parent_path, first_word),
        kind: SymbolKind::StreamTag,
        span,
    });

    let body_node = parse_stream_text_or_richtext(rest_trimmed, span);
    let mut m = IndexMap::new();
    m.insert(first_word.to_string(), body_node);
    Some(HirNode::Form {
        schema: None,
        map: m,
        edges: vec![],
        overlays: vec![],
        span,
    })
}

/// Parses a block of lines in `(` Tuple Mode (1D Lists & 2D Relational Tables).
fn parse_tuple_block(
    lines: &[LexedLine],
    cursor: &mut usize,
    min_indent: usize,
    schema: Option<String>,
    _diagnostics: &mut Vec<Diagnostic>,
) -> HirNode {
    let start_line = lines.get(*cursor).map(|l| l.line_idx).unwrap_or(0);
    let mut rows: Vec<Vec<HirNode>> = Vec::new();
    let mut is_2d_table = false;
    let mut first_line_checked = false;

    while *cursor < lines.len() {
        let line = &lines[*cursor];
        if line.indent < min_indent {
            break;
        }
        if line.is_comment {
            *cursor += 1;
            continue;
        }
        let span = Span {
            line: line.line_idx,
            col: line.indent,
            end_col: line.indent + line.original_trimmed.len(),
        };

        if !first_line_checked {
            first_line_checked = true;
            // 2D Table: Block `(` where the first indented line contains at least one top-level comma defining header columns
            // 1D List: Block `(` where the first indented line contains NO top-level commas outside quotes/brackets
            let top_commas = split_top_level_delims(&line.head_text, &[',']);
            is_2d_table = top_commas.len() > 1;
        }

        if is_2d_table {
            let parsed_rows = parse_tuple_line_rows(&line.head_text, span);
            rows.extend(parsed_rows);
        } else {
            // 1D List: strictly one item per line
            let cell = parse_single_tuple_cell(&line.head_text, span);
            rows.push(vec![cell]);
        }
        *cursor += 1;
    }

    // If trailing comma was used for a single-column table (`col,`), strip the empty trailing header cell
    if is_2d_table {
        if let Some(header) = rows.first_mut() {
            if header.len() > 1 && matches!(header.last(), Some(HirNode::Null(_))) {
                header.pop();
            }
        }
    }

    HirNode::Tuple {
        schema,
        is_2d_table,
        rows,
        span: Span {
            line: start_line,
            col: min_indent,
            end_col: min_indent,
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn build_activated_child(
    activation: &IndentActivation,
    raw_payload: &Option<crate::lexer::RawPayload>,
    schema: Option<String>,
    inline_props: Option<IndexMap<String, HirNode>>,
    lines: &[LexedLine],
    cursor: &mut usize,
    child_min_indent: usize,
    parent_mode: ContextMode,
    diagnostics: &mut Vec<Diagnostic>,
    symbols: &mut Vec<SymbolInfo>,
    current_path: &str,
    span: Span,
) -> HirNode {
    let target_mode = match activation {
        IndentActivation::IndentStream => ContextMode::Stream,
        IndentActivation::IndentForm => ContextMode::Form,
        IndentActivation::IndentTuple => ContextMode::Tuple,
        IndentActivation::IndentRaw(lang) => {
            let content = raw_payload
                .as_ref()
                .map(|r| r.content.clone())
                .unwrap_or_default();
            let rank = raw_payload.as_ref().and_then(|r| r.rank);
            return HirNode::Raw {
                lang: lang.clone(),
                content,
                rank,
                span,
            };
        }
        IndentActivation::IndentInherit | IndentActivation::Leaf => parent_mode,
    };

    let mut block_node = match target_mode {
        ContextMode::Form => parse_form_block(
            lines,
            cursor,
            child_min_indent,
            schema,
            diagnostics,
            symbols,
            current_path,
        ),
        ContextMode::Stream => parse_stream_block(
            lines,
            cursor,
            child_min_indent,
            schema,
            diagnostics,
            symbols,
            current_path,
        ),
        ContextMode::Tuple => {
            parse_tuple_block(lines, cursor, child_min_indent, schema, diagnostics)
        }
        ContextMode::Raw => HirNode::Null(span),
    };

    if let Some(props) = inline_props {
        if let HirNode::Form { map, .. } = &mut block_node {
            for (k, v) in props {
                map.insert(k, v);
            }
        }
    }

    block_node
}

fn extract_first_key_token(s: &str) -> (String, &str) {
    if s.starts_with('"') {
        let chars: Vec<(usize, char)> = s.char_indices().collect();
        let mut i = 1;
        while i < chars.len() {
            if chars[i].1 == '\\' {
                i += 2;
                continue;
            }
            if chars[i].1 == '"' {
                let end_byte = chars[i].0 + 1;
                return (s[..end_byte].to_string(), &s[end_byte..]);
            }
            i += 1;
        }
    }
    for (idx, ch) in s.char_indices() {
        if ch == ' ' || ch == '\t' || ch == '(' || ch == '{' || ch == '[' {
            return (s[..idx].to_string(), &s[idx..]);
        }
    }
    (s.to_string(), "")
}

fn attach_overlays(node: HirNode, mut new_overlays: Vec<OverlayOp>) -> HirNode {
    if new_overlays.is_empty() {
        return node;
    }
    match node {
        HirNode::Form {
            schema,
            map,
            edges,
            mut overlays,
            span,
        } => {
            new_overlays.append(&mut overlays);
            HirNode::Form {
                schema,
                map,
                edges,
                overlays: new_overlays,
                span,
            }
        }
        other => other,
    }
}

/// Inserts a key into a Form map, handling Dot-Path Flattening (`a.b.c {` -> `{ a { b { c { ... } } } }`)
/// and attached positional Slots.
fn insert_folded_key(
    map: &mut IndexMap<String, HirNode>,
    raw_key: &str,
    params_opt: Option<Vec<HirNode>>,
    overlays: Vec<OverlayOp>,
    mut value_node: HirNode,
    span: Span,
) {
    if !overlays.is_empty() {
        value_node = attach_overlays(value_node, overlays);
    }

    let clean_key = unquote_key(raw_key);

    if let Some(params) = params_opt {
        let param_keys: Vec<String> = params
            .iter()
            .map(|p| match p.to_canonical_json() {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            })
            .collect();

        if !param_keys.is_empty() {
            if param_keys.len() == 1 {
                if let HirNode::Form {
                    map: inner_map, ..
                } = &mut value_node
                {
                    inner_map.insert(
                        "()".to_string(),
                        HirNode::Tuple {
                            schema: None,
                            is_2d_table: false,
                            rows: vec![params],
                            span,
                        },
                    );
                }
                map.insert(clean_key, value_node);
                return;
            } else {
                let mut full_chain = vec![clean_key];
                full_chain.extend(param_keys);
                insert_path_slice(map, &full_chain, value_node, span);
                return;
            }
        }
    }

    // Dot-Path Flattening (`a.b.c`): unquoted dotted keys expand into nested Forms
    if should_fold_dotted_key(raw_key) {
        let parts: Vec<String> = clean_key.split('.').map(|s| s.to_string()).collect();
        insert_path_slice(map, &parts, value_node, span);
    } else {
        map.insert(clean_key, value_node);
    }
}

fn should_fold_dotted_key(raw_key: &str) -> bool {
    let t = raw_key.trim();
    if t.starts_with('.') || t.starts_with('"') || t.starts_with('\'') || !t.contains('.') {
        return false;
    }
    if t.contains('#') {
        return false;
    }
    t.split('.').all(|seg| !seg.is_empty())
}

fn insert_path_slice(
    map: &mut IndexMap<String, HirNode>,
    parts: &[String],
    value_node: HirNode,
    span: Span,
) {
    if parts.is_empty() {
        return;
    }
    if parts.len() == 1 {
        if let Some(existing) = map.get_mut(&parts[0]) {
            deep_merge_nodes(existing, value_node);
        } else {
            map.insert(parts[0].clone(), value_node);
        }
        return;
    }

    let head = &parts[0];
    let rest = &parts[1..];
    let entry = map.entry(head.clone()).or_insert_with(|| HirNode::Form {
        schema: None,
        map: IndexMap::new(),
        edges: vec![],
        overlays: vec![],
        span,
    });
    if let HirNode::Form { map: sub_map, .. } = entry {
        insert_path_slice(sub_map, rest, value_node, span);
    }
}

fn merge_into_path(map: &mut IndexMap<String, HirNode>, target_path: &str, new_node: HirNode) {
    let parts: Vec<String> = target_path
        .split('.')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if parts.is_empty() {
        return;
    }
    insert_path_slice(map, &parts, new_node.clone(), new_node.span());
}

pub fn deep_merge_nodes(target: &mut HirNode, source: HirNode) {
    match (target, source) {
        (
            HirNode::Form {
                map: t_map,
                edges: t_edges,
                overlays: t_ov,
                ..
            },
            HirNode::Form {
                map: s_map,
                edges: s_edges,
                overlays: s_ov,
                ..
            },
        ) => {
            for (k, v) in s_map {
                if let Some(existing) = t_map.get_mut(&k) {
                    deep_merge_nodes(existing, v);
                } else {
                    t_map.insert(k, v);
                }
            }
            t_edges.extend(s_edges);
            t_ov.extend(s_ov);
        }
        (HirNode::Stream { items: t_items, .. }, HirNode::Stream { items: s_items, .. }) => {
            t_items.extend(s_items);
        }
        (HirNode::Tuple { rows: t_rows, .. }, HirNode::Tuple { rows: s_rows, .. }) => {
            t_rows.extend(s_rows);
        }
        (t, s) => {
            *t = s;
        }
    }
}

/// Symmetric Difference (`^` XOR): retains only keys present in `target` or `source`, but not both.
pub fn xor_merge_nodes(target: &mut HirNode, source: HirNode) {
    if let (HirNode::Form { map: t_map, .. }, HirNode::Form { map: s_map, .. }) = (target, source) {
        for (k, v) in s_map {
            if t_map.contains_key(&k) {
                t_map.shift_remove(&k);
            } else {
                t_map.insert(k, v);
            }
        }
    }
}

fn parse_and_insert_form_leaf(
    line_text: &str,
    map: &mut IndexMap<String, HirNode>,
    edges: &mut Vec<HirEdge>,
    _overlays: &mut Vec<OverlayOp>,
    span: Span,
    symbols: &mut Vec<SymbolInfo>,
    parent_path: &str,
) {
    let trimmed = line_text.trim();
    if trimmed.is_empty() {
        return;
    }

    let statements = split_top_level_delims(trimmed, &[',', ';']);
    if statements.len() > 1 {
        for stmt in statements {
            if !stmt.trim().is_empty() {
                parse_and_insert_single_form_pair(
                    stmt.trim(),
                    map,
                    edges,
                    span,
                    symbols,
                    parent_path,
                );
            }
        }
    } else {
        parse_and_insert_single_form_pair(trimmed, map, edges, span, symbols, parent_path);
    }
}

fn parse_and_insert_single_form_pair(
    pair_str: &str,
    map: &mut IndexMap<String, HirNode>,
    _edges: &mut Vec<HirEdge>,
    span: Span,
    symbols: &mut Vec<SymbolInfo>,
    parent_path: &str,
) {
    let hdr = parse_node_header(pair_str, false);
    if hdr.clean_key.is_empty() {
        return;
    }

    let full_sym_path = join_path(parent_path, &hdr.clean_key);
    symbols.push(SymbolInfo {
        name: hdr.clean_key.clone(),
        full_path: full_sym_path,
        kind: SymbolKind::FormKey,
        span,
    });

    let val_str = hdr.payload_str.as_deref().unwrap_or("").trim();

    // Omitting `{}` with `& Source` clones the merged result directly (`backup_timeout & base.timeout`)
    if val_str.is_empty() {
        if !hdr.overlays.is_empty() {
            let empty_form = HirNode::Form {
                schema: hdr.schema,
                map: IndexMap::new(),
                edges: vec![],
                overlays: hdr.overlays,
                span,
            };
            insert_folded_key(&mut *map, &hdr.raw_key, hdr.slots, vec![], empty_form, span);
            return;
        }

        // Inside a data Form, a bare `Key` with no payload and no `& Source` evaluates to `null`
        insert_folded_key(
            map,
            &hdr.raw_key,
            hdr.slots,
            vec![],
            HirNode::Null(span),
            span,
        );
        return;
    }

    // Inline `{ ... }` Form value (`env { NODE_ENV production, PORT 8080 }` or `"&:hover" { background-color #222222 }`)
    if val_str.starts_with('{') && val_str.ends_with('}') && val_str.len() >= 2 {
        if val_str == "{}" {
            let empty_form = HirNode::Form {
                schema: hdr.schema,
                map: IndexMap::new(),
                edges: vec![],
                overlays: hdr.overlays,
                span,
            };
            insert_folded_key(map, &hdr.raw_key, hdr.slots, vec![], empty_form, span);
            return;
        }
        if let Some(inline_map) = extract_inline_form_props(val_str) {
            let form_node = HirNode::Form {
                schema: hdr.schema,
                map: inline_map,
                edges: vec![],
                overlays: hdr.overlays,
                span,
            };
            insert_folded_key(map, &hdr.raw_key, hdr.slots, vec![], form_node, span);
            return;
        }
    }

    let val_node = parse_form_value_expr(val_str, span);
    insert_folded_key(map, &hdr.raw_key, hdr.slots, hdr.overlays, val_node, span);
}

/// Scalar Auto-Typing in `{}` and `()`:
/// - `true`, `false` -> `Bool`
/// - `null`, `None` -> `Null`
/// - `8080`, `-3.14`, `0xFF`, `1e-5` -> `Number` (multi-dot tokens like `0.1.0` fall back to Unquoted String)
/// - `'pk`, `'not_null`, `'GET` -> `Symbol`
/// - `Type"..."` -> `TypedLiteral` (`Date"2026-03-30"`)
/// - `"..."` -> Quoted UTF-8 String
/// - Unquoted String -> Everything else
pub fn parse_form_value_expr(val_str: &str, span: Span) -> HirNode {
    let v = val_str.trim();

    match v {
        "\"\"" => return HirNode::String(String::new(), span),
        "{}" => {
            return HirNode::Form {
                schema: None,
                map: IndexMap::new(),
                edges: vec![],
                overlays: vec![],
                span,
            }
        }
        "()" => {
            return HirNode::Tuple {
                schema: None,
                is_2d_table: false,
                rows: vec![],
                span,
            }
        }
        "[]" => {
            return HirNode::Stream {
                schema: None,
                items: vec![],
                span,
            }
        }
        "true" => return HirNode::Bool(true, span),
        "false" => return HirNode::Bool(false, span),
        "null" | "None" => return HirNode::Null(span),
        _ => {}
    }

    // Inline Tuple `(a, b, c)`
    if v.starts_with('(') && v.ends_with(')') {
        let inner = &v[1..v.len() - 1];
        let rows = parse_tuple_line_rows(inner, span);
        return HirNode::Tuple {
            schema: None,
            is_2d_table: false,
            rows,
            span,
        };
    }

    // Inline Stream `[ text ]` (single leading space after `[` and single trailing space before `]` are trimmed)
    if v.starts_with('[') && v.ends_with(']') {
        let inner = trim_inline_stream_spaces(&v[1..v.len() - 1]);
        let segs = parse_stream_segments(inner);
        return HirNode::Stream {
            schema: None,
            items: vec![HirNode::RichText {
                segments: segs,
                span,
            }],
            span,
        };
    }

    // Edge in value position (`-> Orgs.id` or `A -> B`)
    if v.starts_with("-> ") || v.starts_with("<-> ") || v.contains(" -> ") || v.contains(" <-> ")
    {
        if let Some(edge) = parse_edge_str(v, span) {
            return HirNode::Edge(edge);
        }
    }

    // Symbol (`'pk`, `'not_null`, `'GET` — prefix-quoted O(1) interned tag with NO closing quote)
    if let Some(sym) = v.strip_prefix('\'') {
        if !sym.is_empty() && !sym.contains(' ') && !sym.ends_with('\'') {
            return HirNode::Symbol(sym.to_string(), span);
        }
    }

    // Typed Literal (`Date"2026-03-30"`, `Money"199.99"`, `Url"https://razomy.org"`)
    if let Some((type_name, inner_val)) = parse_typed_literal(v) {
        return HirNode::TypedLiteral {
            type_name,
            value: inner_val,
            span,
        };
    }

    // Lineage Reference (`.sibling` or `..Ancestor`)
    if is_valid_ref_path(v) {
        return HirNode::Reference {
            path: v.to_string(),
            span,
        };
    }

    // Number (`8080`, `-3.14`, `0xFF`, `1e-5`)
    if let Some(num) = parse_scalar_number(v) {
        return HirNode::Number(num, span);
    }

    // Quoted UTF-8 String `"..."`
    if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
        return HirNode::String(unquote_string(v), span);
    }

    // Backtick verbatim span `` `...` ``
    if v.starts_with('`') && v.ends_with('`') && v.len() >= 2 {
        return HirNode::String(v[1..v.len() - 1].to_string(), span);
    }

    // Default: Unquoted String (`Razomy Notation`, `postgres`, `node:22-alpine`, `/app`, `8px`, `#111111`, `Orgs.id`)
    HirNode::String(v.to_string(), span)
}

/// Parses Typed Literal `Type"..."` where `Type` is an identifier attached directly to `"..."` without spaces.
pub fn parse_typed_literal(s: &str) -> Option<(String, String)> {
    let q_pos = s.find('"')?;
    if q_pos == 0 || !s.ends_with('"') || s.len() < q_pos + 2 {
        return None;
    }
    let type_part = &s[..q_pos];
    let mut chars = type_part.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let quoted_part = &s[q_pos..];
    Some((type_part.to_string(), unquote_string(quoted_part)))
}

/// Parses scalar numbers (`8080`, `-3.14`, `0xFF`, `1e-5`).
/// Rejects leading zeros (`01234`) and multi-dot tokens (`0.1.0`, `1.2.0`).
pub fn parse_scalar_number(s: &str) -> Option<f64> {
    if s.is_empty() {
        return None;
    }
    // Hexadecimal integer (`0xFF`, `-0xFF`)
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16).ok().map(|v| v as f64);
    }
    if let Some(hex) = s.strip_prefix("-0x").or_else(|| s.strip_prefix("-0X")) {
        return i64::from_str_radix(hex, 16).ok().map(|v| -(v as f64));
    }

    if is_strict_number(s) {
        return s.parse::<f64>().ok();
    }
    None
}

/// Checks decimal integers, floats, and scientific notation (`8080`, `-3.14`, `1e-5`).
/// Rejects leading zeros (`0123`) and multi-dot version strings (`0.1.0`).
pub fn is_strict_number(s: &str) -> bool {
    if s.starts_with("0x") || s.starts_with("0X") || s.starts_with("-0x") || s.starts_with("-0X") {
        let hex = s
            .trim_start_matches('-')
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        return !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
    }

    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut i = 0;
    if bytes[i] == b'-' {
        i += 1;
        if i == bytes.len() {
            return false;
        }
    }
    if bytes[i] == b'0' {
        i += 1;
        // Leading zero followed by another digit (`0123`) is an Unquoted String, not a Number
        if i < bytes.len() && bytes[i] != b'.' && bytes[i] != b'e' && bytes[i] != b'E' {
            return false;
        }
    } else if bytes[i].is_ascii_digit() {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    } else {
        return false;
    }

    // Optional fractional part `.14`
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        if i == bytes.len() || !bytes[i].is_ascii_digit() {
            return false;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }

    // Optional exponent part `e-5` / `E+10`
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        if i == bytes.len() || !bytes[i].is_ascii_digit() {
            return false;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }

    i == bytes.len()
}

pub fn is_valid_ref_path(s: &str) -> bool {
    let rest = if let Some(r) = s.strip_prefix("..") {
        r
    } else if let Some(r) = s.strip_prefix('.') {
        r
    } else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    rest.split('.').all(|seg| {
        let mut chars = seg.chars();
        match chars.next() {
            Some(first) if first.is_ascii_alphabetic() || first == '_' => {
                chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            }
            _ => false,
        }
    })
}

/// Parses a single line using the `unified_node` Pest grammar rule.
pub fn parse_unified_line_ast(line: &str) -> bool {
    if RnPestParser::parse(Rule::document_line, line).is_ok() {
        return true;
    }
    // Also accept legacy margin reset or hyperedge lines
    let t = line.trim();
    t.starts_with("..") || t.contains(" -> ") || t.contains(" <-> ")
}

fn is_standalone_edge_line(s: &str) -> bool {
    let trimmed = s.trim();
    trimmed.starts_with("-> ")
        || trimmed.starts_with("<-> ")
        || find_top_level_substring(trimmed, " -> ").is_some()
        || find_top_level_substring(trimmed, " <-> ").is_some()
}

pub fn parse_edge_str(s: &str, span: Span) -> Option<HirEdge> {
    let mut main_part = s.trim();
    let mut metadata = None;

    if main_part.ends_with('}') {
        if let Some(brace_pos) = find_top_level_char(main_part, '{') {
            let meta_str = &main_part[brace_pos..];
            metadata = extract_inline_form_props(meta_str);
            main_part = main_part[..brace_pos].trim();
        }
    }

    let mut endpoints = Vec::new();
    let mut arrows = Vec::new();

    if let Some(rest) = main_part.strip_prefix("-> ") {
        arrows.push("->".to_string());
        endpoints.push(parse_edge_endpoint(rest.trim(), span));
        return Some(HirEdge {
            endpoints,
            arrows,
            metadata,
            span,
        });
    }
    if let Some(rest) = main_part.strip_prefix("<-> ") {
        arrows.push("<->".to_string());
        endpoints.push(parse_edge_endpoint(rest.trim(), span));
        return Some(HirEdge {
            endpoints,
            arrows,
            metadata,
            span,
        });
    }

    let mut current = main_part;
    loop {
        let next_dir = find_top_level_substring(current, " -> ");
        let next_bi = find_top_level_substring(current, " <-> ");

        match (next_dir, next_bi) {
            (Some(p1), Some(p2)) if p1 < p2 => {
                endpoints.push(parse_edge_endpoint(current[..p1].trim(), span));
                arrows.push("->".to_string());
                current = &current[p1 + 4..];
            }
            (Some(_), Some(p2)) => {
                endpoints.push(parse_edge_endpoint(current[..p2].trim(), span));
                arrows.push("<->".to_string());
                current = &current[p2 + 5..];
            }
            (Some(p1), None) => {
                endpoints.push(parse_edge_endpoint(current[..p1].trim(), span));
                arrows.push("->".to_string());
                current = &current[p1 + 4..];
            }
            (None, Some(p2)) => {
                endpoints.push(parse_edge_endpoint(current[..p2].trim(), span));
                arrows.push("<->".to_string());
                current = &current[p2 + 5..];
            }
            (None, None) => {
                endpoints.push(parse_edge_endpoint(current.trim(), span));
                break;
            }
        }
    }

    if arrows.is_empty() {
        None
    } else {
        Some(HirEdge {
            endpoints,
            arrows,
            metadata,
            span,
        })
    }
}

fn parse_edge_endpoint(ep_str: &str, span: Span) -> HirEndpoint {
    let t = ep_str.trim();
    if t.starts_with('(') && t.ends_with(')') && t.len() >= 2 {
        let inner = &t[1..t.len() - 1];
        let cells = parse_tuple_cells(inner, span);
        HirEndpoint::Group(cells)
    } else {
        HirEndpoint::Target(t.to_string())
    }
}

/// Trims a single leading space after `[` and a single trailing space before `]`.
pub fn trim_inline_stream_spaces(s: &str) -> &str {
    let s1 = s.strip_prefix(' ').unwrap_or(s);
    s1.strip_suffix(' ').unwrap_or(s1)
}

/// Parses a single-line stream block tag such as `h1 [ Why Razomy? ]` or `article{id post} [ Hello ]`.
fn try_parse_single_line_stream_tag(
    line_text: &str,
    span: Span,
    symbols: &mut Vec<SymbolInfo>,
    parent_path: &str,
) -> Option<HirNode> {
    let trimmed = line_text.trim();
    if !trimmed.ends_with(']') {
        return None;
    }

    let (first_tok, rest_after_key) = extract_first_key_token(trimmed);
    if first_tok.is_empty() {
        return None;
    }

    let mut rest = rest_after_key;
    let mut props_opt = None;
    let mut slots_opt = None;

    // Attached `{Props}` / `(Slots)` without space
    loop {
        if rest.starts_with('{') {
            let (inside_brace, after_brace) = extract_balanced_delim(rest, '{', '}')?;
            props_opt = extract_inline_form_props(&format!("{{{}}}", inside_brace));
            rest = after_brace;
            continue;
        }
        if rest.starts_with('(') {
            let (inside_parens, after_parens) = extract_balanced_delim(rest, '(', ')')?;
            slots_opt = Some(parse_tuple_cells(inside_parens, span));
            rest = after_parens;
            continue;
        }
        break;
    }

    // Must have space before payload `[` (otherwise `b[bold]` is an inline tag inside a prose flow!)
    if !rest.starts_with(' ') {
        return None;
    }
    let rest_trimmed = rest.trim_start();
    let bracket_part;

    if props_opt.is_none() && rest_trimmed.starts_with('{') {
        let (inside_brace, after_brace) = extract_balanced_delim(rest_trimmed, '{', '}')?;
        props_opt = extract_inline_form_props(&format!("{{{}}}", inside_brace));
        bracket_part = after_brace.trim();
    } else if rest_trimmed.starts_with('[') {
        bracket_part = rest_trimmed;
    } else {
        return None;
    }

    if !bracket_part.starts_with('[') || !bracket_part.ends_with(']') {
        return None;
    }
    let ( inside_bracket, after_bracket ) = extract_balanced_delim(bracket_part, '[', ']')?;
    if !after_bracket.trim().is_empty() {
        return None;
    }

    // Whitespace rule: In inline `[ text ]`, a single leading space after `[` and a single trailing space before `]` are trimmed.
    let inner_stream_str = trim_inline_stream_spaces(inside_bracket);
    let (clean_tag, id_opt, classes) = split_css_shorthand(&unquote_key(&first_tok));

    symbols.push(SymbolInfo {
        name: clean_tag.clone(),
        full_path: join_path(parent_path, &clean_tag),
        kind: SymbolKind::StreamTag,
        span,
    });

    let segs = parse_stream_segments(inner_stream_str);
    let body_node = if segs.len() == 1 {
        if let StreamSegment::Text(t) = &segs[0] {
            HirNode::String(t.clone(), span)
        } else {
            HirNode::RichText {
                segments: segs,
                span,
            }
        }
    } else {
        HirNode::RichText {
            segments: segs,
            span,
        }
    };

    if id_opt.is_some() || !classes.is_empty() || props_opt.is_some() || slots_opt.is_some() {
        Some(HirNode::Triad {
            name: clean_tag,
            id: id_opt,
            classes,
            params: slots_opt,
            overlays: vec![],
            schema: None,
            props: props_opt,
            body: Some(vec![body_node]),
            span,
        })
    } else {
        let mut m = IndexMap::new();
        m.insert(clean_tag, body_node);
        Some(HirNode::Form {
            schema: None,
            map: m,
            edges: vec![],
            overlays: vec![],
            span,
        })
    }
}

/// Parses inline stream segments (`Text`, `` `Code` ``, `"Quoted"`, `\[`/`\]` escapes, and unspaced `Identifier{Props}[inner]`).
pub fn parse_stream_segments(input: &str) -> Vec<StreamSegment> {
    let mut segments = Vec::new();
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut i = 0;
    let mut text_buf = String::new();

    while i < chars.len() {
        let (byte_pos, ch) = chars[i];

        // Backslash escape (`\[`, `\]`, `\\`)
        if ch == '\\' && i + 1 < chars.len() {
            let next_ch = chars[i + 1].1;
            if next_ch == '[' || next_ch == ']' || next_ch == '\\' || next_ch == '`' || next_ch == '"' {
                text_buf.push(next_ch);
                i += 2;
                continue;
            }
        }

        // 1. Backtick shield `` `...` ``
        if ch == '`' {
            if !text_buf.is_empty() {
                segments.push(StreamSegment::Text(std::mem::take(&mut text_buf)));
            }
            let mut j = i + 1;
            while j < chars.len() && chars[j].1 != '`' {
                j += 1;
            }
            if j < chars.len() {
                let code_content = &input[byte_pos + 1..chars[j].0];
                segments.push(StreamSegment::Code(code_content.to_string()));
                i = j + 1;
                continue;
            }
        }

        // 2. Quoted shield `"..."`
        if ch == '"' {
            if !text_buf.is_empty() {
                segments.push(StreamSegment::Text(std::mem::take(&mut text_buf)));
            }
            let mut j = i + 1;
            while j < chars.len() {
                if chars[j].1 == '\\' {
                    j += 2;
                    continue;
                }
                if chars[j].1 == '"' {
                    break;
                }
                j += 1;
            }
            if j < chars.len() {
                let end_b = chars[j].0 + 1;
                segments.push(StreamSegment::Quoted(input[byte_pos..end_b].to_string()));
                i = j + 1;
                continue;
            }
        }

        // 3. Inline Tags: `Identifier[` or `Identifier{Props}[` (with NO space before `[` or `{`)
        let at_word_boundary = i == 0 || chars[i - 1].1.is_whitespace() || chars[i - 1].1 == '(';
        if at_word_boundary && (ch.is_ascii_alphabetic() || ch == '_') {
            if let Some((tag_seg, next_char_idx)) = try_consume_inline_tag(input, &chars, i) {
                if !text_buf.is_empty() {
                    segments.push(StreamSegment::Text(std::mem::take(&mut text_buf)));
                }
                segments.push(tag_seg);
                i = next_char_idx;
                continue;
            }
        }

        text_buf.push(ch);
        i += 1;
    }

    if !text_buf.is_empty() {
        segments.push(StreamSegment::Text(text_buf));
    }

    if segments.is_empty() {
        segments.push(StreamSegment::Text(String::new()));
    }
    segments
}

fn try_consume_inline_tag(
    input: &str,
    chars: &[(usize, char)],
    start_idx: usize,
) -> Option<(StreamSegment, usize)> {
    let start_byte = chars[start_idx].0;
    let mut j = start_idx;
    while j < chars.len() {
        let c = chars[j].1;
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '#' {
            j += 1;
        } else {
            break;
        }
    }
    if j == start_idx || j >= chars.len() {
        return None;
    }

    let raw_tag = &input[start_byte..chars[j].0];
    let (clean_tag, mut id_opt, mut classes) = split_css_shorthand(raw_tag);

    let mut attrs = IndexMap::new();
    // Optional unspaced `{Props}` and/or `(Slots)` before `[` (`span{class hi}[text]`, `a(/docs)[links]`)
    loop {
        if j >= chars.len() {
            break;
        }
        if chars[j].1 == '{' {
            let brace_start_byte = chars[j].0;
            let (inside_brace, after_brace_str) =
                extract_balanced_delim(&input[brace_start_byte..], '{', '}')?;
            if let Some(mut parsed_attrs) =
                extract_inline_form_props(&format!("{{{}}}", inside_brace))
            {
                if id_opt.is_none() {
                    if let Some(HirNode::String(id_str, _)) = parsed_attrs.shift_remove("id") {
                        id_opt = Some(id_str);
                    }
                }
                if classes.is_empty() {
                    if let Some(HirNode::String(cls_str, _)) = parsed_attrs.shift_remove("class") {
                        classes.extend(cls_str.split_whitespace().map(|s| s.to_string()));
                    }
                }
                for (k, v) in parsed_attrs {
                    attrs.insert(k, v);
                }
            }
            let consumed_bytes = input[brace_start_byte..].len() - after_brace_str.len();
            let target_byte = brace_start_byte + consumed_bytes;
            while j < chars.len() && chars[j].0 < target_byte {
                j += 1;
            }
            continue;
        }
        if chars[j].1 == '(' {
            let paren_start_byte = chars[j].0;
            let (inside_paren, after_paren_str) =
                extract_balanced_delim(&input[paren_start_byte..], '(', ')')?;
            let slot_cells = parse_tuple_cells(inside_paren, Span::default());
            if let Some(first_slot) = slot_cells.first() {
                if clean_tag == "a" && !attrs.contains_key("href") {
                    attrs.insert("href".to_string(), first_slot.clone());
                } else {
                    attrs.insert(
                        "()".to_string(),
                        HirNode::Tuple {
                            schema: None,
                            is_2d_table: false,
                            rows: vec![slot_cells],
                            span: Span::default(),
                        },
                    );
                }
            }
            let consumed_bytes = input[paren_start_byte..].len() - after_paren_str.len();
            let target_byte = paren_start_byte + consumed_bytes;
            while j < chars.len() && chars[j].0 < target_byte {
                j += 1;
            }
            continue;
        }
        break;
    }

    if j >= chars.len() || chars[j].1 != '[' {
        return None;
    }

    let bracket_start_byte = chars[j].0;
    let (inside_bracket, after_bracket_str) =
        extract_balanced_delim(&input[bracket_start_byte..], '[', ']')?;
    let trimmed_inner = trim_inline_stream_spaces(inside_bracket);
    let children = parse_stream_segments(trimmed_inner);

    let consumed_bytes = input[bracket_start_byte..].len() - after_bracket_str.len();
    let end_byte = bracket_start_byte + consumed_bytes;
    let mut next_idx = j;
    while next_idx < chars.len() && chars[next_idx].0 < end_byte {
        next_idx += 1;
    }

    Some((
        StreamSegment::InlineTag {
            tag: clean_tag,
            id: id_opt,
            classes,
            attrs,
            children,
        },
        next_idx,
    ))
}

/// Parses `;` and `,` separated cells in a Tuple line.
pub fn parse_tuple_line_rows(line_text: &str, span: Span) -> Vec<Vec<HirNode>> {
    let trimmed = line_text.trim();
    if trimmed.is_empty() {
        return vec![];
    }
    let semi_rows = split_top_level_delims(trimmed, &[';']);
    semi_rows
        .into_iter()
        .filter(|r| !r.trim().is_empty())
        .map(|r| parse_tuple_cells(r, span))
        .collect()
}

pub fn parse_tuple_cells(row_str: &str, span: Span) -> Vec<HirNode> {
    let raw_cells = split_top_level_delims(row_str, &[',']);
    raw_cells
        .into_iter()
        .map(|cell| parse_single_tuple_cell(cell, span))
        .collect()
}

pub fn parse_single_tuple_cell(cell: &str, span: Span) -> HirNode {
    let c = cell.trim();
    // An empty cell `, ,` in `()` evaluates to `null` / `None`
    if c.is_empty() {
        return HirNode::Null(span);
    }
    if c == "\"\"" {
        return HirNode::String(String::new(), span);
    }
    if c.starts_with('"') && c.ends_with('"') && c.len() >= 2 {
        return HirNode::String(unquote_string(c), span);
    }
    if c.starts_with('`') && c.ends_with('`') && c.len() >= 2 {
        return HirNode::String(c[1..c.len() - 1].to_string(), span);
    }
    if let Some((type_name, inner_val)) = parse_typed_literal(c) {
        return HirNode::TypedLiteral {
            type_name,
            value: inner_val,
            span,
        };
    }
    if let Some(sym) = c.strip_prefix('\'') {
        if !sym.is_empty() && !sym.contains(' ') && !sym.ends_with('\'') {
            return HirNode::Symbol(sym.to_string(), span);
        }
    }
    if c.starts_with("-> ")
        || c.starts_with("<-> ")
        || c.contains(" -> ")
        || c.contains(" <-> ")
    {
        if let Some(edge) = parse_edge_str(c, span) {
            return HirNode::Edge(edge);
        }
    }
    if is_valid_ref_path(c) {
        return HirNode::Reference {
            path: c.to_string(),
            span,
        };
    }
    match c {
        "true" => HirNode::Bool(true, span),
        "false" => HirNode::Bool(false, span),
        "null" | "None" => HirNode::Null(span),
        _ => {
            if let Some(n) = parse_scalar_number(c) {
                HirNode::Number(n, span)
            } else {
                HirNode::String(c.to_string(), span)
            }
        }
    }
}

/// Splits CSS shorthand `tag#id.class1.class2` into `(tag, Option<id>, Vec<class>)`.
pub fn split_css_shorthand(raw: &str) -> (String, Option<String>, Vec<String>) {
    if raw.starts_with('.') || raw.starts_with('#') || raw.starts_with('"') {
        return (raw.to_string(), None, vec![]);
    }
    if !raw.contains('#') && !raw.contains('.') {
        return (raw.to_string(), None, vec![]);
    }

    let mut tag = String::new();
    let mut id_opt = None;
    let mut classes = Vec::new();

    let mut mode = 't';
    let mut buf = String::new();

    for ch in raw.chars() {
        if ch == '#' || ch == '.' {
            match mode {
                't' => tag = buf.clone(),
                '#' => id_opt = Some(buf.clone()),
                '.' => {
                    if !buf.is_empty() {
                        classes.push(buf.clone());
                    }
                }
                _ => {}
            }
            buf.clear();
            mode = ch;
        } else {
            buf.push(ch);
        }
    }
    match mode {
        't' => tag = buf,
        '#' => id_opt = Some(buf),
        '.' => {
            if !buf.is_empty() {
                classes.push(buf);
            }
        }
        _ => {}
    }

    (tag, id_opt, classes)
}

fn extract_inline_form_props(tail_str: &str) -> Option<IndexMap<String, HirNode>> {
    let trimmed = tail_str.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let (inside, _) = extract_balanced_delim(trimmed, '{', '}')?;
    let mut map = IndexMap::new();
    let mut edges = Vec::new();
    let mut overlays = Vec::new();
    let mut dummy_syms = Vec::new();
    parse_and_insert_form_leaf(
        inside,
        &mut map,
        &mut edges,
        &mut overlays,
        Span::default(),
        &mut dummy_syms,
        "",
    );
    Some(map)
}

/// Resolves Algebraic Composition (`&` Deep Merge, `^` Symmetric Difference / XOR, `/` Key Strip)
/// across all Form scopes in the document, enforcing strict DAG acyclicity.
fn resolve_overlays_in_tree(node: &mut HirNode, diagnostics: &mut Vec<Diagnostic>) {
    match node {
        HirNode::Form { map, .. } => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in &keys {
                if let Some(child) = map.get_mut(k) {
                    resolve_overlays_in_tree(child, diagnostics);
                }
            }

            for k in &keys {
                let mut visited = HashSet::new();
                if has_overlay_cycle(k, map, &mut visited) {
                    let span = map.get(k).map(|n| n.span()).unwrap_or_default();
                    diagnostics.push(Diagnostic {
                        line: span.line,
                        col: span.col,
                        end_col: span.end_col,
                        severity: DiagnosticSeverity::Error,
                        message: format!(
                            "Cyclic algebraic overlay `&` detected involving `{}` (requires a strict DAG)",
                            k
                        ),
                        code: "RN004".to_string(),
                    });
                    continue;
                }

                let overlays_to_apply = match map.get(k) {
                    Some(HirNode::Form { overlays, .. }) if !overlays.is_empty() => {
                        overlays.clone()
                    }
                    _ => continue,
                };

                let own_node = map.get(k).unwrap().clone();
                let is_empty_clone_form = match &own_node {
                    HirNode::Form {
                        map: m, edges: e, ..
                    } => m.is_empty() && e.is_empty(),
                    _ => false,
                };

                // Omitting `{}` clones the merged result directly (`backup_timeout & base.timeout`)
                if is_empty_clone_form && overlays_to_apply.len() == 1 {
                    if let OverlayOp::Merge(ref_path) = &overlays_to_apply[0] {
                        let target_key = ref_path.trim_start_matches('.');
                        if let Some(base_node) = lookup_in_map(map, target_key) {
                            let cloned = base_node.clone();
                            if !matches!(cloned, HirNode::Form { .. }) {
                                map.insert(k.clone(), cloned);
                                continue;
                            }
                        }
                    }
                }

                let mut acc = HirNode::Form {
                    schema: match &own_node {
                        HirNode::Form { schema, .. } => schema.clone(),
                        _ => None,
                    },
                    map: IndexMap::new(),
                    edges: vec![],
                    overlays: vec![],
                    span: own_node.span(),
                };

                for op in overlays_to_apply {
                    match op {
                        OverlayOp::Merge(ref_path) => {
                            let target_key = ref_path.trim_start_matches('.');
                            if let Some(base_node) = lookup_in_map(map, target_key) {
                                deep_merge_nodes(&mut acc, base_node.clone());
                            } else {
                                let span = own_node.span();
                                diagnostics.push(Diagnostic {
                                    line: span.line,
                                    col: span.col,
                                    end_col: span.end_col,
                                    severity: DiagnosticSeverity::Warning,
                                    message: format!(
                                        "Unresolved overlay reference `{}` in `{}`",
                                        ref_path, k
                                    ),
                                    code: "RN005".to_string(),
                                });
                            }
                        }
                        OverlayOp::Xor(ref_path) => {
                            let target_key = ref_path.trim_start_matches('.');
                            if let Some(base_node) = lookup_in_map(map, target_key) {
                                xor_merge_nodes(&mut acc, base_node.clone());
                            }
                        }
                        OverlayOp::Omit(omit_key) => {
                            if let HirNode::Form { map: acc_map, .. } = &mut acc {
                                let clean_omit = omit_key.trim_start_matches('\'');
                                remove_dotted_key(acc_map, clean_omit);
                            }
                        }
                    }
                }

                if let HirNode::Form {
                    schema: own_schema,
                    map: own_map,
                    edges: own_edges,
                    span: own_span,
                    ..
                } = own_node
                {
                    deep_merge_nodes(
                        &mut acc,
                        HirNode::Form {
                            schema: own_schema,
                            map: own_map,
                            edges: own_edges,
                            overlays: vec![],
                            span: own_span,
                        },
                    );
                }
                map.insert(k.clone(), acc);
            }
        }
        HirNode::Stream { items, .. } => {
            for item in items {
                resolve_overlays_in_tree(item, diagnostics);
            }
        }
        HirNode::Outliner { children, .. } => {
            for child in children {
                resolve_overlays_in_tree(child, diagnostics);
            }
        }
        HirNode::Triad { body, .. } => {
            if let Some(b) = body {
                for item in b {
                    resolve_overlays_in_tree(item, diagnostics);
                }
            }
        }
        _ => {}
    }
}

fn has_overlay_cycle(
    current_key: &str,
    map: &IndexMap<String, HirNode>,
    visited: &mut HashSet<String>,
) -> bool {
    if !visited.insert(current_key.to_string()) {
        return true;
    }
    if let Some(HirNode::Form { overlays, .. }) = map.get(current_key) {
        for op in overlays {
            match op {
                OverlayOp::Merge(ref_path) | OverlayOp::Xor(ref_path) => {
                    let target = ref_path.trim_start_matches('.');
                    if has_overlay_cycle(target, map, visited) {
                        return true;
                    }
                }
                OverlayOp::Omit(_) => {}
            }
        }
    }
    visited.remove(current_key);
    false
}

fn lookup_in_map<'a>(map: &'a IndexMap<String, HirNode>, path: &str) -> Option<&'a HirNode> {
    let parts: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut cur = map.get(parts[0])?;
    for p in &parts[1..] {
        if let HirNode::Form { map: sub, .. } = cur {
            cur = sub.get(*p)?;
        } else {
            return None;
        }
    }
    Some(cur)
}

fn remove_dotted_key(map: &mut IndexMap<String, HirNode>, key_path: &str) {
    let parts: Vec<&str> = key_path.split('.').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return;
    }
    if parts.len() == 1 {
        map.shift_remove(parts[0]);
        return;
    }
    if let Some(HirNode::Form { map: sub, .. }) = map.get_mut(parts[0]) {
        remove_dotted_key(sub, &parts[1..].join("."));
    }
}

// ----------------------------------------------------------------------------
// String & Delimiter Helpers
// ----------------------------------------------------------------------------

fn unquote_key(s: &str) -> String {
    let t = s.trim();
    if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
        unquote_string(t)
    } else {
        t.to_string()
    }
}

pub fn unquote_string(s: &str) -> String {
    let t = s.trim();
    if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
        let inner = &t[1..t.len() - 1];
        inner
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
            .replace("\\n", "\n")
            .replace("\\t", "\t")
    } else {
        t.to_string()
    }
}

fn extract_balanced_delim(s: &str, open: char, close: char) -> Option<(&str, &str)> {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    if chars.is_empty() || chars[0].1 != open {
        return None;
    }
    let mut depth = 0;
    let mut in_quotes = false;
    let mut in_backticks = false;
    let mut i = 0;

    while i < chars.len() {
        let (byte_pos, ch) = chars[i];
        if ch == '\\' && in_quotes {
            i += 2;
            continue;
        }
        if ch == '"' && !in_backticks {
            in_quotes = !in_quotes;
        } else if ch == '`' && !in_quotes {
            in_backticks = !in_backticks;
        } else if !in_quotes && !in_backticks {
            if ch == open {
                depth += 1;
            } else if ch == close {
                depth -= 1;
                if depth == 0 {
                    let inside = &s[chars[0].0 + open.len_utf8()..byte_pos];
                    let after = &s[byte_pos + close.len_utf8()..];
                    return Some((inside, after));
                }
            }
        }
        i += 1;
    }
    None
}

fn split_top_level_delims<'a>(s: &'a str, delims: &[char]) -> Vec<&'a str> {
    let mut result = Vec::new();
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut depth_paren: i32 = 0;
    let mut depth_brace: i32 = 0;
    let mut depth_bracket: i32 = 0;
    let mut in_quotes = false;
    let mut in_backticks = false;
    let mut last_start = 0;
    let mut i = 0;

    while i < chars.len() {
        let (byte_pos, ch) = chars[i];
        if ch == '\\' && in_quotes {
            i += 2;
            continue;
        }
        if ch == '"' && !in_backticks {
            in_quotes = !in_quotes;
        } else if ch == '`' && !in_quotes {
            in_backticks = !in_backticks;
        } else if !in_quotes && !in_backticks {
            match ch {
                '(' => depth_paren += 1,
                ')' => depth_paren = (depth_paren - 1).max(0),
                '{' => depth_brace += 1,
                '}' => depth_brace = (depth_brace - 1).max(0),
                '[' => depth_bracket += 1,
                ']' => depth_bracket = (depth_bracket - 1).max(0),
                c if delims.contains(&c)
                    && depth_paren == 0
                    && depth_brace == 0
                    && depth_bracket == 0 =>
                {
                    result.push(&s[last_start..byte_pos]);
                    last_start = byte_pos + c.len_utf8();
                }
                _ => {}
            }
        }
        i += 1;
    }
    result.push(&s[last_start..]);
    result
}

fn find_top_level_char(s: &str, target: char) -> Option<usize> {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut depth_paren: i32 = 0;
    let mut depth_brace: i32 = 0;
    let mut depth_bracket: i32 = 0;
    let mut in_quotes = false;
    let mut in_backticks = false;
    let mut i = 0;

    while i < chars.len() {
        let (byte_pos, ch) = chars[i];
        if ch == '\\' && in_quotes {
            i += 2;
            continue;
        }
        if ch == '"' && !in_backticks {
            in_quotes = !in_quotes;
        } else if ch == '`' && !in_quotes {
            in_backticks = !in_backticks;
        } else if !in_quotes && !in_backticks {
            if ch == target && depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 {
                return Some(byte_pos);
            }
            match ch {
                '(' => depth_paren += 1,
                ')' => depth_paren = (depth_paren - 1).max(0),
                '{' => depth_brace += 1,
                '}' => depth_brace = (depth_brace - 1).max(0),
                '[' => depth_bracket += 1,
                ']' => depth_bracket = (depth_bracket - 1).max(0),
                _ => {}
            }
        }
        i += 1;
    }
    None
}

fn find_top_level_substring(s: &str, needle: &str) -> Option<usize> {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut depth_paren: i32 = 0;
    let mut depth_brace: i32 = 0;
    let mut depth_bracket: i32 = 0;
    let mut in_quotes = false;
    let mut in_backticks = false;
    let mut i = 0;

    while i < chars.len() {
        let (byte_pos, ch) = chars[i];
        if ch == '\\' && in_quotes {
            i += 2;
            continue;
        }
        if ch == '"' && !in_backticks {
            in_quotes = !in_quotes;
        } else if ch == '`' && !in_quotes {
            in_backticks = !in_backticks;
        } else if !in_quotes && !in_backticks {
            if depth_paren == 0
                && depth_brace == 0
                && depth_bracket == 0
                && s[byte_pos..].starts_with(needle)
            {
                return Some(byte_pos);
            }
            match ch {
                '(' => depth_paren += 1,
                ')' => depth_paren = (depth_paren - 1).max(0),
                '{' => depth_brace += 1,
                '}' => depth_brace = (depth_brace - 1).max(0),
                '[' => depth_bracket += 1,
                ']' => depth_bracket = (depth_bracket - 1).max(0),
                _ => {}
            }
        }
        i += 1;
    }
    None
}

fn join_path(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_string()
    } else {
        format!("{}.{}", parent, key)
    }
}