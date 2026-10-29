use razomy_language::parser::{classify_sentence, parse_code, Literal, SemanticKind, Token};

#[test]
fn micro_range_int_to_int() {
    let ast = parse_code("r1 1..100\nr2 2..6").unwrap();
    assert_eq!(
        ast[0][1],
        Token::Range(
            Box::new(Token::Lit(Literal::Int(1))),
            Box::new(Token::Lit(Literal::Int(100)))
        )
    );
    assert_eq!(
        ast[1][1],
        Token::Range(
            Box::new(Token::Lit(Literal::Int(2))),
            Box::new(Token::Lit(Literal::Int(6)))
        )
    );
}

#[test]
fn micro_range_int_to_ident() {
    let ast = parse_code("r (1..limit)").unwrap();
    if let Token::Group('(', inner) = &ast[0][1] {
        assert_eq!(
            inner[0][0],
            Token::Range(
                Box::new(Token::Lit(Literal::Int(1))),
                Box::new(Token::Path("limit".into()))
            )
        );
    } else {
        panic!("Expected paren group");
    }
}

#[test]
fn micro_range_strings() {
    let ast = parse_code("digit \"0\"..\"9\"\nalpha \"a\"..\"z\"").unwrap();
    assert_eq!(
        ast[0][1],
        Token::Range(
            Box::new(Token::Lit(Literal::String("0".into()))),
            Box::new(Token::Lit(Literal::String("9".into())))
        )
    );
}

#[test]
fn micro_block_comment_with_quoted_hash() {
    let code = r##"
#
Block comment containing '#' and "#" safely
#
key value
"##;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 1);
    assert_eq!(ast[0], vec![Token::Path("key".into()), Token::Path("value".into())]);
}

#[test]
fn micro_lineage_paths_and_negative_index() {
    let ast = parse_code("MyCar.SportsCar.Car.brand\narr.0\narr.-1\n_.age\n_1.0\n_2.explanation").unwrap();
    assert_eq!(ast[0][0], Token::Path("MyCar.SportsCar.Car.brand".into()));
    assert_eq!(ast[1][0], Token::Path("arr.0".into()));
    assert_eq!(ast[2][0], Token::Path("arr.-1".into()));
    assert_eq!(ast[3][0], Token::Path("_.age".into()));
    assert_eq!(ast[4][0], Token::Path("_1.0".into()));
    assert_eq!(ast[5][0], Token::Path("_2.explanation".into()));
}

#[test]
fn micro_unary_bang_and_inequality() {
    let ast = parse_code("not_op (!x)\nneq (val != 0)").unwrap();
    assert_eq!(ast.len(), 2);
}

#[test]
fn micro_multiline_pipeline_and_ampersand_continuation() {
    let code = r#"
process(user_id String) [
  user_id
  .  db.find_user
  .? [ _.age >= 18 ]
  .& db.find_cart
  .> [ "Log" . Shell.log ]
  .| { a [ 1 ], b [ 2 ] }
]

EmailGrammar String & [
  _ . Parse.run(
    Parse.one_or_more(alpha)
    & Parse.match("@")
    & Parse.one_or_more(alpha)
  )
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    if let SemanticKind::FunctionDef { body, .. } = classify_sentence(&ast[0]) {
        assert_eq!(body.len(), 1);
        assert!(matches!(classify_sentence(&body[0]), SemanticKind::Pipeline(_)));
    } else {
        panic!("Expected FunctionDef");
    }
}

#[test]
fn micro_semantic_classification_variants() {
    let code = r#"
simple_bind 123
(a, b) source
MyForm { x Int }
SubForm BaseForm { y Int }
my_func(x Int) Int [ x + 1 ]
MyGeneric(T) None | Some(T)
reactive_field [ self.price * self.quantity ]
+ (other Vec2) [ other ]
(acc Int, val Int) [ acc + val ]
[ _ > 10 ] [ "Big" ]
UnionType 'a' | 'b'
call_me(1, 2)
x . transform
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 13);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::Binding { .. }));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::TupleBinding { .. }));
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::FormDef { .. }));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[4]), SemanticKind::FunctionDef { .. }));
    assert!(matches!(classify_sentence(&ast[5]), SemanticKind::GenericDef { .. }));
    assert!(matches!(classify_sentence(&ast[6]), SemanticKind::StreamOrBranch { .. }));
    assert!(matches!(classify_sentence(&ast[7]), SemanticKind::FunctionDef { .. }));
    assert!(matches!(classify_sentence(&ast[8]), SemanticKind::LambdaOrTupleBranch { .. }));
    assert!(matches!(classify_sentence(&ast[9]), SemanticKind::PatternBranch { .. }));
    assert!(matches!(classify_sentence(&ast[10]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[11]), SemanticKind::CallExpr { .. }));
    assert!(matches!(classify_sentence(&ast[12]), SemanticKind::Pipeline(_)));
}
