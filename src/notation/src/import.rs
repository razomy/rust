//! Multi-Format Importers into Razomy Notation (`.rn`) v12.0 HIR.
//!
//! Converts legacy formats (JSON, JSON5, YAML, Protobuf `.proto`, Mermaid `.mmd`,
//! SQL `.sql`, HTML `.html`, Markdown `.md`, and CSV `.csv`) into `HirDocument`.

use indexmap::IndexMap;
use regex::Regex;
use serde_json::Value;

use crate::codegen::Format;
use crate::hir::*;
use crate::lexer::ContextMode;

/// Imports source text from the specified `Format` into an `HirDocument`.
pub fn import_from(source: &str, format: Format) -> Result<HirDocument, String> {
    match format {
        Format::Rn => Ok(crate::parser::parse_rn(source)),
        Format::Json => from_json(source),
        Format::Yaml => from_yaml(source),
        Format::Proto => from_proto(source),
        Format::Mermaid => from_mermaid(source),
        Format::Sql => from_sql(source),
        Format::Html => from_html(source),
        Format::Markdown => from_markdown(source),
        Format::Csv => from_csv(source),
    }
}

// ============================================================================
// 1. JSON, JSON5 & YAML Importers (Law 3.5 Canonical Bijection)
// ============================================================================

pub fn from_json(source: &str) -> Result<HirDocument, String> {
    let val: Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
    Ok(json_value_to_hir_doc(&val))
}

pub fn from_yaml(source: &str) -> Result<HirDocument, String> {
    let val: Value = serde_yaml::from_str(source).map_err(|e| e.to_string())?;
    Ok(json_value_to_hir_doc(&val))
}

pub fn json_value_to_hir_doc(val: &Value) -> HirDocument {
    let root_mode = match val {
        Value::Object(_) => ContextMode::Form,
        Value::Array(_) => ContextMode::Stream,
        _ => ContextMode::Stream,
    };
    let root = json_value_to_hir_node(val);
    HirDocument {
        root_mode,
        root,
        diagnostics: vec![],
        symbols: vec![],
    }
}

pub fn json_value_to_hir_node(val: &Value) -> HirNode {
    let span = Span::default();
    match val {
        Value::Null => HirNode::Null(span),
        Value::Bool(b) => HirNode::Bool(*b, span),
        Value::Number(n) => HirNode::Number(n.as_f64().unwrap_or(0.0), span),
        Value::String(s) => HirNode::String(s.clone(), span),
        Value::Array(arr) => {
            if arr.is_empty() {
                return HirNode::Tuple {
                    rows: vec![],
                    span,
                };
            }
            // Check if 2D matrix of primitives -> Tuple 2D Grid
            if arr.iter().all(|r| r.is_array()) {
                let rows: Vec<Vec<HirNode>> = arr
                    .iter()
                    .filter_map(|r| r.as_array())
                    .map(|r| r.iter().map(json_value_to_hir_node).collect())
                    .collect();
                return HirNode::Tuple { rows, span };
            }
            // Check if 1D array of primitives -> Tuple 1D or Stream
            if arr.iter().all(|x| !x.is_object() && !x.is_array()) {
                let row: Vec<HirNode> = arr.iter().map(json_value_to_hir_node).collect();
                return HirNode::Tuple {
                    rows: vec![row],
                    span,
                };
            }
            let items: Vec<HirNode> = arr.iter().map(json_value_to_hir_node).collect();
            HirNode::Stream { items, span }
        }
        Value::Object(obj) => {
            let mut map = IndexMap::new();
            let mut edges = Vec::new();

            for (k, v) in obj {
                if k == "->" {
                    if let Value::Array(edge_list) = v {
                        for e_val in edge_list {
                            if let Some(edge) = json_to_hir_edge(e_val) {
                                edges.push(edge);
                            }
                        }
                        continue;
                    }
                }
                let unescaped = unescape_reserved_key(k);
                map.insert(unescaped, json_value_to_hir_node(v));
            }
            HirNode::Form {
                map,
                edges,
                overlays: vec![],
                span,
            }
        }
    }
}

fn json_to_hir_edge(val: &Value) -> Option<HirEdge> {
    let arr = val.as_array()?;
    if arr.is_empty() {
        return None;
    }
    let mut endpoints = Vec::new();
    let mut metadata = None;

    for item in arr {
        match item {
            Value::String(s) => endpoints.push(HirEndpoint::Target(s.clone())),
            Value::Array(group) => {
                let nodes: Vec<HirNode> = group.iter().map(json_value_to_hir_node).collect();
                endpoints.push(HirEndpoint::Group(nodes));
            }
            Value::Object(meta_obj) => {
                let mut m = IndexMap::new();
                for (mk, mv) in meta_obj {
                    m.insert(mk.clone(), json_value_to_hir_node(mv));
                }
                metadata = Some(m);
            }
            _ => {}
        }
    }

    let arrow_count = endpoints.len().saturating_sub(1).max(1);
    let arrows = vec!["->".to_string(); arrow_count];
    Some(HirEdge {
        endpoints,
        arrows,
        metadata,
        span: Span::default(),
    })
}

// ============================================================================
// 2. Protocol Buffers (.proto) Importer
// ============================================================================

pub fn from_proto(source: &str) -> Result<HirDocument, String> {
    let msg_re = Regex::new(r"(?s)message\s+([A-Za-z0-9_]+)\s*\{([^}]*)\}").unwrap();
    let field_re =
        Regex::new(r"(?m)^\s*(repeated\s+)?([A-Za-z0-9_.]+)\s+([A-Za-z0-9_]+)\s*=\s*(\d+)").unwrap();

    let mut root_map = IndexMap::new();
    let span = Span::default();

    for cap in msg_re.captures_iter(source) {
        let msg_name = cap[1].to_string();
        let body = &cap[2];

        let mut rows = vec![vec![
            HirNode::String("col".to_string(), span),
            HirNode::String("type".to_string(), span),
            HirNode::String("tag".to_string(), span),
        ]];

        for fcap in field_re.captures_iter(body) {
            let is_repeated = fcap.get(1).is_some();
            let ftype = if is_repeated {
                format!("{}[]", &fcap[2])
            } else {
                fcap[2].to_string()
            };
            let fname = fcap[3].to_string();
            let ftag: f64 = fcap[4].parse().unwrap_or(1.0);

            rows.push(vec![
                HirNode::String(fname, span),
                HirNode::String(ftype, span),
                HirNode::Number(ftag, span),
            ]);
        }

        root_map.insert(msg_name, HirNode::Tuple { rows, span });
    }

    Ok(HirDocument {
        root_mode: ContextMode::Form,
        root: HirNode::Form {
            map: root_map,
            edges: vec![],
            overlays: vec![],
            span,
        },
        diagnostics: vec![],
        symbols: vec![],
    })
}

// ============================================================================
// 3. Mermaid Diagram Importer (`flowchart` / `graph` -> Hypergraph Edges `->`)
// ============================================================================

pub fn from_mermaid(source: &str) -> Result<HirDocument, String> {
    let span = Span::default();
    let mut edges = Vec::new();
    let edge_re = Regex::new(
        r#"([A-Za-z0-9_]+)(?:\[[^\]]*\])?\s*(<-->|-->|-\.->|==>)\s*(?:\|\"?([^|\"]+)\"?\|)?\s*([A-Za-z0-9_]+)"#,
    )
    .unwrap();

    for line in source.lines() {
        let t = line.trim();
        if t.is_empty()
            || t.starts_with("flowchart")
            || t.starts_with("graph")
            || t.starts_with("%%")
        {
            continue;
        }
        if let Some(cap) = edge_re.captures(t) {
            let src = cap[1].to_string();
            let raw_arrow = &cap[2];
            let arrow = if raw_arrow.starts_with('<') {
                "<->".to_string()
            } else {
                "->".to_string()
            };
            let dst = cap[4].to_string();
            let mut metadata = None;
            if let Some(lbl) = cap.get(3) {
                let mut m = IndexMap::new();
                for pair in lbl.as_str().split(',') {
                    if let Some((k, v)) = pair.split_once(':') {
                        m.insert(
                            k.trim().to_string(),
                            crate::parser::parse_form_value_expr(v.trim(), span),
                        );
                    } else {
                        m.insert(
                            "label".to_string(),
                            HirNode::String(pair.trim().to_string(), span),
                        );
                    }
                }
                metadata = Some(m);
            }

            edges.push(HirEdge {
                endpoints: vec![HirEndpoint::Target(src), HirEndpoint::Target(dst)],
                arrows: vec![arrow],
                metadata,
                span,
            });
        }
    }

    let mut root_map = IndexMap::new();
    root_map.insert(
        "graph".to_string(),
        HirNode::Form {
            map: IndexMap::new(),
            edges,
            overlays: vec![],
            span,
        },
    );

    Ok(HirDocument {
        root_mode: ContextMode::Form,
        root: HirNode::Form {
            map: root_map,
            edges: vec![],
            overlays: vec![],
            span,
        },
        diagnostics: vec![],
        symbols: vec![],
    })
}

// ============================================================================
// 4. SQL DDL (`CREATE TABLE`) Importer (Example 4 Bijection)
// ============================================================================

pub fn from_sql(source: &str) -> Result<HirDocument, String> {
    let span = Span::default();
    let table_re =
        Regex::new(r"(?is)CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([A-Za-z0-9_]+)\s*\((.*?)\)\s*;")
            .unwrap();
    let ref_re = Regex::new(r"(?i)REFERENCES\s+([A-Za-z0-9_]+)\s*\(\s*([A-Za-z0-9_]+)\s*\)").unwrap();
    let def_re = Regex::new(r"(?i)DEFAULT\s+([^\s,]+)").unwrap();

    let mut root_map = IndexMap::new();

    for cap in table_re.captures_iter(source) {
        let table_name = cap[1].to_string();
        let body = &cap[2];

        let mut rows = vec![vec![
            HirNode::String("col".to_string(), span),
            HirNode::String("type".to_string(), span),
            HirNode::String("ref".to_string(), span),
            HirNode::String("default".to_string(), span),
        ]];

        for raw_line in body.lines() {
            let line = raw_line.trim().trim_end_matches(',').trim();
            if line.is_empty() || line.to_ascii_uppercase().starts_with("PRIMARY KEY") {
                continue;
            }
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() < 2 {
                continue;
            }
            let col_name = tokens[0].to_string();
            let col_type = tokens[1].to_ascii_lowercase();

            let upper = line.to_ascii_uppercase();
            let ref_node = if let Some(rcap) = ref_re.captures(line) {
                let target = format!("{}.{}", &rcap[1], &rcap[2]);
                HirNode::Edge(HirEdge {
                    endpoints: vec![HirEndpoint::Target(target)],
                    arrows: vec!["->".to_string()],
                    metadata: None,
                    span,
                })
            } else if upper.contains("PRIMARY KEY") {
                HirNode::String("pk".to_string(), span)
            } else if upper.contains("NOT NULL") {
                HirNode::String("not_null".to_string(), span)
            } else {
                HirNode::Null(span)
            };

            let def_node = if let Some(dcap) = def_re.captures(line) {
                let raw_def = dcap[1].trim_matches('\'');
                if raw_def.eq_ignore_ascii_case("NULL") {
                    HirNode::Null(span)
                } else {
                    crate::parser::parse_form_value_expr(raw_def, span)
                }
            } else {
                HirNode::Null(span)
            };

            rows.push(vec![
                HirNode::String(col_name, span),
                HirNode::String(col_type, span),
                ref_node,
                def_node,
            ]);
        }

        root_map.insert(table_name, HirNode::Tuple { rows, span });
    }

    Ok(HirDocument {
        root_mode: ContextMode::Form,
        root: HirNode::Form {
            map: root_map,
            edges: vec![],
            overlays: vec![],
            span,
        },
        diagnostics: vec![],
        symbols: vec![],
    })
}

// ============================================================================
// 5. HTML DOM Importer (Example 2 Bijection)
// ============================================================================

pub fn from_html(source: &str) -> Result<HirDocument, String> {
    let items = parse_html_fragment(source.trim());
    Ok(HirDocument {
        root_mode: ContextMode::Stream,
        root: HirNode::Stream {
            items,
            span: Span::default(),
        },
        diagnostics: vec![],
        symbols: vec![],
    })
}

fn parse_html_fragment(html: &str) -> Vec<HirNode> {
    let span = Span::default();
    let tag_re =
        Regex::new(r"(?s)^\s*<([A-Za-z0-9]+)((?:\s+[^>]*)?)>(.*)</([A-Za-z0-9]+)>\s*$").unwrap();
    let attr_re = Regex::new(r#"([A-Za-z0-9_-]+)\s*=\s*"([^"]*)""#).unwrap();

    let mut results = Vec::new();
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return results;
    }

    if let Some(cap) = tag_re.captures(trimmed) {
        if cap[1] == cap[4] && !trimmed[1..].contains(&format!("<{}", &cap[1])) {
            let tag_name = cap[1].to_string();
            let attr_str = &cap[2];
            let inner_html = cap[3].trim();

            let mut id_opt = None;
            let mut classes = Vec::new();
            let mut props = IndexMap::new();

            for acap in attr_re.captures_iter(attr_str) {
                let k = &acap[1];
                let v = &acap[2];
                if k == "id" {
                    id_opt = Some(v.to_string());
                } else if k == "class" {
                    classes.extend(v.split_whitespace().map(|s| s.to_string()));
                } else {
                    props.insert(k.to_string(), HirNode::String(v.to_string(), span));
                }
            }

            let body_nodes = if inner_html.contains('<') && inner_html.trim_start().starts_with('<')
            {
                split_top_level_html_elements(inner_html)
                    .into_iter()
                    .flat_map(|el| parse_html_fragment(&el))
                    .collect()
            } else {
                vec![HirNode::RichText {
                    segments: html_inline_to_rn_segments(inner_html),
                    span,
                }]
            };

            results.push(HirNode::Triad {
                name: tag_name,
                id: id_opt,
                classes,
                params: None,
                overlays: vec![],
                props: if props.is_empty() { None } else { Some(props) },
                body: Some(body_nodes),
                span,
            });
            return results;
        }
    }

    let elements = split_top_level_html_elements(trimmed);
    if elements.len() > 1 {
        for el in elements {
            results.extend(parse_html_fragment(&el));
        }
    } else {
        results.push(HirNode::RichText {
            segments: html_inline_to_rn_segments(trimmed),
            span,
        });
    }
    results
}

fn split_top_level_html_elements(html: &str) -> Vec<String> {
    let mut elements = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '<' {
            if i + 1 < chars.len() && chars[i + 1] == '/' {
                depth -= 1;
            } else if i + 1 < chars.len() && chars[i + 1].is_ascii_alphabetic() {
                depth += 1;
            }
        }
        current.push(chars[i]);
        if chars[i] == '>' && depth <= 0 {
            if !current.trim().is_empty() {
                elements.push(current.trim().to_string());
            }
            current.clear();
            depth = 0;
        }
        i += 1;
    }
    if !current.trim().is_empty() {
        elements.push(current.trim().to_string());
    }
    elements
}

fn html_inline_to_rn_segments(s: &str) -> Vec<StreamSegment> {
    let inline_re =
        Regex::new(r#"(?s)<([A-Za-z0-9]+)((?:\s+[^>]*)?)>(.*?)</([A-Za-z0-9]+)>"#).unwrap();
    let attr_re = Regex::new(r#"([A-Za-z0-9_-]+)\s*=\s*"([^"]*)""#).unwrap();

    let converted = inline_re.replace_all(s, |caps: &regex::Captures| {
        let tag = &caps[1];
        let raw_attrs = &caps[2];
        let inner = &caps[3];
        let mut attr_pairs = Vec::new();
        for acap in attr_re.captures_iter(raw_attrs) {
            attr_pairs.push(format!("{} {}", &acap[1], &acap[2]));
        }
        if attr_pairs.is_empty() {
            format!("{}[{}]", tag, inner)
        } else {
            format!("{}{{{}}}[{}]", tag, attr_pairs.join(", "), inner)
        }
    });

    crate::parser::parse_stream_segments(&converted)
}

// ============================================================================
// 6. Markdown Importer (Example 1 Bijection)
// ============================================================================

pub fn from_markdown(source: &str) -> Result<HirDocument, String> {
    let span = Span::default();
    let lines: Vec<&str> = source.lines().collect();
    let mut items = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }

        // Code block ```lang
        if let Some(lang) = line.strip_prefix("```") {
            let lang_tag = lang.trim().to_string();
            i += 1;
            let mut code_lines = Vec::new();
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                code_lines.push(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            let mut m = IndexMap::new();
            m.insert(
                "snippet".to_string(),
                HirNode::Raw {
                    lang: lang_tag,
                    content: code_lines.join("\n"),
                    span,
                },
            );
            items.push(HirNode::Form {
                map: m,
                edges: vec![],
                overlays: vec![],
                span,
            });
            continue;
        }

        // Heading `# Title`
        if line.starts_with('#') {
            let hashes = line.chars().take_while(|c| *c == '#').count();
            let title = line[hashes..].trim();
            let mut m = IndexMap::new();
            m.insert(
                format!("h{}", hashes),
                HirNode::String(title.to_string(), span),
            );
            items.push(HirNode::Form {
                map: m,
                edges: vec![],
                overlays: vec![],
                span,
            });
            i += 1;
            continue;
        }

        // Unordered list `- item`
        if line.starts_with("- ") || line.starts_with("* ") {
            let mut list_items = Vec::new();
            while i < lines.len() {
                let l = lines[i].trim();
                if let Some(rest) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
                    list_items.push(HirNode::String(rest.trim().to_string(), span));
                    i += 1;
                } else {
                    break;
                }
            }
            let mut m = IndexMap::new();
            m.insert(
                "ul".to_string(),
                HirNode::Stream {
                    items: list_items,
                    span,
                },
            );
            items.push(HirNode::Form {
                map: m,
                edges: vec![],
                overlays: vec![],
                span,
            });
            continue;
        }

        // Markdown Table `| A | B |`
        if line.starts_with('|') && line.ends_with('|') {
            let mut rows = Vec::new();
            while i < lines.len() {
                let l = lines[i].trim();
                if l.starts_with('|') && l.ends_with('|') {
                    let inner = &l[1..l.len() - 1];
                    let cells: Vec<&str> = inner.split('|').map(|c| c.trim()).collect();
                    // Skip separator row `| --- | --- |`
                    if cells
                        .iter()
                        .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'))
                    {
                        i += 1;
                        continue;
                    }
                    let hir_cells = cells
                        .into_iter()
                        .map(|c| crate::parser::parse_form_value_expr(c, span))
                        .collect();
                    rows.push(hir_cells);
                    i += 1;
                } else {
                    break;
                }
            }
            let mut m = IndexMap::new();
            m.insert("table".to_string(), HirNode::Tuple { rows, span });
            items.push(HirNode::Form {
                map: m,
                edges: vec![],
                overlays: vec![],
                span,
            });
            continue;
        }

        // Paragraph with inline `**bold**` and `*italic*`
        let converted = convert_md_inline_to_rn(line);
        let mut m = IndexMap::new();
        m.insert(
            "p".to_string(),
            HirNode::RichText {
                segments: crate::parser::parse_stream_segments(&converted),
                span,
            },
        );
        items.push(HirNode::Form {
            map: m,
            edges: vec![],
            overlays: vec![],
            span,
        });
        i += 1;
    }

    Ok(HirDocument {
        root_mode: ContextMode::Stream,
        root: HirNode::Stream { items, span },
        diagnostics: vec![],
        symbols: vec![],
    })
}

fn convert_md_inline_to_rn(s: &str) -> String {
    let bold_re = Regex::new(r"\*\*([^*]+)\*\*").unwrap();
    let italic_re = Regex::new(r"\*([^*]+)\*").unwrap();
    let link_re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();

    let s1 = bold_re.replace_all(s, "b[$1]");
    let s2 = italic_re.replace_all(&s1, "i[$1]");
    let s3 = link_re.replace_all(&s2, "a{href $2}[$1]");
    s3.into_owned()
}

// ============================================================================
// 7. CSV Importer (Tuple 2D Grid)
// ============================================================================

pub fn from_csv(source: &str) -> Result<HirDocument, String> {
    let span = Span::default();
    let mut rows = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let cells = crate::parser::parse_tuple_cells(line, span);
        rows.push(cells);
    }

    Ok(HirDocument {
        root_mode: ContextMode::Tuple,
        root: HirNode::Tuple { rows, span },
        diagnostics: vec![],
        symbols: vec![],
    })
}