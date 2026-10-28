// tests/hir_tests.rs
use razomy_language::hir::{lower_to_hir, HirVisibility};
use razomy_language::parser::parse_code;

#[test]
fn hir_01_axioms_and_deep_lineage_history() {
    let code = r#"
HasParentName {
  ParentName String
}
Jonn String
JonnName HasParentName {
  ParentName Jonn
}
JonnDoy Jonn
JonnDoyName1 JonnName {
  ParentName JonnDoy
}
JonnDoyName2 JonnName {
  ParentName JonnDoy
} & { ParentName "JonnDoyBoy" }
"#;
    let ast = parse_code(code).expect("Парсинг должен пройти успешно");
    let hir = lower_to_hir(&ast);

    let jdn2 = hir.forms.get("JonnDoyName2").expect("JonnDoyName2 должен быть в HIR");
    // Активное значение переопределено через `& { ParentName "JonnDoyBoy" }` -> тип String
    let active_field = jdn2.fields.get("ParentName").unwrap();
    assert_eq!(active_field.inferred_type, "String");
    assert_eq!(active_field.origin_form, "JonnDoyName2");

    // Проверяем полную цепочку предков (Lineage)
    assert!(jdn2.ancestors.contains(&"JonnName".to_string()));
    assert!(jdn2.ancestors.contains(&"HasParentName".to_string()));

    // Проверяем сохраненную историю полей предков (Ancestor Slices)
    let jonn_name_slice = jdn2
        .ancestor_fields
        .get("JonnName")
        .expect("Срез предка JonnName должен быть сохранен");
    assert_eq!(jonn_name_slice.get("ParentName").unwrap().inferred_type, "Jonn");

    let base_slice = jdn2
        .ancestor_fields
        .get("HasParentName")
        .expect("Срез предка HasParentName должен быть сохранен");
    assert_eq!(base_slice.get("ParentName").unwrap().inferred_type, "String");
}

#[test]
fn hir_02_all_17_rich_human_literals_type_inference() {
    let code = r#"
HumanLiterals {
  IntVal      42
  FloatVal    3.14
  BoolVal     true
  ByteVal     0xFF
  StrVal      "hello"
  SymVal      'draft'
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
  LocalFile   %./data/users.json
}
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);
    let form = hir.forms.get("HumanLiterals").unwrap();

    let expected = [
        ("IntVal", "Int"),
        ("FloatVal", "Float"),
        ("BoolVal", "Bool"),
        ("ByteVal", "Byte"),
        ("StrVal", "String"),
        ("SymVal", "'draft'"),
        ("MoneyVal", "Money"),
        ("CurrencyVal", "Money"),
        ("PercentVal", "Percent"),
        ("DateVal", "Date"),
        ("TimeVal", "Time"),
        ("DurationVal", "Duration"),
        ("CoordPair", "Coord"),
        ("ColorVal", "Color"),
        ("IpAddr", "IpAddr"),
        ("VersionVal", "Version"),
        ("EmailAddr", "Email"),
        ("WebUrl", "Url"),
        ("LocalFile", "File"),
    ];

    for (field_name, expected_type) in expected {
        let f = form
            .fields
            .get(field_name)
            .unwrap_or_else(|| panic!("Поле {field_name} отсутствует"));
        assert_eq!(
            f.inferred_type, expected_type,
            "Неверный тип для поля {field_name}"
        );
    }
}

#[test]
fn hir_03_law2_lineage_collision_and_disambiguation_via_subtraction() {
    let code = r#"
Identifiable { Id String }
User Identifiable {
  Id           String
  Name         String
  Age          Int
  Role         'admin' | 'manager' | 'user' | 'guest'
  PasswordHash String
}
Cloud Identifiable {
  Id     String
  Name   String
  Region String
}

PublicUser        User / User.PasswordHash
AnonymousUsr      PublicUser / 'Name'
UserAndCloud      User & Cloud
UserWithCloudName UserAndCloud / User.Name
CloudWithUserName UserAndCloud / Cloud.Name
NoNameEntity      UserAndCloud / 'Name'
RestoredUser      UserAndCloud / Cloud
GuestUser         PublicUser / PublicUser.Role & { Role 'guest' }
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    // Case A: Вычитание поля по пути и по символу
    let pub_user = hir.forms.get("PublicUser").unwrap();
    assert!(!pub_user.fields.contains_key("PasswordHash"));
    assert!(pub_user.fields.contains_key("Name"));

    let anon_user = hir.forms.get("AnonymousUsr").unwrap();
    assert!(!anon_user.fields.contains_key("Name"));

    // Case B: Коллизия полей при слиянии `User & Cloud` (Law 2)
    let uac = hir.forms.get("UserAndCloud").unwrap();
    let name_field = uac.fields.get("Name").unwrap();
    assert!(
        name_field.is_ambiguous,
        "Поле Name должно быть помечено как неоднозначное (Law 2)"
    );
    assert!(name_field.conflicting_origins.contains(&"User".to_string()));
    assert!(name_field.conflicting_origins.contains(&"Cloud".to_string()));
    // Уникальные поля доступны напрямую без коллизии
    assert!(!uac.fields.get("Age").unwrap().is_ambiguous);
    assert!(!uac.fields.get("Region").unwrap().is_ambiguous);

    // Case C: Разрешение коллизии вычитанием конкретного предка `UserAndCloud / User.Name`
    let uwcn = hir.forms.get("UserWithCloudName").unwrap();
    let resolved_cloud_name = uwcn.fields.get("Name").unwrap();
    assert!(!resolved_cloud_name.is_ambiguous);
    assert_eq!(resolved_cloud_name.origin_form, "Cloud");

    let cwun = hir.forms.get("CloudWithUserName").unwrap();
    let resolved_user_name = cwun.fields.get("Name").unwrap();
    assert!(!resolved_user_name.is_ambiguous);
    assert_eq!(resolved_user_name.origin_form, "User");

    // Case D: Вычитание символьного ключа `'Name'` удаляет поле из всех предков
    let no_name = hir.forms.get("NoNameEntity").unwrap();
    assert!(!no_name.fields.contains_key("Name"));

    // Case E: Вычитание целого предка `UserAndCloud / Cloud`
    let restored = hir.forms.get("RestoredUser").unwrap();
    assert!(!restored.fields.contains_key("Region"));
    let restored_name = restored.fields.get("Name").unwrap();
    assert!(
        !restored_name.is_ambiguous,
        "После удаления предка Cloud коллизия на Name должна исчезнуть!"
    );
    assert_eq!(restored_name.origin_form, "User");

    // Case F: Вычитание и переопределение в одном выражении
    let guest = hir.forms.get("GuestUser").unwrap();
    assert_eq!(guest.fields.get("Role").unwrap().inferred_type, "'guest'");
}

#[test]
fn hir_04_law1c_named_vs_anonymous_function_slots_and_defaults() {
    let code = r#"
Identifiable { Id String }
User Identifiable { Id String, Name String }
Cart { CartId String, Amount Int }
Order { Id String, Amount Int }
Cancelled { CancelledAt Int }

FindUser(Id String) [ Id ]
FindCart(Identifiable) [ Id ]
FinalizeBill(User & Cart) [ Amount ]
ReserveStock(O Order / Cancelled) [ O.Amount ]
Connect(Host "localhost", Port 8080, Tls false) [ Host ]
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    // 1. Именованный параметр `FindUser(Id String)` НЕ распаковывает поля (Law 1c)
    let find_user = hir.functions.get("FindUser").unwrap();
    assert!(!find_user.auto_unpacks);
    assert_eq!(find_user.params[0].name, "Id");
    assert_eq!(find_user.params[0].type_constraint, "String");
    assert!(!find_user.params[0].is_anonymous_type_slot);

    // 2. Анонимный слот `FindCart(Identifiable)` распаковывает поля (Law 1c)
    let find_cart = hir.functions.get("FindCart").unwrap();
    assert!(find_cart.auto_unpacks);
    assert!(find_cart.params[0].is_anonymous_type_slot);
    assert_eq!(find_cart.params[0].type_constraint, "Identifiable");

    // 3. Анонимный пересеченный слот `FinalizeBill(User & Cart)`
    let finalize = hir.functions.get("FinalizeBill").unwrap();
    assert!(finalize.auto_unpacks);
    assert!(finalize.params[0].is_anonymous_type_slot);
    assert_eq!(finalize.params[0].type_constraint, "User & Cart");

    // 4. Именованный параметр со сложным типом вычитания `ReserveStock(O Order / Cancelled)`
    let reserve = hir.functions.get("ReserveStock").unwrap();
    assert!(!reserve.auto_unpacks);
    assert_eq!(reserve.params[0].name, "O");
    assert_eq!(reserve.params[0].type_constraint, "Order / Cancelled");

    // 5. Дефолтные параметры `Connect(Host "localhost", Port 8080, Tls false)`
    let connect = hir.functions.get("Connect").unwrap();
    assert!(!connect.auto_unpacks);
    assert_eq!(connect.params.len(), 3);
    assert_eq!(connect.params[0].type_constraint, "String");
    assert!(!connect.params[0].default_expr.is_empty());
    assert_eq!(connect.params[1].type_constraint, "Int");
    assert_eq!(connect.params[2].type_constraint, "Bool");
}

#[test]
fn hir_05_reactive_fields_and_operator_overloading_in_forms() {
    let code = r#"
Invoice {
  Price    $100.00
  Quantity 3
  TaxRate  20%

  Subtotal [ Price * Quantity ]
  Tax      [ Subtotal * TaxRate ]
  Total    [ Subtotal + Tax ]
}

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
    let hir = lower_to_hir(&ast);

    let inv = hir.forms.get("Invoice").unwrap();
    assert!(!inv.fields.get("Price").unwrap().is_reactive);
    assert!(inv.fields.get("Subtotal").unwrap().is_reactive);
    assert!(inv.fields.get("Tax").unwrap().is_reactive);
    assert!(inv.fields.get("Total").unwrap().is_reactive);

    let vec2 = hir.forms.get("Vec2").unwrap();
    assert_eq!(vec2.methods.len(), 4);
    assert!(vec2.methods.contains_key("+"));
    assert!(vec2.methods.contains_key("-"));
    assert!(vec2.methods.contains_key("*"));
    assert!(vec2.methods.contains_key("=="));
    assert!(!vec2.methods.get("+").unwrap().is_contract);
}

#[test]
fn hir_06_generics_recursive_types_and_f_bounded_contracts() {
    let code = r#"
Map(K, V) { (K) V }
None {}
Cons(T) {
  Head T
  Tail List(T)
}
List(T) None | Cons(T)

Comparable(Other Self) {
  <(Other) Bool
}
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    let map_form = hir.forms.get("Map").expect("Дженерик Map должен быть в HIR");
    assert_eq!(map_form.type_params, vec!["K", "V"]);
    assert!(map_form.fields.contains_key("(K)"));

    let cons_form = hir.forms.get("Cons").expect("Дженерик Cons должен быть в HIR");
    assert_eq!(cons_form.type_params, vec!["T"]);
    assert_eq!(cons_form.fields.get("Tail").unwrap().inferred_type, "List(T)");

    let list_form = hir.forms.get("List").expect("Алгебраический дженерик List должен быть в HIR");
    assert_eq!(list_form.type_params, vec!["T"]);
    assert_eq!(list_form.union_variants, vec!["None", "Cons"]);

    let comp = hir.forms.get("Comparable").expect("Comparable должен быть в HIR");
    assert_eq!(comp.type_params, vec!["Other"]);
    let lt_contract = comp.methods.get("<").expect("Контракт `<` должен быть в HIR");
    assert!(lt_contract.is_contract);
    assert_eq!(lt_contract.return_type.as_deref(), Some("Bool"));
}

#[test]
fn hir_07_module_visibility_encapsulation_and_subtraction() {
    let code = r#"
Public  { }
Private { }

AuthInternal {
  Salt         "x9f2a" & Private
  Hash(Raw)    [ "{Raw}_{Salt}" . native_sha256 ] & Private
  Verify(U, P) [ (P . Hash) == U.PasswordHash ] & Public
}

AuthModule  AuthInternal & Public
CleanModule AuthInternal / 'Salt' / 'Hash'
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    let internal = hir.forms.get("AuthInternal").unwrap();
    assert_eq!(
        internal.fields.get("Salt").unwrap().visibility,
        HirVisibility::Private
    );
    assert_eq!(
        internal.methods.get("Hash").unwrap().visibility,
        HirVisibility::Private
    );
    assert_eq!(
        internal.methods.get("Verify").unwrap().visibility,
        HirVisibility::Public
    );

    // `AuthInternal & Public` оставляет только Public члены
    let pub_mod = hir.forms.get("AuthModule").unwrap();
    assert!(!pub_mod.fields.contains_key("Salt"));
    assert!(!pub_mod.methods.contains_key("Hash"));
    assert!(pub_mod.methods.contains_key("Verify"));

    // `AuthInternal / 'Salt' / 'Hash'` явно вычитает и поле, и метод
    let clean_mod = hir.forms.get("CleanModule").unwrap();
    assert!(!clean_mod.fields.contains_key("Salt"));
    assert!(!clean_mod.methods.contains_key("Hash"));
    assert!(clean_mod.methods.contains_key("Verify"));
}

#[test]
fn hir_08_refinement_types_and_unions() {
    let code = r#"
Status 'draft' | 'published' | 'archived'
User { Name String, Age Int }
IsAdult [ _.Age >= 18 ]
Adult User & IsAdult
CompileError { Code Int, Log String }
FatalError CompileError & [ _.Code >= 500 ]
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    let status = hir.forms.get("Status").unwrap();
    assert_eq!(
        status.union_variants,
        vec!["'draft'", "'published'", "'archived'"]
    );

    let adult = hir.forms.get("Adult").unwrap();
    assert!(adult.ancestors.contains(&"User".to_string()));
    assert_eq!(adult.refinement_predicates, vec!["IsAdult"]);

    let fatal = hir.forms.get("FatalError").unwrap();
    assert!(fatal.ancestors.contains(&"CompileError".to_string()));
    assert_eq!(fatal.refinement_predicates, vec!["<inline_predicate>"]);
}

#[test]
fn hir_09_full_v4_7_specification_lowers_cleanly() {
    // Проверяем, что вся 20-секционная спецификация Razomy V4.7 строится в HIR без паник и потерь
    let code = r#"
Identifiable { Id String }
User Identifiable { Id String, Name String, Age Int, Role 'admin' | 'user', PasswordHash String }
PublicUser User / User.PasswordHash
Invoice {
  Price $100.00
  Quantity 3
  Subtotal [ Price * Quantity ]
}
Order1 Invoice & { Quantity 5 }
Product { Id Int, Title String } . DeriveJson
(String, Int) root.base
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    assert!(hir.forms.contains_key("Identifiable"));
    assert!(hir.forms.contains_key("User"));
    assert!(hir.forms.contains_key("PublicUser"));
    assert!(hir.forms.contains_key("Invoice"));
    assert!(hir.forms.contains_key("Order1"));
    assert!(hir.forms.contains_key("Product"));
    assert!(hir.globals.contains_key("String"));
    assert!(hir.globals.contains_key("Int"));
}

#[test]
fn hir_10_advanced_spec_v4_7_edge_cases() {
    let code = r#"
// 1. Числа Пеано и инстанцирование дженериков (Раздел 11.1)
Zero    {}
Succ(N) { Prev N }
One     Succ(Zero)
Two     Succ(One)

// 2. Дженерик-наследование с подстановкой типа T -> CounterState (Раздел 13.3)
CounterState { Count Int }
Atom(T) {
  Value  T
  Read() [ Self . native_atom_read ]
}
GlobalCounter Atom(CounterState) {
  Value CounterState { Count 0 }
}

// 3. Диапазоны в объединениях (Раздел 16.1)
Digit "0".."9"
Alpha "a".."z" | "A".."Z"

// 4. Композиция потоков и макрос DeriveJson (Раздел 17)
Step1 [ _ + 10 ]
Step2 [ _ * 2 ]
CombinedPipeline Step1 & Step2

DeriveJson(TargetForm) [
  TargetForm & {
    ToJson() [ "{}" ]
  }
]
Product { Id Int, Title String } . DeriveJson

// 5. Вложенные подмодули (Раздел 17.5)
ComplexSystem {
  Name "Core"
  SubModule {
    Nested {
      Calc [ _ * 1.15 ]
    }
  }
}

// 6. Импорты (Раздел 18.1)
Base          root.base
(String, Int) root.base
_             %../../../base
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    // 1. Проверяем инстанцирование Пеано `One Succ(Zero)`
    let one_form = hir.forms.get("One").expect("One должен быть формой в HIR");
    assert_eq!(one_form.type_args, vec!["Zero"]);
    assert_eq!(one_form.fields.get("Prev").unwrap().inferred_type, "Zero");

    let two_form = hir.forms.get("Two").expect("Two должен быть формой в HIR");
    assert_eq!(two_form.fields.get("Prev").unwrap().inferred_type, "One");

    // 2. Проверяем GlobalCounter Atom(CounterState)
    let gc = hir.forms.get("GlobalCounter").unwrap();
    assert_eq!(gc.type_args, vec!["CounterState"]);
    assert!(gc.methods.contains_key("Read"));

    // 3. Проверяем Alpha "a".."z" | "A".."Z"
    let alpha = hir.forms.get("Alpha").unwrap();
    assert_eq!(alpha.union_variants, vec!["\"a\"..\"z\"", "\"A\"..\"Z\""]);

    // 4. Проверяем композицию потоков `CombinedPipeline Step1 & Step2` и макрос `DeriveJson`
    assert!(
        hir.functions.contains_key("CombinedPipeline"),
        "CombinedPipeline должен быть зарегистрирован как функция-поток"
    );
    let product = hir.forms.get("Product").unwrap();
    assert!(
        product.methods.contains_key("ToJson"),
        "Макрос DeriveJson должен статически добавить метод ToJson в Product"
    );

    // 5. Проверяем вложенные подмодули `ComplexSystem.SubModule.Nested`
    let cs = hir.forms.get("ComplexSystem").unwrap();
    let sub = cs.submodules.get("SubModule").expect("SubModule должен быть в submodules");
    let nested = sub.submodules.get("Nested").expect("Nested должен быть в SubModule.submodules");
    assert!(nested.fields.get("Calc").unwrap().is_reactive);

    // 6. Проверяем таблицу импортов
    assert_eq!(hir.imports.len(), 3);
    assert_eq!(hir.imports[0].targets, vec!["Base"]);
    assert_eq!(hir.imports[1].targets, vec!["String", "Int"]);
    assert_eq!(hir.imports[2].source_path, "%../../../base");
}