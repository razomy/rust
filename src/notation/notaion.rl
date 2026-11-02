name "Razomy Notation"
version "16.4"
extension ".rn"
tags (
 "ONTOLOGICAL-DATA"
 "RL-ISOMORPHIC"
 "SCHEMA-AGNOSTIC-CORE"
 "SCHEMA-PORTALS"
 "SYMMETRIC-TRIAD"
 "INDENT-TOPOLOGICAL"
 "COLUMN-ZERO-STRINGS"
description r3"
================================================================================
PART I. AXIOMATIC SPECIFICATION (FROM ROOT TO LEAVES)
================================================================================

0. THE ROOT AXIOM (ONE UNIVERSAL NODE GRAMMAR)
--------------------------------------------------------------------------------
The root of an (.rn) or (.rl) file is an implicit Form `{}` at indentation level 0.
Every statement in a Form `{}` or Stream `[]` is a single canonical Node
(whereas Tuple `()` elements are bare `Payload` / `Value` items without a `Key`):

  Key (& Source (/ Key)*)* (Schema)? ({Props})? ( {Form} | (Tuple) | [Stream] | Value )?

  • Key       : Identifier (`[a-zA-Z_][a-zA-Z0-9_]*`), Dot-Path (`a.b.c`), or `"quoted"`.
  • Algebra   : Optional inheritance `& Source` and key stripping `/ Key` (see Rule 4).
  • Schema    : Optional `PascalCase` adapter before `{`, `(`, or `[` (`Sql`, `Html`, `Scss`).
  • {Props}   : Optional inline metadata Form attached to `Key` (`div{id main} [...]`).
  • Payload   : One of the 3 structural modes (`{}`, `()`, `[]`), a scalar `Value`, or empty.


1. TOPOLOGY AXIOM (INLINE VS. BLOCK — ONE RULE FOR ALL BRACKETS)
--------------------------------------------------------------------------------
1.1. Inline Mode (Same-Line Close):
     If `{`, `(`, or `[` opens and closes on the same line, elements in `{}` and `()` are
     separated by `,` (surrounding spaces are ignored). Example: `env { A 1, B 2 }`, `pt (10, 20)`.

1.2. Block Mode (End-of-Line Open, Auto-Close on Dedent):
     If `{`, `(`, or `[` is the LAST token on a line, it opens an indented block:
     • 1 leading space (` `) = 1 indentation level.
     • Dedenting automatically closes the block. Explicit closing brackets and trailing
       end-of-line commas are NEVER used.

1.3. Dot-Path Flattening (`a.b.c`):
     A dotted key `a.b.c {` expands into `{ a { b { c { ... } } } }` using only a SINGLE
     1-space indent on the next line.


2. THE 3 STRUCTURAL MODES (`{}` FORM, `()` TUPLE, `[]` STREAM)
--------------------------------------------------------------------------------
2.1. `{` Form Mode (Key-Value Map):
     • Maps `Key Payload` statements separated by `\n` (block) or `,` (inline).
     • Bare `Key` with no payload and no `& Source` = `null`.
     • Empty `{}` = empty map.

2.2. `(` Tuple Mode (1D Lists & 2D Tables):
     • Empty `()` = empty list.
     • 1D List  : Inline `(a, b, c)` OR block `(\n` where indented lines contain NO top-level
                  commas outside quotes/brackets (one item per line, like `tags (` above).
     • 2D Table : Block `(\n` where indented lines contain top-level `,`-separated cells
                  (surrounding spaces around `,` are ignored; each line is a row).

2.3. `[` Stream Mode (Free Text & Markup Trees):
     • Empty `[]` = empty stream/list.
     • Whitespace : In inline `[ text ]`, a single leading space after `[` and a single
                    trailing space before `]` are automatically trimmed.
     • Escaping   : Raw text flows without quotes. Literal brackets are escaped via code spans
                    `` `arr[0]` `` or quotes `"[raw]"`, where outer quotes are stripped on unescape.
     • Inline Tags: Inside a text flow, child nodes attach WITHOUT a space before `{` or `[`:
                    `b[bold]`, `span{class hi}[text]`, `a{href "/docs"}[link]`.
     • Block Mode : Inside a multiline `[\n`, any indented line ending with `[`, `{`, or `(`
                    opens a nested child node; plain lines become a list of text/paragraph items.


3. SCHEMA PORTALS & MULTI-FORMAT COMPOSITION
--------------------------------------------------------------------------------
3.1. Anonymous by Default:
     Nodes without a Schema (`styles { ... }`) parse into raw, format-agnostic AST.

3.2. Explicit Schema Portal:
     A `PascalCase` token immediately preceding `{`, `(`, or `[` binds a schema adapter
     to that subtree (`styles Scss {`, `query Sql (`, `view Html [`).
     Note: A `PascalCase` token at the end of a line (`id Uuid`) is a scalar Value, not a Portal.

3.3. File-Level Schema:
     Compound extensions (`*.css.rn`, `*.sql.rn`) bind a root Schema Portal to the whole file.


4. ALGEBRAIC COMPOSITION (`&` MERGE & `/` STRIP)
--------------------------------------------------------------------------------
4.1. Deep Merge (`&`):
     `Target & Source { ... }` deep-merges all fields from `Source` into `Target`.
     Multiple sources merge left-to-right (`Target & S1 & S2`). Omitting `{}` clones the merged result.

4.2. Key Strip (`/`):
     `Target & Source / key { ... }` removes `key` (or dotted `a.b`) from the inherited Form.
     Multiple keys can be stripped sequentially: `Target & Source / key1 / key2`.


5. LEXICON, SCALARS & COLUMN-0 STRINGS
--------------------------------------------------------------------------------
5.1. Comments:
     `#` outside quotes starts a comment until the end of the line (`# text`).

5.2. Scalar Auto-Typing (in `{}` and `()`):
     • `true`, `false`            -> Boolean
     • `null` (or bare key)       -> Null
     • `8080`, `-3.14`            -> Number
     • `'pk`, `'not_null`         -> Symbol (prefix-quoted tag)
     • Identifier / Dot-Ref       -> Bare String (`postgres`, `u_1`, `_blank`, `Uuid`, `Orgs.id`)
     • `"..."`                    -> Quoted String (required for special chars: `".btn"`, `"/app"`, `"#fff"`)

5.3. Column-0 Multiline & Raw Strings (`(tag)?(N)?"`):
     • Opening `(tag)?(N)?"` at the end of a line (`"`, `ts"`, `sh"`, `r3"`) suspends indentation
       until a matching `"(N)?` (`"`, `"3`) at Column 0 on its own line.
     • Content starts strictly at Column 0 (every leading space is 100% literal text).
     • The first `\n` after the opening quote and the last `\n` before the closing quote are stripped.


================================================================================
PART II. CANONICAL EXAMPLES (VERIFYING EVERY AXIOM)
================================================================================

--------------------------------------------------------------------------------
EXAMPLE 1. MARKDOWN DOCUMENT (Rules 2.2, 2.3, 5.3)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  h1 [ Razomy Guide ]
  p [ Fast b[zero-noise] notation with span{class highlight}[inline] tags and `arr[0]` code. ]
  ul [
   No closing tags
   1-space indentation
  benchmarks (
   Format, Tokens, Notes
   JSON,   140,    "Verbose, requires quotes"
   RN,     48,     "Ultra-compact, lean parser"
  snippet ts"
const x = 42;
function hello() {
  return x;
}
"

• Lowered Markdown (.md):
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

--------------------------------------------------------------------------------
EXAMPLE 2. HTML DOM TREE (Rule 0 Props `{}` + Rule 2.3 Stream `[`)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  article{id post, class featured, lang en} [
   h1 [ Why Razomy? ]
   p [ Read the a{href "/docs", target _blank}[documentation]. ]

• Lowered HTML (.html):
  <article id="post" class="featured" lang="en">
    <h1>Why Razomy?</h1>
    <p>Read the <a href="/docs" target="_blank">documentation</a>.</p>
  </article>

--------------------------------------------------------------------------------
EXAMPLE 3. JSON / YAML CONFIG (Rules 1.3 Dot-Path, 2.1 Form, 2.3 Block Stream, 5.3 Col-0)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  cluster.primary.database {
   engine postgres
   port 5432
   token
   empty_str ""
   extra_tags ()
   welcome_msg "
Line 1 starts at Column 0 (zero ambiguity with parent indentation!).
  Line 2 has exactly 2 leading spaces of actual text content.
"
   steps [
    1. Check replication lag
    2. Promote primary node

• Lowered JSON (.json):
  {
    "cluster": {
      "primary": {
        "database": {
          "engine": "postgres",
          "port": 5432,
          "token": null,
          "empty_str": "",
          "extra_tags": [],
          "welcome_msg": "Line 1 starts at Column 0 (zero ambiguity with parent indentation!).\n  Line 2 has exactly 2 leading spaces of actual text content.",
          "steps": [
            "1. Check replication lag",
            "2. Promote primary node"
          ]
        }
      }
    }
  }

--------------------------------------------------------------------------------
EXAMPLE 4. RELATIONAL SCHEMA & TABULAR DATA (Rule 2.2 2D Tuple & Rule 5.2)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  users_schema (
   column, type, constraint, default
   id,     Uuid, 'pk,        gen_uuid
   org_id, Uuid, Orgs.id,    null
   role,   Text, 'not_null,  member

  users_rows (
   id,  org_id, role
   u_1, o_10,   admin
   u_2, o_10,   member

• Lowered SQL (.sql):
  CREATE TABLE users (
    id     UUID PRIMARY KEY DEFAULT gen_uuid(),
    org_id UUID REFERENCES Orgs(id) DEFAULT NULL,
    role   TEXT NOT NULL DEFAULT 'member'
  );
  INSERT INTO users (id, org_id, role) VALUES
    ('u_1', 'o_10', 'admin'),
    ('u_2', 'o_10', 'member');

--------------------------------------------------------------------------------
EXAMPLE 5. CSS / SCSS STYLES (Rule 2.1 Form & Rule 5.2 Quoted Selectors)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  styles {
   ".btn" {
    background "#111111"
    padding ("8px", "16px")
    "&:hover" { background "#222222" }

• Lowered SCSS (.scss):
  .btn {
    background: #111111;
    padding: 8px 16px;
    &:hover {
      background: #222222;
    }
  }

--------------------------------------------------------------------------------
EXAMPLE 6. DOCKER & TERRAFORM INFRASTRUCTURE (Rules 1.3, 2.1, 5.3)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  container {
   from "node:22-alpine"
   workdir "/app"
   copy (".", "/app")
   run sh"
npm ci --omit=dev
"
   env { NODE_ENV production, PORT 8080 }
   cmd (node, "server.js")

  resource.aws_s3_bucket.logs {
   bucket "my-corp-logs"
   acl private
   lifecycle_rule.expiration {
    enabled true
    days 90

• Lowered Dockerfile & Terraform (.tf):
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

--------------------------------------------------------------------------------
EXAMPLE 7. ALGEBRAIC INHERITANCE (Rule 4.1 `&` Merge & Rule 4.2 `/` Strip)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
  mesh {
   ingress {
    to (auth, api)
    protocol grpc
    tls true
   base { timeout 30, retries 3, debug true }
   prod & base / debug {
    replicas 5

• Lowered Output (Resolved JSON):
  {
    "mesh": {
      "ingress": { "to": ["auth", "api"], "protocol": "grpc", "tls": true },
      "base":    { "timeout": 30, "retries": 3, "debug": true },
      "prod":    { "timeout": 30, "retries": 3, "replicas": 5 }
    }
  }

--------------------------------------------------------------------------------
EXAMPLE 8. MULTI-SCHEMA COMPONENT PORTALS (Rule 3.2 Nested `PascalCase` Portals)
--------------------------------------------------------------------------------
• Razomy Notation (.rn):
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
     background "#111111"
     "&:hover" { background "#222222" }
"3