use razomy_language::parser::{
    classify_sentence, parse_code, split_pipeline, Literal, PipeOp, SemanticKind, Token,
};

// 1. ФУНДАМЕНТАЛЬНОЕ РАЗЛИЧИЕ RAZOMY:
// `Key (Expr)` (с пробелом) — это Binding (Axiom 1),
// а `Fn(Args)` (без пробела) — это CallExpr / FunctionDef / GenericDef!
#[test]
fn edge_space_binding_vs_tight_call() {
    let code = r#"
Add (A + B)
Origin (0.0, 0.0)
OrderList (Item { Name "Laptop" }, Item { Name "Mouse" })
JsonStr (Id . Db.FindUser . Json.Encode)

CallMe(1, 2)
RunTestSuite()
Map(K, V) { (K) V }
MathOps(A Int, B Int) [ A + B ]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 8);

    // С пробелом перед `(` -> всегда Binding!
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::Binding { key, .. } if key == "Add"));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::Binding { key, .. } if key == "Origin"));
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::Binding { key, .. } if key == "OrderList"));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::Binding { key, .. } if key == "JsonStr"));

    // Без пробела перед `(` -> CallExpr, GenericDef, FunctionDef!
    assert!(matches!(classify_sentence(&ast[4]), SemanticKind::CallExpr { name, .. } if name == "CallMe"));
    assert!(matches!(classify_sentence(&ast[5]), SemanticKind::CallExpr { name, .. } if name == "RunTestSuite"));
    assert!(matches!(classify_sentence(&ast[6]), SemanticKind::GenericDef { name, .. } if name == "Map"));
    assert!(matches!(classify_sentence(&ast[7]), SemanticKind::FunctionDef { name, .. } if name == "MathOps"));
}

// 2. ПРОВЕРКА MATHOPS ВНУТРИ ФОРМЫ: все поля `Add (A + B)` должны быть Binding, а не CallExpr!
#[test]
fn edge_math_ops_fields_are_bindings() {
    let code = r#"
MathOps(A Int, B Int) [
  {
    Add (A + B)
    Sub (A - B)
    ShiftL (A << 2)
  }
]
"#;
    let ast = parse_code(code).unwrap();
    if let SemanticKind::FunctionDef { body, .. } = classify_sentence(&ast[0]) {
        if let Token::Group('{', fields) = &body[0][0] {
            assert_eq!(fields.len(), 3);
            for field in fields {
                assert!(
                    matches!(classify_sentence(field), SemanticKind::Binding { .. }),
                    "Поле внутри MathOps должно быть Binding, а не CallExpr: {:?}",
                    field
                );
            }
        } else {
            panic!("Ожидалась форма внутри MathOps");
        }
    } else {
        panic!("Ожидалась функция MathOps");
    }
}

// 3. ДЕКОМПОЗИЦИЯ ПАЙПЛАЙНА (`split_pipeline` для Meta.StepsOf и Law 3)
#[test]
fn edge_pipeline_step_decomposition() {
    let code = r#"
UserId
.  Db.FindUser
.? [ _.Age >= 18 ]
.& Db.FindCart
.> [ "Log" . Shell.Log ]
.| { A [ 1 ], B [ 2 ] }
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 1);

    let (head, steps) = split_pipeline(&ast[0]);
    assert_eq!(head, vec![Token::Path("UserId".into())]);
    assert_eq!(steps.len(), 5);
    assert_eq!(steps[0].op, PipeOp::Dot);
    assert_eq!(steps[1].op, PipeOp::Filter);
    assert_eq!(steps[2].op, PipeOp::Enrich);
    assert_eq!(steps[3].op, PipeOp::Tap);
    assert_eq!(steps[4].op, PipeOp::Parallel);
}

// 4. МНОГОСТРОЧНЫЙ ПАЙПЛАЙН С ПУСТЫМИ СТРОКАМИ, INLINE И БЛОЧНЫМИ КОММЕНТАРИЯМИ МЕЖДУ ШАГАМИ
#[test]
fn edge_multiline_pipeline_with_heavy_comments() {
    let code = r#"
PipelineWithComments(X Int) [
  X
  // Комментарий перед первым шагом
  . StepOne

  #
  Блочный комментарий между шагами пайплайна
  #

  .? [ _ > 0 ] // Хвостовой комментарий после шага
  .> [ _ . Shell.Log ]
]
"#;
    let ast = parse_code(code).unwrap();
    if let SemanticKind::FunctionDef { body, .. } = classify_sentence(&ast[0]) {
        assert_eq!(body.len(), 1, "Все шаги должны склеиться в 1 пайплайн");
        let (_, steps) = split_pipeline(&body[0]);
        assert_eq!(steps.len(), 3);
    } else {
        panic!("Expected FunctionDef");
    }
}

// 5. ЦЕПОЧКИ МЕТОДОВ И ПОЛЕЙ: `State.Read().Count` и `Jobs.Recv() . { ... }`
#[test]
fn edge_method_call_chains_in_paths_and_strings() {
    let code = r#"
Val State.Read().Count
Msg "Current Count: {State.Read().Count}"
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    assert_eq!(ast[0][1], Token::Path("State.Read().Count".into()));
    assert_eq!(
        ast[1][1],
        Token::Lit(Literal::String("Current Count: {State.Read().Count}".into()))
    );
}

// 6. ПУСТЫЕ СКОБКИ ВСЕХ ТРЕХ ТИПОВ `{}` `[]` `()`
#[test]
fn edge_empty_brackets_all_three_types() {
    let code = r#"
EmptyForm   {}
SpacedForm  {   }
EmptyStream []
EmptyTuple  ()
Tuple(T)    ()
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 5);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::FormDef { body, .. } if body.is_empty()));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::FormDef { body, .. } if body.is_empty()));
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::StreamOrBranch { stream, .. } if stream.is_empty()));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::Binding { .. }));
    assert!(matches!(classify_sentence(&ast[4]), SemanticKind::GenericDef { .. }));
}

// 7. НЕГАТИВНЫЕ ТЕСТЫ ЛЕКСЕРА: нарушение Правила Пробелов вокруг бинарных операторов
#[test]
fn edge_negative_lexer_space_delimited_rule() {
    // 1. Бинарные операторы без пробелов должны отвергаться парсером:
    assert!(parse_code("X (A+B)").is_err(), "A+B без пробелов должно быть ошибкой");
    assert!(parse_code("X (A*B)").is_err(), "A*B без пробелов должно быть ошибкой");
    assert!(parse_code("X (A==B)").is_err(), "A==B без пробелов должно быть ошибкой");

    // 2. Оператор с пробелом слева, но без пробела перед скобкой `A +(B)` (не на позиции метода):
    assert!(parse_code("X (A +(B))").is_err(), "Бинарный + перед ( без пробела должен быть ошибкой");

    // 3. Незакрытый блочный комментарий:
    assert!(parse_code("#\nUnclosed comment\nX 1").is_err());

    // 4. Число, слипшееся с идентификатором:
    assert!(parse_code("Bad 123abc").is_err(), "123abc должно быть ошибкой лексера");
}