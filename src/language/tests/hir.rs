// tests/hir.rs
// =============================================================================
// RAZOMY V5.7 — HIGH-LEVEL INTERMEDIATE REPRESENTATION (HIR) TEST SUITE
// =============================================================================

use razomy_language::hir::{lower_to_hir, HirVisibility};
use razomy_language::parser::parse_code;
use std::fs;

#[test]
fn hir_01_axioms_and_deep_lineage_history_v57() {
    let code = r#"
BrandName String

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
    let ast = parse_code(code).expect("Парсинг должен пройти успешно");
    let hir = lower_to_hir(&ast);

    let ev = hir
        .forms
        .get("ElectricVehicle")
        .expect("ElectricVehicle должен быть в HIR");
    assert!(ev.ancestors.contains(&"Vehicle".to_string()));
    assert!(ev.ancestors.contains(&"HasBattery".to_string()));
    assert_eq!(ev.fields.get("battery_kwh").unwrap().inferred_type, "Int");
    assert_eq!(
        ev.fields.get("charging_port").unwrap().inferred_type,
        "String"
    );

    let my_car = hir.forms.get("MyCar").expect("MyCar должен быть в HIR");
    let active_brand = my_car.fields.get("brand").unwrap();
    assert_eq!(active_brand.inferred_type, "String");
    assert_eq!(active_brand.origin_form, "MyCar");

    // Проверяем полную цепочку предков (Lineage)
    assert!(my_car.ancestors.contains(&"SportsCar".to_string()));
    assert!(my_car.ancestors.contains(&"Car".to_string()));
    assert!(my_car.ancestors.contains(&"Vehicle".to_string()));

    // Проверяем сохраненную историю полей предков (Ancestor Slices)
    let sports_slice = my_car
        .ancestor_fields
        .get("SportsCar")
        .expect("Срез предка SportsCar должен быть сохранен");
    assert_eq!(sports_slice.get("brand").unwrap().origin_form, "SportsCar");

    let vehicle_slice = my_car
        .ancestor_fields
        .get("Vehicle")
        .expect("Срез предка Vehicle должен быть сохранен");
    assert_eq!(
        vehicle_slice.get("brand").unwrap().inferred_type,
        "BrandName"
    );
}

#[test]
fn hir_02_primitives_tagged_strings_and_raw_strings_type_inference() {
    let code = r##"
HumanLiterals {
  int_val       42
  float_val     3.14
  bool_val      true
  byte_val      0xFF
  str_val       "hello"
  raw_str_val   r"^[^@]+@[^@]+\.[^@]+$"
  sym_val       'draft'
  money_val     Money"199.99"
  currency_val  Usd"500.00"
  date_val      Date"2026-03-30"
  time_val      Time"14:30:00"
  duration_val  Duration"5m30s"
  coord_pair    Pair"1920x1080"
  color_val     Hex"#2A9D8F"
  ip_addr       Ip"192.168.0.1:8080"
  version_val   Version"1.4.2-beta"
  web_url       Url"https://razomy.org"
  local_file    Path"./data/users.json"
  tax_rate      0.155
}
"##;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);
    let form = hir.forms.get("HumanLiterals").unwrap();

    let expected = [
        ("int_val", "Int"),
        ("float_val", "Float"),
        ("bool_val", "Bool"),
        ("byte_val", "Byte"),
        ("str_val", "String"),
        ("raw_str_val", "String"),
        ("sym_val", "'draft'"),
        ("money_val", "Money"),
        ("currency_val", "Usd"),
        ("date_val", "Date"),
        ("time_val", "Time"),
        ("duration_val", "Duration"),
        ("coord_pair", "Pair"),
        ("color_val", "Hex"),
        ("ip_addr", "Ip"),
        ("version_val", "Version"),
        ("web_url", "Url"),
        ("local_file", "Path"),
        ("tax_rate", "Float"),
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
Identifiable { id String }
User Identifiable {
  id            String
  name          String
  age           Int
  role          'admin' | 'manager' | 'user' | 'guest'
  password_hash String
}
Cloud Identifiable {
  id     String
  name   String
  region String
}

PublicUser        User / User.password_hash
AnonymousUsr      PublicUser / 'name'
UserAndCloud      User & Cloud
UserWithCloudName UserAndCloud / User.name
CloudWithUserName UserAndCloud / Cloud.name
NoNameEntity      UserAndCloud / 'name'
RestoredUser      UserAndCloud / Cloud
GuestUser         PublicUser / PublicUser.role & { role 'guest' }
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    let pub_user = hir.forms.get("PublicUser").unwrap();
    assert!(!pub_user.fields.contains_key("password_hash"));
    assert!(pub_user.fields.contains_key("name"));

    let anon_user = hir.forms.get("AnonymousUsr").unwrap();
    assert!(!anon_user.fields.contains_key("name"));

    let uac = hir.forms.get("UserAndCloud").unwrap();
    let name_field = uac.fields.get("name").unwrap();
    assert!(name_field.is_ambiguous);
    assert!(name_field.conflicting_origins.contains(&"User".to_string()));
    assert!(name_field
        .conflicting_origins
        .contains(&"Cloud".to_string()));
    assert!(!uac.fields.get("age").unwrap().is_ambiguous);
    assert!(!uac.fields.get("region").unwrap().is_ambiguous);

    let uwcn = hir.forms.get("UserWithCloudName").unwrap();
    let resolved_cloud_name = uwcn.fields.get("name").unwrap();
    assert!(!resolved_cloud_name.is_ambiguous);
    assert_eq!(resolved_cloud_name.origin_form, "Cloud");

    let cwun = hir.forms.get("CloudWithUserName").unwrap();
    let resolved_user_name = cwun.fields.get("name").unwrap();
    assert!(!resolved_user_name.is_ambiguous);
    assert_eq!(resolved_user_name.origin_form, "User");

    let no_name = hir.forms.get("NoNameEntity").unwrap();
    assert!(!no_name.fields.contains_key("name"));

    let restored = hir.forms.get("RestoredUser").unwrap();
    assert!(!restored.fields.contains_key("region"));
    let restored_name = restored.fields.get("name").unwrap();
    assert!(!restored_name.is_ambiguous);
    assert_eq!(restored_name.origin_form, "User");

    let guest = hir.forms.get("GuestUser").unwrap();
    assert_eq!(guest.fields.get("role").unwrap().inferred_type, "'guest'");
}

#[test]
fn hir_04_law1c_single_slot_projection_and_defaults_v57() {
    let code = r#"
Identifiable { id String }
User Identifiable { id String, name String }
Cart { cart_id String, amount Int }
Order { id String, amount Int }
Cancelled { cancelled_at Int }
NotFound { entity String, id String } & Bypass

find_user(id String) User | NotFound [
  User { id id, name "Alice" }
]
find_cart(Identifiable) [
  Cart { cart_id "c_for_{identifiable.id}", amount 2500 }
]
finalize_bill(ctx User & Cart) [ ctx.amount ]
reserve_stock(o Order / Cancelled) [ o.amount ]
connect(host String "localhost", port Int 8080, tls Bool false) [ host ]
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    // 1. `NotFound` помечен тегом `& Bypass` (Law 3)
    assert!(hir.forms.get("NotFound").unwrap().is_bypass);

    // 2. `find_user(id String) User | NotFound`
    let find_user = hir.functions.get("find_user").unwrap();
    assert!(!find_user.auto_unpacks);
    assert_eq!(find_user.params[0].name, "id");
    assert_eq!(find_user.params[0].type_constraint, "String");
    assert_eq!(
        find_user.return_type.as_deref(),
        Some("User | NotFound")
    );

    // 3. Закон 1c: Одиночный известный тип `find_cart(Identifiable)` авто-привязывает `identifiable` (`snake_case`!)
    let find_cart = hir.functions.get("find_cart").unwrap();
    assert!(find_cart.auto_unpacks);
    assert!(find_cart.params[0].is_anonymous_type_slot);
    assert_eq!(find_cart.params[0].name, "identifiable");
    assert_eq!(find_cart.params[0].type_constraint, "Identifiable");

    // 4. Закон 1d: Составной слот `finalize_bill(ctx User & Cart)` имеет явное имя `ctx`
    let finalize = hir.functions.get("finalize_bill").unwrap();
    assert!(!finalize.auto_unpacks);
    assert_eq!(finalize.params[0].name, "ctx");
    assert_eq!(finalize.params[0].type_constraint, "User & Cart");

    // 5. `reserve_stock(o Order / Cancelled)`
    let reserve = hir.functions.get("reserve_stock").unwrap();
    assert_eq!(reserve.params[0].name, "o");
    assert_eq!(reserve.params[0].type_constraint, "Order / Cancelled");

    // 6. Дефолтные параметры `connect(host String "localhost", port Int 8080, tls Bool false)`
    let connect = hir.functions.get("connect").unwrap();
    assert_eq!(connect.params.len(), 3);
    assert_eq!(connect.params[0].name, "host");
    assert_eq!(connect.params[0].type_constraint, "String");
    assert!(!connect.params[0].default_expr.is_empty());
    assert_eq!(connect.params[1].type_constraint, "Int");
    assert_eq!(connect.params[2].type_constraint, "Bool");
}

#[test]
fn hir_05_reactive_fields_and_operator_overloading_in_forms() {
    let code = r#"
Invoice {
  price    Money"100.00"
  quantity 3.0
  tax_rate 0.20

  subtotal [ self.price * self.quantity ]
  tax      [ self.subtotal * self.tax_rate ]
  total    [ self.subtotal + self.tax ]
}

Vec2 {
  x Float
  y Float

  +  (other Vec2)  [ Vec2 { x (self.x + other.x), y (self.y + other.y) } ]
  -  (other Vec2)  [ Vec2 { x (self.x - other.x), y (self.y - other.y) } ]
  *  (scale Float) [ Vec2 { x (self.x * scale),   y (self.y * scale) } ]
  /  (scale Float) [ Vec2 { x (self.x / scale),   y (self.y / scale) } ]
  == (other Vec2)  [ (self.x == other.x) && (self.y == other.y) ]
}
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    let inv = hir.forms.get("Invoice").unwrap();
    assert!(!inv.fields.get("price").unwrap().is_reactive);
    assert!(inv.fields.get("subtotal").unwrap().is_reactive);
    assert!(inv.fields.get("tax").unwrap().is_reactive);
    assert!(inv.fields.get("total").unwrap().is_reactive);

    let vec2 = hir.forms.get("Vec2").unwrap();
    assert_eq!(vec2.methods.len(), 5);
    assert!(vec2.methods.contains_key("+"));
    assert!(vec2.methods.contains_key("-"));
    assert!(vec2.methods.contains_key("*"));
    assert!(vec2.methods.contains_key("/"));
    assert!(vec2.methods.contains_key("=="));
}

#[test]
fn hir_06_first_second_and_third_order_generics_hkt() {
    let code = r#"
Map(K, V) { (K) V }
None {} & Bypass
Cons(T) {
  head T
  tail List(T)
}
List(T) None | Cons(T)

Comparable(Other) {
  < (other Other) Bool
}

Functor(F(_)) {
  fmap(A, B, fa F(A), f (A) [B]) F(B)
}

Monad(M(_)) & Functor(M) {
  pure(A, a A)                          M(A)
  flat_map(A, B, ma M(A), f (A) [M(B)]) M(B)
}

MonadTrans(T(_(_), _)) {
  lift(M(_) Monad, A, ma M(A)) T(M, A)
}
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    let map_form = hir.forms.get("Map").unwrap();
    assert_eq!(map_form.type_params, vec!["K", "V"]);
    assert!(map_form.fields.contains_key("(K)"));

    let functor = hir.forms.get("Functor").unwrap();
    assert_eq!(functor.type_params, vec!["F"]);
    let fmap = functor.methods.get("fmap").unwrap();
    assert!(fmap.is_contract);
    assert!(fmap.params[0].is_type_param); // `A`
    assert!(fmap.params[1].is_type_param); // `B`
    assert!(!fmap.params[2].is_type_param); // `fa F(A)`

    let monad = hir.forms.get("Monad").unwrap();
    assert_eq!(monad.type_params, vec!["M"]);
    assert!(monad.ancestors.contains(&"Functor".to_string()));
    assert!(monad.methods.contains_key("pure"));
    assert!(monad.methods.contains_key("flat_map"));

    let monad_trans = hir.forms.get("MonadTrans").unwrap();
    assert_eq!(monad_trans.type_params, vec!["T"]);
    assert!(monad_trans.methods.contains_key("lift"));
}

#[test]
fn hir_07_module_visibility_encapsulation_via_subtract_private() {
    let code = r#"
Public  {}
Private {}

AuthInternal {
  salt                     "x9f2a" & Private
  hash(raw String)         [ "{raw}_{self.salt}" . native_sha256 ] & Private
  verify(u User, p String) [ (p . self.hash) == u.password_hash ] & Public
}

AuthModule  AuthInternal / Private
CleanModule AuthInternal / 'salt' / 'hash'
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    let internal = hir.forms.get("AuthInternal").unwrap();
    assert_eq!(
        internal.fields.get("salt").unwrap().visibility,
        HirVisibility::Private
    );
    assert_eq!(
        internal.methods.get("hash").unwrap().visibility,
        HirVisibility::Private
    );
    assert_eq!(
        internal.methods.get("verify").unwrap().visibility,
        HirVisibility::Public
    );

    // `AuthInternal / Private` удаляет все поля и методы с тегом `& Private`!
    let pub_mod = hir.forms.get("AuthModule").unwrap();
    assert!(!pub_mod.fields.contains_key("salt"));
    assert!(!pub_mod.methods.contains_key("hash"));
    assert!(pub_mod.methods.contains_key("verify"));

    let clean_mod = hir.forms.get("CleanModule").unwrap();
    assert!(!clean_mod.fields.contains_key("salt"));
    assert!(!clean_mod.methods.contains_key("hash"));
    assert!(clean_mod.methods.contains_key("verify"));
}

#[test]
fn hir_08_full_v5_7_specification_lowers_cleanly() {
    let spec = fs::read_to_string("./specification.rl").expect("Failed to read specification.rl");
    let ast = parse_code(&spec).unwrap();
    let hir = lower_to_hir(&ast);

    assert!(hir.forms.contains_key("Vehicle"));
    assert!(hir.forms.contains_key("ElectricVehicle"));
    assert!(hir.forms.contains_key("MyCar"));
    assert!(hir.forms.contains_key("Money"));
    assert!(hir.forms.contains_key("PublicUser"));
    assert!(hir.forms.contains_key("Invoice"));
    assert!(hir.forms.contains_key("Functor"));
    assert!(hir.forms.contains_key("Monad"));
    assert!(hir.forms.contains_key("OptionMonad"));
    assert!(hir.forms.contains_key("AuthModule"));
    assert!(hir.forms.contains_key("Product"));
    assert!(hir.functions.contains_key("main"));

    // Verify that `Monad(_M(_)) { ..Functor(_M) }` substituted `_F` -> `_M` in inherited `fmap`
    let monad = hir.forms.get("Monad").unwrap();
    let monad_fmap = monad.methods.get("fmap").unwrap();
    assert_eq!(monad_fmap.params[0].type_constraint, "_M(_A)");
    assert_eq!(monad_fmap.return_type.as_deref(), Some("_M(_B)"));
}

#[test]
fn hir_09_return_autotype_checks_all_leaves_of_function() {
    let code = r#"
User     { id String, name String }
Company  { id String, title String }
NotFound { entity String, id String } & Bypass
Invalid  { reason String } & Bypass

route_actor(kind Symbol, id String) [
  id == "" .! Invalid { reason "empty id" }
  kind . {
    'user'    [ User { id id, name "Alice" } ]
    'company' [ Company { id id, title "Acme" } ]
    _         [ NotFound { entity "Actor", id id } ]
  }
]

early_return_branch(flag Bool) [
  flag .? Return("early_str")
  1_000_000_000
]
"#;
    let ast = parse_code(code).unwrap();
    let hir = lower_to_hir(&ast);

    let route_fn = hir.functions.get("route_actor").unwrap();
    assert_eq!(
        route_fn.effective_return_type(),
        Some("Invalid | User | Company | NotFound")
    );

    let early_fn = hir.functions.get("early_return_branch").unwrap();
    assert_eq!(early_fn.effective_return_type(), Some("String | Int"));
}
