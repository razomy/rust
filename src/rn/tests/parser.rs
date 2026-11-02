use razomy_rn::*;
use serde_json::{json, Value};

#[test]
fn test_spec_file_notaion_rl_parses_cleanly() {
    let spec_src = include_str!("../specification.rn");
    let doc = parse_rn(spec_src);
    assert!(
        doc.diagnostics.is_empty(),
        "Unexpected diagnostics in notaion.rl: {:?}",
        doc.diagnostics
    );
    assert_eq!(doc.root_mode, lexer::ContextMode::Stream);
    let json_val = doc.root.to_canonical_json();
    let arr = json_val.as_array().expect("Root of specification.rn is a Stream []");
    assert!(!arr.is_empty());

    // First item in specification.rn is `meta { ... }`
    let meta = &arr[0]["meta"];
    assert_eq!(meta["name"], "Razomy Notation");
    assert_eq!(meta["version"], "0.1.0");
    assert_eq!(meta["extension"], ".rn");
    assert_eq!(meta["schema"], "Md");
    assert!(meta["tags"].is_array());
    assert_eq!(meta["tags"][0], "ONTOLOGICAL-DATA");
}

#[test]
fn test_unified_node_pest_grammar_recognizes_all_constructs() {
    let samples = [
        "# Top-level comment",
        "article{id post, class featured, lang en} [",
        "Button{variant 'primary}(label) [",
        "p [ Fast b[zero-noise] rn with a{href /docs}[links] and `arr[0]` ]",
        "port 8080",
        "prod & base / debug {",
        "query Sql (",
        "styles Scss {",
        "created_at Date\"12.12.23\"",
    ];
    for line in samples {
        assert!(
            parser::parse_unified_line_ast(line),
            "Failed to parse unified_node line: {}",
            line
        );
    }
}

#[test]
fn test_canonical_example_1_plain_text_and_outliner_notes() {
    let rn_src = r#"how my day?
 good
todo list
 a
 b
 c
"#;
    let doc = parse_rn(rn_src);
    assert!(doc.diagnostics.is_empty());
    let json_val = doc.root.to_canonical_json();
    let expected = json!([
        { "text": "how my day?", "children": ["good"] },
        { "text": "todo list", "children": ["a", "b", "c"] }
    ]);
    assert_eq!(json_val, expected);
}

#[test]
fn test_canonical_example_2_markdown_document_flat_headings_tables_and_col0_code() {
    let rn_src = r#"title Razomy Guide

Fast b[zero-noise] rn a(/docs)[links], with span{class highlight}[inline] tags and `arr[0]` code.

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
"#;
    let doc = parse_rn(rn_src);
    assert!(doc.diagnostics.is_empty());
    let md = generate(&doc, Format::Markdown).unwrap();

    let expected_md = r#"# Razomy Guide

Fast **zero-noise** rn [links](/docs), with <span class="highlight">inline</span> tags and `arr[0]` code.

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
"#;
    assert_eq!(md, expected_md);
}

#[test]
fn test_canonical_example_3_html_dom_tree_attached_props() {
    let rn_src = r#"article{id post, class featured, lang en} [
 h1 [ Why Razomy? ]
 p [ Read the a{href /docs, target _blank}[documentation]. ]
"#;
    let doc = parse_rn(rn_src);
    assert!(doc.diagnostics.is_empty());
    let html = generate(&doc, Format::Html).unwrap();
    let expected_html = r#"<article id="post" class="featured" lang="en">
  <h1>Why Razomy?</h1>
  <p>Read the <a href="/docs" target="_blank">documentation</a>.</p>
</article>
"#;
    assert_eq!(html, expected_html);

    // Import HTML back into RN
    let imported = import_from(&html, Format::Html).unwrap();
    let rn_roundtrip = generate(&imported, Format::Rn).unwrap();
    assert!(rn_roundtrip.contains("article{id post, class featured, lang en}"));
    assert!(rn_roundtrip.contains("a{href /docs, target _blank}[documentation]"));
}

#[test]
fn test_canonical_example_4_json_yaml_config_dot_path_typed_literals_and_col0_strings() {
    let rn_src = r#"cluster.primary.database {
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
"#;
    let doc = parse_rn(rn_src);
    assert!(doc.diagnostics.is_empty());
    let json_val = doc.root.to_canonical_json();
    let expected = json!({
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
    });
    assert_eq!(json_val, expected);

    // Export to YAML and import back
    let yaml_str = generate(&doc, Format::Yaml).unwrap();
    let from_yaml_doc = import_from(&yaml_str, Format::Yaml).unwrap();
    assert_eq!(from_yaml_doc.root.to_canonical_json(), expected);
}

#[test]
fn test_canonical_example_5_relational_schema_and_tabular_data_sql() {
    let rn_src = r#"users_schema (
 column, type, constraint, default
 id,     Uuid, 'pk,        gen_uuid
 org_id, Uuid, Orgs.id,    null
 role,   Text, 'not_null,  member

users_rows (
 id,  org_id, role,   created_at
 u_1, o_10,   admin,  Date"12.12.23"
 u_2, o_10,   member, Date"13.12.23"
"#;
    let doc = parse_rn(rn_src);
    assert!(doc.diagnostics.is_empty());
    let sql = generate(&doc, Format::Sql).unwrap();
    let expected_sql = r#"CREATE TABLE users (
  id     UUID PRIMARY KEY DEFAULT gen_uuid(),
  org_id UUID REFERENCES Orgs(id) DEFAULT NULL,
  role   TEXT NOT NULL DEFAULT 'member'
);
INSERT INTO users (id, org_id, role, created_at) VALUES
  ('u_1', 'o_10', 'admin', '12.12.23'),
  ('u_2', 'o_10', 'member', '13.12.23');
"#;
    assert_eq!(sql, expected_sql);
}

#[test]
fn test_canonical_example_6_css_scss_styles() {
    let rn_src = r#"".btn" {
 background-color #111111
 padding (8px, 16px)
 "&:hover" { background-color #222222 }
"#;
    let doc = parse_rn(rn_src);
    assert!(doc.diagnostics.is_empty());
    let scss = generate(&doc, Format::Scss).unwrap();
    let expected_scss = r#".btn {
  background-color: #111111;
  padding: 8px 16px;
  &:hover {
    background-color: #222222;
  }
}
"#;
    assert_eq!(scss, expected_scss);
}

#[test]
fn test_canonical_example_7_docker_and_terraform_infrastructure() {
    let rn_src = r#"container {
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
"#;
    let doc = parse_rn(rn_src);
    assert!(doc.diagnostics.is_empty());
    let tf = generate(&doc, Format::Terraform).unwrap();
    let expected_tf = r#"# Dockerfile:
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
"#;
    assert_eq!(tf, expected_tf);
}

#[test]
fn test_canonical_example_8_algebraic_inheritance_merge_xor_and_strip() {
    let rn_src = r#"mesh {
 ingress {
  to (auth, api)
  protocol grpc
  tls true
 base { timeout 30, retries 3, debug true }
 prod & base / debug {
  replicas 5
 backup_timeout & base.timeout
 xor_test & base ^ prod
"#;
    let doc = parse_rn(rn_src);
    assert!(doc.diagnostics.is_empty());
    let json_val = doc.root.to_canonical_json();

    assert_eq!(
        json_val["mesh"]["ingress"],
        json!({ "to": ["auth", "api"], "protocol": "grpc", "tls": true })
    );
    assert_eq!(
        json_val["mesh"]["base"],
        json!({ "timeout": 30, "retries": 3, "debug": true })
    );
    assert_eq!(
        json_val["mesh"]["prod"],
        json!({ "timeout": 30, "retries": 3, "replicas": 5 })
    );
    assert_eq!(json_val["mesh"]["backup_timeout"], 30);
    // `base ^ prod` retains only keys in base or prod, not both (`debug` from base, `replicas` from prod)
    assert_eq!(
        json_val["mesh"]["xor_test"],
        json!({ "debug": true, "replicas": 5 })
    );
}

#[test]
fn test_canonical_example_9_multi_schema_component_portals() {
    let rn_src = r#"UserCard {
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
"#;
    let doc = parse_rn(rn_src);
    assert!(doc.diagnostics.is_empty());
    let json_val = doc.root.to_canonical_json();
    let expected = json!({
        "UserCard": {
            "props": { "id": "Uuid", "theme": "dark" },
            "query": "SELECT * FROM users WHERE id = :id;",
            "view": "<div class=\"card\"><h2>User Profile</h2><p>Welcome back!</p></div>",
            "styles": ".card { background-color: #111111; }\n.card:hover { background-color: #222222; }"
        }
    });
    assert_eq!(json_val, expected);
}

#[test]
fn test_scalar_auto_typing_hex_scientific_symbols_and_empty_cells() {
    let rn_src = r#"config {
 zip_code 01234
 semver 0.1.0
 port 8080
 ratio -3.14
 hex_val 0xFF
 sci_val 1e-5
 tag 'pk
 none_val None
 sparse_row (1, , 3)
"#;
    let doc = parse_rn(rn_src);
    let json_val = doc.root.to_canonical_json();
    assert_eq!(json_val["config"]["zip_code"], "01234");
    assert_eq!(json_val["config"]["semver"], "0.1.0");
    assert_eq!(json_val["config"]["port"], 8080);
    assert_eq!(json_val["config"]["ratio"], -3.14);
    assert_eq!(json_val["config"]["hex_val"], 255);
    assert_eq!(json_val["config"]["sci_val"], 1e-5);
    assert_eq!(json_val["config"]["tag"], "pk");
    assert_eq!(json_val["config"]["none_val"], Value::Null);
    assert_eq!(json_val["config"]["sparse_row"], json!([1, null, 3]));
}

#[test]
fn test_lsp_hover_symbols_and_goto_definition() {
    let rn_src = r#"mesh {
 Base { timeout 30 }
 Prod & .Base { replicas 3 }
"#;
    let mut lsp = LspServer::new();
    let diags = lsp.update_document("file:///test.rn", rn_src.to_string());
    assert!(diags.is_empty());

    let syms = lsp.document_symbols("file:///test.rn");
    assert!(syms.len() >= 3);

    // Go-to-definition on `.Base` on line 2
    let loc = lsp
        .goto_definition(
            "file:///test.rn",
            lsp::LspPosition {
                line: 2,
                character: 10,
            },
        )
        .expect("Should find Base definition");
    assert_eq!(loc.range.start.line, 1);
}
