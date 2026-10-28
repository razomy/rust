use razomy_language::parser::{classify_sentence, parse_code, Literal, SemanticKind, Token};
use std::fs;

// --- ШАГ 1: Комментарии, пустые строки и символ '#' внутри комментария ---
#[test]
fn test_step_01_comments_and_blank_lines() {
    let code = r#"
// Однострочный комментарий
#
Многострочный блочный комментарий (when '#' is followed by whitespace or newline)
#

Name String


// Несколько пустых строк подряд перед следующим выражением
Jonn String
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    assert_eq!(ast[0], vec![Token::Path("Name".into()), Token::Path("String".into())]);
    assert_eq!(ast[1], vec![Token::Path("Jonn".into()), Token::Path("String".into())]);
}

// --- ШАГ 2: Четыре базовые аксиомы и наследование форм (Axioms 1-4) ---
#[test]
fn test_step_02_core_axioms_1_to_4() {
    let code = r#"
Name String

Name {
  First String
}

HasParentName {
  ParentName String
}

Name {
  First String
} & HasParentName

JonnName HasParentName {
  ParentName Jonn
}

JonnDoyName2 JonnName {
  ParentName JonnDoy
} & { ParentName "JonnDoyBoy" }
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 6);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::Binding { .. }));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::FormDef { .. }));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::FormDef { .. }));
    assert!(matches!(classify_sentence(&ast[4]), SemanticKind::SubtypeFormDef { .. }));
    assert!(matches!(classify_sentence(&ast[5]), SemanticKind::SubtypeFormDef { .. }));
}

// --- ШАГ 3: Слитные пути (Lineage) и доступ по отрицательным индексам ---
#[test]
fn test_step_03_lineage_paths_and_negative_indices() {
    let code = r#"
JonnDoyName2.ParentName
JonnDoyName2.JonnName.ParentName
JonnDoyName2.JonnName.HasParentName.ParentName
JonnDoyName3.0
JonnDoyName3.-1
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 5);
    assert_eq!(ast[0], vec![Token::Path("JonnDoyName2.ParentName".into())]);
    assert_eq!(ast[2], vec![Token::Path("JonnDoyName2.JonnName.HasParentName.ParentName".into())]);
    assert_eq!(ast[3], vec![Token::Path("JonnDoyName3.0".into())]);
    assert_eq!(ast[4], vec![Token::Path("JonnDoyName3.-1".into())]);
}

// --- ШАГ 4: Примитивы и символы ---
#[test]
fn test_step_04_primitives_and_symbols() {
    let code = r#"
String      ""
Symbol      'symbol'
Int         0
Float       0.0
Bool        false
Byte        0x00
Money       $0.00
File        %./
Any         _
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 9);
    assert_eq!(ast[1][1], Token::Lit(Literal::Symbol("symbol".into())));
    assert_eq!(ast[4][1], Token::Lit(Literal::Bool(false)));
    assert_eq!(ast[5][1], Token::Lit(Literal::Byte(0)));
    assert_eq!(ast[6][1], Token::Lit(Literal::Money("$0.00".into())));
    assert_eq!(ast[7][1], Token::Lit(Literal::File("%./".into())));
    assert_eq!(ast[8][1], Token::Path("_".into()));
}

// --- ШАГ 5: Все типы богатых человеческих литералов (Rich Human Literals) ---
#[test]
fn test_step_05_rich_human_literals() {
    let code = r#"
HumanLiterals {
  MoneyVal    $199.99
  CurrencyVal USD$500.00
  PercentVal  15.5%
  DateVal     2026-03-30
  TimeVal     14:30:00
  DurationVal 5m30s
  CoordPair   1920x1080
  ColorVal    #2A9D8F
  IpAddr      192.168.0.1:8080
  VersionVal  1.4.2-beta
  EmailAddr   admin@razomy.org
  WebUrl      https://razomy.org
  InterpUrl   https://api.ex/{Id}
  LocalFile   %./data/users.json
}
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 1);
    if let Token::Group('{', fields) = &ast[0][1] {
        assert_eq!(fields.len(), 14);
        assert_eq!(fields[0][1], Token::Lit(Literal::Money("$199.99".into())));
        assert_eq!(fields[1][1], Token::Lit(Literal::Money("USD$500.00".into())));
        assert_eq!(fields[2][1], Token::Lit(Literal::Percent(15.5)));
        assert_eq!(fields[3][1], Token::Lit(Literal::Date("2026-03-30".into())));
        assert_eq!(fields[4][1], Token::Lit(Literal::Time("14:30:00".into())));
        assert_eq!(fields[5][1], Token::Lit(Literal::Duration("5m30s".into())));
        assert_eq!(fields[6][1], Token::Lit(Literal::Coord("1920x1080".into())));
        assert_eq!(fields[7][1], Token::Lit(Literal::Color("#2A9D8F".into())));
        assert_eq!(fields[8][1], Token::Lit(Literal::IpAddr("192.168.0.1:8080".into())));
        assert_eq!(fields[9][1], Token::Lit(Literal::Version("1.4.2-beta".into())));
        assert_eq!(fields[10][1], Token::Lit(Literal::Email("admin@razomy.org".into())));
        assert_eq!(fields[11][1], Token::Lit(Literal::Url("https://razomy.org".into())));
        assert_eq!(fields[12][1], Token::Lit(Literal::Url("https://api.ex/{Id}".into())));
        assert_eq!(fields[13][1], Token::Lit(Literal::File("%./data/users.json".into())));
    } else {
        panic!("Ожидалась группа Form");
    }
}

// --- ШАГ 6: Кортежи, Суммы типов (|), Пересечения (&), Вычитание (/) и Карты ---
#[test]
fn test_step_06_tuples_maps_and_type_algebra() {
    let code = r#"
Point2D (Float, Float)
Origin  (0.0, 0.0)
Status  'draft' | 'published' | 'archived'

PublicUser        User / User.PasswordHash
AnonymousUsr      PublicUser / 'Name'
UserAndCloud      User & Cloud
GuestUser         PublicUser / PublicUser.Role & { Role 'guest' }

Map(K, V) { (K) V }
Set(T)    { (T) Bool }
UpdatedConfig ConfigMap & { "port" "9090", "tls" "true" } / "host"
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 10);
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[6]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[7]), SemanticKind::GenericDef { .. }));
}

// --- ШАГ 7: Арифметические, побитовые и логические операции + диапазоны ---
#[test]
fn test_step_07_math_bitwise_logic_and_ranges() {
    let code = r#"
MathOps(A Int, B Int) [
  {
    Add (A + B)
    Sub (A - B)
    Mul (A * B)
    Div (A / B)
    Mod (A % B)
    Pow (A ** B)
    BitAnd (A . bit_and(B))
    ShiftL (A << 2)
    ShiftR (A >> 2)
  }
]

LogicOps(X Bool, Y Bool, Val Int) [
  {
    And     (X && Y)
    Or      (X || Y)
    Not     (!X)
    Eq      (Val == 42)
    Neq     (Val != 0)
    InRange (Val . (1..100))
  }
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::FunctionDef { .. }));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::FunctionDef { .. }));
}

// --- ШАГ 8: Перегрузка операторов внутри форм ---
#[test]
fn test_step_08_operator_overloading_in_forms() {
    let code = r#"
Vec2 {
  X Float
  Y Float

  +(Other Vec2)  [ Vec2 { X (X + Other.X), Y (Y + Other.Y) } ]
  -(Other Vec2)  [ Vec2 { X (X - Other.X), Y (Y - Other.Y) } ]
  *(Scale Float) [ Vec2 { X (X * Scale),   Y (Y * Scale) } ]
  ==(Other Vec2) [ (X == Other.X) && (Y == Other.Y) ]
}
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 1);
    if let Token::Group('{', members) = &ast[0][1] {
        assert_eq!(members.len(), 6);
        assert!(matches!(classify_sentence(&members[2]), SemanticKind::OperatorMethod { .. }));
        assert!(matches!(classify_sentence(&members[5]), SemanticKind::OperatorMethod { .. }));
    } else {
        panic!("Ожидалось тело Vec2");
    }
}

// --- ШАГ 9: Многострочные пайплайны и композиция потоков ---
#[test]
fn test_step_09_multiline_pipelines_and_stream_composition() {
    let code = r#"
StringOps(First String, Last String) [
  Full "{First} {Last}"

  Full
  . Trim
  . ToUpper
  . Split(" ")
]

CleanAndUpper (Trim & ToUpper)
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    if let SemanticKind::FunctionDef { body, .. } = classify_sentence(&ast[0]) {
        assert_eq!(body.len(), 2);
        assert!(matches!(classify_sentence(&body[1]), SemanticKind::Pipeline(_)));
    } else {
        panic!("Ожидалась функция StringOps");
    }
}

// --- ШАГ 10: Слоты функций, значения по умолчанию и частичное применение ---
#[test]
fn test_step_10_function_slots_defaults_and_rebinding() {
    let code = r#"
Connect(Host "localhost", Port 8080, Tls false) [
  "Connecting to {Host}:{Port} (TLS: {Tls})"
]

DefaultConn   Connect()
PartialConn   Connect("db.internal")
SecureConnect (Connect & { Port 9090, Tls true })
ActiveConn    SecureConnect()
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 5);
}

// --- ШАГ 11: Все виды операторов пайплайна (., .>, .&, .?) ---
#[test]
fn test_step_11_pipeline_operators_dot_tap_enrich_filter() {
    let code = r#"
FinalizeBill(User & Cart) [
  _ & {
    Discount [
      Amount . {
        [ _ >= 2000 ] [ 300 ]
        [ _ >= 1000 ] [ 100 ]
        _             [ 0 ]
      }
    ]
    FinalPrice [ Amount - Discount ]
  }
  .> [ "Receipt for {Name}: {FinalPrice} USD (Discount: {Discount})" . Shell.Log ]
]

ProcessOrderPipeline(UserId String) [
  UserId
  .  Db.FindUser          // `.`  Standard Pipe
  .? [ _.Age >= 18 ]      // `.?` Filter Pipe
  .& Db.FindCart          // `.&` Enrich Pipe
  .  FinalizeBill
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    if let SemanticKind::FunctionDef { body, .. } = classify_sentence(&ast[1]) {
        assert_eq!(body.len(), 1);
        assert!(matches!(classify_sentence(&body[0]), SemanticKind::Pipeline(_)));
    } else {
        panic!("Ожидалась функция ProcessOrderPipeline");
    }
}

// --- ШАГ 12: Уточняющие типы (Refinement Types) и роутинг по типам ---
#[test]
fn test_step_12_refinement_types_and_pattern_routing() {
    let code = r#"
IsAdult   [ _.Age >= 18 ]
IsManager [ _.Role . ('admin' | 'manager') ]
EmailStr  String & [ _ . Regex.Matches("^[^@]+@[^@]+\\.[^@]+$") ]
Adult     User & IsAdult

Area(S Shape) [
  S . {
    Circle [ 3.14159 * Radius * Radius ]
    Rect   [ Width * Height ]
  }
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 5);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::StreamOrBranch { .. }));
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::AlgebraicDef { .. }));
}

// --- ШАГ 13: Коллекции в круглых скобках (), итерация и FizzBuzz ---
#[test]
fn test_step_13_collections_iteration_and_tuple_matching() {
    let code = r#"
OrderList (
  Item { Name "Laptop", Price 1200, InStock true }
  Item { Name "Mouse",  Price 25,   InStock false }
  Item { Name "Screen", Price 400,  InStock true }
)

CalculateTotalInStock(Items) [
  Items
  . Filter([ _.InStock == true ])
  . Map([ _.Price ])
  . Fold(0, (Acc, Price) [ Acc + Price ])
]

FizzBuzz(Limit Int) [
  (1..Limit) . Map((N) [
    (N % 3, N % 5) . {
      (0, 0) [ "FizzBuzz" ]
      (0, _) [ "Fizz" ]
      (_, 0) [ "Buzz" ]
      _      [ "{N}" ]
    }
  ])
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 3);
}

// --- ШАГ 14: Разрешение коллизий Lineage (Law 2) и миграция типов ---
#[test]
fn test_step_14_lineage_collision_and_type_migration() {
    let code = r#"
UserOrder      User & Order
UserOrderClean UserOrder / Order.Id

MigrateUser(User) [
  ClientAccount {
    AccountId Id
    Name      Name
    IsAdult   (Age >= 18)
  }
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 3);
}

// --- ШАГ 15: Реактивные формы (Excel-style Dataflow) ---
#[test]
fn test_step_15_reactive_forms() {
    let code = r#"
Invoice {
  Price    $100.00
  Quantity 3
  TaxRate  20%

  Subtotal [ Price * Quantity ]
  Tax      [ Subtotal * TaxRate ]
  Total    [ Subtotal + Tax ]
}

Order1 Invoice & { Quantity 5 }
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    if let Token::Group('{', fields) = &ast[0][1] {
        assert_eq!(fields.len(), 6);
        assert!(matches!(classify_sentence(&fields[3]), SemanticKind::StreamOrBranch { .. }));
    } else {
        panic!("Ожидалась форма Invoice");
    }
}

// --- ШАГ 16: Онтологическая типизация (Typestate) и Law 3 ---
#[test]
fn test_step_16_ontological_typing_and_law_3() {
    let code = r#"
ReserveStock(O Order / Cancelled) [
  O.Amount . {
    [ _ > 0 ] [ O & Reserved { ReservedAt 1620000000 } ]
    _         [ OutOfStock { Reason "Warehouse empty" } ]
  }
]

ExecuteFulfillment(InitialOrder Order / Cancelled) [
  InitialOrder
  . ReserveStock
  . ChargePayment
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
}

// --- ШАГ 17: Вычисления на уровне типов, F-Bounded полиморфизм и HKT ---
#[test]
fn test_step_17_type_level_peano_generics_and_hkt() {
    let code = r#"
Zero    {}
Succ(N) { Prev N }
One     Succ(Zero)
Tuple(T) ()

Comparable(Other Self) {
  <(Other) Bool
}

SortUsers(Users List(User & Comparable(User))) [
  Users . Sort
]

Box(T)         { Item T }
NestedBox(T)   Box(Box(T))
ApplyHKT(F, V) [ F { Item V } ]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 9);
}

// --- ШАГ 18: Функции высшего порядка, ранний выход (Done) и Катаморфизмы ---
#[test]
fn test_step_18_higher_order_functions_done_and_catamorphisms() {
    let code = r#"
SumUntilLimit(Numbers List(Int), Max Int) [
  Numbers . Fold(0, (Acc, N) [
    (Acc + N) . {
      [ _ > Max ] [ Done { Value _ } ]
      _           [ _ ]
    }
  ])
]

Expr Num(Int) | Add(Expr, Expr) | Mul(Expr, Expr)

Cata(Expression, Algebra) [
  Expression . {
    Num [ Algebra.OnNum(Value) ]
    Add [ Algebra.OnAdd(Left . Cata(Algebra), Right . Cata(Algebra)) ]
    Mul [ Algebra.OnMul(Left . Cata(Algebra), Right . Cata(Algebra)) ]
  }
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 3);
}

// --- ШАГ 19: Конкурентность (.|), каналы CSP и транзакционные атомы ---
#[test]
fn test_step_19_concurrency_parallel_pipe_channels_and_atoms() {
    let code = r#"
FetchAds() [ https://api.ex/ads . Http.Get . Timeout(2s) ]

LoadDashboardData(UserId String) [
  UserId .| {
    Profile [ _ . FetchProfile ]
    Recs    [ _ . FetchRecommendations ]
    Ads     [ FetchAds() ]
  }
]

WorkerLoop(Jobs Channel(String), Results Channel(Int)) [
  Jobs.Recv() . {
    None [ "Channel closed, terminating worker." . Shell.Log ]
    Some [
      Value
      . Length
      . Results.Send

      WorkerLoop(Jobs, Results)
    ]
  }
]

ModifyGlobalCounter() [
  GlobalCounter.Swap((State) [ UpdateCounter(State, Increment { By 5 }) ])
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 4);
}

// --- ШАГ 20: Управление ресурсами (WithFile) и унифицированные порты ---
#[test]
fn test_step_20_resource_management_and_ports() {
    let code = r#"
Fs {
  WithFile(Path File, Action) [
    Path . Fs.Open . {
      FileHandle [
        Handle _
        Handle . Action
        .> [ Handle . Fs.Close ]
      ]
    }
  ]
}

SyncRemoteBackup() [
  https://api.razomy.org/v1/users
  . Port.Read
  . Port.Write(%./backup/users.json)
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
}

// --- ШАГ 21: Глубокий паттерн-роутинг (Структуры + Кортежи) ---
#[test]
fn test_step_21_deep_pattern_routing() {
    let code = r#"
ApiRouter(Req HttpRequest) [
  Req . {
    { Method 'GET', Path ("health") } [
      HttpResponse { Status 200, Body "OK" }
    ]

    { Method 'GET', Path ("users", Id) } [
      Req . AuthMiddleware . {
        Unauthorized [ HttpResponse { Status 401, Body Reason } ]
        HttpRequest  [
          JsonStr (Id . Db.FindUser . Json.Encode)
          HttpResponse { Status 200, Body JsonStr }
        ]
      }
    ]

    _ [ HttpResponse { Status 404, Body "Not Found" } ]
  }
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 1);
}

// --- ШАГ 22: Встроенные диалекты Parse (PEG) и View/Draw (UI) ---
#[test]
fn test_step_22_parse_and_ui_draw_dialects() {
    let code = r#"
Digit "0".."9"
Alpha "a".."z" | "A".."Z"

EmailGrammar String & [
  _ . Parse([
    Some(Alpha) . Match("@") . Some(Alpha) . Match(".") . Repeat(2..6, Alpha)
  ])
]

CounterApp(State Atom(CounterState)) [
  UI.View({
    Title "Razomy Counter"
    Size  1920x1080
    Body  (
      Heading("Current Count: {State.Read().Count}")
      Canvas(200x100, [
        UI.Draw(
          Pen(#FF0000),
          FillPen(#00FF00),
          Circle((50, 50), 40)
        )
      ])
    )
  })
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 4);
}

// --- ШАГ 23: Метапрограммирование, рефлексия и переписывание AST ---
#[test]
fn test_step_23_metaprogramming_and_ast_rewriting() {
    let code = r#"
DeriveJson(TargetForm) [
  TargetForm & {
    ToJson() [
      Inner (
        Self . Meta.FieldsOf . Map((Field) [
          "\"{Field.Name}\": {Field.Value . Json.FormatValue}"
        ])
        . Join(", ")
      )
      "\{ {Inner} \}"
    ]
  }
]

Product { Id Int, Title String } . DeriveJson

UpgradedSystem ComplexSystem . Meta.Rewrite({
  Calc(_)   [ [ _ * 1.20 ] ]
  Shell.Log [ Audit.Write ]
})
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 3);
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::Pipeline(_)));
}

// --- ШАГ 24: Импорты, инкапсуляция модулей и тестирование ---
#[test]
fn test_step_24_imports_encapsulation_and_testing() {
    let code = r#"
Base          root.base
(String, Int) root.base
_             root.base
_             %../../../base

AuthInternal {
  Salt         "x9f2a" & Private
  Hash(Raw)    [ "{Raw}_{Salt}" . native_sha256 ] & Private
  Verify(U, P) [ (P . Hash) == U.PasswordHash ] & Public
}

AuthModule  AuthInternal & Public
CleanModule AuthInternal / 'Salt' / 'Hash'
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 7);
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::TupleBinding { .. }));
}

// --- ШАГ 25: Полная спецификация Razomy v4.7 целиком (Интеграционный тест) ---
#[test]
fn test_step_25_full_v4_7_specification_parses_100_percent() -> Result<(), Box<dyn std::error::Error>> {
    let full_spec = fs::read_to_string("./tests/spec.rl")?;

    let ast = parse_code(&full_spec).expect("Вся спецификация v4.7 должна парситься без единой ошибки!");
    assert!(ast.len() > 150, "Ожидалось более 150 корневых инструкций, получено: {}", ast.len());

    for sentence in &ast {
        let _ = classify_sentence(sentence);
    }
    Ok(())
}
