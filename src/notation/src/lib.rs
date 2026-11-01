//! Razomy Notation (`.rn`) v12.0 — Complete Rust Implementation.
//!
//! Includes:
//! - `lexer`: Phase 1 Two-Phase Topological Pre-Lexer (`[` Stream, `{` Form, `(` Tuple, `"` Raw, `#` Comments)
//! - `parser`: Phase 2 Pest Parser & HIR Resolver (`..` Margin Resets, `&`/`/` Overlays, `->` Hypergraphs)
//! - `hir`: High-Level Intermediate Representation & Canonical JSON Bijection (Law 3.5)
//! - `codegen`: Multi-Format Code Generator (`rn`, `json`, `json5`, `yaml`, `proto`, `mermaid`, `sql`, `html`, `md`, `csv`)
//! - `import`: Multi-Format Importer from all supported formats into `.rn` HIR
//! - `lsp`: Language Server Protocol (LSP) engine & stdio server

pub mod codegen;
pub mod hir;
pub mod import;
pub mod lexer;
pub mod lsp;
pub mod parser;

pub use codegen::{generate, Format};
pub use hir::{HirDocument, HirEdge, HirEndpoint, HirNode};
pub use import::import_from;
pub use lsp::LspServer;
pub use parser::parse_rn;

