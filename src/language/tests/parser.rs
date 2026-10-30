use razomy_language::parser::{classify_sentence, extract_import, parse_code, ImportDirective, ImportSource, Literal, SemanticKind, Token};
use std::fs;

// --- ШАГ 1: Комментарии, пустые строки и контракты Compile / Bypass ---
#[test]
fn test_step_01_comments_and_compile_bypass_contracts() {
    let code = r#"
// Однострочный комментарий
#1
Многострочный блочный комментарий (when '#' is followed by whitespace or newline)
1#

Compile {}
Bypass  {}

BrandName String
default_user_name "Alice"
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 4);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::FormDef { name, .. } if name == "Compile"));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::FormDef { name, .. } if name == "Bypass"));
    assert_eq!(ast[2], vec![Token::Path("BrandName".into()), Token::Path("String".into())]);
}

// --- ШАГ 2: Четыре базовые аксиомы и наследование форм (Axioms 1-4) ---
#[test]
fn test_step_02_core_axioms_1_to_4_vehicle_hierarchy() {
    let code = r#"
Vehicle {
  brand     BrandName
  max_speed Int 180
  tag       "standard"
}

HasBattery {
  battery_kwh Int 75
}

ElectricVehicle Vehicle & HasBattery & {
  charging_port String "Type2"
}

Car Vehicle {
  brand     "GenericMotor"
  max_speed 200
}

SportsCar Car {
  brand     "Porsche"
  max_speed 300
}

MyCar SportsCar {
  brand "CustomPorsche"
}
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 6);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::FormDef { .. }));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::FormDef { .. }));
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[4]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[5]), SemanticKind::AlgebraicDef { .. }));
}

// --- ШАГ 3: Слитные пути (Lineage), отрицательные индексы и Perceus in-place мутация ---
#[test]
fn test_step_03_lineage_paths_negative_indices_and_mutation() {
    let code = r#"
MyCar.brand
MyCar.SportsCar.brand
MyCar.SportsCar.Car.brand
MyCar.Vehicle.brand
MyGarageCar.0
MyGarageCar.-1

mutate_car_demo() [
  my_car (MyGarageCar {})
  upgraded_car (my_car & { max_speed 350, garage_slot 2 })
  my_car.brand "Tesla"
  my_car.0  "Rivian"
  my_car.-1 42
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 7);
    assert_eq!(ast[0], vec![Token::Path("MyCar.brand".into())]);
    assert_eq!(ast[2], vec![Token::Path("MyCar.SportsCar.Car.brand".into())]);
    assert_eq!(ast[4], vec![Token::Path("MyGarageCar.0".into())]);
    assert_eq!(ast[5], vec![Token::Path("MyGarageCar.-1".into())]);
    assert!(matches!(classify_sentence(&ast[6]), SemanticKind::FunctionDef { .. }));
}

// --- ШАГ 4: Правило универсальных слотов (Generics, Bounded Generics, Law 1c & Depth-Indexed _1) ---
#[test]
fn test_step_04_universal_slots_and_depth_indexed_subjects() {
    let code = r#"
Box(T) { item T }

CategorizedUser(TC Category, active_cat TC) {
  backup_cat TC
  current    active_cat
}

UserWithAge(Age) { new_age age }
is_user_18(Age) [ age == 18 ]
compare_ages(candidate_age Age, minimum_age Age) [ candidate_age >= minimum_age ]
read_unpacked({ name } WithName) [ name ]
extract_name_demo(w WithName) [ w . [ _1.name ] ]
sum_pair_pipe(pair (Int, Int)) [ pair . [ _1.0 + _1.1 ] ]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 8);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::GenericDef { .. }));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::GenericDef { .. }));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::FunctionDef { .. }));
    assert!(matches!(classify_sentence(&ast[6]), SemanticKind::FunctionDef { .. }));
    assert!(matches!(classify_sentence(&ast[7]), SemanticKind::FunctionDef { .. }));
}

// --- ШАГ 5: Примитивы, точный Money, Raw-строки и Tagged String Sugar (`Type"..."`) ---
#[test]
fn test_step_05_primitives_money_raw_strings_and_tagged_strings() {
    let code = r##"
String ""
Symbol 'symbol'
Int    0
Float  0.0
Bool   false
Byte   0x00
Any    _

Money(raw String "0.00") {
  scaled (raw . Decimal.parse_scale4)
  from_scaled(val Int) [ Money { scaled val } ]
  +  (other Money)   [ Money.from_scaled(self.scaled + other.scaled) ]
  *  (factor Float)  [ Money.from_scaled((self.scaled.to_float * factor).round_to_int) ]
  == (other Money)   [ self.scaled == other.scaled ]
}

StringExamples {
  regex_pat   r"^[^@]+@[^@]+\.[^@]+$"
  nested_json r1"{"query": "SELECT * FROM \"users\"", "interpolated": "{ignored}"}"1
  nested_code r2"Can freely embed both " and "1 and # without any lexer conflict!"2
}

HumanLiterals {
  money_explicit Money("199.99"), money_val Money"199.99"
  url_explicit   Url("https://razomy.org"), web_url Url"https://razomy.org"
  path_explicit  Path("./data/users.json"), local_file Path"./data/users.json"
  tax_rate       0.155
}
"##;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 10);
    assert_eq!(ast[1][1], Token::Lit(Literal::Symbol("symbol".into())));
    assert_eq!(ast[4][1], Token::Lit(Literal::Bool(false)));
    assert_eq!(ast[5][1], Token::Lit(Literal::Byte(0)));
    assert!(matches!(classify_sentence(&ast[7]), SemanticKind::GenericDef { name, .. } if name == "Money"));
}

// --- ШАГ 6: Кортежи, Суммы типов (|), Пересечения (&), Вычитание (/) и Карты ---
#[test]
fn test_step_06_tuples_maps_and_type_algebra() {
    let code = r#"
Point2D (Float, Float)
origin  (0.0, 0.0)
Status  'draft' | 'published' | 'archived'

PublicUser    User / User.password_hash
AnonymousUsr  PublicUser / 'name'
UserAndCloud  User & Cloud
RestoredUser  UserAndCloud / Cloud
GuestUser     PublicUser / PublicUser.role & { role 'guest' }

Map(K, V) {
  (K) V
  get(m Map(K, V), key K) [ (m, key) . native_map_get ]
}
Set(T) { (T) Bool }
updated_config config_map & { "port" "9090", "tls" "true" } / "host"
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 11);
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[7]), SemanticKind::AlgebraicDef { .. }));
    assert!(matches!(classify_sentence(&ast[8]), SemanticKind::GenericDef { .. }));
}

// --- ШАГ 7: Обновление данных: Глубокие линзы (in-place rebinding) и 7 комбинаций `&` ---
#[test]
fn test_step_07_deep_inplace_rebinding_and_7_ampersand_combinations() {
    let code = r#"
update_order_demo() [
  raw_order Order { id "o1", amount 100 }
  counter 0
  counter (counter + 1)
  raw_order.amount 14
  corp Company { name "Acme", departments (Department { title "R&D", budget 1000 }) }
  corp.departments.0.budget 2500
  raw_order.1 20
  next_order (raw_order & { amount 25 })
  next_order (next_order .& { amount 50 })
]

Comparable(Other) {
  < (other Other) Bool
}

SortableUser User & Comparable(User) {
  < (other User) [ self.age < other.age ]
}

None     {} & Bypass
NotFound { entity String, id String } & Bypass
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 5);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::FunctionDef { .. }));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::GenericDef { .. }));
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::AlgebraicDef { .. }));
}

// --- ШАГ 8: Перегрузка операторов внутри форм (с пробелом перед скобкой) ---
#[test]
fn test_step_08_operator_overloading_in_forms() {
    let code = r#"
Vec2 {
  x Float
  y Float

  +  (other Vec2)  [ Vec2 { x (self.x + other.x), y (self.y + other.y) } ]
  -  (other Vec2)  [ Vec2 { x (self.x - other.x), y (self.y - other.y) } ]
  *  (scale Float) [ Vec2 { x (self.x * scale),   y (self.y * scale) } ]
  /  (scale Float) [ Vec2 { x (self.x / scale),   y (self.y / scale) } ]
  == (other Vec2)  [ (self.x == other.x) && (self.y == other.y) ]
}

Shape {
  vertices Tuple(Vec2)
  / (cutter Shape) [ (self, cutter) . native_csg_subtract ]
}
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    if let Token::Group('{', members) = &ast[0][1] {
        assert_eq!(members.len(), 7);
        assert!(matches!(classify_sentence(&members[2]), SemanticKind::FunctionDef { .. }));
        assert!(matches!(classify_sentence(&members[5]), SemanticKind::FunctionDef { .. }));
        assert!(matches!(classify_sentence(&members[6]), SemanticKind::FunctionDef { .. }));
    } else {
        panic!("Ожидалось тело Vec2");
    }
}

// --- ШАГ 9: Ранний выход через `Break`, роутеры и классические паттерны без `if`/`return` ---
#[test]
fn test_step_09_classical_patterns_and_break() {
    let code = r#"
Break(T) { value T, label Symbol 'default' }

classical_patterns_demo(numbers Tuple(Int), limit Int) [
  limit . {
    [ _ <= 0 ] [ 0 ]
    _ [
      numbers . fold(0, (acc Int, n Int) [
        (acc + n) . {
          [ _ > limit ] [ Break { value _ } ]
          _             [ _ ]
        }
      ])
    ]
  }
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::GenericDef { .. }));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::FunctionDef { .. }));
}

// --- ШАГ 10: Слоты функций, значения по умолчанию, возвращаемые типы и DI ---
#[test]
fn test_step_10_function_slots_return_types_and_di() {
    let code = r#"
Db {
  find_user(id String) User | NotFound [
    User { id id, name "Alice", age 25, role 'user', password_hash "xyz" }
  ]
  find_cart(Identifiable) [
    Cart { cart_id "c_for_{identifiable.id}", amount 2500 }
  ]
}

default_conn   connect()
partial_conn   connect("db.internal")
named_call     connect(port 9090, tls true)
custom_connect connect & { port 9090, tls true }
active_conn    custom_connect()
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 6);
}

// --- ШАГ 11: Пайплайны (., .>, .&, .?) и multi-query do-нотация ---
#[test]
fn test_step_11_pipeline_operators_and_do_notation() {
    let code = r#"
finalize_bill(ctx User & Cart) [
  ctx & {
    discount [
      self.amount . {
        [ _ >= 2000 ] [ 300 ]
        [ _ >= 1000 ] [ 100 ]
        _             [ 0 ]
      }
    ]
    final_price [ self.amount - self.discount ]
  }
  .> [ "Receipt for {_1.name}: {_1.final_price} USD (Discount: {_1.discount})" . Shell.log ]
]

load_user_and_friend(uid1 String, uid2 String) [
  { first_id uid1, second_id uid2 }
  .& [ _.first_id . Db.find_user . [ { u1 _ } ] ]
  .& [ _.second_id . Db.find_user . [ { u2 _ } ] ]
  .  [ "{_.u1.name} is friends with {_.u2.name}" ]
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
}

// --- ШАГ 12: Уточняющие типы (Refinement Types) и роутинг по типам (Law 1c) ---
#[test]
fn test_step_12_refinement_types_and_pattern_routing() {
    let code = r#"
is_adult(User)   [ user.age >= 18 ]
is_manager(User) [ user.role == ('admin' | 'manager') ]
EmailStr         String & [ _ . Regex.matches(r"^[^@]+@[^@]+\.[^@]+$") ]
Adult            User & is_adult

area(g Geometry) [
  g . {
    Circle [ 3.14159 * circle.radius * circle.radius ]
    Rect   [ rect.width * rect.height ]
  }
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 5);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::FunctionDef { .. }));
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::AlgebraicDef { .. }));
}

// --- ШАГ 13: Коллекции в круглых скобках (), zip, enumerate и FizzBuzz ---
#[test]
fn test_step_13_collections_zip_enumerate_and_fizzbuzz() {
    let code = r##"
format_numbered_catalog(items Tuple(Item), discounts Tuple(Int)) [
  items
  . zip(discounts)
  . enumerate
  . map((idx Int, (item Item, disc Int)) [
    "#{idx}: {item.name} -> {item.price - disc} USD"
  ])
]

fizz_buzz(limit Int) [
  (1..limit) . map((n Int) [
    (n % 3, n % 5) . {
      (0, 0) [ "FizzBuzz" ]
      (0, _) [ "Fizz" ]
      (_, 0) [ "Buzz" ]
      _      [ "{n}" ]
    }
  ])
]
"##;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
}

// --- ШАГ 14: Реактивные формы (`self`) и реактивные сигналы (`Signal`, `Effect`) ---
#[test]
fn test_step_14_reactive_forms_and_signals() {
    let code = r#"
Invoice {
  price    Money"100.00"
  quantity 3.0
  tax_rate 0.20

  subtotal [ self.price * self.quantity ]
  tax      [ self.subtotal * self.tax_rate ]
  total    [ self.subtotal + self.tax ]
}

count_sig   Signal(2)
doubled_sig [ count_sig.get() * 2 ]

run_signal_demo() [
  Effect([
    "Count: {count_sig.get()}, Doubled: {doubled_sig()}" . Shell.log
  ])
  count_sig.set(10)
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 4);
}

// --- ШАГ 15: Дженерики 1-го, 2-го (HKT `F(_)`) и 3-го порядка (`T(_(_), _)`) ---
#[test]
fn test_step_15_first_second_and_third_order_generics_hkt() {
    let code = r#"
Functor(F(_)) {
  fmap(A, B, fa F(A), f (A) [B]) F(B)
}

Monad(M(_)) & Functor(M) {
  pure(A, a A)                          M(A)
  flat_map(A, B, ma M(A), f (A) [M(B)]) M(B)
}

OptionT(M(_), A) {
  run M(Option(A))
}

MonadTrans(T(_(_), _)) {
  lift(M(_) Monad, A, ma M(A)) T(M, A)
}
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 4);
    assert!(matches!(classify_sentence(&ast[0]), SemanticKind::GenericDef { name, .. } if name == "Functor"));
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::GenericDef { name, .. } if name == "Monad"));
    assert!(matches!(classify_sentence(&ast[2]), SemanticKind::GenericDef { name, .. } if name == "OptionT"));
    assert!(matches!(classify_sentence(&ast[3]), SemanticKind::GenericDef { name, .. } if name == "MonadTrans"));
}

// --- ШАГ 16: Конкурентность (.|), каналы CSP и конечные автоматы (FSM) ---
#[test]
fn test_step_16_concurrency_channels_and_fsm() {
    let code = r#"
fetch_ads() [ Url"https://api.ex/ads" . Http.get . timeout(Duration"2s") ]

step_connection(state ConnState, event ConnEvent) ConnState [
  (state, event) . {
    (Disconnected, ConnectCmd)  [ Connecting { host connect_cmd.host, attempt 1 } ]
    (Connecting, HandshakeOk)   [ Connected { host connecting.host, session_id handshake_ok.session_id } ]
    (Connecting, NetworkError)  [ Disconnected { reason network_error.reason } ]
    (Connected, DisconnectCmd)  [ Disconnected { reason "User closed" } ]
    _                           [ state ]
  }
]
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    assert!(matches!(classify_sentence(&ast[1]), SemanticKind::FunctionDef { name, .. } if name == "step_connection"));
}

// --- ШАГ 17: Встроенные диалекты Parse (PEG) и Ui (View/Draw) ---
#[test]
fn test_step_17_parse_and_ui_draw_dialects() {
    let code = r##"
EmailGrammar String & [
  _ . Parse.run(
    Parse.one_or_more(alpha)
    & Parse.match("@")
    & Parse.one_or_more(alpha)
    & Parse.match(".")
    & Parse.repeat(2..6, alpha)
  )
]

counter_app(state Atom(CounterState)) [
  Ui.view({
    title "Razomy Counter"
    size  Pair"1920x1080"
    body  (
      heading("Current Count: {state.read().count}")
      canvas(Pair"200x100", [
        Ui.draw(
          pen(Hex"#FF0000"),
          fill_pen(Hex"#00FF00"),
          circle((50, 50), 40)
        )
      ])
    )
  })
]
"##;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
}

// --- ШАГ 18: Метапрограммирование (`& Compile`), `Meta.rewrite` и переписывание графов ---
#[test]
fn test_step_18_compile_time_metaprogramming_and_rewrite() {
    let code = r#"
Meta {
  type_of(entity) [ entity . native_meta_type_of ] & Compile
  rewrite(node, rules) [
    walked (node . {
      Form   [ node . Meta.map_fields((k, v) [ (k, v . Meta.rewrite(rules)) ]) ]
      Tuple  [ node . map([ _ . Meta.rewrite(rules) ]) ]
      Stream [
        node . Meta.steps_of
        . map([ _ . Meta.rewrite(rules) ])
        . fold([ _ ], (acc, step) [ acc & step ])
      ]
      _      [ node ]
    })
    walked . (rules & { _ [ walked ] })
  ] & Compile
}

UpgradedSystem (Compile & [
  ComplexSystem . Meta.rewrite({
    (calc(_))   [ (x Float) [ x * 1.20 ] ]
    (Shell.log) [ Audit.write ]
  })
])
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 2);
    if let Token::Group('{', members) = &ast[0][1] {
        assert_eq!(members.len(), 2);
        assert!(matches!(classify_sentence(&members[0]), SemanticKind::FunctionDef { name, .. } if name == "type_of"));
        assert!(matches!(classify_sentence(&members[1]), SemanticKind::FunctionDef { name, .. } if name == "rewrite"));
    } else {
        panic!("Expected Meta form");
    }
}

// --- ШАГ 19: Полная спецификация Razomy v5.7 целиком (Интеграционный тест) ---
#[test]
fn test_step_19_full_v5_7_specification_parses_100_percent() -> Result<(), Box<dyn std::error::Error>> {
    let full_spec = fs::read_to_string("./specification.rl")?;

    let ast = parse_code(&full_spec).expect("Вся спецификация v5.7 должна парситься без единой ошибки!");
    assert!(
        ast.len() > 160,
        "Ожидалось более 160 корневых инструкций в specification.rl, получено: {}",
        ast.len()
    );

    for sentence in &ast {
        let _ = classify_sentence(sentence);
    }
    Ok(())
}

#[test]
fn test_step_21_canonical_razomy_v5_7_imports() {
    let code = r#"
Base          root.base
(String, Int) root.base
_             root.base
_             base
_             Path"../../../base"
Http          std.net.http
Sqlite        std.db.sqlite
"#;
    let ast = parse_code(code).unwrap();
    assert_eq!(ast.len(), 7);

    // 1. Base root.base -> Namespaced
    let imp0 = extract_import(&ast[0]).unwrap();
    assert_eq!(
        imp0,
        ImportDirective::Namespaced {
            alias: "Base".into(),
            source: ImportSource::ModulePath("root.base".into())
        }
    );

    // 2. (String, Int) root.base -> Destructured
    let imp1 = extract_import(&ast[1]).unwrap();
    assert_eq!(
        imp1,
        ImportDirective::Destructured {
            names: vec!["String".into(), "Int".into()],
            source: ImportSource::ModulePath("root.base".into())
        }
    );

    // 3. _ root.base -> Wildcard
    let imp2 = extract_import(&ast[2]).unwrap();
    assert_eq!(
        imp2,
        ImportDirective::Wildcard {
            source: ImportSource::ModulePath("root.base".into())
        }
    );

    // 4. _ Path"../../../base" -> Wildcard from FilePath
    let imp4 = extract_import(&ast[4]).unwrap();
    assert_eq!(
        imp4,
        ImportDirective::Wildcard {
            source: ImportSource::FilePath("../../../base".into())
        }
    );
}