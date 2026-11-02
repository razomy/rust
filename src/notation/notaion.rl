meta {
 name Razomy Notation
 version 0.1.0
 extension ".rn"
 schema Md
 tags (
  ONTOLOGICAL-DATA
  RL-ISOMORPHIC-LAYER-0
  SCHEMA-AGNOSTIC-CORE
  SCHEMA-PORTALS
  SYMMETRIC-TRIAD
  INDENT-TOPOLOGICAL
  COLUMN-ZERO-STRINGS
  STREAM-ROOT-DEFAULT


part AXIOMATIC SPECIFICATION (LAYER 0 — DEDUCTIVE HIERARCHY)

section AXIOM I: THE STREAM ROOT (ONTOLOGICAL CONTINUUM)

Razomy Notation (`.rn`) is an ontological data and markup notation that serves as the isomorphic Layer 0 substrate of the Razomy Language (`.rl`).

ul [
 b[Universal Stream Root] — Every `.rn` file begins as an implicit Stream `[]` at indentation level 0. Free text, outliner trees, markup nodes, and structured data blocks (`config {`) coexist in a single continuum right from line 1.
 b[Root Schema Binding] — Compound extensions (`*.md.rn`, `*.json.rn`, `*.scss.rn`, `*.sql.rn`, `*.rl`) bind a file-level Schema adapter (or the `.rl` compiler) directly to the root stream.


section AXIOM II: SPATIAL TOPOLOGY (INLINE VS. BLOCK — ONE RULE FOR ALL BRACKETS)

All three structural brackets (`[]`, `{}`, `()`) obey one invariant topological rule based on where the opening bracket is placed:

topic Inline Mode (Same-Line Closure)
If `{`, `(`, or `[` opens and closes on the same line:
ul [
 Inside `{}` and `()`, elements are separated by `,` (surrounding whitespace is ignored): `env { A 1, B 2 }`, `pt (10, 20)`.
 Inside `[...]`, content is a continuous text/markup flow (in `.rn`) or inline pipeline (in `.rl`), with a single leading and single trailing space trimmed: `h1 [ Hello ]`, `span{class hi}[text]`.

topic Block Mode (End-of-Line Open, Topological Auto-Close)
If a space-preceded `{`, `(`, or `[` is the LAST token on a line (ignoring trailing comments), it opens an indented block:
ul [
 b[1 Space = 1 Quantum of Depth] — Every child line is indented by strictly 1 leading space (` `) relative to its parent.
 b[Dedent Auto-Closure] — Dedenting on a non-blank line automatically closes all blocks deeper than the new indentation level. Explicit closing brackets and trailing end-of-line commas are NEVER used in block mode.
 b[Blank-Line Invariance] — Blank or whitespace-only lines never trigger a dedent; they are ignored inside `{}` and `()`, and act as paragraph separators inside `Md [` streams.
 b[Dot-Path Flattening (`a.b.c`)] — A dotted key `a.b.c {` unfolds into `{ a { b { c { ... } } } }` while requiring only a SINGLE 1-space indent on the next line.


section AXIOM III: THE SYMMETRIC TRIAD (`[]` STREAM, `{}` FORM, `()` TUPLE)

topic `[` Stream Mode (Sequential Flow, Outliner, Markup & Pipelines)
Represents an ordered continuum of text, markup, or operations. Empty `[]` = empty stream.
ul [
 b[Lines & Paragraphs] — In a raw Stream `[]`, each plain line at the same depth is a distinct item. Under the `Md` schema (`*.md.rn` or `Md [`), consecutive plain lines merge into paragraphs separated by blank lines.
 b[Outliner Recursion] — Indenting (`+1 space`) beneath a plain text line implicitly nests a child Stream `[]` under that line.
 b[Block Nodes] — A line is parsed as a structured Node if its prefix matches the Node grammar followed by a space-separated `[`, `{`, `(`, or Column-0 quote `(tag)?(N)?"` at the end of the line (or starts with a flat document tag in `*.md.rn`: `title`, `subtitle`, `part`, `section`, `topic`, `quote`).
 b[Inline Nodes & Escaping] — Inside text flow, `Identifier[` or `Identifier{Props}[` (with NO space before `{` or `[`) is ALWAYS parsed as an inline child node (`b[bold]`, `span{class hi}[text]`). Literal brackets are escaped via backticks `` `arr[0]` ``, quotes `"[raw]"`, or backslash (`\[`, `\]`).

topic `{` Form Mode (Key-Value Map / Object / Type Archetype)
Maps `Key Payload` statements separated by `\n` (block) or `,` (inline). Empty `{}` = empty form.
ul [
 b[Case-Aware Empty Key Duality] — Inside a `snake_case` (data/instance) Form, a bare `Key` with no payload and no `& Source` evaluates to `null` (`None` in `.rl`). Inside a `PascalCase` (Type Archetype) Form in `.rl`, a bare `snake_case` `Key` defines an open generic slot `Key _T`.

topic `(` Tuple Mode (1D Lists & 2D Relational Relations)
Represents positional sequences or relational tables. Empty `()` = empty tuple.
ul [
 b[1D List] — Inline `(a, b, c)` OR a block `(` where the first indented line contains NO top-level commas outside quotes/brackets (strictly one item per line; items with literal commas must be quoted `"Hello, world!"`).
 b[2D Relational Table] — A block `(` where the first indented line contains at least one top-level comma defining the header columns (use a trailing comma `col,` for a 1-column table). Each subsequent line is a comma-separated row mapped to those headers (empty cells `, ,` evaluate to `null` / `None`). In `.rl`, a 2D Tuple natively constructs a typed `Tuple` of `Form` records.


section AXIOM IV: LEXICON, AUTO-TYPED SCALARS & COLUMN-0 STRINGS

topic Homoiconic Comments (`# `)
A `# ` (hash followed by a space, or a bare `#` at the end of a line) outside quotes and preceded by start-of-line or whitespace starts a comment attached to the AST node for doc generation. Unspaced tokens like `#42` or `#111111` are treated as literal scalars/text.

topic Scalar Auto-Typing (inside `{}` and `()`)
lexicon_table (
 Token Pattern, Examples, Ontological Type & Semantics
 Boolean, `true` / `false`, `Bool`
 Null / Empty, `null` / `None` / bare key / `, ,`, `Null` (`None` in `.rl`)
 Numeric, `8080` / `-3.14` / `0xFF` / `1e-5`, `Int` / `Float` / `Byte` (multi-dot `0.1.0` falls back to String)
 Symbol, `'pk` / `'not_null` / `'GET`, O(1) interned tag (no closing quote; serializes to `"pk"` in JSON)
 Typed Literal, `Date"2026-03-30"` / `Money"19.99"`, `Type` or tag attached directly to `"..."` (desugars to `Type("...")` in `.rl`)
 Unquoted String, `postgres` / `/app` / `8px` / `#111111`, Any raw text after `Key ` up to `\n` (in block `{}`) or `,` (in `()` / inline `{}`)
 Quoted String, `"Hello, world!"` / `""` / `".btn"`, UTF-8 string. Supports `\"`, `\\`, `\n`, `\t`, `\{`, and `{expr}` interpolation in `.rl`

topic Column-0 Multiline Raw Strings (`(tag)?(N)?"`)
An opening `(tag)?(N)?"` at the end of a line (`"`, `ts"`, `sh"`, `Sql"`, `1"`, `rn2"`) suspends indentation topology until a matching `"(N)?` (`"`, `"1`, `"2`) placed strictly alone on its own line at Column 0:
ul [
 b[Column-0 Invariance] — Content starts strictly at Column 0. Every leading space is 100% literal text, and zero escape sequences or `{expr}` interpolations are processed. The first `\n` after the opening quote and the last `\n` before the closing quote are stripped.
 b[Tag & Delimiter Rank (`tag`, `N`)] — Optional `tag` binds a language/type constructor; optional rank `N` (`1`, `2`, `3`...) prevents inner Column-0 quotes of lower rank from closing the outer string.


section AXIOM V: UNIFIED NODE GRAMMAR, ALGEBRA & SCHEMA PORTALS

Every structured Node in both `.rn` (Layer 0) and `.rl` (Layer 1) obeys one universal grammar:

code 1"
Node ::= Key({Props})?((Slots))? (Algebra)* (Schema)? (" " Payload)?
"1

ul [
 b[`Key`] — Unquoted identifier (`[a-zA-Z0-9_][a-zA-Z0-9_-]*`, including numeric keys like `200`), Dot-Path (`a.b.c`), `'symbol`, or `"quoted"` (when containing spaces or starting with `.` / `&` / `#`).
 b[`{Props}` & `(Slots)`] — Optional inline metadata Form `{...}` or positional Tuple `(...)` attached directly to `Key` with NO leading space: `div{id main} [...]`, `Button{variant 'primary}(label) [...]`.
 b[`Algebra` (Set & Lineage Operators)] — Space-surrounded operators (` & `, ` ^ `, ` / `) applied before the payload:
  `Target & Source` — Deep-merges all fields from `Source` (Identifier or Dot-Path) into `Target` left-to-right (`Target & S1 & S2`). Omitting `{}` clones the merged value directly (`port & config.default_port`).
  `Target ^ Source` — Symmetric difference (retains only keys present in `Target` or `Source`, but not both).
  `Target & Source / key` — Strips `key` (identifier, `'symbol`, or dotted `a.b`) from the inherited Form (`Target & Source / k1 / k2`).
 b[`Schema` Portal] — An optional `PascalCase` token placed AFTER `Key` (and `Algebra`) and BEFORE `{`, `(`, `[` binds a schema adapter (written in `.rl`) to that subtree (`styles Scss {`, `query Sql (`, `view Html [`). Without a trailing bracket (`id Uuid`), a `PascalCase` token is a scalar value in `.rn` or a field type in `.rl`.
 b[`Payload`] — Space-separated block (`{}`, `()`, `[]`), Column-0 string (`(tag)?(N)?"`), scalar `Value`, or empty.

topic Isomorphic Bridge (`.rn` <-> `.rl` Zero-Rewrite Upgrade)
Because `.rn` is strictly isomorphic to Layer 0 of `.rl`, any static `.rn` file can be upgraded into a reactive `.rl` module without changing a single bracket: attaching `& [ .port > 1024 ]` adds a compile-time Noether validation invariant, and replacing a static value with `[ base.port + 10 ]` turns it into a lazy computed lens.


part CANONICAL EXAMPLES (VERIFYING EVERY AXIOM)

section PLAIN TEXT & OUTLINER NOTES (Axiom I & III — Stream Root + Outliner)
example {
 input rn"
how my day?
 good
todo list
 a
 b
 c
"
 output json"
[
  { "text": "how my day?", "children": ["good"] },
  { "text": "todo list", "children": ["a", "b", "c"] }
]
"

section MARKDOWN DOCUMENT (`*.md.rn` — Flat Headings, Inline Nodes & 2D Tables)
example {
 input rn2"
title Razomy Guide

Fast b[zero-noise] notation a(/docs)[links], with span{class highlight}[inline] tags and `arr[0]` code.

ul [
 No closing tags
 1-space indentation

benchmarks (
 Format, Tokens, Notes
 JSON,   140,    "Verbose, requires quotes"
 RN,     48,     "Ultra-compact, lean parser"

code ts"
const x = 42;
function hello() {
  return x;
}
"
"2
 output md"
# Razomy Guide

Fast **zero-noise** notation with <span class="highlight">inline</span> tags and `arr[0]` code.

- No closing tags
- 1-space indentation

| Format | Tokens | Notes                      |
| ------ | ------ | -------------------------- |
| JSON   | 140    | Verbose, requires quotes   |
| RN     | 48     | Ultra-compact, lean parser |

```ts
const x = 42;
function hello() {
  return x;
}
```
"

section HTML DOM TREE (Axiom V — Attached `{Props}` + Stream `[`)
example {
 input rn"
article{id post, class featured, lang en} [
 h1 [ Why Razomy? ]
 p [ Read the a{href /docs, target _blank}[documentation]. ]
"
 output html"
<article id="post" class="featured" lang="en">
  <h1>Why Razomy?</h1>
  <p>Read the <a href="/docs" target="_blank">documentation</a>.</p>
</article>
"

section JSON / YAML CONFIG (Axiom II & IV — Dot-Path, Duality & Col-0 Strings)
example {
 input rn2"
cluster.primary.database {
 engine postgres
 port 5432
 created_at Date"12.12.23"
 token
 empty_str ""
 extra_tags ()
 welcome_msg 1"
Line 1 starts at Column 0 (zero ambiguity with parent indentation!).
  Line 2 has exactly 2 leading spaces of actual text content.
"1
 steps [
  Check replication lag
  Promote primary node
"2
 output json"
{
  "cluster": {
    "primary": {
      "database": {
        "engine": "postgres",
        "port": 5432,
        "created_at": "12.12.23",
        "token": null,
        "empty_str": "",
        "extra_tags": [],
        "welcome_msg": "Line 1 starts at Column 0 (zero ambiguity with parent indentation!).\n  Line 2 has exactly 2 leading spaces of actual text content.",
        "steps": [
          "Check replication lag",
          "Promote primary node"
        ]
      }
    }
  }
}
"

section RELATIONAL SCHEMA & TABULAR DATA (Axiom III & IV — 2D Tuple & Symbols)
example {
 input rn"
users_schema (
 column, type, constraint, default
 id,     Uuid, 'pk,        gen_uuid
 org_id, Uuid, Orgs.id,    null
 role,   Text, 'not_null,  member

users_rows (
 id,  org_id, role,   created_at
 u_1, o_10,   admin,  Date"12.12.23"
 u_2, o_10,   member, Date"13.12.23"
"
 output sql"
CREATE TABLE users (
  id     UUID PRIMARY KEY DEFAULT gen_uuid(),
  org_id UUID REFERENCES Orgs(id) DEFAULT NULL,
  role   TEXT NOT NULL DEFAULT 'member'
);
INSERT INTO users (id, org_id, role, created_at) VALUES
  ('u_1', 'o_10', 'admin', '12.12.23'),
  ('u_2', 'o_10', 'member', '13.12.23');
"

section CSS / SCSS STYLES (Axiom IV — Unquoted Hex/Units & Quoted Keys)
example {
 input rn"
".btn" {
 background-color #111111
 padding (8px, 16px)
 "&:hover" { background-color #222222 }
"
 output scss"
.btn {
  background-color: #111111;
  padding: 8px 16px;
  &:hover {
    background-color: #222222;
  }
}
"

section DOCKER & TERRAFORM INFRASTRUCTURE (Dot-Path, Form & Col-0)
example {
 input rn2"
container {
 from node:22-alpine
 workdir /app
 copy (., /app)
 run sh1"
npm ci --omit=dev
"1
 env { NODE_ENV production, PORT 8080 }
 cmd (node, server.js)

resource.aws_s3_bucket.logs {
 bucket my-corp-logs
 acl private
 lifecycle_rule.expiration {
  enabled true
  days 90
"2
 output tf"
# Dockerfile:
FROM node:22-alpine
WORKDIR /app
COPY . /app
RUN npm ci --omit=dev
ENV NODE_ENV=production PORT=8080
CMD ["node", "server.js"]

# Terraform (.tf):
resource "aws_s3_bucket" "logs" {
  bucket = "my-corp-logs"
  acl    = "private"
  lifecycle_rule {
    expiration {
      enabled = true
      days    = 90
    }
  }
}
"

section ALGEBRAIC INHERITANCE (Axiom V — `&` Merge & `/` Strip)
example {
 input rn"
mesh {
 ingress {
  to (auth, api)
  protocol grpc
  tls true
 base { timeout 30, retries 3, debug true }
 prod & base / debug {
  replicas 5
 backup_timeout & base.timeout
"
 output json"
{
  "mesh": {
    "ingress": { "to": ["auth", "api"], "protocol": "grpc", "tls": true },
    "base":    { "timeout": 30, "retries": 3, "debug": true },
    "prod":    { "timeout": 30, "retries": 3, "replicas": 5 },
    "backup_timeout": 30
  }
}
"

section MULTI-SCHEMA COMPONENT PORTALS (Axiom V — `Key Schema` Portals)
example {
 input rn"
UserCard {
 props {
  id Uuid
  theme dark

 query Sql (
  select, from,  where
  "*",    users, "id = :id"

 view Html [
  div{class card} [
   h2 [ User Profile ]
   p [ Welcome back! ]

 styles Scss {
  ".card" {
   background-color #111111
   "&:hover" { background-color #222222 }
"
 output json"
{
  "UserCard": {
    "props": { "id": "Uuid", "theme": "dark" },
    "query": "SELECT * FROM users WHERE id = :id;",
    "view": "<div class=\"card\"><h2>User Profile</h2><p>Welcome back!</p></div>",
    "styles": ".card { background-color: #111111; }\n.card:hover { background-color: #222222; }"
  }
}
"