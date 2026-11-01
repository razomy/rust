//! High-Level Intermediate Representation (HIR) for Razomy Notation (`.rn`) v12.0.
//!
//! Captures both the rich ontological AST (Triad nodes, Inline Markup, 2D Tuple Grids,
//! Hypergraph Edges `->`, Lineage References `.` / `..`, and Algebraic Overlays `&` / `/`)
//! AND lowers deterministically into Canonical JSON Value (Law 3.5 Bijection) or resolved HIR.

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

/// Algebraic Overlay Operator (`&` Deep Merge or `/` Key Omission — Law 4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayOp {
    /// `& .Base` or `& ..Ancestor.Base`
    Merge(String),
    /// `/ debug` or `/ a.b`
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
    /// `null` literal or bare key with no value (Law 1.2)
    Null(Span),
    /// `true` or `false`
    Bool(bool, Span),
    /// Strict number (`8080`, `-3.14` — Law 1.2)
    Number(f64, Span),
    /// String literal or unquoted leaf text
    String(String, Span),
    /// Raw block (`key "lang` or `rN"..."N` — Law 5.1)
    Raw {
        lang: String,
        content: String,
        span: Span,
    },
    /// Lineage Reference (`.sibling` or `..Ancestor.field` — Law 4.2)
    Reference {
        path: String,
        span: Span,
    },
    /// First-class Edge or Hyperedge (`A -> B { ... }` or `-> Orgs.id` — Law 4.1)
    Edge(HirEdge),
    /// Stream `[` node — ordered list of prose lines, inline markup, or child blocks (Law 1.1)
    Stream {
        items: Vec<HirNode>,
        span: Span,
    },
    /// Inline rich text line containing mixed prose and inline tags (`p [ Fast b[zero-noise] ]`)
    RichText {
        segments: Vec<StreamSegment>,
        span: Span,
    },
    /// Form `{` node — ordered key-value map with optional algebraic overlays (Law 1.2)
    Form {
        map: IndexMap<String, HirNode>,
        edges: Vec<HirEdge>,
        overlays: Vec<OverlayOp>,
        span: Span,
    },
    /// Tuple `(` node — 1D positional array or 2D relational grid/table (Law 1.3)
    Tuple {
        /// If rows.len() == 1, represents a 1D array; if rows.len() > 1 and multi-column, a 2D table
        rows: Vec<Vec<HirNode>>,
        span: Span,
    },
    /// Composite Triad Node (`Name(Params) & .Base {Props} [Body]` — Law 3.4 & Law 3.5)
    Triad {
        name: String,
        id: Option<String>,
        classes: Vec<String>,
        params: Option<Vec<HirNode>>,
        overlays: Vec<OverlayOp>,
        props: Option<IndexMap<String, HirNode>>,
        body: Option<Vec<HirNode>>,
        span: Span,
    },
    /// Homoiconic Comment (`# ...` — Law 5.2)
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
            | HirNode::Comment(_, s) => *s,
            HirNode::Raw { span, .. }
            | HirNode::Reference { span, .. }
            | HirNode::Stream { span, .. }
            | HirNode::RichText { span, .. }
            | HirNode::Form { span, .. }
            | HirNode::Tuple { span, .. }
            | HirNode::Triad { span, .. } => *span,
            HirNode::Edge(e) => e.span,
        }
    }

    /// Converts an `HirNode` to a Canonical `serde_json::Value` following Law 3.5
    /// (Lossless Bijection & Reserved Key Escaping `"()"`, `"[]"`, `"->"`).
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
            HirNode::Raw { content, .. } => Value::String(content.clone()),
            HirNode::Reference { path, .. } => Value::String(path.clone()),
            HirNode::Comment(_, _) => Value::Null,
            HirNode::Edge(edge) => edge_to_json(edge),
            HirNode::RichText { segments, .. } => Value::String(render_segments_plain(segments)),
            HirNode::Stream { items, .. } => {
                let arr: Vec<Value> = items
                    .iter()
                    .filter(|i| !matches!(i, HirNode::Comment(_, _)))
                    .map(|i| i.to_canonical_json())
                    .collect();
                Value::Array(arr)
            }
            HirNode::Tuple { rows, .. } => {
                if rows.is_empty() {
                    Value::Array(vec![])
                } else if rows.len() == 1 {
                    // 1D array
                    Value::Array(rows[0].iter().map(|c| c.to_canonical_json()).collect())
                } else if rows.iter().all(|r| r.len() == 1) {
                    // Multiline single-column `(` creates a 1D array (Law 1.3)
                    Value::Array(rows.iter().map(|r| r[0].to_canonical_json()).collect())
                } else {
                    // 2D relational grid
                    Value::Array(
                        rows.iter()
                            .map(|r| Value::Array(r.iter().map(|c| c.to_canonical_json()).collect()))
                            .collect(),
                    )
                }
            }
            HirNode::Form { map, edges, .. } => {
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
                props,
                body,
                ..
            } => {
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
                    // Only `[Body]` -> lowers directly to `[...]` or single string
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