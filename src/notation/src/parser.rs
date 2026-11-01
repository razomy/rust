//! Phase 2 Unified Node Parser & HIR Lowering Engine for Razomy Notation (`.rn`) v13.0.
//!
//! Implements Axiom I (Unified Node `Prefix? Key? Params? Props? Body? Edge? Value?`)
//! and Axiom V (AST-Level Auto-Typing & Lexical Simplicity).

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

/// Parses a `.rn` source string into a resolved `HirDocument`.
pub fn parse_rn(source: &str) -> HirDocument {
    let pre = pre_lex(source);
    let mut diagnostics = pre.diagnostics;
    let mut symbols = Vec::new();

    if pre.lines.is_empty() {
        return HirDocument {
            root_mode: ContextMode::Stream,
            root: HirNode::Stream {
                items: vec![],
                span: Span::default(),
            },
            diagnostics,
            symbols,
        };
    }

    // Determine root context mode (Law 1.1 Root Bijection Rule)
    let first_non_comment = pre.lines.iter().find(|l| !l.is_comment);
    let mut root_mode = ContextMode::Stream;

    if let Some(first) = first_non_comment {
        if first.head_text.is_empty() && first.activation == IndentActivation::IndentForm {
            root_mode = ContextMode::Form;
        } else if first.head_text.is_empty() && first.activation == IndentActivation::IndentTuple {
            root_mode = ContextMode::Tuple;
        } else if first.activation == IndentActivation::IndentForm
            || is_likely_form_root(&pre.lines)
        {
            root_mode = ContextMode::Form;
        }
    }

    let mut cursor = 0;
    let root = if root_mode == ContextMode::Form {
        // If first line is a bare `{`, skip it and parse its children
        if let Some(first) = first_non_comment {
            if first.head_text.is_empty() && first.activation == IndentActivation::IndentForm {
                let first_idx = pre.lines.iter().position(|l| !l.is_comment).unwrap_or(0);
                cursor = first_idx + 1;
                parse_form_block(&pre.lines, &mut cursor, 0, &mut diagnostics, &mut symbols, "")
            } else {
                parse_form_block(&pre.lines, &mut cursor, 0, &mut diagnostics, &mut symbols, "")
            }
        } else {
            parse_form_block(&pre.lines, &mut cursor, 0, &mut diagnostics, &mut symbols, "")
        }
    } else if root_mode == ContextMode::Tuple {
        if let Some(first) = first_non_comment {
            if first.head_text.is_empty() && first.activation == IndentActivation::IndentTuple {
                let first_idx = pre.lines.iter().position(|l| !l.is_comment).unwrap_or(0);
                cursor = first_idx + 1;
            }
        }
        parse_tuple_block(&pre.lines, &mut cursor, 0, &mut diagnostics)
    } else {
        parse_stream_block(&pre.lines, &mut cursor, 0, &mut diagnostics, &mut symbols, "")
    };

    // Resolve Algebraic Overlays (`&` and `/` — Law 4.3)
    let mut resolved_root = root;
    resolve_overlays_in_tree(&mut resolved_root, &mut diagnostics);

    HirDocument {
        root_mode,
        root: resolved_root,
        diagnostics,
        symbols,
    }
}

/// Heuristic for Law 1.1 / Law 1.2: Determine if top-level document is rooted in Form `{` mode.
fn is_likely_form_root(lines: &[LexedLine]) -> bool {
    let top_lines: Vec<&LexedLine> = lines
        .iter()
        .filter(|l| !l.is_comment && l.indent == 0)
        .collect();
    if top_lines.is_empty() {
        return false;
    }
    // If any top-level line uses `..` margin reset or `{` activation, or all top-level lines are key-value pairs
    if top_lines.iter().any(|l| {
        l.activation == IndentActivation::IndentForm || l.head_text.starts_with("..")
    }) {
        return true;
    }
    // Check if first line is an explicit stream tag like `h1 [...]` or `p [...]`
    let first = top_lines[0];
    if first.activation == IndentActivation::IndentStream
        || first.original_trimmed.contains('[')
    {
        return false;
    }
    // If single named root block with Tuple `(`, e.g., `Users (` -> Form root containing `Users`
    if top_lines.len() == 1
        && first.activation == IndentActivation::IndentTuple
        && !first.head_text.is_empty()
    {
        return true;
    }
    // Check if top-level lines look like `key value` or `key (`
    top_lines.iter().all(|l| {
        !l.head_text.is_empty()
            && !l.head_text.starts_with('[')
            && (l.head_text.contains(' ')
                || l.activation != IndentActivation::Leaf
                || is_single_identifier(&l.head_text))
    })
}

fn is_single_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// Parses a block of lines in `{` Form Context.
fn parse_form_block(
    lines: &[LexedLine],
    cursor: &mut usize,
    min_indent: usize,
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

        // Law 3.3: Topological Margin Reset (`..target` at Column 0)
        if line.indent == 0 && line.head_text.starts_with("..") {
            let reset_str = &line.head_text[2..];
            let (target_path, line_overlays, inline_tail) = parse_header_components(reset_str);
            symbols.push(SymbolInfo {
                name: target_path.clone(),
                full_path: target_path.clone(),
                kind: SymbolKind::MarginReset,
                span,
            });
            *cursor += 1;

            let child_node = build_activated_child(
                &line.activation,
                &line.raw_payload,
                inline_tail.as_deref(),
                lines,
                cursor,
                cur_indent + 1,
                ContextMode::Form,
                diagnostics,
                symbols,
                &target_path,
                span,
            );
            let child_with_overlays = attach_overlays(child_node, line_overlays);
            merge_into_path(&mut map, &target_path, child_with_overlays);
            continue;
        }

        // Law 4.1: Standalone Edge Expression in Form (`Ingress -> (Auth, Api) { ... }`)
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

        // Raw Block with key (`snippet "ts` or `description r3"..."3`)
        if let Some(raw) = &line.raw_payload {
            let key = if line.head_text.is_empty() {
                "\"".to_string()
            } else {
                unquote_key(&line.head_text)
            };
            let raw_node = HirNode::Raw {
                lang: raw.lang.clone(),
                content: raw.content.clone(),
                span,
            };
            insert_folded_key(&mut map, &key, None, vec![], raw_node, span);
            *cursor += 1;
            continue;
        }

        // Check if this Form line has indented children or is a leaf pair
        if line.activation != IndentActivation::Leaf {
            let (raw_key, params_opt, line_overlays, inline_tail) =
                parse_key_and_header(&line.head_text);
            let full_sym_path = join_path(parent_path, &raw_key);
            symbols.push(SymbolInfo {
                name: raw_key.clone(),
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
                inline_tail.as_deref(),
                lines,
                cursor,
                cur_indent + 1,
                ContextMode::Form,
                diagnostics,
                symbols,
                &full_sym_path,
                span,
            );

            let child_with_overlays = attach_overlays(child_node, line_overlays);
            insert_folded_key(&mut map, &raw_key, params_opt, vec![], child_with_overlays, span);
        } else {
            // Leaf Form Pair (`key value`, `key`, `key { a 1 }`, `key(a, b) { ... }`)
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

/// Parses a block of lines in `[` Stream Context (Law 1.1).
fn parse_stream_block(
    lines: &[LexedLine],
    cursor: &mut usize,
    min_indent: usize,
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

        // Raw Block inside Stream (`snippet "ts`)
        if let Some(raw) = &line.raw_payload {
            let raw_node = HirNode::Raw {
                lang: raw.lang.clone(),
                content: raw.content.clone(),
                span,
            };
            if line.head_text.is_empty() {
                items.push(raw_node);
            } else {
                let mut m = IndexMap::new();
                m.insert(unquote_key(&line.head_text), raw_node);
                items.push(HirNode::Form {
                    map: m,
                    edges: vec![],
                    overlays: vec![],
                    span,
                });
            }
            *cursor += 1;
            continue;
        }

        // Indented block tag / table / form inside Stream (`ul [`, `benchmarks (`, `article#post { lang en } [`)
        if line.activation != IndentActivation::Leaf {
            let (raw_key, params_opt, line_overlays, inline_tail) =
                parse_key_and_header(&line.head_text);
            let (clean_tag, id_opt, classes) = split_css_shorthand(&raw_key);
            let full_sym_path = join_path(parent_path, &clean_tag);
            symbols.push(SymbolInfo {
                name: clean_tag.clone(),
                full_path: full_sym_path.clone(),
                kind: SymbolKind::StreamTag,
                span,
            });
            *cursor += 1;

            let inline_props = inline_tail
                .as_deref()
                .and_then(extract_inline_form_props);

            let child_node = build_activated_child(
                &line.activation,
                &line.raw_payload,
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

            if id_opt.is_some() || !classes.is_empty() || inline_props.is_some() || params_opt.is_some() {
                let body_vec = match child_node {
                    HirNode::Stream { items: b, .. } => Some(b),
                    other => Some(vec![other]),
                };
                items.push(HirNode::Triad {
                    name: clean_tag,
                    id: id_opt,
                    classes,
                    params: params_opt,
                    overlays: line_overlays,
                    props: inline_props,
                    body: body_vec,
                    span,
                });
            } else {
                let mut m = IndexMap::new();
                let child_with_overlays = attach_overlays(child_node, line_overlays);
                m.insert(clean_tag, child_with_overlays);
                items.push(HirNode::Form {
                    map: m,
                    edges: vec![],
                    overlays: vec![],
                    span,
                });
            }
            continue;
        }

        // Single-line block tag in Stream: e.g., `h1 [ Razomy Guide ]` or `article#id { a 1 } [ hello ]`
        if let Some(triad_node) = try_parse_single_line_stream_tag(&line.head_text, span, symbols, parent_path) {
            items.push(triad_node);
            *cursor += 1;
            continue;
        }

        // Ordinary Stream Leaf line (prose with optional unspaced inline tags `b[bold]`)
        let segs = parse_stream_segments(&line.head_text);
        if segs.len() == 1 {
            if let StreamSegment::Text(t) = &segs[0] {
                items.push(HirNode::String(t.clone(), span));
                *cursor += 1;
                continue;
            }
        }
        items.push(HirNode::RichText {
            segments: segs,
            span,
        });
        *cursor += 1;
    }

    HirNode::Stream {
        items,
        span: Span {
            line: start_line,
            col: min_indent,
            end_col: min_indent,
        },
    }
}

/// Parses a block of lines in `(` Tuple Context (Law 1.3).
fn parse_tuple_block(
    lines: &[LexedLine],
    cursor: &mut usize,
    min_indent: usize,
    _diagnostics: &mut Vec<Diagnostic>,
) -> HirNode {
    let start_line = lines.get(*cursor).map(|l| l.line_idx).unwrap_or(0);
    let mut rows: Vec<Vec<HirNode>> = Vec::new();

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

        let parsed_rows = parse_tuple_line_rows(&line.head_text, span);
        rows.extend(parsed_rows);
        *cursor += 1;
    }

    HirNode::Tuple {
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
    inline_tail: Option<&str>,
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
            return HirNode::Raw {
                lang: lang.clone(),
                content,
                span,
            };
        }
        IndentActivation::IndentInherit | IndentActivation::Leaf => parent_mode,
    };

    let mut block_node = match target_mode {
        ContextMode::Form => {
            parse_form_block(lines, cursor, child_min_indent, diagnostics, symbols, current_path)
        }
        ContextMode::Stream => {
            parse_stream_block(lines, cursor, child_min_indent, diagnostics, symbols, current_path)
        }
        ContextMode::Tuple => parse_tuple_block(lines, cursor, child_min_indent, diagnostics),
        ContextMode::Raw => HirNode::Null(span),
    };

    // Merge inline `{ ... }` tail if present alongside block children
    if let Some(tail_str) = inline_tail {
        if let Some(inline_props) = extract_inline_form_props(tail_str) {
            if let HirNode::Form { map, .. } = &mut block_node {
                for (k, v) in inline_props {
                    map.insert(k, v);
                }
            }
        }
    }

    block_node
}

/// Splits a header line into `(key_path, optional_params, overlay_ops, optional_inline_tail)`.
fn parse_key_and_header(
    head: &str,
) -> (
    String,
    Option<Vec<HirNode>>,
    Vec<OverlayOp>,
    Option<String>,
) {
    let trimmed = head.trim();
    if trimmed.is_empty() {
        return (String::new(), None, vec![], None);
    }

    // Extract key token (may be quoted or followed immediately by `(params)`)
    let (key_part, rest_after_key) = extract_first_key_token(trimmed);
    let mut rest = rest_after_key.trim_start();

    // Law 3.4: Unspaced `name(a, b)` in Key position attaches positional parameters to `name`
    let mut params_opt = None;
    if !rest_after_key.starts_with(' ') && rest.starts_with('(') {
        if let Some((inside_parens, after_parens)) = extract_balanced_delim(rest, '(', ')') {
            let row = parse_tuple_cells(inside_parens, Span::default());
            params_opt = Some(row);
            rest = after_parens.trim_start();
        }
    }

    let (_, overlays, inline_tail) = parse_header_components(rest);
    (unquote_key(&key_part), params_opt, overlays, inline_tail)
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

/// Extracts `(leading_path, overlay_chain, inline_tail)` from a header or margin reset string.
fn parse_header_components(s: &str) -> (String, Vec<OverlayOp>, Option<String>) {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return (String::new(), vec![], None);
    }

    // Check for inline `{...}` or `[...]` tail
    let mut main_part = trimmed;
    let mut inline_tail = None;
    if let Some(brace_pos) = find_top_level_char(trimmed, '{') {
        main_part = trimmed[..brace_pos].trim();
        inline_tail = Some(trimmed[brace_pos..].trim().to_string());
    } else if let Some(bracket_pos) = find_top_level_char(trimmed, '[') {
        main_part = trimmed[..bracket_pos].trim();
        inline_tail = Some(trimmed[bracket_pos..].trim().to_string());
    }

    let tokens: Vec<&str> = main_part.split_whitespace().collect();
    if tokens.is_empty() {
        return (String::new(), vec![], inline_tail);
    }

    let mut first_path = String::new();
    let mut idx = 0;
    if tokens[0] != "&" && tokens[0] != "/" {
        first_path = tokens[0].to_string();
        idx = 1;
    }

    let mut overlays = Vec::new();
    while idx + 1 < tokens.len() {
        match tokens[idx] {
            "&" => {
                overlays.push(OverlayOp::Merge(tokens[idx + 1].to_string()));
                idx += 2;
            }
            "/" => {
                overlays.push(OverlayOp::Omit(tokens[idx + 1].to_string()));
                idx += 2;
            }
            _ => break,
        }
    }

    (first_path, overlays, inline_tail)
}

fn attach_overlays(node: HirNode, mut new_overlays: Vec<OverlayOp>) -> HirNode {
    if new_overlays.is_empty() {
        return node;
    }
    match node {
        HirNode::Form {
            map,
            edges,
            mut overlays,
            span,
        } => {
            new_overlays.append(&mut overlays);
            HirNode::Form {
                map,
                edges,
                overlays: new_overlays,
                span,
            }
        }
        other => other,
    }
}

/// Inserts a key into a Form map, handling Law 3.3 (Dotted Path Folding `a.b.c`)
/// and Law 3.4 (Multi-Block Parameter Folding `resource(aws_s3, logs)` & CSS `#id.class`).
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

    // If parameterized key `resource(aws_s3_bucket, logs)` in Form (Law 3.4)
    if let Some(params) = params_opt {
        let param_keys: Vec<String> = params
            .iter()
            .map(|p| match p.to_canonical_json() {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            })
            .collect();

        if !param_keys.is_empty() {
            // Fold parameters as nested sub-keys so `..resource.aws_s3_bucket.logs` can target it (Example 7)
            // and also store `"()"` on the leaf if only 1 parameter is used (Example 6 `container(node:22-alpine)`).
            if param_keys.len() == 1 {
                if let HirNode::Form { map: inner_map, .. } = &mut value_node {
                    inner_map.insert(
                        "()".to_string(),
                        HirNode::Tuple {
                            rows: vec![params],
                            span,
                        },
                    );
                }
                map.insert(raw_key.to_string(), value_node);
                return;
            } else {
                let mut full_chain = vec![raw_key.to_string()];
                full_chain.extend(param_keys);
                insert_path_slice(map, &full_chain, value_node, span);
                return;
            }
        }
    }

    // Law 3.3: Path Folding (`a.b.c`)
    // Leading dot (`.btn`), standalone dot (`.`), or quoted key (`"a.b"`) is a literal token!
    if should_fold_dotted_key(raw_key) {
        let parts: Vec<String> = raw_key.split('.').map(|s| s.to_string()).collect();
        insert_path_slice(map, &parts, value_node, span);
    } else {
        map.insert(raw_key.to_string(), value_node);
    }
}

fn should_fold_dotted_key(key: &str) -> bool {
    if key.starts_with('.') || key.starts_with('"') || !key.contains('.') {
        return false;
    }
    // Do not fold CSS tag#id.class if `#` is present
    if key.contains('#') {
        return false;
    }
    key.split('.').all(|seg| !seg.is_empty())
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
        map: IndexMap::new(),
        edges: vec![],
        overlays: vec![],
        span,
    });
    if let HirNode::Form { map: sub_map, .. } = entry {
        insert_path_slice(sub_map, rest, value_node, span);
    }
}

/// Law 3.3: Merge & Append Semantics when re-entering an existing node via `..target`.
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

/// Parses a single leaf line in Form `{` context and inserts into `map`.
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

    // If the line contains top-level `,` or `;` outside brackets/quotes, split into multiple pairs (Law 3.2)
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
    let (raw_key, rest_after_key) = extract_first_key_token(pair_str);
    let clean_key = unquote_key(&raw_key);
    let mut rest = rest_after_key;

    // Check unspaced `name(a, b)` in Key position (Law 1.2 & Law 3.4)
    let mut params_opt = None;
    if !rest.starts_with(' ') && rest.starts_with('(') {
        if let Some((inside_parens, after_parens)) = extract_balanced_delim(rest, '(', ')') {
            params_opt = Some(parse_tuple_cells(inside_parens, span));
            rest = after_parens;
        }
    }

    let val_str = rest.trim();
    let full_sym_path = join_path(parent_path, &clean_key);
    symbols.push(SymbolInfo {
        name: clean_key.clone(),
        full_path: full_sym_path.clone(),
        kind: SymbolKind::FormKey,
        span,
    });

    // Law 1.2: A bare key with no value and no indented children evaluates to `null`
    if val_str.is_empty() {
        insert_folded_key(
            map,
            &clean_key,
            params_opt,
            vec![],
            HirNode::Null(span),
            span,
        );
        return;
    }

    // Check if value starts with inline `{ ... }` block (e.g., `&:hover { background #222222 }`)
    if val_str.starts_with('{') && val_str.ends_with('}') && val_str.len() >= 2 {
        if val_str == "{}" {
            // Law 1.2: Closed inline `{}` evaluates to empty Object `{}`
            let empty_form = HirNode::Form {
                map: IndexMap::new(),
                edges: vec![],
                overlays: vec![],
                span,
            };
            insert_folded_key(map, &clean_key, params_opt, vec![], empty_form, span);
            return;
        }
        if let Some(inline_map) = extract_inline_form_props(val_str) {
            let form_node = HirNode::Form {
                map: inline_map,
                edges: vec![],
                overlays: vec![],
                span,
            };
            insert_folded_key(map, &clean_key, params_opt, vec![], form_node, span);
            return;
        }
    }

    // Parse value expression
    let val_node = parse_form_value_expr(val_str, span);
    insert_folded_key(map, &clean_key, params_opt, vec![], val_node, span);
}

/// Law 1.2: Safe Auto-Typing & Atomic Empty Literals in Form Value position.
pub fn parse_form_value_expr(val_str: &str, span: Span) -> HirNode {
    let v = val_str.trim();

    // Atomic Empty Literals (Law 1.2)
    match v {
        "\"\"" => return HirNode::String(String::new(), span),
        "{}" => {
            return HirNode::Form {
                map: IndexMap::new(),
                edges: vec![],
                overlays: vec![],
                span,
            }
        }
        "()" => {
            return HirNode::Tuple {
                rows: vec![],
                span,
            }
        }
        "[]" => {
            return HirNode::Stream {
                items: vec![],
                span,
            }
        }
        "true" => return HirNode::Bool(true, span),
        "false" => return HirNode::Bool(false, span),
        "null" => return HirNode::Null(span),
        _ => {}
    }

    // Spaced `name (a, b)` inside `{` assigns the array `["a", "b"]` as the value (Law 3.4)
    if v.starts_with('(') && v.ends_with(')') {
        let inner = &v[1..v.len() - 1];
        let rows = parse_tuple_line_rows(inner, span);
        return HirNode::Tuple { rows, span };
    }

    // Inline `[ ... ]` stream value
    if v.starts_with('[') && v.ends_with(']') {
        let inner = v[1..v.len() - 1].trim();
        let segs = parse_stream_segments(inner);
        return HirNode::Stream {
            items: vec![HirNode::RichText {
                segments: segs,
                span,
            }],
            span,
        };
    }

    // Unary or Binary Edge in value position (`-> Orgs.id` or `A -> B` — Law 4.1)
    if v.starts_with("-> ") || v.starts_with("<-> ") || v.contains(" -> ") || v.contains(" <-> ")
    {
        if let Some(edge) = parse_edge_str(v, span) {
            return HirNode::Edge(edge);
        }
    }

    // Law 4.2: Named Ancestor Lineage `.` and `..` as Value References
    if is_valid_ref_path(v) {
        return HirNode::Reference {
            path: v.to_string(),
            span,
        };
    }

    // Law 1.2: Safe Auto-Typing for Strict Numbers (no leading zeros like `0123`, no multi-dots like `1.2.0`)
    if is_strict_number(v) {
        if let Ok(num) = v.parse::<f64>() {
            return HirNode::Number(num, span);
        }
    }

    // Quoted string `"..."`
    if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
        return HirNode::String(unquote_string(v), span);
    }

    // Backtick verbatim span `` `...` ``
    if v.starts_with('`') && v.ends_with('`') && v.len() >= 2 {
        return HirNode::String(v[1..v.len() - 1].to_string(), span);
    }

    // Default: unquoted literal string (`The Matrix`, `postgres`, `gen_uuid()`, `#111111`)
    HirNode::String(v.to_string(), span)
}

/// Law 5.2 (AST Auto-Typing): Fast O(N) check for strict numbers (`8080`, `-3.14`).
/// Rejects leading zeros (`0123`) and multi-dot version strings (`1.2.0`).
pub fn is_strict_number(s: &str) -> bool {
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
        // Leading zero followed by another digit (e.g. `0123`) is a String, not a Number
        if i < bytes.len() && bytes[i] != b'.' {
            return false;
        }
    } else if bytes[i].is_ascii_digit() {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    } else {
        return false;
    }

    if i < bytes.len() {
        if bytes[i] != b'.' {
            return false;
        }
        i += 1;
        if i == bytes.len() {
            return false;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    i == bytes.len()
}

/// Checks Axiom IV Lineage Reference: Leading `.` or `..` immediately followed by an identifier.
/// Standalone `.` or `..` remains a literal string.
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

/// Parses a single line using the v13.0 `unified_node` Pest grammar rule.
pub fn parse_unified_line_ast(line: &str) -> bool {
    RnPestParser::parse(Rule::document_line, line).is_ok()
}

fn is_standalone_edge_line(s: &str) -> bool {
    let trimmed = s.trim();
    // Ensure `->` or `<->` appears outside quotes/parens as a space-delimited operator
    trimmed.starts_with("-> ")
        || trimmed.starts_with("<-> ")
        || find_top_level_substring(trimmed, " -> ").is_some()
        || find_top_level_substring(trimmed, " <-> ").is_some()
}

/// Parses a hypergraph edge expression (`Ingress -> (Auth, Api) { protocol grpc }` or `-> Orgs.id`).
pub fn parse_edge_str(s: &str, span: Span) -> Option<HirEdge> {
    let mut main_part = s.trim();
    let mut metadata = None;

    // Check trailing `{ ... }` metadata
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

    // Split chain of ` -> ` and ` <-> `
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

/// Parses a single-line stream block tag such as `h1 [ Razomy Guide ]` or `article#post { lang en } [ Hello ]`.
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

    // Must start with a tag identifier followed by space before `[` or `{` (Law 1.1 Block Tag vs Inline Tag)
    let (first_tok, rest) = extract_first_key_token(trimmed);
    if first_tok.is_empty() || !rest.starts_with(' ') {
        return None;
    }
    let rest_trimmed = rest.trim_start();
    let mut props_opt = None;
    let bracket_part;

    if rest_trimmed.starts_with('{') {
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
    let inner_stream_str = bracket_part[1..bracket_part.len() - 1].trim();
    let (clean_tag, id_opt, classes) = split_css_shorthand(&first_tok);

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

    if id_opt.is_some() || !classes.is_empty() || props_opt.is_some() {
        Some(HirNode::Triad {
            name: clean_tag,
            id: id_opt,
            classes,
            params: None,
            overlays: vec![],
            props: props_opt,
            body: Some(vec![body_node]),
            span,
        })
    } else {
        let mut m = IndexMap::new();
        m.insert(clean_tag, body_node);
        Some(HirNode::Form {
            map: m,
            edges: vec![],
            overlays: vec![],
            span,
        })
    }
}

/// Parses inline stream segments (`Text`, `` `Code` ``, `"Quoted"`, and unspaced `tag{attr}[inner]`).
pub fn parse_stream_segments(input: &str) -> Vec<StreamSegment> {
    let mut segments = Vec::new();
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    let mut i = 0;
    let mut text_start = 0;

    while i < chars.len() {
        let (byte_pos, ch) = chars[i];

        // 1. Backtick shield `` `...` ``
        if ch == '`' {
            if byte_pos > text_start {
                segments.push(StreamSegment::Text(input[text_start..byte_pos].to_string()));
            }
            let mut j = i + 1;
            while j < chars.len() && chars[j].1 != '`' {
                j += 1;
            }
            if j < chars.len() {
                let code_content = &input[byte_pos + 1..chars[j].0];
                segments.push(StreamSegment::Code(code_content.to_string()));
                i = j + 1;
                text_start = if i < chars.len() {
                    chars[i].0
                } else {
                    input.len()
                };
                continue;
            }
        }

        // 2. Quoted shield `"..."`
        if ch == '"' {
            if byte_pos > text_start {
                segments.push(StreamSegment::Text(input[text_start..byte_pos].to_string()));
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
                text_start = if i < chars.len() {
                    chars[i].0
                } else {
                    input.len()
                };
                continue;
            }
        }

        // 3. Law 1.1 Inline Tag (Unspaced Rule): `b[bold]` or `a{href /docs}[link]`
        // Must start at word boundary and attach `{` or `[` WITHOUT space
        let at_word_boundary = i == 0 || chars[i - 1].1.is_whitespace();
        if at_word_boundary && (ch.is_ascii_alphabetic() || ch == '_') {
            if let Some((tag_seg, next_char_idx)) = try_consume_inline_tag(input, &chars, i) {
                if byte_pos > text_start {
                    segments.push(StreamSegment::Text(input[text_start..byte_pos].to_string()));
                }
                segments.push(tag_seg);
                i = next_char_idx;
                text_start = if i < chars.len() {
                    chars[i].0
                } else {
                    input.len()
                };
                continue;
            }
        }

        i += 1;
    }

    if text_start < input.len() {
        segments.push(StreamSegment::Text(input[text_start..].to_string()));
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
    let (clean_tag, id_opt, classes) = split_css_shorthand(raw_tag);

    let mut attrs = IndexMap::new();
    // Optional unspaced `{attrs}` before `[`
    if chars[j].1 == '{' {
        let brace_start_byte = chars[j].0;
        let (inside_brace, after_brace_str) =
            extract_balanced_delim(&input[brace_start_byte..], '{', '}')?;
        if let Some(parsed_attrs) = extract_inline_form_props(&format!("{{{}}}", inside_brace)) {
            attrs = parsed_attrs;
        }
        let consumed_bytes = input[brace_start_byte..].len() - after_brace_str.len();
        let target_byte = brace_start_byte + consumed_bytes;
        while j < chars.len() && chars[j].0 < target_byte {
            j += 1;
        }
    }

    if j >= chars.len() || chars[j].1 != '[' {
        return None;
    }

    let bracket_start_byte = chars[j].0;
    let (inside_bracket, after_bracket_str) =
        extract_balanced_delim(&input[bracket_start_byte..], '[', ']')?;
    let children = parse_stream_segments(inside_bracket);

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

/// Parses `;` and `,` separated cells in a Tuple line (Law 1.3 & Law 3.2).
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
        .map(|cell| {
            let c = cell.trim();
            if c.is_empty() {
                return HirNode::String(String::new(), span);
            }
            // Quoted cell `"Verbose, requires quotes"`
            if c.starts_with('"') && c.ends_with('"') && c.len() >= 2 {
                return HirNode::String(unquote_string(c), span);
            }
            // Backtick cell
            if c.starts_with('`') && c.ends_with('`') && c.len() >= 2 {
                return HirNode::String(c[1..c.len() - 1].to_string(), span);
            }
            // Law 4.1: Edge expression inside Tuple cell (`-> Orgs.id` or `Ingest -> Transform -> Load`)
            if c.starts_with("-> ")
                || c.starts_with("<-> ")
                || c.contains(" -> ")
                || c.contains(" <-> ")
            {
                if let Some(edge) = parse_edge_str(c, span) {
                    return HirNode::Edge(edge);
                }
            }
            // Law 4.2: Lineage reference `.sibling` or `..Ancestor` (standalone `.` is literal string!)
            if is_valid_ref_path(c) {
                return HirNode::Reference {
                    path: c.to_string(),
                    span,
                };
            }
            match c {
                "true" => HirNode::Bool(true, span),
                "false" => HirNode::Bool(false, span),
                "null" => HirNode::Null(span),
                _ if is_strict_number(c) => {
                    if let Ok(n) = c.parse::<f64>() {
                        HirNode::Number(n, span)
                    } else {
                        HirNode::String(c.to_string(), span)
                    }
                }
                _ => HirNode::String(c.to_string(), span),
            }
        })
        .collect()
}

/// Splits CSS shorthand `tag#id.class1.class2` into `(tag, Option<id>, Vec<class>)` (Law 3.4).
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

    let mut mode = 't'; // 't' = tag, '#' = id, '.' = class
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

/// Resolves Left-Associative Algebraic Overlays (`&` deep merge and `/` key omission — Law 4.3)
/// across all Form scopes in the document, enforcing strict DAG acyclicity.
fn resolve_overlays_in_tree(node: &mut HirNode, diagnostics: &mut Vec<Diagnostic>) {
    match node {
        HirNode::Form { map, .. } => {
            // First recursively resolve children
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in &keys {
                if let Some(child) = map.get_mut(k) {
                    resolve_overlays_in_tree(child, diagnostics);
                }
            }

            // Detect cycles in `&` reference graph within this Form map
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
                            "Cyclic algebraic overlay `&` detected involving `{}` (Law 4.3 requires a strict DAG)",
                            k
                        ),
                        code: "RN004".to_string(),
                    });
                    continue;
                }

                // Evaluate left-to-right overlay chain: `((Prod & .Base) / debug) & .Overrides`
                let overlays_to_apply = match map.get(k) {
                    Some(HirNode::Form { overlays, .. }) if !overlays.is_empty() => {
                        overlays.clone()
                    }
                    _ => continue,
                };

                let own_node = map.get(k).unwrap().clone();
                let mut acc = HirNode::Form {
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
                        OverlayOp::Omit(omit_key) => {
                            if let HirNode::Form { map: acc_map, .. } = &mut acc {
                                remove_dotted_key(acc_map, &omit_key);
                            }
                        }
                    }
                }

                // Finally deep-merge the node's own explicit properties on top
                if let HirNode::Form {
                    map: own_map,
                    edges: own_edges,
                    span: own_span,
                    ..
                } = own_node
                {
                    deep_merge_nodes(
                        &mut acc,
                        HirNode::Form {
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
            if let OverlayOp::Merge(ref_path) = op {
                let target = ref_path.trim_start_matches('.');
                if has_overlay_cycle(target, map, visited) {
                    return true;
                }
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