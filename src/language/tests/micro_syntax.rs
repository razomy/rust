use razomy_parser::{classify_sentence, parse_code, Literal, SemanticKind, Token};

#[test]
fn micro_range_int_to_int() {
    // Regression test for Bug #2: 1..100 and 2..6
    let ast = parse_code("R1 1..100\nR2 2..6").unwrap();
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
    // Regression test for Bug #2: 1..Limit
    let ast = parse_code("R (1..Limit)").unwrap();
    if let Token::Group('(', inner) = &ast[0][1] {
        assert_eq!(
            inner[0][0],
            Token::Range(
                Box::new(Token::Lit(Literal::Int(1))),
                Box::new(Token::Path("Limit".into()))
            )
        );
    } else {
        panic!("Expected paren group");
    }
}

#[test]
fn micro_range_strings() {
    let ast = parse_code("Digit \"0\"..\"9\"\nAlpha \"a\"..\"z\"").unwrap();
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
Key Value
"##;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 1);
    assert_eq!(ast[0], vec![Token::Path("Key".into()), Token::Path("Value".into())]);
}

#[test]
fn micro_lineage_paths_and_negative_index() {
    let ast = parse_code("A.B.C\nArr.0\nArr.-1\n_.Age").unwrap();
    assert_eq!(ast[0][0], Token::Path("A.B.C".into()));
    assert_eq!(ast[1][0], Token::Path("Arr.0".into()));
    assert_eq!(ast[2][0], Token::Path("Arr.-1".into()));
    assert_eq!(ast[3][0], Token::Path("_.Age".into()));
}

#[test]
fn micro_unary_bang_and_inequality() {
    let ast = parse_code("Not (!X)\nNeq (Val != 0)").unwrap();
    assert_eq!(ast.len(), 2);
}

#[test]
fn micro_multiline_pipeline_continuation() {
    let code = r#"
Process(UserId String) [
  UserId
  .  Db.FindUser
  .? [ _.Age >= 18 ]
  .& Db.FindCart
  .> [ "Log" . Shell.Log ]
  .| { A [ 1 ], B [ 2 ] }
]
"#;
    let ast = parse_code(code).unwrap();
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
SimpleBind 123
(A, B) Source
MyForm { X Int }
SubForm BaseForm { Y Int }
MyFunc(X Int) [ X + 1 ]
MyGeneric(T) None | Some(T)
ReactiveField [ Price * Quantity ]
+(Other Vec2) [ Other ]
(Acc, Val) [ Acc + Val ]
[ _ > 10 ] [ "Big" ]
UnionType 'a' | 'b'
CallMe(1, 2)
X . Transform
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 13);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::Binding { .. }));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::TupleBinding { .. }));
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::FormDef { .. }));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::SubtypeFormDef { .. }));
    assert!(matches!(classify_sentence(&ast[4]), SemanticKind::FunctionDef { .. }));
    assert!(matches!(classify_sentence(&ast[5]), SemanticKind::GenericDef { .. }));
    assert!(matches!(classify_sentence(&ast[6]), SemanticKind::StreamOrBranch { .. }));
    assert!(matches!(classify_sentence(&ast[7]), SemanticKind::OperatorMethod { .. }));
    assert!(matches!(classify_sentence(&ast[8]), SemanticKind::LambdaOrTupleBranch { .. }));
    assert!(matches!(classify_sentence(&ast[9]), SemanticKind::PatternBranch { .. }));
    assert!(matches!(classify_sentence(&ast[10]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[11]), SemanticKind::CallExpr { .. }));
    assert!(matches!(classify_sentence(&ast[12]), SemanticKind::Pipeline(_)));
}
