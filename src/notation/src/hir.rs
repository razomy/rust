//! High-Level Intermediate Representation (HIR) for Razomy Notation (`.rn`) v0.1.0.
//!
//! Captures the universal Layer 0 AST:
//! - Universal Stream Root `[]`, Form `{}`, Tuple `()` (1D List & 2D Relational Table)
//! - Outliner trees (`{ "text": ..., "children": [...] }`)
//! - Symbols (`'pk`), Typed Literals (`Date"2026-03-30"`), Column-0 Strings (`(tag)?(N)?"`)
//! - Algebraic Composition (`&` Deep Merge, `^` Symmetric Difference / XOR, `/` Key Strip)
//! - Schema Portals (`Key Schema {` — `Sql`, `Html`, `Scss`, `Md`, etc.)

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use crate::lexer::{ContextMode, Diagnostic};

/// Source span information for IDE/LSP navigation and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub end_col: usize,
}

/// Algebraic Composition Operator (`&` Deep Merge, `^` Symmetric Difference, `/` Key Strip).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayOp {
    /// `& Source` (Identifier or Dot-Path)
    Merge(String),
    /// `^ Source` (Symmetric Difference / XOR)
    Xor(String),
    /// `/ Key` or `/ 'key` or `/ a.b` (Key Strip)
    Omit(String),
}

/// First-Class Hypergraph Edge (`->` or `<->` — Law 4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirEdge {
    pub endpoints: Vec<HirEndpoint>,
    /// `"->"` or `"<->"` between consecutive endpoints
    pub arrows: Vec<String>,
    /// Optional metadata Form `{ protocol grpc, tls true }`
    pub metadata: Option<IndexMap<String, HirNode>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirEndpoint {
    /// Single node/key or lineage ref (`Ingress`, `Orgs.id`, `.Sibling`)
    Target(String),
    /// Hyperedge target group `(Auth, Billing)`
    Group(Vec<HirNode>),
}

/// Inline Stream Item inside `[` Stream context (Law 1.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StreamSegment {
    /// Plain unquoted prose text
    Text(String),
    /// Inline verbatim backtick code `` `arr[0]` ``
    Code(String),
    /// Shielded double-quoted span `"..."`
    Quoted(String),
    /// Unspaced inline tag `b[bold]` or `a{href /docs}[link]`
    InlineTag {
        tag: String,
        id: Option<String>,
        classes: Vec<String>,
        attrs: IndexMap<String, HirNode>,
        children: Vec<StreamSegment>,
    },
}

/// High-Level Intermediate Representation (HIR) Node for `.rn`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirNode {
    /// `null` / `None`, bare key in data Form `{}`, or empty cell `, ,` in Tuple `()`
    Null(Span),
    /// `true` or `false`
    Bool(bool, Span),
    /// Number (`8080`, `-3.14`, `0xFF`, `1e-5`)
    Number(f64, Span),
    /// String literal or unquoted leaf text
    String(String, Span),
    /// Prefix-quoted Symbol (`'pk`, `'not_null`, `'GET` — serializes to string `"pk"` in JSON)
    Symbol(String, Span),
    /// Typed Literal (`Date"2026-03-30"`, `Money"199.99"`, `Url"https://razomy.org"`)
    TypedLiteral {
        type_name: String,
        value: String,
        span: Span,
    },
    /// Column-0 Multiline & Raw String (`(tag)?(N)?"` ... `"(N)?`)
    Raw {
        lang: String,
        content: String,
        rank: Option<usize>,
        span: Span,
    },
    /// Lineage Reference or Cloned Source (`& config.default_port`)
    Reference {
        path: String,
        span: Span,
    },
    /// First-class Edge or Hyperedge (`A -> B { ... }`)
    Edge(HirEdge),
    /// Outliner node: plain text line with indented child Stream items beneath it
    Outliner {
        text: Box<HirNode>,
        children: Vec<HirNode>,
        span: Span,
    },
    /// Stream `[` node — ordered list of plain text, outliner trees, markup tags, and data blocks
    Stream {
        schema: Option<String>,
        items: Vec<HirNode>,
        span: Span,
    },
    /// Inline rich text line containing mixed prose and inline tags (`Fast b[zero-noise]`)
    RichText {
        segments: Vec<StreamSegment>,
        span: Span,
    },
    /// Form `{` node — key-value map with optional Schema Portal and Algebraic Composition
    Form {
        schema: Option<String>,
        map: IndexMap<String, HirNode>,
        edges: Vec<HirEdge>,
        overlays: Vec<OverlayOp>,
        span: Span,
    },
    /// Tuple `(` node — 1D List or 2D Relational Table
    Tuple {
        schema: Option<String>,
        /// True if the first indented line contained a top-level comma defining header columns
        is_2d_table: bool,
        rows: Vec<Vec<HirNode>>,
        span: Span,
    },
    /// Universal Triad Node (`Key{Props}(Slots) & Algebra Schema Payload`)
    Triad {
        name: String,
        id: Option<String>,
        classes: Vec<String>,
        params: Option<Vec<HirNode>>,
        overlays: Vec<OverlayOp>,
        schema: Option<String>,
        props: Option<IndexMap<String, HirNode>>,
        body: Option<Vec<HirNode>>,
        span: Span,
    },
    /// Homoiconic Comment (`# ...`)
    Comment(String, Span),
}

/// Complete HIR Document for a `.rn` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirDocument {
    pub root_mode: ContextMode,
    pub root: HirNode,
    pub diagnostics: Vec<Diagnostic>,
    /// Symbol table of top-level and nested named nodes for LSP & Overlay resolution
    pub symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub full_path: String,
    pub kind: SymbolKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    FormKey,
    StreamTag,
    TupleTable,
    TriadComponent,
    Edge,
    MarginReset,
}

impl HirNode {
    pub fn span(&self) -> Span {
        match self {
            HirNode::Null(s)
            | HirNode::Bool(_, s)
            | HirNode::Number(_, s)
            | HirNode::String(_, s)
            | HirNode::Symbol(_, s)
            | HirNode::Comment(_, s) => *s,
            HirNode::TypedLiteral { span, .. }
            | HirNode::Raw { span, .. }
            | HirNode::Reference { span, .. }
            | HirNode::Outliner { span, .. }
            | HirNode::Stream { span, .. }
            | HirNode::RichText { span, .. }
            | HirNode::Form { span, .. }
            | HirNode::Tuple { span, .. }
            | HirNode::Triad { span, .. } => *span,
            HirNode::Edge(e) => e.span,
        }
    }

    /// Evaluates Schema Portals (`Key Schema {`, `(`, `[`) or lowers directly to Canonical `serde_json::Value`.
    pub fn to_canonical_json(&self) -> serde_json::Value {
        use serde_json::{Map, Number, Value};

        match self {
            HirNode::Null(_) => Value::Null,
            HirNode::Bool(b, _) => Value::Bool(*b),
            HirNode::Number(n, _) => {
                if n.fract() == 0.0 && *n >= (i64::MIN as f64) && *n <= (i64::MAX as f64) {
                    Value::Number(Number::from(*n as i64))
                } else {
                    Number::from_f64(*n)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                }
            }
            HirNode::String(s, _) => Value::String(s.clone()),
            HirNode::Symbol(s, _) => Value::String(s.clone()),
            HirNode::TypedLiteral { value, .. } => Value::String(value.clone()),
            HirNode::Raw { content, .. } => Value::String(content.clone()),
            HirNode::Reference { path, .. } => Value::String(path.clone()),
            HirNode::Comment(_, _) => Value::Null,
            HirNode::Edge(edge) => edge_to_json(edge),
            HirNode::RichText { segments, .. } => Value::String(render_segments_plain(segments)),
            HirNode::Outliner { text, children, .. } => {
                let mut obj = Map::new();
                obj.insert("text".to_string(), text.to_canonical_json());
                let child_arr: Vec<Value> = children
                    .iter()
                    .filter(|c| !matches!(c, HirNode::Comment(_, _)))
                    .map(|c| c.to_canonical_json())
                    .collect();
                obj.insert("children".to_string(), Value::Array(child_arr));
                Value::Object(obj)
            }
            HirNode::Stream { schema, items, .. } => {
                if let Some(portal) = schema {
                    if let Some(rendered) = crate::codegen::eval_schema_portal(portal, self) {
                        return Value::String(rendered);
                    }
                }
                let arr: Vec<Value> = items
                    .iter()
                    .filter(|i| !matches!(i, HirNode::Comment(_, _)))
                    .map(|i| i.to_canonical_json())
                    .collect();
                Value::Array(arr)
            }
            HirNode::Tuple {
                schema,
                is_2d_table,
                rows,
                ..
            } => {
                if let Some(portal) = schema {
                    if let Some(rendered) = crate::codegen::eval_schema_portal(portal, self) {
                        return Value::String(rendered);
                    }
                }
                if rows.is_empty() {
                    Value::Array(vec![])
                } else if !is_2d_table && rows.len() == 1 {
                    Value::Array(rows[0].iter().map(|c| c.to_canonical_json()).collect())
                } else if !is_2d_table && rows.iter().all(|r| r.len() == 1) {
                    Value::Array(rows.iter().map(|r| r[0].to_canonical_json()).collect())
                } else if *is_2d_table && rows.len() >= 2 {
                    // 2D Table: First indented line defines header columns; subsequent lines map to records or arrays
                    let headers: Vec<String> = rows[0]
                        .iter()
                        .map(|h| match h.to_canonical_json() {
                            Value::String(s) => s,
                            other => other.to_string(),
                        })
                        .collect();
                    let records: Vec<Value> = rows
                        .iter()
                        .skip(1)
                        .map(|r| {
                            let mut row_obj = Map::new();
                            for (i, col_name) in headers.iter().enumerate() {
                                let cell_val = r
                                    .get(i)
                                    .map(|c| c.to_canonical_json())
                                    .unwrap_or(Value::Null);
                                row_obj.insert(col_name.clone(), cell_val);
                            }
                            Value::Object(row_obj)
                        })
                        .collect();
                    Value::Array(records)
                } else {
                    Value::Array(
                        rows.iter()
                            .map(|r| {
                                Value::Array(r.iter().map(|c| c.to_canonical_json()).collect())
                            })
                            .collect(),
                    )
                }
            }
            HirNode::Form {
                schema, map, edges, ..
            } => {
                if let Some(portal) = schema {
                    if let Some(rendered) = crate::codegen::eval_schema_portal(portal, self) {
                        return Value::String(rendered);
                    }
                }
                let mut obj = Map::new();
                for (k, v) in map {
                    if matches!(v, HirNode::Comment(_, _)) {
                        continue;
                    }
                    let escaped_key = escape_reserved_key(k);
                    obj.insert(escaped_key, v.to_canonical_json());
                }
                if !edges.is_empty() {
                    let edge_arr: Vec<Value> = edges.iter().map(edge_to_json).collect();
                    obj.insert("->".to_string(), Value::Array(edge_arr));
                }
                Value::Object(obj)
            }
            HirNode::Triad {
                name,
                id,
                classes,
                params,
                schema,
                props,
                body,
                ..
            } => {
                if let Some(portal) = schema {
                    if let Some(rendered) = crate::codegen::eval_schema_portal(portal, self) {
                        let mut wrapper = Map::new();
                        wrapper.insert(name.clone(), Value::String(rendered));
                        return Value::Object(wrapper);
                    }
                }
                let has_params = params.as_ref().map(|p| !p.is_empty()).unwrap_or(false);
                let has_props = props.as_ref().map(|p| !p.is_empty()).unwrap_or(false)
                    || id.is_some()
                    || !classes.is_empty();
                let has_body = body.as_ref().map(|b| !b.is_empty()).unwrap_or(false);

                let mut inner_obj = Map::new();

                if let Some(id_val) = id {
                    inner_obj.insert("id".to_string(), Value::String(id_val.clone()));
                }
                if !classes.is_empty() {
                    inner_obj.insert("class".to_string(), Value::String(classes.join(" ")));
                }
                if let Some(p_map) = props {
                    for (k, v) in p_map {
                        inner_obj.insert(escape_reserved_key(k), v.to_canonical_json());
                    }
                }
                if has_params {
                    if let Some(p_vec) = params {
                        inner_obj.insert(
                            "()".to_string(),
                            Value::Array(p_vec.iter().map(|x| x.to_canonical_json()).collect()),
                        );
                    }
                }

                let value_part = if !has_params && !has_props && has_body {
                    let b = body.as_ref().unwrap();
                    if b.len() == 1 {
                        b[0].to_canonical_json()
                    } else {
                        Value::Array(b.iter().map(|x| x.to_canonical_json()).collect())
                    }
                } else {
                    if has_body {
                        if let Some(b_vec) = body {
                            inner_obj.insert(
                                "[]".to_string(),
                                Value::Array(b_vec.iter().map(|x| x.to_canonical_json()).collect()),
                            );
                        }
                    }
                    Value::Object(inner_obj)
                };

                let mut wrapper = Map::new();
                wrapper.insert(name.clone(), value_part);
                Value::Object(wrapper)
            }
        }
    }
}

/// Law 3.5: Lossless Bijection Escape for reserved keys `"()"`, `"[]"`, `"->"`
pub fn escape_reserved_key(key: &str) -> String {
    match key {
        "()" => "(())".to_string(),
        "[]" => "[[]]".to_string(),
        "->" => "-->".to_string(),
        other => other.to_string(),
    }
}

/// Unescapes reserved keys when importing from JSON back into `.rn` (Law 3.5).
pub fn unescape_reserved_key(key: &str) -> String {
    match key {
        "(())" => "()".to_string(),
        "[[]]" => "[]".to_string(),
        "-->" => "->".to_string(),
        other => other.to_string(),
    }
}

fn edge_to_json(edge: &HirEdge) -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut tuple = Vec::new();
    for ep in &edge.endpoints {
        match ep {
            HirEndpoint::Target(s) => tuple.push(Value::String(s.clone())),
            HirEndpoint::Group(g) => {
                tuple.push(Value::Array(g.iter().map(|n| n.to_canonical_json()).collect()))
            }
        }
    }
    if let Some(meta) = &edge.metadata {
        let mut m = Map::new();
        for (k, v) in meta {
            m.insert(k.clone(), v.to_canonical_json());
        }
        tuple.push(Value::Object(m));
    }
    Value::Array(tuple)
}

pub fn render_segments_plain(segments: &[StreamSegment]) -> String {
    let mut out = String::new();
    for seg in segments {
        match seg {
            StreamSegment::Text(t) => out.push_str(t),
            StreamSegment::Code(c) => {
                out.push('`');
                out.push_str(c);
                out.push('`');
            }
            StreamSegment::Quoted(q) => out.push_str(q),
            StreamSegment::InlineTag {
                tag,
                id,
                classes,
                attrs,
                children,
            } => {
                out.push_str(tag);
                if let Some(i) = id {
                    out.push('#');
                    out.push_str(i);
                }
                for c in classes {
                    out.push('.');
                    out.push_str(c);
                }
                if !attrs.is_empty() {
                    out.push('{');
                    let pairs: Vec<String> = attrs
                        .iter()
                        .map(|(k, v)| match v.to_canonical_json() {
                            serde_json::Value::String(s) => format!("{} {}", k, s),
                            other => format!("{} {}", k, other),
                        })
                        .collect();
                    out.push_str(&pairs.join(", "));
                    out.push('}');
                }
                out.push('[');
                out.push_str(&render_segments_plain(children));
                out.push(']');
            }
        }
    }
    out
}