use razomy_language::parser::{
    classify_sentence, parse_code, split_pipeline, Literal, PipeOp, SemanticKind, Token,
};

// 1. ФУНДАМЕНТАЛЬНОЕ РАЗЛИЧИЕ RAZOMY:
// `key (expr)` (с пробелом) — это Binding (Axiom 1),
// а `fn(args)` (без пробела) — это CallExpr / FunctionDef / GenericDef!
#[test]
fn edge_space_binding_vs_tight_call() {
    let code = r#"
add (a + b)
origin (0.0, 0.0)
order_list (Item { name "Laptop" }, Item { name "Mouse" })
json_str (id . Db.find_user . Json.encode)

call_me(1, 2)
run_test_suite()
Map(K, V) { (K) V }
MathOps(a Int, b Int) { add (a + b) }
calc(x Float) [ x * 1.15 ]
find_user(id String) User | NotFound [ User { id id } ]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 10);

    // С пробелом перед `(` -> всегда Binding!
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::Binding { key, .. } if key == "add"));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::Binding { key, .. } if key == "origin"));
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::Binding { key, .. } if key == "order_list"));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::Binding { key, .. } if key == "json_str"));

    // Без пробела перед `(` -> CallExpr, GenericDef, FunctionDef!
    assert!(matches!(classify_sentence(&ast[4]), SemanticKind::CallExpr { name, .. } if name == "call_me"));
    assert!(matches!(classify_sentence(&ast[5]), SemanticKind::CallExpr { name, .. } if name == "run_test_suite"));
    assert!(matches!(classify_sentence(&ast[6]), SemanticKind::GenericDef { name, .. } if name == "Map"));
    assert!(matches!(classify_sentence(&ast[7]), SemanticKind::GenericDef { name, .. } if name == "MathOps"));
    assert!(matches!(classify_sentence(&ast[8]), SemanticKind::FunctionDef { name, .. } if name == "calc"));
    assert!(matches!(classify_sentence(&ast[9]), SemanticKind::FunctionDef { name, .. } if name == "find_user"));
}

// 2. ПРОВЕРКА MATHOPS ВНУТРИ ПАРАМЕТРИЗОВАННОЙ ФОРМЫ:
// `add (a + b)` — это Eager Binding, а `lazy_div [ self.a / self.b ]` — Lazy StreamOrBranch!
#[test]
fn edge_math_ops_eager_vs_lazy_fields() {
    let code = r#"
MathOps(a Int, b Int) {
  add      (a + b)
  sub      (a - b)
  shift_l  (a << 2)
  lazy_div [ self.a / self.b ]
}
"#;
    let ast = parse_code(code).unwrap();
    if let SemanticKind::GenericDef { body, .. } = classify_sentence(&ast[0]) {
        if let Token::Group('{', fields) = &body[0] {
            assert_eq!(fields.len(), 4);
            assert!(matches!(classify_sentence(&fields[0]), SemanticKind::Binding { key, .. } if key == "add"));
            assert!(matches!(classify_sentence(&fields[1]), SemanticKind::Binding { key, .. } if key == "sub"));
            assert!(matches!(classify_sentence(&fields[2]), SemanticKind::Binding { key, .. } if key == "shift_l"));
            assert!(matches!(classify_sentence(&fields[3]), SemanticKind::StreamOrBranch { name, .. } if name == "lazy_div"));
        } else {
            panic!("Ожидалась форма внутри MathOps");
        }
    } else {
        panic!("Ожидалась параметризованная форма MathOps");
    }
}

// 3. ДЕКОМПОЗИЦИЯ ПАЙПЛАЙНА (`split_pipeline` для Meta.steps_of и Law 3)
#[test]
fn edge_pipeline_step_decomposition() {
    let code = r#"
user_id
.  db.find_user
.? [ _.age >= 18 ]
.& db.find_cart
.> [ "Log" . Shell.log ]
.| { a [ 1 ], b [ 2 ] }
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 1);

    let (head, steps) = split_pipeline(&ast[0]);
    assert_eq!(head, vec![Token::Path("user_id".into())]);
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
pipeline_with_comments(x Int) [
  x
  // Комментарий перед первым шагом
  . step_one

  #
  Блочный комментарий между шагами пайплайна
  #

  .? [ _ > 0 ] // Хвостовой комментарий после шага
  .> [ _ . Shell.log ]
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

// 5. ЦЕПОЧКИ МЕТОДОВ, ГЛУБИННЫЕ ИНДЕКСЫ `_1`, `_2` И ВЫЗОВЫ ПОСЛЕ СКОБОК `(expr).round_to_int`
#[test]
fn edge_method_call_chains_and_depth_indexed_subjects() {
    let code = r#"
val state.read().count
msg "Current Count: {state.read().count}"
scaled Money.from_scaled((self.scaled.to_float * factor).round_to_int)
sum_pair_pipe(pair (Int, Int)) [ pair . [ _1.0 + _1.1 ] ]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 4);
    assert_eq!(ast[0][1], Token::Path("state.read().count".into()));
    assert_eq!(
        ast[1][1],
        Token::Lit(Literal::String("Current Count: {state.read().count}".into()))
    );
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::Binding { key, .. } if key == "scaled"));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::FunctionDef { name, .. } if name == "sum_pair_pipe"));
}

// 6. ПУСТЫЕ СКОБКИ ВСЕХ ТРЕХ ТИПОВ `{}` `[]` `()`
#[test]
fn edge_empty_brackets_all_three_types() {
    let code = r#"
EmptyForm   {}
SpacedForm  {   }
empty_stream []
empty_tuple  ()
BoundedQueue(T, capacity Int) {
  items Tuple(T) ()
  limit capacity
}
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
    assert!(parse_code("x (a+b)").is_err(), "a+b без пробелов должно быть ошибкой");
    assert!(parse_code("x (a*b)").is_err(), "a*b без пробелов должно быть ошибкой");
    assert!(parse_code("x (a==b)").is_err(), "a==b без пробелов должно быть ошибкой");

    // 2. Оператор с пробелом слева, но без пробела перед скобкой `a +(b)` (не на позиции метода):
    assert!(
        parse_code("x (a +(b))").is_err(),
        "Бинарный + перед ( без пробела должен быть ошибкой"
    );

    // 3. Незакрытый блочный комментарий:
    assert!(parse_code("#\nUnclosed comment\nx 1").is_err());

    // 4. Число, слипшееся с идентификатором:
    assert!(parse_code("bad 123abc").is_err(), "123abc должно быть ошибкой лексера");
}