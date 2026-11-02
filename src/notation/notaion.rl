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

part AXIOMATIC SPECIFICATION (LAYER 0 — FROM ROOT TO LEAVES)

section THE ROOT AXIOM (UNIVERSAL STREAM ROOT & UNIFIED TRIAD GRAMMAR)

By default, the root of every `.rn` file is an implicit Stream `[]` at indentation level 0.
Plain text lines, outliner trees, markup tags, and data blocks `config {` coexist right from line 1.
Compound file extensions (`*.md.rn`, `*.json.rn`, `*.scss.rn`, `*.sql.rn`, `*.rl`) bind a file-level Schema adapter (or the full Razomy Language compiler) to the root.

Every structured Node in both `.rn` (Layer 0 Data/Markup) and `.rl` (Layer 1 Language) follows one universal grammar:

code 1"
Node ::= Key ({Props})? ((Slots))? (Algebra)* (Schema)? ( " " Payload )?
"1

ul [
 `Key` — Unquoted token up to whitespace/delimiter (`[a-zA-Z0-9_][a-zA-Z0-9_-]*`, allowing numeric keys like `200`), Dot-Path (`a.b.c`), `'symbol`, or `"quoted"` (when containing spaces or starting with special characters like `".btn"`).
 `{Props}` / `(Slots)` — Optional inline metadata Form `{...}` or positional Tuple `(...)` attached directly to `Key` WITHOUT a leading space (`div{id main} [...]`, `Button{variant 'primary}(label) [...]`).
 `Algebra` — Optional space-separated lineage inheritance ` & Source`, symmetric difference ` ^ Source`, and key/tag stripping ` / Key` (where `Key` in header position may be an identifier `debug`, `'symbol`, or Dot-Path `a.b`).
 `Schema` — Optional `PascalCase` adapter (in `.rn`) or Type Archetype (in `.rl`) placed AFTER `Key` (and any `Algebra`) and BEFORE `{`, `(`, `[` or scalar value (`query Sql (`, `styles Scss {`, `max_speed Int 180`).
 `Payload` — Space-separated structural block (`{}`, `()`, `[]`), Column-0 string (`(tag)?(N)?"`), scalar `Value`, or empty.


section TOPOLOGY AXIOM (INLINE VS. BLOCK — ONE RULE FOR ALL BRACKETS)

topic Inline Mode (Same-Line Close)
If `{`, `(`, or `[` opens and closes on the same line, elements in `{}` and `()` are separated by `,` (surrounding spaces are ignored), while inline `[...]` represents a single text/markup flow (in `.rn`) or inline pipeline (in `.rl`).
Example: `env { A 1, B 2 }`, `pt (10, 20)`, `span{class hi}[text]`.

topic Block Mode (End-of-Line Open, Auto-Close on Dedent)
If a space-preceded `{`, `(`, or `[` is the LAST token on a line (ignoring trailing comments), it opens an indented block:
1 leading space (` `) = 1 indentation level.
Dedenting on a non-blank line automatically closes all blocks deeper than the new indentation level. Blank or whitespace-only lines never trigger a dedent (they are ignored inside `{}` and `()`, and act as paragraph separators in `Md [` streams). Explicit closing brackets and trailing end-of-line commas are NEVER used in block mode.

topic Dot-Path Flattening (`a.b.c`)
A dotted key `a.b.c {` expands into `{ a { b { c { ... } } } }` using only a SINGLE 1-space indent on the next line.


section THE 3 STRUCTURAL MODES (`[]` STREAM, `{}` FORM, `()` TUPLE)

topic `[` Stream Mode (Default Root — Free Text, Outliner, Markup & Pipelines)
Empty `[]` = empty stream/list.
Line Items & Paragraphs: In a raw Stream `[]`, each plain line at the same indentation level is a distinct item. Under the `Md` schema adapter (`*.md.rn` or `Md [`), consecutive plain lines automatically merge into paragraphs separated by blank lines.
Structured Nodes: A line is parsed as a structured Node if its prefix matches the Node header grammar followed by a space-separated `[`, `{`, `(`, or `(tag)?(N)?"` at the end of the line (or starts with a flat document tag in `*.md.rn` such as `part`, `section`, `topic`, `title`, `subtitle`, `quote`).
Whitespace: In inline `[ text ]`, a single leading space after `[` and a single trailing space before `]` are trimmed.
Inline Tags & Escaping: Inside a text flow, `Identifier[` or `Identifier{Props}[` (with NO space before `[` or `{`) is ALWAYS parsed as an inline child node (`b[bold]`, `span{class hi}[text]`). Literal brackets or code expressions must be escaped via backticks `` `arr[0]` ``, quotes `"[raw]"`, or backslash (`\[`, `\]`).
Outliner: Indenting (`+1 space`) beneath a plain text line implicitly nests a child Stream under that line.

topic `{` Form Mode (Key-Value Map / Object / Type Prototype)
Maps `Key Payload` statements separated by `\n` (block) or `,` (inline).
Empty `{}` = empty map/form.
Case-Aware Empty Key Duality (Isomorphism with `.rl` Axiom V):
Inside a `snake_case` (data/instance) Form, a bare `Key` with no payload and no `& Source` evaluates to `null` (`None` in `.rl`).
Inside a `PascalCase` (Type Archetype) Form in `.rl`, a bare `snake_case` `Key` defines an open generic slot `Key _T`.

topic `(` Tuple Mode (1D Lists & 2D Relational Tables)
Empty `()` = empty list/tuple.
1D List: Inline `(a, b, c)` (comma-separated) OR block `(` where the first indented line contains NO top-level commas outside quotes/brackets (strictly one item per line; items containing literal commas must be quoted `"Hello, world!"`).
2D Table: Block `(` where the first indented line contains at least one top-level comma defining the header columns (use a trailing comma `col,` for a single-column table). Each subsequent indented line is a comma-separated row mapped to those headers (spaces around `,` are ignored; an empty cell `, ,` evaluates to `null` / `None`). In `.rl`, a 2D Tuple literal natively constructs a typed `Tuple` of `Form` records!


section SCHEMA PORTALS & MULTI-FORMAT COMPOSITION (`RN` <-> `RL` BRIDGE)

topic Anonymous by Default
A single token before a bracket is ALWAYS the `Key` (`styles {`, `UserCard {`) and parses into raw, format-agnostic AST.

topic Explicit Schema Portal (`Key Schema {`)
A `PascalCase` token placed between `Key` and `{`, `(`, or `[` binds a schema adapter (written in `.rl`!) to that subtree (`styles Scss {`, `query Sql (`, `view Md [`).
Note: A `PascalCase` token inside a Form with no trailing bracket (`id Uuid`) is a scalar Value in `.rn` (or a Field Type signature in `.rl`), not a Portal.

topic Zero-Rewrite Data-to-Code Upgrade
Because `.rn` is Layer 0 of `.rl`, any static `.rn` file can be upgraded to a reactive `.rl` module without changing a single bracket: attaching `& [ .port > 1024 ]` adds a compile-time Noether validation invariant, and replacing a static value with `[ base.port + 10 ]` turns it into a lazy computed lens.


section ALGEBRAIC COMPOSITION (`&` MERGE, `^` XOR & `/` STRIP)

topic Deep Merge (`&`) & Symmetric Difference (`^`)
`Target & Source { ... }` deep-merges all fields from `Source` (Identifier or Dot-Path) into `Target`.
Multiple sources merge left-to-right (`Target & S1 & S2`). Omitting `{}` clones the merged result directly (`port & config.default_port`).
`Target ^ Source` retains only keys present in `Target` or `Source`, but not both.

topic Key Strip (`/`)
`Target & Source / key { ... }` (or `/ 'key`) removes `key` (or dotted `a.b`) from the inherited Form.
Multiple keys can be stripped sequentially: `Target & Source / key1 / key2`.
Note: Operators ` & `, ` ^ `, and ` / ` MUST always be surrounded by spaces (unspaced `/app` or `a/b` is a single string token).


section LEXICON, SCALARS, TYPED LITERALS & COLUMN-0 STRINGS

topic Homoiconic Comments (`# `)
`# ` (a hash followed by a space, or a bare `#` at the end of a line) outside quotes and preceded by start-of-line or whitespace starts a comment until the end of the line (attached to the AST node for doc generation). Unspaced tokens like `#42` or `#111111` are treated as literal text/scalars!

topic Scalar Auto-Typing (in `{}` and `()`)
`true`, `false` -> Boolean (`Bool`)
`null` (or `None`, or bare key in data `{}`, or empty cell `, ,` in `()`) -> Null (`None`)
`8080`, `-3.14`, `0xFF`, `1e-5` -> Number (`Int` / `Float` / `Byte`; multi-dot tokens like `0.1.0` fall back to Unquoted String)
`'pk`, `'not_null`, `'GET` -> Symbol (prefix-quoted O(1) interned tag with NO closing quote; serializes to string `"pk"` in JSON targets)
`Type"..."` -> Typed Literal: A `PascalCase` type name (or lowercase format tag) attached directly to a quoted string without spaces (`Date"2026-03-30"`, `Money"199.99"`, `Url"https://razomy.org"`). In `.rl`, desugars to constructor call `Type("...")`.
Unquoted String -> In block data `{}`, any unquoted text after `Key ` up to the end of the line (or in `()` / inline `{}` up to `,`) that does not end with a space-separated block opener (`Razomy Notation`, `postgres`, `node:22-alpine`, `/app`, `8px`, `#111111`, `Orgs.id`).
`"..."` -> Quoted UTF-8 String (required for empty strings `""`, strings containing `,` in inline/1D/table mode, strings ending with a bracket, or keys starting with a dot/special char: `".btn"`). Supports `{expr}` interpolation in `.rl` and standard escapes `\"`, `\\`, `\n`, `\t`, `\{`.

topic Column-0 Multiline & Raw Strings (`(tag)?(N)?"`)
An opening `(tag)?(N)?"` at the end of a line (`"`, `ts"`, `sh"`, `Sql"`, `1"`, `2"`, `rn2"`) suspends indentation until a matching `"(N)?` (`"`, `"1`, `"2`) placed strictly alone on its own line at Column 0.
Optional `tag` (`ts`, `sh`, `json`, `Date`, `Sql`) attaches a language/type constructor; optional `N` (`1`, `2`, `3`...) sets the delimiter rank so inner quotes of lower rank at Column 0 do not close the string.
Content starts strictly at Column 0 (every leading space is 100% literal text, zero escape sequences or `{expr}` interpolations are processed).
The first `\n` after the opening quote and the last `\n` before the closing quote are stripped.


part CANONICAL EXAMPLES (VERIFYING EVERY AXIOM)

section PLAIN TEXT & OUTLINER NOTES (Stream Root + Outliner)
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

section MARKDOWN DOCUMENT (`*.md.rn` — Flat Headings, Auto-Paragraphs & Tables)
example {
 input rn2"
title Razomy Guide

Fast b[zero-noise] notation with span{class highlight}[inline] tags and `arr[0]` code.

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

section HTML DOM TREE (Attached `{Props}` + Stream `[`)
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

section JSON / YAML CONFIG (Dot-Path, Typed Literals, Form & Col-0 Strings)
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

section RELATIONAL SCHEMA & TABULAR DATA (2D Tuple & Auto-Typing)
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

section CSS / SCSS STYLES (Form & Unquoted Hex/Units)
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

section ALGEBRAIC INHERITANCE (`&` Merge & `/` Strip)
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

section MULTI-SCHEMA COMPONENT PORTALS (`Key Schema` Portals)
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