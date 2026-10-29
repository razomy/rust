use razomy_language::parser::{parse_code, Literal, Token};

#[test]
fn micro_lit_int_positive_and_negative() {
    let ast = parse_code("a 42\nb -17\nc 0").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Int(42)));
    assert_eq!(ast[1][1], Token::Lit(Literal::Int(-17)));
    assert_eq!(ast[2][1], Token::Lit(Literal::Int(0)));
}

#[test]
fn micro_lit_float() {
    let ast = parse_code("pi 3.14159\nzero 0.0\nneg -1.25\ntax_rate 0.155").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Float(3.14159)));
    assert_eq!(ast[1][1], Token::Lit(Literal::Float(0.0)));
    assert_eq!(ast[2][1], Token::Lit(Literal::Float(-1.25)));
    assert_eq!(ast[3][1], Token::Lit(Literal::Float(0.155)));
}

#[test]
fn micro_lit_bool() {
    let ast = parse_code("t true\nf false").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Bool(true)));
    assert_eq!(ast[1][1], Token::Lit(Literal::Bool(false)));
}

#[test]
fn micro_lit_hex_byte() {
    let ast = parse_code("b1 0x00\nb2 0xFF\nb3 0x1A").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Byte(0)));
    assert_eq!(ast[1][1], Token::Lit(Literal::Byte(255)));
    assert_eq!(ast[2][1], Token::Lit(Literal::Byte(26)));
}

#[test]
fn micro_lit_string_and_escaped_braces() {
    let ast = parse_code(
        r#"s1 "hello"
s2 "\"quoted\""
s3 "\{ {inner} \}""#,
    )
        .unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::String("hello".into())));
    assert_eq!(ast[1][1], Token::Lit(Literal::String("\\\"quoted\\\"".into())));
    assert_eq!(ast[2][1], Token::Lit(Literal::String("\\{ {inner} \\}".into())));
}

#[test]
fn micro_lit_raw_strings_r_r1_r2() {
    let code = r##"
regex_pat   r"^[^@]+@[^@]+\.[^@]+$"
nested_json r1"{"query": "SELECT * FROM \"users\"", "interpolated": "{ignored}"}"1
nested_code r2"Can freely embed both " and "1 and # without any lexer conflict!"2
"##;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 3);
    assert_eq!(
        ast[0][1],
        Token::Lit(Literal::RawString(r#"^[^@]+@[^@]+\.[^@]+$"#.into()))
    );
    assert_eq!(
        ast[1][1],
        Token::Lit(Literal::RawString(
            r#"{"query": "SELECT * FROM \"users\"", "interpolated": "{ignored}"}"#.into()
        ))
    );
    assert_eq!(
        ast[2][1],
        Token::Lit(Literal::RawString(
            r#"Can freely embed both " and "1 and # without any lexer conflict!"#.into()
        ))
    );
}

#[test]
fn micro_lit_symbol() {
    let ast = parse_code("sym 'draft'\nmethod 'GET'").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Symbol("draft".into())));
    assert_eq!(ast[1][1], Token::Lit(Literal::Symbol("GET".into())));
}

#[test]
fn micro_lit_tagged_string_sugar_equivalence() {
    // In Razomy v5.7, Type"..." is 100% syntax sugar for the constructor call Type("...")!
    let code = r##"
money_explicit    Money("199.99"),        money_val    Money"199.99"
currency_explicit Usd("500.00"),          currency_val Usd"500.00"
date_explicit     Date("2026-03-30"),     date_val     Date"2026-03-30"
time_explicit     Time("14:30:00"),       time_val     Time"14:30:00"
duration_explicit Duration("5m30s"),      duration_val Duration"5m30s"
coord_explicit    Pair("1920x1080"),      coord_pair   Pair"1920x1080"
color_explicit    Hex("#2A9D8F"),         color_val    Hex"#2A9D8F"
ip_explicit       Ip("192.168.0.1:8080"), ip_addr      Ip"192.168.0.1:8080"
version_explicit  Version("1.4.2-beta"),  version_val  Version"1.4.2-beta"
url_explicit      Url("https://razomy.org"), web_url   Url"https://razomy.org"
        path_explicit     Path("./data/users.json"), local_file Path"./data/users.json"
    "##;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 22);

    // Every pair (explicit constructor vs tagged string sugar) must produce identical AST tokens
    for pair_idx in (0..22).step_by(2) {
        let explicit_tokens = &ast[pair_idx][1..];
        let sugar_tokens = &ast[pair_idx + 1][1..];
        assert_eq!(
            explicit_tokens, sugar_tokens,
            "Tagged string sugar must desugar identically to explicit constructor at index {}",
            pair_idx
        );
    }
}