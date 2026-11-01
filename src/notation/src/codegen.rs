//! Multi-Format Code Generator for Razomy Notation (`.rn`) v12.0.
//!
//! Lowers `HirDocument` into:
//! - Canonical `.rn` (1-space topological formatter)
//! - Canonical `.json` (Lossless Bijection — Law 3.5)
//! - `.json5` (Unquoted keys, trailing commas)
//! - `.yaml` (YAML 1.2)
//! - `.proto` (Protocol Buffers v3 schema inference & generation)
//! - `.mmd` / `.mermaid` (Mermaid Flowcharts & ER Diagrams from Hypergraphs `->` & Tables)
//! - `.sql` (SQL DDL `CREATE TABLE` + `INSERT INTO` from Tuple grids & `->` Foreign Keys)
//! - `.html` (HTML5 DOM from Stream `[` + Form `{` attributes + inline tags)
//! - `.md` (Markdown from Stream `[`, Tuple tables `(`, and Raw code blocks `"lang`)
//! - `.csv` (RFC 4180 CSV from Tuple 2D grids)

use indexmap::IndexMap;
use serde_json::Value;

use crate::hir::*;

/// Supported target formats for codegen and import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Rn,
    Json,
    Yaml,
    Proto,
    Mermaid,
    Sql,
    Html,
    Markdown,
    Csv,
}

impl Format {
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "rn" | "razomy" => Some(Format::Rn),
            "json" => Some(Format::Json),
            "yaml" | "yml" => Some(Format::Yaml),
            "proto" | "protobuf" => Some(Format::Proto),
            "mermaid" | "mmd" => Some(Format::Mermaid),
            "sql" => Some(Format::Sql),
            "html" | "htm" => Some(Format::Html),
            "md" | "markdown" => Some(Format::Markdown),
            "csv" => Some(Format::Csv),
            _ => None,
        }
    }
}

/// Generates output in the requested target `Format` from an `HirDocument`.
pub fn generate(doc: &HirDocument, format: Format) -> Result<String, String> {
    match format {
        Format::Rn => Ok(to_rn(doc)),
        Format::Json => to_json(doc, true),
        Format::Yaml => to_yaml(doc),
        Format::Proto => Ok(to_proto(doc)),
        Format::Mermaid => Ok(to_mermaid(doc)),
        Format::Sql => Ok(to_sql(doc)),
        Format::Html => Ok(to_html(doc)),
        Format::Markdown => Ok(to_markdown(doc)),
        Format::Csv => Ok(to_csv(doc)),
    }
}

// ============================================================================
// 1. Canonical Razomy Notation (.rn) Formatter (1-Space Indent Topology)
// ============================================================================

pub fn to_rn(doc: &HirDocument) -> String {
    let mut out = String::new();
    match &doc.root {
        HirNode::Form { map, edges, .. } => {
            for (k, v) in map {
                format_rn_form_entry(k, v, 0, &mut out);
            }
            for e in edges {
                out.push_str(&format_rn_edge(e));
                out.push('\n');
            }
        }
        HirNode::Stream { items, .. } => {
            for item in items {
                format_rn_stream_item(item, 0, &mut out);
            }
        }
        HirNode::Tuple { rows, .. } => {
            out.push_str("(\n");
            format_rn_tuple_rows(rows, 1, &mut out);
        }
        other => {
            format_rn_stream_item(other, 0, &mut out);
        }
    }
    out.trim_end().to_string() + "\n"
}

fn format_rn_form_entry(key: &str, val: &HirNode, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    let formatted_key = format_rn_key(key);

    match val {
        HirNode::Null(_) => {
            out.push_str(&format!("{}{}\n", pad, formatted_key));
        }
        HirNode::Bool(b, _) => {
            out.push_str(&format!("{}{} {}\n", pad, formatted_key, b));
        }
        HirNode::Number(n, _) => {
            out.push_str(&format!(
                "{}{} {}\n",
                pad,
                formatted_key,
                format_number(*n)
            ));
        }
        HirNode::String(s, _) => {
            if s.is_empty() {
                out.push_str(&format!("{}{} \"\"\n", pad, formatted_key));
            } else if s.contains('\n') {
                out.push_str(&format!("{}{} \"\n", pad, formatted_key));
                let child_pad = " ".repeat(indent + 1);
                for line in s.lines() {
                    out.push_str(&format!("{}{}\n", child_pad, line));
                }
            } else if needs_quotes_in_form_value(s) {
                out.push_str(&format!(
                    "{}{} \"{}\"\n",
                    pad,
                    formatted_key,
                    escape_quotes(s)
                ));
            } else {
                out.push_str(&format!("{}{} {}\n", pad, formatted_key, s));
            }
        }
        HirNode::Raw { lang, content, .. } => {
            out.push_str(&format!("{}{} \"{}\n", pad, formatted_key, lang));
            let child_pad = " ".repeat(indent + 1);
            for line in content.lines() {
                out.push_str(&format!("{}{}\n", child_pad, line));
            }
        }
        HirNode::Reference { path, .. } => {
            out.push_str(&format!("{}{} {}\n", pad, formatted_key, path));
        }
        HirNode::Edge(edge) => {
            out.push_str(&format!(
                "{}{} {}\n",
                pad,
                formatted_key,
                format_rn_edge(edge)
            ));
        }
        HirNode::Form { map, edges, .. } => {
            if map.is_empty() && edges.is_empty() {
                out.push_str(&format!("{}{} {{}}\n", pad, formatted_key));
            } else {
                // Law 2.1: Context Inertia inside Form (explicit `{` at root, inherited below)
                if indent == 0 {
                    out.push_str(&format!("{}{} {{\n", pad, formatted_key));
                } else {
                    out.push_str(&format!("{}{}\n", pad, formatted_key));
                }
                for (ck, cv) in map {
                    format_rn_form_entry(ck, cv, indent + 1, out);
                }
                let child_pad = " ".repeat(indent + 1);
                for e in edges {
                    out.push_str(&format!("{}{}\n", child_pad, format_rn_edge(e)));
                }
            }
        }
        HirNode::Tuple { rows, .. } => {
            if rows.is_empty() {
                out.push_str(&format!("{}{} ()\n", pad, formatted_key));
            } else if rows.len() == 1 && rows[0].len() <= 5 {
                let cells: Vec<String> = rows[0].iter().map(format_rn_cell).collect();
                out.push_str(&format!(
                    "{}{} ({})\n",
                    pad,
                    formatted_key,
                    cells.join(", ")
                ));
            } else {
                out.push_str(&format!("{}{} (\n", pad, formatted_key));
                format_rn_tuple_rows(rows, indent + 1, out);
            }
        }
        HirNode::Stream { items, .. } => {
            if items.is_empty() {
                out.push_str(&format!("{}{} []\n", pad, formatted_key));
            } else {
                out.push_str(&format!("{}{} [\n", pad, formatted_key));
                for item in items {
                    format_rn_stream_item(item, indent + 1, out);
                }
            }
        }
        HirNode::RichText { segments, .. } => {
            out.push_str(&format!(
                "{}{} {}\n",
                pad,
                formatted_key,
                render_segments_plain(segments)
            ));
        }
        HirNode::Triad { .. } => {
            format_rn_stream_item(val, indent, out);
        }
        HirNode::Comment(c, _) => {
            out.push_str(&format!("{}# {}\n", pad, c));
        }
    }
}

fn format_rn_stream_item(item: &HirNode, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    match item {
        HirNode::String(s, _) => {
            out.push_str(&format!("{}{}\n", pad, s));
        }
        HirNode::RichText { segments, .. } => {
            out.push_str(&format!("{}{}\n", pad, render_segments_plain(segments)));
        }
        HirNode::Form { map, .. } if map.len() == 1 => {
            let (k, v) = map.iter().next().unwrap();
            match v {
                HirNode::String(s, _) => {
                    out.push_str(&format!("{}{} [ {} ]\n", pad, k, s));
                }
                HirNode::RichText { segments, .. } => {
                    out.push_str(&format!(
                        "{}{} [ {} ]\n",
                        pad,
                        k,
                        render_segments_plain(segments)
                    ));
                }
                HirNode::Stream { items, .. } => {
                    out.push_str(&format!("{}{} [\n", pad, k));
                    for child in items {
                        format_rn_stream_item(child, indent + 1, out);
                    }
                }
                HirNode::Tuple { rows, .. } => {
                    out.push_str(&format!("{}{} (\n", pad, k));
                    format_rn_tuple_rows(rows, indent + 1, out);
                }
                HirNode::Raw { lang, content, .. } => {
                    out.push_str(&format!("{}{} \"{}\n", pad, k, lang));
                    let child_pad = " ".repeat(indent + 1);
                    for l in content.lines() {
                        out.push_str(&format!("{}{}\n", child_pad, l));
                    }
                }
                _ => {
                    format_rn_form_entry(k, v, indent, out);
                }
            }
        }
        HirNode::Triad {
            name,
            id,
            classes,
            props,
            body,
            ..
        } => {
            let mut header = name.clone();
            if let Some(i) = id {
                header.push('#');
                header.push_str(i);
            }
            for c in classes {
                header.push('.');
                header.push_str(c);
            }
            if let Some(p) = props {
                if !p.is_empty() {
                    let pairs: Vec<String> = p
                        .iter()
                        .map(|(pk, pv)| format!("{} {}", format_rn_key(pk), format_rn_cell(pv)))
                        .collect();
                    header.push_str(&format!(" {{ {} }}", pairs.join(", ")));
                }
            }
            if let Some(b) = body {
                if b.len() == 1 && matches!(b[0], HirNode::String(_, _) | HirNode::RichText { .. })
                {
                    let text = match &b[0] {
                        HirNode::String(s, _) => s.clone(),
                        HirNode::RichText { segments, .. } => render_segments_plain(segments),
                        _ => String::new(),
                    };
                    out.push_str(&format!("{}{} [ {} ]\n", pad, header, text));
                } else {
                    out.push_str(&format!("{}{} [\n", pad, header));
                    for child in b {
                        format_rn_stream_item(child, indent + 1, out);
                    }
                }
            } else {
                out.push_str(&format!("{}{}\n", pad, header));
            }
        }
        other => {
            format_rn_form_entry("item", other, indent, out);
        }
    }
}

fn format_rn_tuple_rows(rows: &[Vec<HirNode>], indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    for row in rows {
        let cells: Vec<String> = row.iter().map(format_rn_cell).collect();
        out.push_str(&format!("{}{}\n", pad, cells.join(", ")));
    }
}

fn format_rn_cell(node: &HirNode) -> String {
    match node {
        HirNode::Null(_) => "null".to_string(),
        HirNode::Bool(b, _) => b.to_string(),
        HirNode::Number(n, _) => format_number(*n),
        HirNode::String(s, _) => {
            if s.contains(',') || s.contains(';') || s.contains('"') {
                format!("\"{}\"", escape_quotes(s))
            } else {
                s.clone()
            }
        }
        HirNode::Edge(e) => format_rn_edge(e),
        HirNode::Reference { path, .. } => path.clone(),
        other => other.to_canonical_json().to_string(),
    }
}

fn format_rn_edge(edge: &HirEdge) -> String {
    let mut out = String::new();
    if edge.endpoints.len() == 1 && edge.arrows.len() == 1 {
        out.push_str(&edge.arrows[0]);
        out.push(' ');
        out.push_str(&format_rn_endpoint(&edge.endpoints[0]));
    } else {
        for (i, ep) in edge.endpoints.iter().enumerate() {
            if i > 0 {
                let arrow = edge.arrows.get(i - 1).map(|s| s.as_str()).unwrap_or("->");
                out.push(' ');
                out.push_str(arrow);
                out.push(' ');
            }
            out.push_str(&format_rn_endpoint(ep));
        }
    }
    if let Some(meta) = &edge.metadata {
        if !meta.is_empty() {
            let pairs: Vec<String> = meta
                .iter()
                .map(|(k, v)| format!("{} {}", format_rn_key(k), format_rn_cell(v)))
                .collect();
            out.push_str(&format!(" {{ {} }}", pairs.join(", ")));
        }
    }
    out
}

fn format_rn_endpoint(ep: &HirEndpoint) -> String {
    match ep {
        HirEndpoint::Target(s) => s.clone(),
        HirEndpoint::Group(g) => {
            let items: Vec<String> = g.iter().map(format_rn_cell).collect();
            format!("({})", items.join(", "))
        }
    }
}

fn format_rn_key(k: &str) -> String {
    if k.is_empty()
        || k.contains(' ')
        || k.contains(',')
        || k.contains(';')
        || k.contains('{')
        || k.contains('}')
        || k.contains('[')
        || k.contains(']')
        || k.contains('(')
        || k.contains(')')
    {
        format!("\"{}\"", escape_quotes(k))
    } else {
        k.to_string()
    }
}

fn needs_quotes_in_form_value(s: &str) -> bool {
    s == "true"
        || s == "false"
        || s == "null"
        || s == "\"\""
        || s == "{}"
        || s == "()"
        || s == "[]"
        || crate::parser::is_strict_number(s)
        || s.contains(',')
        || s.contains(';')
}

// ============================================================================
// 2. JSON & JSON5 Codegen
// ============================================================================

pub fn to_json(doc: &HirDocument, pretty: bool) -> Result<String, String> {
    let val = doc.root.to_canonical_json();
    if pretty {
        serde_json::to_string_pretty(&val).map_err(|e| e.to_string())
    } else {
        serde_json::to_string(&val).map_err(|e| e.to_string())
    }
}

pub fn to_json5(doc: &HirDocument) -> String {
    let val = doc.root.to_canonical_json();
    format_json5_value(&val, 0)
}

fn format_json5_value(val: &Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let child_pad = "  ".repeat(indent + 1);
    match val {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("\"{}\"", escape_quotes(s)),
        Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else if arr.iter().all(|v| !v.is_object() && !v.is_array()) && arr.len() <= 6 {
                let items: Vec<String> = arr.iter().map(|v| format_json5_value(v, 0)).collect();
                format!("[{}]", items.join(", "))
            } else {
                let mut out = String::from("[\n");
                for item in arr {
                    out.push_str(&format!(
                        "{}{},\n",
                        child_pad,
                        format_json5_value(item, indent + 1)
                    ));
                }
                out.push_str(&format!("{}]", pad));
                out
            }
        }
        Value::Object(map) => {
            if map.is_empty() {
                "{}".to_string()
            } else {
                let mut out = String::from("{\n");
                for (k, v) in map {
                    let key_str = if is_valid_js_identifier(k) {
                        k.clone()
                    } else {
                        format!("\"{}\"", escape_quotes(k))
                    };
                    out.push_str(&format!(
                        "{}{}: {},\n",
                        child_pad,
                        key_str,
                        format_json5_value(v, indent + 1)
                    ));
                }
                out.push_str(&format!("{}}}", pad));
                out
            }
        }
    }
}

fn is_valid_js_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

// ============================================================================
// 3. YAML Codegen
// ============================================================================

pub fn to_yaml(doc: &HirDocument) -> Result<String, String> {
    let val = doc.root.to_canonical_json();
    serde_yaml::to_string(&val).map_err(|e| e.to_string())
}

// ============================================================================
// 4. Protocol Buffers v3 (.proto) Codegen
// ============================================================================

pub fn to_proto(doc: &HirDocument) -> String {
    let mut out = String::from("syntax = \"proto3\";\n\npackage razomy;\n\n");
    let mut messages = Vec::new();

    match &doc.root {
        HirNode::Form { map, .. } => {
            // Check if top-level keys are message definitions or fields of a Root message
            let all_composite = !map.is_empty()
                && map
                    .values()
                    .all(|v| matches!(v, HirNode::Form { .. } | HirNode::Tuple { .. }));
            if all_composite {
                for (name, node) in map {
                    collect_proto_messages(&to_pascal_case(name), node, &mut messages);
                }
            } else {
                collect_proto_messages("RootDocument", &doc.root, &mut messages);
            }
        }
        other => {
            collect_proto_messages("RootDocument", other, &mut messages);
        }
    }

    out.push_str(&messages.join("\n\n"));
    out.push('\n');
    out
}

fn collect_proto_messages(msg_name: &str, node: &HirNode, out_msgs: &mut Vec<String>) {
    match node {
        HirNode::Form { map, .. } => {
            let mut fields = Vec::new();
            let mut tag_num = 1;
            for (k, v) in map {
                if k == "()" || k == "[]" || k == "->" {
                    continue;
                }
                let field_name = to_snake_case(k);
                let proto_type = match v {
                    HirNode::Bool(_, _) => "bool".to_string(),
                    HirNode::Number(n, _) => {
                        if n.fract() == 0.0 {
                            "int64".to_string()
                        } else {
                            "double".to_string()
                        }
                    }
                    HirNode::String(s, _) => map_sql_or_proto_type_hint(s),
                    HirNode::Form { .. } => {
                        let sub_msg = to_pascal_case(k);
                        collect_proto_messages(&sub_msg, v, out_msgs);
                        sub_msg
                    }
                    HirNode::Tuple { rows, .. } => {
                        let elem_type = rows
                            .first()
                            .and_then(|r| r.first())
                            .map(|cell| match cell {
                                HirNode::Number(n, _) if n.fract() == 0.0 => "int64",
                                HirNode::Number(_, _) => "double",
                                HirNode::Bool(_, _) => "bool",
                                _ => "string",
                            })
                            .unwrap_or("string");
                        format!("repeated {}", elem_type)
                    }
                    HirNode::Stream { .. } => "repeated string".to_string(),
                    _ => "string".to_string(),
                };
                fields.push(format!("  {} {} = {};", proto_type, field_name, tag_num));
                tag_num += 1;
            }
            out_msgs.push(format!(
                "message {} {{\n{}\n}}",
                msg_name,
                fields.join("\n")
            ));
        }
        HirNode::Tuple { rows, .. } => {
            // If Tuple is a 2D schema table like `Users ( col, type, ... )`
            let mut fields = Vec::new();
            if rows.len() >= 2 {
                let header: Vec<String> = rows[0]
                    .iter()
                    .map(|c| format_rn_cell(c).to_ascii_lowercase())
                    .collect();
                let col_idx = header.iter().position(|h| h == "col" || h == "name" || h == "field").unwrap_or(0);
                let type_idx = header.iter().position(|h| h == "type").unwrap_or(1);

                for (idx, row) in rows.iter().skip(1).enumerate() {
                    let col_name = row
                        .get(col_idx)
                        .map(|c| to_snake_case(&format_rn_cell(c)))
                        .unwrap_or_else(|| format!("field_{}", idx + 1));
                    let raw_type = row
                        .get(type_idx)
                        .map(format_rn_cell)
                        .unwrap_or_else(|| "string".to_string());
                    let proto_type = sql_type_to_proto(&raw_type);
                    fields.push(format!("  {} {} = {};", proto_type, col_name, idx + 1));
                }
            }
            out_msgs.push(format!(
                "message {} {{\n{}\n}}",
                msg_name,
                fields.join("\n")
            ));
        }
        _ => {}
    }
}

fn map_sql_or_proto_type_hint(s: &str) -> String {
    match s.trim().to_ascii_lowercase().as_str() {
        "int32" | "int64" | "uint32" | "uint64" | "sint32" | "sint64" | "bool" | "string"
        | "bytes" | "float" | "double" => s.trim().to_ascii_lowercase(),
        "int" | "integer" => "int64".to_string(),
        "text" | "uuid" | "varchar" => "string".to_string(),
        _ => "string".to_string(),
    }
}

fn sql_type_to_proto(t: &str) -> &'static str {
    match t.trim().to_ascii_lowercase().as_str() {
        "int" | "integer" | "bigint" | "int64" | "serial" => "int64",
        "smallint" | "int32" => "int32",
        "float" | "real" | "double" | "numeric" | "decimal" => "double",
        "bool" | "boolean" => "bool",
        "bytea" | "blob" | "bytes" => "bytes",
        _ => "string",
    }
}

// ============================================================================
// 5. Mermaid Diagram Codegen (Graph Flowchart & ER Diagrams from `->` & Tables)
// ============================================================================

pub fn to_mermaid(doc: &HirDocument) -> String {
    let mut out = String::from("flowchart TD\n");
    let mut lines = Vec::new();
    collect_mermaid_edges("root", &doc.root, &mut lines);
    if lines.is_empty() {
        out.push_str("  Empty[No Nodes]\n");
    } else {
        for l in lines {
            out.push_str(&format!("  {}\n", l));
        }
    }
    out
}

fn collect_mermaid_edges(parent: &str, node: &HirNode, lines: &mut Vec<String>) {
    match node {
        HirNode::Edge(edge) => {
            emit_mermaid_hir_edge(edge, lines);
        }
        HirNode::Form { map, edges, .. } => {
            for e in edges {
                emit_mermaid_hir_edge(e, lines);
            }
            for (k, v) in map {
                match v {
                    HirNode::Edge(e) => {
                        if e.endpoints.len() == 1 {
                            let target = format_rn_endpoint(&e.endpoints[0]);
                            lines.push(format!(
                                "{} -->|{}| {}",
                                sanitize_mermaid_id(parent),
                                k,
                                sanitize_mermaid_id(&target)
                            ));
                        } else {
                            emit_mermaid_hir_edge(e, lines);
                        }
                    }
                    HirNode::Form { .. } => {
                        if parent != "root" {
                            lines.push(format!(
                                "{} --> {}",
                                sanitize_mermaid_id(parent),
                                sanitize_mermaid_id(k)
                            ));
                        }
                        collect_mermaid_edges(k, v, lines);
                    }
                    HirNode::Tuple { rows, .. } => {
                        // Check if any cell in the Tuple is an Edge (`-> Orgs.id` or `Ingest -> Transform -> Load`)
                        for row in rows {
                            for cell in row {
                                if let HirNode::Edge(e) = cell {
                                    if e.endpoints.len() == 1 {
                                        let target = format_rn_endpoint(&e.endpoints[0]);
                                        let target_table =
                                            target.split('.').next().unwrap_or(&target);
                                        lines.push(format!(
                                            "{} --> {}",
                                            sanitize_mermaid_id(k),
                                            sanitize_mermaid_id(target_table)
                                        ));
                                    } else {
                                        emit_mermaid_hir_edge(e, lines);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        HirNode::Tuple { rows, .. } => {
            for row in rows {
                for cell in row {
                    if let HirNode::Edge(e) = cell {
                        emit_mermaid_hir_edge(e, lines);
                    }
                }
            }
        }
        HirNode::Stream { items, .. } => {
            for item in items {
                collect_mermaid_edges(parent, item, lines);
            }
        }
        _ => {}
    }
}

fn emit_mermaid_hir_edge(edge: &HirEdge, lines: &mut Vec<String>) {
    let label = edge
        .metadata
        .as_ref()
        .map(|m| {
            let kv: Vec<String> = m
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_rn_cell(v)))
                .collect();
            kv.join(", ")
        })
        .unwrap_or_default();

    for i in 0..edge.arrows.len() {
        let Some(src) = edge.endpoints.get(i) else {
            continue;
        };
        let Some(dst) = edge.endpoints.get(i + 1) else {
            continue;
        };
        let arrow = if edge.arrows[i] == "<->" {
            "<-->"
        } else {
            "-->"
        };

        let src_ids = expand_endpoint_ids(src);
        let dst_ids = expand_endpoint_ids(dst);

        for s in &src_ids {
            for d in &dst_ids {
                if label.is_empty() {
                    lines.push(format!("{} {} {}", s, arrow, d));
                } else {
                    lines.push(format!("{} {}|\"{}\"| {}", s, arrow, label, d));
                }
            }
        }
    }
}

fn expand_endpoint_ids(ep: &HirEndpoint) -> Vec<String> {
    match ep {
        HirEndpoint::Target(s) => vec![sanitize_mermaid_id(s)],
        HirEndpoint::Group(g) => g
            .iter()
            .map(|n| sanitize_mermaid_id(&format_rn_cell(n)))
            .collect(),
    }
}

fn sanitize_mermaid_id(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "Node".to_string()
    } else {
        cleaned
    }
}

// ============================================================================
// 6. SQL DDL & DML Codegen (Example 4: Tuple Grid `(` & Edges `->`)
// ============================================================================

pub fn to_sql(doc: &HirDocument) -> String {
    let mut statements = Vec::new();
    match &doc.root {
        HirNode::Form { map, .. } => {
            for (table_name, node) in map {
                if let HirNode::Tuple { rows, .. } = node {
                    statements.push(tuple_to_sql_table(table_name, rows));
                }
            }
        }
        HirNode::Tuple { rows, .. } => {
            statements.push(tuple_to_sql_table("Data", rows));
        }
        _ => {}
    }
    statements.join("\n\n") + "\n"
}

fn tuple_to_sql_table(table_name: &str, rows: &[Vec<HirNode>]) -> String {
    if rows.is_empty() {
        return format!("CREATE TABLE {} ();", table_name);
    }

    let header: Vec<String> = rows[0]
        .iter()
        .map(|c| format_rn_cell(c).trim().to_ascii_lowercase())
        .collect();

    // Check if this is a DDL Schema table (`col, type, ref, default` — Example 4)
    let is_ddl_schema = header.contains(&"col".to_string()) && header.contains(&"type".to_string());

    if is_ddl_schema {
        let col_idx = header.iter().position(|h| h == "col").unwrap_or(0);
        let type_idx = header.iter().position(|h| h == "type").unwrap_or(1);
        let ref_idx = header.iter().position(|h| h == "ref");
        let def_idx = header.iter().position(|h| h == "default");

        let mut col_defs = Vec::new();
        for row in rows.iter().skip(1) {
            let col_name = row.get(col_idx).map(format_rn_cell).unwrap_or_default();
            let col_type = row
                .get(type_idx)
                .map(|c| format_rn_cell(c).to_ascii_uppercase())
                .unwrap_or_else(|| "TEXT".to_string());

            let mut parts = vec![format!("  {} {}", col_name, col_type)];

            if let Some(ri) = ref_idx {
                if let Some(ref_cell) = row.get(ri) {
                    match ref_cell {
                        HirNode::Edge(edge) if !edge.endpoints.is_empty() => {
                            let target = format_rn_endpoint(&edge.endpoints[0]);
                            if let Some((ref_table, ref_col)) = target.split_once('.') {
                                parts.push(format!("REFERENCES {}({})", ref_table, ref_col));
                            } else {
                                parts.push(format!("REFERENCES {}", target));
                            }
                        }
                        _ => {
                            let ref_str = format_rn_cell(ref_cell);
                            match ref_str.to_ascii_lowercase().as_str() {
                                "pk" | "primary_key" => parts.push("PRIMARY KEY".to_string()),
                                "not_null" | "notnull" => parts.push("NOT NULL".to_string()),
                                "unique" => parts.push("UNIQUE".to_string()),
                                "null" | "" => {}
                                other => parts.push(other.to_ascii_uppercase()),
                            }
                        }
                    }
                }
            }

            if let Some(di) = def_idx {
                if let Some(def_cell) = row.get(di) {
                    match def_cell {
                        HirNode::Null(_) => parts.push("DEFAULT NULL".to_string()),
                        HirNode::Bool(b, _) => {
                            parts.push(format!("DEFAULT {}", b.to_string().to_ascii_uppercase()))
                        }
                        HirNode::Number(n, _) => {
                            parts.push(format!("DEFAULT {}", format_number(*n)))
                        }
                        HirNode::String(s, _) if !s.is_empty() => {
                            if s.ends_with("()") {
                                // Function call default like `gen_uuid()` (Law 1.2)
                                parts.push(format!("DEFAULT {}", s));
                            } else {
                                parts.push(format!("DEFAULT '{}'", s.replace('\'', "''")));
                            }
                        }
                        _ => {}
                    }
                }
            }

            col_defs.push(parts.join(" "));
        }

        format!(
            "CREATE TABLE {} (\n{}\n);",
            table_name,
            col_defs.join(",\n")
        )
    } else {
        // Relational data table: emit CREATE TABLE + INSERT INTO rows
        let cols: Vec<String> = rows[0].iter().map(format_rn_cell).collect();
        let col_defs: Vec<String> = cols.iter().map(|c| format!("  {} TEXT", c)).collect();
        let mut sql = format!(
            "CREATE TABLE {} (\n{}\n);",
            table_name,
            col_defs.join(",\n")
        );

        for row in rows.iter().skip(1) {
            let vals: Vec<String> = row
                .iter()
                .map(|cell| match cell {
                    HirNode::Null(_) => "NULL".to_string(),
                    HirNode::Bool(b, _) => b.to_string().to_ascii_uppercase(),
                    HirNode::Number(n, _) => format_number(*n),
                    other => format!("'{}'", format_rn_cell(other).replace('\'', "''")),
                })
                .collect();
            sql.push_str(&format!(
                "\nINSERT INTO {} ({}) VALUES ({});",
                table_name,
                cols.join(", "),
                vals.join(", ")
            ));
        }
        sql
    }
}

// ============================================================================
// 7. HTML5 DOM Codegen (Example 2: Stream `[` + Form `{` + Inline Tags)
// ============================================================================

pub fn to_html(doc: &HirDocument) -> String {
    let mut out = String::new();
    render_html_node(&doc.root, 0, &mut out);
    out.trim_end().to_string() + "\n"
}

fn render_html_node(node: &HirNode, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match node {
        HirNode::String(s, _) => {
            out.push_str(&format!("{}{}\n", pad, escape_html(s)));
        }
        HirNode::RichText { segments, .. } => {
            out.push_str(&format!("{}{}\n", pad, render_html_segments(segments)));
        }
        HirNode::Stream { items, .. } => {
            for item in items {
                render_html_node(item, indent, out);
            }
        }
        HirNode::Triad {
            name,
            id,
            classes,
            props,
            body,
            ..
        } => {
            let attr_str = build_html_attrs(id.as_deref(), classes, props.as_ref());
            if let Some(b) = body {
                if b.len() == 1 && matches!(b[0], HirNode::String(_, _) | HirNode::RichText { .. })
                {
                    let inner = match &b[0] {
                        HirNode::String(s, _) => escape_html(s),
                        HirNode::RichText { segments, .. } => render_html_segments(segments),
                        _ => String::new(),
                    };
                    out.push_str(&format!("{}<{}{}>{}</{}>\n", pad, name, attr_str, inner, name));
                } else {
                    out.push_str(&format!("{}<{}{}>\n", pad, name, attr_str));
                    for child in b {
                        render_html_node(child, indent + 1, out);
                    }
                    out.push_str(&format!("{}</{}>\n", pad, name));
                }
            } else {
                out.push_str(&format!("{}<{}{} />\n", pad, name, attr_str));
            }
        }
        HirNode::Form { map, .. } => {
            for (k, v) in map {
                let (clean_tag, id_opt, classes) = crate::parser::split_css_shorthand(k);
                let attr_str = build_html_attrs(id_opt.as_deref(), &classes, None);
                match v {
                    HirNode::String(s, _) => {
                        out.push_str(&format!(
                            "{}<{}{}>{}</{}>\n",
                            pad,
                            clean_tag,
                            attr_str,
                            escape_html(s),
                            clean_tag
                        ));
                    }
                    HirNode::RichText { segments, .. } => {
                        out.push_str(&format!(
                            "{}<{}{}>{}</{}>\n",
                            pad,
                            clean_tag,
                            attr_str,
                            render_html_segments(segments),
                            clean_tag
                        ));
                    }
                    HirNode::Stream { items, .. } => {
                        out.push_str(&format!("{}<{}{}>\n", pad, clean_tag, attr_str));
                        for item in items {
                            if clean_tag == "ul" || clean_tag == "ol" {
                                match item {
                                    HirNode::String(s, _) => {
                                        out.push_str(&format!(
                                            "{}  <li>{}</li>\n",
                                            pad,
                                            escape_html(s)
                                        ));
                                    }
                                    HirNode::RichText { segments, .. } => {
                                        out.push_str(&format!(
                                            "{}  <li>{}</li>\n",
                                            pad,
                                            render_html_segments(segments)
                                        ));
                                    }
                                    other => render_html_node(other, indent + 1, out),
                                }
                            } else {
                                render_html_node(item, indent + 1, out);
                            }
                        }
                        out.push_str(&format!("{}</{}>\n", pad, clean_tag));
                    }
                    HirNode::Tuple { rows, .. } => {
                        out.push_str(&format!("{}<table{}>\n", pad, attr_str));
                        if let Some(header) = rows.first() {
                            out.push_str(&format!("{}  <thead><tr>", pad));
                            for h in header {
                                out.push_str(&format!(
                                    "<th>{}</th>",
                                    escape_html(&format_rn_cell(h))
                                ));
                            }
                            out.push_str("</tr></thead>\n");
                        }
                        if rows.len() > 1 {
                            out.push_str(&format!("{}  <tbody>\n", pad));
                            for row in rows.iter().skip(1) {
                                out.push_str(&format!("{}    <tr>", pad));
                                for c in row {
                                    out.push_str(&format!(
                                        "<td>{}</td>",
                                        escape_html(&format_rn_cell(c))
                                    ));
                                }
                                out.push_str("</tr>\n");
                            }
                            out.push_str(&format!("{}  </tbody>\n", pad));
                        }
                        out.push_str(&format!("{}</table>\n", pad));
                    }
                    HirNode::Raw { lang, content, .. } => {
                        out.push_str(&format!(
                            "{}<pre><code class=\"language-{}\">{}</code></pre>\n",
                            pad,
                            lang,
                            escape_html(content)
                        ));
                    }
                    other => {
                        out.push_str(&format!("{}<{}{}>\n", pad, clean_tag, attr_str));
                        render_html_node(other, indent + 1, out);
                        out.push_str(&format!("{}</{}>\n", pad, clean_tag));
                    }
                }
            }
        }
        _ => {}
    }
}

fn build_html_attrs(
    id: Option<&str>,
    classes: &[String],
    props: Option<&IndexMap<String, HirNode>>,
) -> String {
    let mut attrs = Vec::new();
    if let Some(i) = id {
        attrs.push(format!("id=\"{}\"", escape_html(i)));
    }
    if !classes.is_empty() {
        attrs.push(format!("class=\"{}\"", escape_html(&classes.join(" "))));
    }
    if let Some(p) = props {
        for (k, v) in p {
            if k == "()" || k == "[]" || k == "->" {
                continue;
            }
            attrs.push(format!(
                "{}=\"{}\"",
                escape_html(k),
                escape_html(&format_rn_cell(v))
            ));
        }
    }
    if attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", attrs.join(" "))
    }
}

fn render_html_segments(segments: &[StreamSegment]) -> String {
    let mut out = String::new();
    for seg in segments {
        match seg {
            StreamSegment::Text(t) => out.push_str(&escape_html(t)),
            StreamSegment::Code(c) => {
                out.push_str(&format!("<code>{}</code>", escape_html(c)));
            }
            StreamSegment::Quoted(q) => out.push_str(&escape_html(q)),
            StreamSegment::InlineTag {
                tag,
                id,
                classes,
                attrs,
                children,
            } => {
                let attr_str = build_html_attrs(id.as_deref(), classes, Some(attrs));
                out.push_str(&format!(
                    "<{}{}>{}</{}>",
                    tag,
                    attr_str,
                    render_html_segments(children),
                    tag
                ));
            }
        }
    }
    out
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ============================================================================
// 8. Markdown Codegen (Example 1: Stream `[` + Table `(` + Raw `"`)
// ============================================================================

pub fn to_markdown(doc: &HirDocument) -> String {
    let mut blocks = Vec::new();
    render_markdown_node(&doc.root, &mut blocks);
    blocks.join("\n\n") + "\n"
}

fn render_markdown_node(node: &HirNode, blocks: &mut Vec<String>) {
    match node {
        HirNode::Stream { items, .. } => {
            for item in items {
                render_markdown_node(item, blocks);
            }
        }
        HirNode::String(s, _) => {
            blocks.push(s.clone());
        }
        HirNode::RichText { segments, .. } => {
            blocks.push(render_markdown_segments(segments));
        }
        HirNode::Raw { lang, content, .. } => {
            blocks.push(format!("```{}\n{}\n```", lang, content));
        }
        HirNode::Tuple { rows, .. } => {
            blocks.push(render_markdown_table(rows));
        }
        HirNode::Form { map, .. } => {
            for (k, v) in map {
                match (k.as_str(), v) {
                    ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", _) => {
                        let level = k[1..].parse::<usize>().unwrap_or(1);
                        let hashes = "#".repeat(level);
                        blocks.push(format!("{} {}", hashes, extract_markdown_inline(v)));
                    }
                    ("p", _) => {
                        blocks.push(extract_markdown_inline(v));
                    }
                    ("ul", HirNode::Stream { items, .. }) => {
                        let list: Vec<String> = items
                            .iter()
                            .map(|i| format!("- {}", extract_markdown_inline(i)))
                            .collect();
                        blocks.push(list.join("\n"));
                    }
                    ("ol", HirNode::Stream { items, .. }) => {
                        let list: Vec<String> = items
                            .iter()
                            .enumerate()
                            .map(|(idx, i)| format!("{}. {}", idx + 1, extract_markdown_inline(i)))
                            .collect();
                        blocks.push(list.join("\n"));
                    }
                    (_, HirNode::Tuple { rows, .. }) => {
                        blocks.push(render_markdown_table(rows));
                    }
                    (_, HirNode::Raw { lang, content, .. }) => {
                        blocks.push(format!("```{}\n{}\n```", lang, content));
                    }
                    _ => {
                        render_markdown_node(v, blocks);
                    }
                }
            }
        }
        HirNode::Triad { name, body, .. } => {
            if let Some(b) = body {
                for item in b {
                    if name.starts_with('h') && name.len() == 2 {
                        let level = name[1..].parse::<usize>().unwrap_or(1);
                        blocks.push(format!(
                            "{} {}",
                            "#".repeat(level),
                            extract_markdown_inline(item)
                        ));
                    } else {
                        render_markdown_node(item, blocks);
                    }
                }
            }
        }
        _ => {}
    }
}

fn extract_markdown_inline(node: &HirNode) -> String {
    match node {
        HirNode::String(s, _) => s.clone(),
        HirNode::RichText { segments, .. } => render_markdown_segments(segments),
        other => format_rn_cell(other),
    }
}

fn render_markdown_segments(segments: &[StreamSegment]) -> String {
    let mut out = String::new();
    for seg in segments {
        match seg {
            StreamSegment::Text(t) => out.push_str(t),
            StreamSegment::Code(c) => out.push_str(&format!("`{}`", c)),
            StreamSegment::Quoted(q) => out.push_str(q),
            StreamSegment::InlineTag {
                tag,
                attrs,
                children,
                ..
            } => {
                let inner = render_markdown_segments(children);
                match tag.as_str() {
                    "b" | "strong" => out.push_str(&format!("**{}**", inner)),
                    "i" | "em" => out.push_str(&format!("*{}*", inner)),
                    "code" => out.push_str(&format!("`{}`", inner)),
                    "a" => {
                        let href = attrs
                            .get("href")
                            .map(format_rn_cell)
                            .unwrap_or_else(|| "#".to_string());
                        out.push_str(&format!("[{}]({})", inner, href));
                    }
                    _ => out.push_str(&inner),
                }
            }
        }
    }
    out
}

fn render_markdown_table(rows: &[Vec<HirNode>]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let str_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            r.iter()
                .map(|c| match c {
                    HirNode::String(s, _) => s.clone(),
                    other => format_rn_cell(other),
                })
                .collect()
        })
        .collect();

    let cols = str_rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![3usize; cols];
    for r in &str_rows {
        for (i, cell) in r.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }

    let mut lines = Vec::new();
    let format_row = |r: &[String]| -> String {
        let padded: Vec<String> = (0..cols)
            .map(|i| {
                let val = r.get(i).map(|s| s.as_str()).unwrap_or("");
                format!("{:<width$}", val, width = widths[i])
            })
            .collect();
        format!("| {} |", padded.join(" | "))
    };

    lines.push(format_row(&str_rows[0]));
    let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    lines.push(format!("| {} |", sep.join(" | ")));
    for r in str_rows.iter().skip(1) {
        lines.push(format_row(r));
    }
    lines.join("\n")
}

// ============================================================================
// 9. CSV Codegen (Tuple 2D Grids)
// ============================================================================

pub fn to_csv(doc: &HirDocument) -> String {
    let rows = find_first_tuple_rows(&doc.root);
    let mut out = String::new();
    if let Some(r_list) = rows {
        for row in r_list {
            let cells: Vec<String> = row
                .iter()
                .map(|c| {
                    let s = match c {
                        HirNode::String(st, _) => st.clone(),
                        other => format_rn_cell(other),
                    };
                    if s.contains(',') || s.contains('"') || s.contains('\n') {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s
                    }
                })
                .collect();
            out.push_str(&cells.join(","));
            out.push('\n');
        }
    }
    out
}

fn find_first_tuple_rows(node: &HirNode) -> Option<&Vec<Vec<HirNode>>> {
    match node {
        HirNode::Tuple { rows, .. } => Some(rows),
        HirNode::Form { map, .. } => {
            for v in map.values() {
                if let Some(r) = find_first_tuple_rows(v) {
                    return Some(r);
                }
            }
            None
        }
        HirNode::Stream { items, .. } => {
            for i in items {
                if let Some(r) = find_first_tuple_rows(i) {
                    return Some(r);
                }
            }
            None
        }
        _ => None,
    }
}

// ----------------------------------------------------------------------------
// Formatting Utilities
// ----------------------------------------------------------------------------

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n >= (i64::MIN as f64) && n <= (i64::MAX as f64) {
        (n as i64).to_string()
    } else {
        n.to_string()
    }
}

fn escape_quotes(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}