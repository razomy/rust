name "Razomy Notation"
version "14.0"
extension ".rn"
tags (
  "ONTOLOGICAL-DATA"
  "TEXT-FIRST"
  "INDENT-TOPOLOGICAL"
  "ULTRA-LEAN-UNIFIED-NODE"
  "RECURSIVE-TRIAD"
  "ZERO-DRIFT-ANCHORS"
  "JSON-MD-CSV-HTML-DSL-UNIFIED"
  "CANONICAL-AST-LOWERING"
)
description r3"
================================================================================
PART I. THE 5 ONTOLOGICAL AXIOMS & LEAN ENGINEERING LAWS OF (.rn)
================================================================================
Razomy partitions computing into two isomorphic projections:
• Razomy Language (.rl) — Ontology of Action (Code, Pipelines & Execution).
• Razomy Notation (.rn) — Ontology of Being (Data, Prose, Markup & Topology).

The v13.0 architecture is founded on ONE universal primitive:
Every line, inline tag, edge, reset, or leaf is an instance of the UNIFIED NODE:
  Node ::= Prefix? Key? Params? Props? Body? (Arrow Node)? Value?

--------------------------------------------------------------------------------
AXIOM I. THE UNIFIED RECURSIVE TRIAD (`[` STREAM <-> `{` FORM <-> `(` TUPLE)
--------------------------------------------------------------------------------
• Philosophical Axiom (Fractal Semiotic Invariance):
  A document is a fractal tree of nodes. A node may express Firstness (Stream `[`),
  Secondness (Tuple `(`), or Thirdness (Form `{`). Inline markup is not a separate
  Markdown engine; an inline tag `b[bold]` is structurally and syntactically
  identical to a root block node `b [bold]`.

• Engineering Laws:
  - Law 1.1 (The Unified Node Structure):
    Every construct in .rn matches the exact sequence:
      [Prefix] [Key] [(Params)] [{Props}] [[Body]] [-> Edge] [Value]
    All components are optional.
    * Block tag:  `article#main { lang en } [ ... ]`
    * Inline tag: `b[bold]` or `a{href /docs}[documentation]`
    * Field pair: `port 8080` (Key + Value)
    * Edge:       `Ingress -> Auth` (Key + Arrow + Node)
    * Reset:      `..target { ... }` (Prefix + Key + Props)
  - Law 1.2 (`[` Stream Context — Ordered Prose & Inline Elements):
    * Free text flows without quotes. 
    * When an unquoted token immediately touches `[` without a preceding space
      (`b[text]`, `span.highlight[text]`), it recursively parses as an inline Child Node.
    * Backticks (`` `arr[0]` ``) and double quotes (`"text"`) shield text from parser brackets.
  - Law 1.3 (`{` Form Context — Key-Value Maps & Declarations):
    * The first token is the Key; the trailing token/block is the Value.
    * Atomic empty values: Bare key with no value = `null`; `""` = empty string;
      `{}` = empty object; `()` = empty array.
    * Separators: Commas `,` or semicolons `;` allow inline multi-field declarations.
  - Law 1.4 (`(` Tuple Context — Positional Grids & Matrices):
    * Comma-separated values form 1D arrays or 2D tables (CSV replacement).
    * Delimiters inside values are shielded using quotes (`"A, B"`) or backticks.

--------------------------------------------------------------------------------
AXIOM II. CONTEXT INERTIA & TOPOLOGICAL PROPAGATION
--------------------------------------------------------------------------------
• Philosophical Axiom (Cognitive Economy):
  A container's active mode (`[`, `{`, `(`) automatically propagates down through
  indented children until explicitly transformed by an inner bracket.

• Engineering Laws:
  - Law 2.1 (Mode Inheritance):
    Inside `{`, indented children automatically parse as Key-Value Form pairs.
    Inside `[`, indented children automatically parse as Stream text lines.
    Inside `(`, indented children automatically parse as Tuple rows.
  - Law 2.2 (Trailing Bracket Activation):
    A trailing `[`, `{`, or `(` at the end of a line before `Indent + 1` explicitly
    switches the active mode for all nested children.

--------------------------------------------------------------------------------
AXIOM III. INDENT TOPOLOGY, AUTO-CLOSE & ZERO-DRIFT RESET
--------------------------------------------------------------------------------
• Philosophical Axiom (Infinite Depth with Bounded Width):
  Physical screen indentation must not drift indefinitely to the right. 

• Engineering Laws:
  - Law 3.1 (Canonical 1-Space Indentation):
    An indent step of 1 space attaches children to the preceding node. Dedenting
    automatically closes open scopes without closing tags.
  - Law 3.2 (Topological Reset via Prefix `..`):
    Writing `..target_node` at Column 0 teleports the parsing cursor into that node
    within the active root, resetting indentation back to column 1 while preserving
    its deep logical path in the AST.
  - Law 3.3 (Lossless JSON Bijection):
    Single-bracket nodes lower to their native JSON equivalent (`{}`, `[]`).
    Composite nodes `Name(P){K}[B]` lower to `{ "()": P, "[]": B, ...K }`.

--------------------------------------------------------------------------------
AXIOM IV. FIRST-CLASS HYPERGRAPHS & OVERLAYS (`->`, `&`, `/`)
--------------------------------------------------------------------------------
• Philosophical Axiom (Reality is a Hypergraph):
  Relationships and overlays are native structural syntax, not string annotations.

• Engineering Laws:
  - Law 4.1 (Edges):
    Arrows `->` and `<->` connect endpoints into first-class graph edges.
  - Law 4.2 (Overlays):
    In Form declarations, `& .Ref` deep-merges properties from a target node;
    `/ key` removes keys from an inherited structure.

--------------------------------------------------------------------------------
AXIOM V. LEXICAL SIMPLICITY & AST-LEVEL TYPING
--------------------------------------------------------------------------------
• Philosophical Axiom (Parser Lightness, Compiler Intelligence):
  The parser is a lightweight structural recognizer (LL(1) / PEG). Semantic typing
  belongs in the AST Lowering Phase, keeping the grammar tiny and blazingly fast.

• Engineering Laws:
  - Law 5.1 (Raw Canvas `key "lang` & `rN" ... "N`):
    Indented raw blocks and ranked fences capture unescaped source code verbatim.
  - Law 5.2 (AST Auto-Typing):
    Grammar emits atomic text tokens. The AST builder transparently coerces:
    * `true` / `false`  -> Boolean
    * `null`            -> Null
    * `8080`, `-3.14`   -> Strict Number (tokens with leading zeros remain String)
    * Bare strings      -> String


================================================================================
PART II. CATALOG OF CANONICAL EXAMPLES
================================================================================

--------------------------------------------------------------------------------
EXAMPLE 1. MARKDOWN DOCUMENT (Stream `[` + Table `(` + Raw `"`)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  h1 [ Razomy Guide ]
  p [ Fast b[zero-noise] notation with i[inline] tags and `arr[0]` code. ]
  ul [
   No closing tags
   1-space indentation
  benchmarks (
   Format, Tokens, Notes
   JSON,   140,    "Verbose, requires quotes"
   RN,     48,     "Ultra-compact, lean parser"
  snippet "ts
   const x = 42;

• Original Markdown (.md):
  # Razomy Guide

  Fast **zero-noise** notation with *inline* tags and `arr[0]` code.

  - No closing tags
  - 1-space indentation

  | Format | Tokens | Notes                      |
  | ------ | ------ | -------------------------- |
  | JSON   | 140    | Verbose, requires quotes   |
  | RN     | 48     | Ultra-compact, lean parser |

  ```ts
  const x = 42;
  ```

--------------------------------------------------------------------------------
EXAMPLE 2. HTML DOM & COMPONENT TREE (Stream `[` + Form `{`)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  article#post { lang en } [
   h1 [ Why Razomy? ]
   p [ Read the a{href /docs, target _blank}[documentation]. ]

• Original HTML (.html):
  <article id="post" lang="en">
    <h1>Why Razomy?</h1>
    <p>Read the <a href="/docs" target="_blank">documentation</a>.</p>
  </article>

--------------------------------------------------------------------------------
EXAMPLE 3. YAML / JSON CONFIGURATION (Form `{` & Inertia)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  cluster {
   primary
    database
     engine postgres
     port 5432
     token
     empty_str ""
     extra_tags ()
     steps [
      1. Check replication lag
      2. Promote primary node

• Original YAML (.yaml):
  cluster:
    primary:
      database:
        engine: "postgres"
        port: 5432
        token: null
        empty_str: ""
        extra_tags: []
        steps:
          - "1. Check replication lag"
          - "2. Promote primary node"

--------------------------------------------------------------------------------
EXAMPLE 4. CSV + SQL DDL SCHEMA (Tuple `(` & Edges `->`)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  Users (
   col,    type, ref,        default
   id,     uuid, pk,         gen_uuid()
   org_id, uuid, -> Orgs.id, null
   role,   text, not_null,   member

• Original SQL (.sql):
  CREATE TABLE Users (
    id     UUID PRIMARY KEY DEFAULT gen_uuid(),
    org_id UUID REFERENCES Orgs(id) DEFAULT NULL,
    role   TEXT NOT NULL DEFAULT 'member'
  );

--------------------------------------------------------------------------------
EXAMPLE 5. CSS / SCSS STYLESHEET (Form `{` & Literal `#hex`)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  styles {
   .btn {
    background #111111
    padding (8px, 16px)
    &:hover { background #222222 }

• Original SCSS (.scss):
  .btn {
    background: #111111;
    padding: 8px 16px;
    &:hover {
      background: #222222;
    }
  }

--------------------------------------------------------------------------------
EXAMPLE 6. DOCKERFILE REPLACEMENT (Parameterized Node & Commands)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  container(node:22-alpine) {
   workdir /app
   copy (., /app)
   run "sh
    npm ci --omit=dev
   env { NODE_ENV production, PORT 8080 }
   cmd (node, server.js)

• Original Dockerfile:
  FROM node:22-alpine
  WORKDIR /app
  COPY . /app
  RUN npm ci --omit=dev
  ENV NODE_ENV=production PORT=8080
  CMD ["node", "server.js"]

--------------------------------------------------------------------------------
EXAMPLE 7. TERRAFORM (HCL) + ZERO-DRIFT MARGIN RESET (`..target`)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  resource(aws_s3_bucket, logs) {
   bucket my-corp-logs
   acl private

  ..resource.aws_s3_bucket.logs {
   lifecycle_rule {
    enabled true
    expiration { days 90 }

• Original Terraform (.tf):
  resource "aws_s3_bucket" "logs" {
    bucket = "my-corp-logs"
    acl    = "private"

    lifecycle_rule {
      enabled = true
      expiration {
        days = 90
      }
    }
  }

--------------------------------------------------------------------------------
EXAMPLE 8. MERMAID / GRAPHVIZ + OVERLAYS (`->`, `&`, `/`)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  mesh {
   Ingress -> (Auth, Api) { protocol grpc, tls true }
   Base { timeout 30, retries 3, debug true }
   Prod & .Base / debug {
    replicas 5

• Original Graphviz DOT + YAML Anchors:
  # Graphviz DOT:
  digraph mesh {
    Ingress -> {Auth Api} [protocol="grpc", tls=true];
  }
  # YAML Overlay:
  Base: &base
    timeout: 30
    retries: 3
    debug: true
  Prod:
    <<: *base
    replicas: 5
"3
