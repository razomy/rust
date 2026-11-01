use razomy_notation::*;
use serde_json::json;

#[test]
fn test_spec_file_notaion_rl_parses_cleanly() {
    let spec_src = include_str!("../notaion.rl");
    let doc = parse_rn(spec_src);
    assert!(
        doc.diagnostics.is_empty(),
        "Unexpected diagnostics in notaion.rl: {:?}",
        doc.diagnostics
    );
    let json_val = doc.root.to_canonical_json();
    assert_eq!(json_val["name"], "Razomy Notation");
    assert_eq!(json_val["version"], "13.0");
    assert_eq!(json_val["extension"], ".rn");
    assert!(json_val["tags"].is_array());
    assert!(json_val["description"]
        .as_str()
        .unwrap()
        .contains("PART I. THE 5 ONTOLOGICAL AXIOMS"));
}

#[test]
fn test_v13_unified_node_pest_grammar_recognizes_all_constructs() {
    let samples = [
        "# Top-level comment",
        "article#main { lang en } [ Hello b[world] ]",
        "p [ Fast b[zero-noise] notation with a{href /docs}[links] and `arr[0]` ]",
        "port 8080",
        "Ingress -> (Auth, Api) { protocol grpc, tls true }",
        "..resource.aws_s3_bucket.logs { enabled true }",
        "Prod & .Base / debug { replicas 5 }",
        "container(node:22-alpine) { workdir /app }",
    ];
    for line in samples {
        assert!(
            parser::parse_unified_line_ast(line),
            "Failed to parse v13 unified_node line: {}",
            line
        );
    }
}

#[test]
fn test_example_1_markdown_replacement_and_codegen() {
    let rn_src = r#"h1 [ Razomy Guide ]
p [ Fast b[zero-noise] notation with i[inline] tags and `arr[0]` code. ]
ul [
 No closing tags
 1-space indentation
benchmarks (
 Format, Tokens, Notes
 JSON,   140,    "Verbose, requires quotes"
 RN,     62,     "Compact, zero-noise"
snippet "ts
 const x = 42;
"#;
    let doc = parse_rn(rn_src);
    let md = generate(&doc, Format::Markdown).unwrap();
    assert!(md.contains("# Razomy Guide"));
    assert!(md.contains("Fast **zero-noise** notation with *inline* tags and `arr[0]` code."));
    assert!(md.contains("- No closing tags"));
    assert!(md.contains("| JSON   | 140    | Verbose, requires quotes |"));
    assert!(md.contains("```ts\nconst x = 42;\n```"));

    // Roundtrip import from Markdown
    let imported = import_from(&md, Format::Markdown).unwrap();
    let rn_out = generate(&imported, Format::Rn).unwrap();
    assert!(rn_out.contains("h1 [ Razomy Guide ]"));
    assert!(rn_out.contains("const x = 42;"));
}

#[test]
fn test_example_2_html_dom_bijection() {
    let rn_src = r#"article#post { lang en } [
 h1 [ Why Razomy? ]
 p [ Read the a{href /docs, target _blank}[documentation]. ]
"#;
    let doc = parse_rn(rn_src);
    let html = generate(&doc, Format::Html).unwrap();
    assert!(html.contains("<article id=\"post\" lang=\"en\">"));
    assert!(html.contains("<h1>Why Razomy?</h1>"));
    assert!(html.contains("<p>Read the <a href=\"/docs\" target=\"_blank\">documentation</a>.</p>"));

    // Import HTML back into RN
    let imported = import_from(&html, Format::Html).unwrap();
    let rn_roundtrip = generate(&imported, Format::Rn).unwrap();
    assert!(rn_roundtrip.contains("article#post"));
    assert!(rn_roundtrip.contains("a{href /docs, target _blank}[documentation]"));
}

#[test]
fn test_example_3_yaml_json_config_and_context_inertia() {
    let rn_src = r#"cluster {
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
"#;
    let doc = parse_rn(rn_src);
    let json_val = doc.root.to_canonical_json();
    let expected = json!({
        "cluster": {
            "primary": {
                "database": {
                    "engine": "postgres",
                    "port": 5432,
                    "token": null,
                    "empty_str": "",
                    "extra_tags": [],
                    "steps": [
                        "1. Check replication lag",
                        "2. Promote primary node"
                    ]
                }
            }
        }
    });
    assert_eq!(json_val, expected);

    // Export to YAML & JSON5 and import back
    let yaml_str = generate(&doc, Format::Yaml).unwrap();
    let from_yaml_doc = import_from(&yaml_str, Format::Yaml).unwrap();
    assert_eq!(from_yaml_doc.root.to_canonical_json(), expected);
}

#[test]
fn test_example_4_sql_ddl_and_proto_codegen_and_import() {
    let rn_src = r#"Users (
 col,    type, ref,        default
 id,     uuid, pk,         gen_uuid()
 org_id, uuid, -> Orgs.id, null
 role,   text, not_null,   member
"#;
    let doc = parse_rn(rn_src);
    let sql = generate(&doc, Format::Sql).unwrap();
    assert!(sql.contains("CREATE TABLE Users"));
    assert!(sql.contains("id UUID PRIMARY KEY DEFAULT gen_uuid()"));
    assert!(sql.contains("org_id UUID REFERENCES Orgs(id) DEFAULT NULL"));
    assert!(sql.contains("role TEXT NOT NULL DEFAULT 'member'"));

    // Import SQL back into RN
    let imported_sql = import_from(&sql, Format::Sql).unwrap();
    let rn_from_sql = generate(&imported_sql, Format::Rn).unwrap();
    assert!(rn_from_sql.contains("-> Orgs.id"));
    assert!(rn_from_sql.contains("gen_uuid()"));

    // Generate Protobuf v3 schema from the same table
    let proto = generate(&doc, Format::Proto).unwrap();
    assert!(proto.contains("syntax = \"proto3\";"));
    assert!(proto.contains("message Users"));
    assert!(proto.contains("string id = 1;"));
}

#[test]
fn test_example_5_css_literal_hex_and_leading_dot() {
    let rn_src = r#"styles {
 .btn {
  background #111111
  padding (8px, 16px)
  &:hover { background #222222 }
"#;
    let doc = parse_rn(rn_src);
    let json_val = doc.root.to_canonical_json();
    assert_eq!(json_val["styles"][".btn"]["background"], "#111111");
    assert_eq!(
        json_val["styles"][".btn"]["padding"],
        json!(["8px", "16px"])
    );
    assert_eq!(
        json_val["styles"][".btn"]["&:hover"]["background"],
        "#222222"
    );
}

#[test]
fn test_example_6_dockerfile_parameterized_triad_and_standalone_dot() {
    let rn_src = r#"container(node:22-alpine) {
 workdir /app
 copy (., /app)
 run "sh
  npm ci --omit=dev
 env { NODE_ENV production, PORT 8080 }
 cmd (node, server.js)
"#;
    let doc = parse_rn(rn_src);
    let json_val = doc.root.to_canonical_json();
    assert_eq!(json_val["container"]["()"], json!(["node:22-alpine"]));
    assert_eq!(json_val["container"]["copy"], json!([".", "/app"]));
    assert_eq!(json_val["container"]["run"], "npm ci --omit=dev");
    assert_eq!(json_val["container"]["env"]["PORT"], 8080);
}

#[test]
fn test_example_7_terraform_and_zero_drift_margin_reset() {
    let rn_src = r#"resource(aws_s3_bucket, logs) {
 bucket my-corp-logs
 acl private

..resource.aws_s3_bucket.logs {
 lifecycle_rule {
  enabled true
  expiration { days 90 }
"#;
    let doc = parse_rn(rn_src);
    let json_val = doc.root.to_canonical_json();
    let logs = &json_val["resource"]["aws_s3_bucket"]["logs"];
    assert_eq!(logs["bucket"], "my-corp-logs");
    assert_eq!(logs["acl"], "private");
    assert_eq!(logs["lifecycle_rule"]["enabled"], true);
    assert_eq!(logs["lifecycle_rule"]["expiration"]["days"], 90);
}

#[test]
fn test_example_8_mermaid_hyperedges_and_algebraic_overlays() {
    let rn_src = r#"mesh {
 Ingress -> (Auth, Api) { protocol grpc, tls true }
 Base { timeout 30, retries 3, debug true }
 Prod & .Base / debug {
  replicas 5
"#;
    let doc = parse_rn(rn_src);
    assert!(doc.diagnostics.is_empty());
    let json_val = doc.root.to_canonical_json();

    // Check algebraic overlay `Prod & .Base / debug`
    let prod = &json_val["mesh"]["Prod"];
    assert_eq!(prod["timeout"], 30);
    assert_eq!(prod["retries"], 3);
    assert_eq!(prod["replicas"], 5);
    assert!(
        prod.get("debug").is_none(),
        "debug key should be omitted by `/ debug`"
    );

    // Check Mermaid flowchart generation
    let mermaid = generate(&doc, Format::Mermaid).unwrap();
    assert!(mermaid.contains("Ingress -->|\"protocol: grpc, tls: true\"| Auth"));
    assert!(mermaid.contains("Ingress -->|\"protocol: grpc, tls: true\"| Api"));

    // Import Mermaid back
    let imported_mmd = import_from(&mermaid, Format::Mermaid).unwrap();
    let mmd_json = imported_mmd.root.to_canonical_json();
    assert!(mmd_json["graph"]["->"].is_array());
}

#[test]
fn test_safe_auto_typing_and_reserved_key_escaping() {
    let rn_src = r#"config {
 zip_code 01234
 semver 1.2.0
 port 8080
 ratio -3.14
"(())" literal_parens
"#;
    let doc = parse_rn(rn_src);
    let json_val = doc.root.to_canonical_json();
    assert_eq!(json_val["config"]["zip_code"], "01234");
    assert_eq!(json_val["config"]["semver"], "1.2.0");
    assert_eq!(json_val["config"]["port"], 8080);
    assert_eq!(json_val["config"]["ratio"], -3.14);
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
