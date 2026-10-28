use razomy_language::parser::{parse_code, Literal, Token};

#[test]
fn micro_lit_int_positive_and_negative() {
    let ast = parse_code("A 42\nB -17\nC 0").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Int(42)));
    assert_eq!(ast[1][1], Token::Lit(Literal::Int(-17)));
    assert_eq!(ast[2][1], Token::Lit(Literal::Int(0)));
}

#[test]
fn micro_lit_float() {
    let ast = parse_code("Pi 3.14159\nZero 0.0\nNeg -1.25").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Float(3.14159)));
    assert_eq!(ast[1][1], Token::Lit(Literal::Float(0.0)));
    assert_eq!(ast[2][1], Token::Lit(Literal::Float(-1.25)));
}

#[test]
fn micro_lit_bool() {
    let ast = parse_code("T true\nF false").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Bool(true)));
    assert_eq!(ast[1][1], Token::Lit(Literal::Bool(false)));
}

#[test]
fn micro_lit_hex_byte_vs_coord() {
    // Regression test for Bug #1: 0x00 must be Byte(0), not Coord("0x00")
    let ast = parse_code("B1 0x00\nB2 0xFF\nB3 0x1A\nRes 1920x1080\nSmall 200x100").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Byte(0)));
    assert_eq!(ast[1][1], Token::Lit(Literal::Byte(255)));
    assert_eq!(ast[2][1], Token::Lit(Literal::Byte(26)));
    assert_eq!(ast[3][1], Token::Lit(Literal::Coord("1920x1080".into())));
    assert_eq!(ast[4][1], Token::Lit(Literal::Coord("200x100".into())));
}

#[test]
fn micro_lit_string_and_escaped_braces() {
    let ast = parse_code(r#"S1 "hello"
S2 "\"quoted\""
S3 "\{ {Inner} \}""#).unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::String("hello".into())));
    assert_eq!(ast[1][1], Token::Lit(Literal::String("\\\"quoted\\\"".into())));
    assert_eq!(ast[2][1], Token::Lit(Literal::String("\\{ {Inner} \\}".into())));
}

#[test]
fn micro_lit_symbol() {
    let ast = parse_code("Sym 'draft'\nMethod 'GET'").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Symbol("draft".into())));
    assert_eq!(ast[1][1], Token::Lit(Literal::Symbol("GET".into())));
}

#[test]
fn micro_lit_money() {
    let ast = parse_code("M1 $0.00\nM2 $199.99\nM3 USD$500.00").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Money("$0.00".into())));
    assert_eq!(ast[1][1], Token::Lit(Literal::Money("$199.99".into())));
    assert_eq!(ast[2][1], Token::Lit(Literal::Money("USD$500.00".into())));
}

#[test]
fn micro_lit_percent() {
    let ast = parse_code("P1 15.5%\nP2 20%").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Percent(15.5)));
    assert_eq!(ast[1][1], Token::Lit(Literal::Percent(20.0)));
}

#[test]
fn micro_lit_date_time_duration() {
    let ast = parse_code("D 2026-03-30\nT 14:30:00\nDur1 5m30s\nDur2 2s").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Date("2026-03-30".into())));
    assert_eq!(ast[1][1], Token::Lit(Literal::Time("14:30:00".into())));
    assert_eq!(ast[2][1], Token::Lit(Literal::Duration("5m30s".into())));
    assert_eq!(ast[3][1], Token::Lit(Literal::Duration("2s".into())));
}

#[test]
fn micro_lit_color_ip_version() {
    let ast = parse_code("C #2A9D8F\nIp 192.168.0.1:8080\nVer 1.4.2-beta").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Color("#2A9D8F".into())));
    assert_eq!(ast[1][1], Token::Lit(Literal::IpAddr("192.168.0.1:8080".into())));
    assert_eq!(ast[2][1], Token::Lit(Literal::Version("1.4.2-beta".into())));
}

#[test]
fn micro_lit_email_url_file() {
    let ast = parse_code("E admin@razomy.org\nU1 https://razomy.org\nU2 https://api.ex/{Id}\nF1 %./data/users.json\nF2 %../../../base").unwrap();
    assert_eq!(ast[0][1], Token::Lit(Literal::Email("admin@razomy.org".into())));
    assert_eq!(ast[1][1], Token::Lit(Literal::Url("https://razomy.org".into())));
    assert_eq!(ast[2][1], Token::Lit(Literal::Url("https://api.ex/{Id}".into())));
    assert_eq!(ast[3][1], Token::Lit(Literal::File("%./data/users.json".into())));
    assert_eq!(ast[4][1], Token::Lit(Literal::File("%../../../base".into())));
}
