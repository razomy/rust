// tests/analyzer.rs
// =============================================================================
// RAZOMY V5.7 — STATIC HIR ANALYZER, ERROR CATALOG (E01..E07) & IDE TEST SUITE
// =============================================================================

use razomy_language::analyzer::analyze_source;
use razomy_language::hir::HirVisibility;

#[test]
fn test_analyzer_law1_named_vs_auto_named_slots_v57() {
    let code = r#"
        User { id String, name String, age Int }

        greet_anon(User) [
            "Hello, {user.name}!"
        ]

        greet_named(u User) [
            "Hello, {name}!"
        ]
    "#;

    let report = analyze_source(code);

    let anon_fn = report
        .hir
        .functions
        .get("greet_anon")
        .expect("greet_anon function");
    assert!(anon_fn.auto_unpacks);
    assert_eq!(anon_fn.params[0].name, "user");

    let named_fn = report
        .hir
        .functions
        .get("greet_named")
        .expect("greet_named function");
    assert!(!named_fn.auto_unpacks);

    let law1_warnings: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("W001_LAW1"))
        .collect();
    assert_eq!(law1_warnings.len(), 1);
    assert_eq!(
        law1_warnings[0].context_symbol.as_deref(),
        Some("greet_named")
    );
}

#[test]
fn test_analyzer_e01_ambiguous_call_or_bind() {
    let code = r#"
        Circle { radius Float }
        area(Circle) [ 3.14159 * circle.radius * circle.radius ]

        bad_caller() [
            area (Circle { radius 5.0 })
        ]
    "#;

    let report = analyze_source(code);
    let e01: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "E01_AMBIGUOUS_CALL_OR_BIND")
        .collect();
    assert_eq!(
        e01.len(),
        1,
        "Ожидалась ошибка E01_AMBIGUOUS_CALL_OR_BIND, получено: {:?}",
        report.diagnostics
    );
}

#[test]
fn test_analyzer_e02_duplicate_unnamed_type_and_e03_type_param_shadows_type() {
    let code = r#"
        Account { iban String, balance Int }
        User    { id String, name String }

        // E02: Duplicate unnamed type `Account`
        transfer_bad(Account, Account) [
            account
        ]

        // E03: `User` already exists in scope; using `User` as a generic type param inside `{ item User }` is rejected!
        Wrapper(User) {
            item User
        }
    "#;

    let report = analyze_source(code);

    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "E02_DUPLICATE_UNNAMED_TYPE"),
        "Ожидалась ошибка E02_DUPLICATE_UNNAMED_TYPE: {:?}",
        report.diagnostics
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "E03_TYPE_PARAM_SHADOWS_TYPE"),
        "Ожидалась ошибка E03_TYPE_PARAM_SHADOWS_TYPE: {:?}",
        report.diagnostics
    );
}

#[test]
fn test_analyzer_law3_e04_unhandled_union_branch_and_typestate() {
    let code = r#"
        User     { id String, name String }
        Company  { id String, name String }
        NotFound { entity String, id String } & Bypass

        Order     { id String, amount Int }
        Reserved  { reserved_at Int }
        Paid      { transaction_id String }
        Cancelled { cancelled_at Int }

        fetch_actor(id String) User | Company | NotFound [
            User { id id, name "Alice" }
        ]

        send_welcome_email(User) [ "Sent email to {user.name}" ]

        // E04: `NotFound` is `& Bypass` (OK), `User` matches `send_welcome_email` (OK),
        // but `Company` is NOT a subtype of `User` and is NOT `& Bypass`!
        handle_actor_bad(id String) [
            id . fetch_actor . send_welcome_email
        ]

        ship_to_customer(order Order & Reserved & Paid) [
            "Shipping {order.id}"
        ]

        reserve_stock(o Order / Cancelled) [
            o.amount
        ]

        raw_order       Order { id "o1", amount 100 }
        cancelled_order Order { id "o2", amount 100 } & Cancelled

        test_typestate_violations() [
            raw_order . ship_to_customer
            cancelled_order . reserve_stock
        ]
    "#;

    let report = analyze_source(code);

    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.code == "E04_UNHANDLED_UNION_BRANCH"),
        "Ожидалась ошибка E04_UNHANDLED_UNION_BRANCH: {:?}",
        report.diagnostics
    );

    let typestate_errors: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("E003_TYPESTATE"))
        .collect();
    assert!(
        typestate_errors.len() >= 2,
        "Ожидались ошибки Typestate, получено: {:?}",
        report.diagnostics
    );
}

#[test]
fn test_analyzer_law2_e05_ambiguous_lineage_field_and_e06_immutable_type_binding() {
    let code = r#"
        User  { id String, name String }
        Cloud { id String, name String, region String }
        Car   { brand String, max_speed Int }

        UserAndCloud      User & Cloud
        UserWithCloudName UserAndCloud / User.name

        resolve_diamond_bad(entity User & Cloud) [
            entity.name . Shell.log
        ]

        resolve_diamond_good(entity User & Cloud) [
            clean_entity (entity / Cloud.name)
            clean_entity.name . Shell.log
        ]

        immutable_bad() [
            Car.max_speed 500
        ]
    "#;

    let report = analyze_source(code);

    let e05: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "E05_AMBIGUOUS_LINEAGE_FIELD")
        .collect();
    assert_eq!(e05.len(), 1);
    assert_eq!(
        e05[0].context_symbol.as_deref(),
        Some("resolve_diamond_bad")
    );

    let e06: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "E06_IMMUTABLE_TYPE_BINDING")
        .collect();
    assert_eq!(e06.len(), 1);
    assert_eq!(e06[0].context_symbol.as_deref(), Some("immutable_bad"));
}

#[test]
fn test_analyzer_law4_hover_and_ufcs_completions() {
    let code = r#"
        Identifiable { id String }
        User Identifiable {
            name String
            age  Int
            greet() [ "Hi {self.name}" ]
        }

        migrate_user(User) [ user.name ]
    "#;

    let report = analyze_source(code);
    let sym = report.hir.forms.get("User").unwrap();
    assert!(!sym.is_algebraic_alias);

    let hover_md = report.hover("User").expect("Hover markdown");
    assert!(hover_md.contains("Identifiable"));
    assert!(hover_md.contains("name"));
    assert!(hover_md.contains("greet"));

    let completions = report.completions("User.");
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    assert!(labels.contains(&"id"));
    assert!(labels.contains(&"name"));
    assert!(labels.contains(&"greet"));
    assert!(labels.contains(&"Identifiable"));
    // Закон 4 (UFCS Autocomplete): внешняя функция `migrate_user(User)` предлагается при вводе `User.`!
    assert!(labels.contains(&"migrate_user"));
}

#[test]
fn test_analyzer_module_visibility_and_imports_v57() {
    let code = r#"
        Base          root.base
        (String, Int) root.base
        _             Path"../../../base"

        AuthInternal {
            salt                     "x9f2a" & Private
            hash(raw String)         [ raw ] & Private
            verify(u User, p String) [ true ] & Public
        }

        AuthModule  AuthInternal / Private
        CleanModule AuthInternal / 'salt' / 'hash'
    "#;

    let report = analyze_source(code);

    assert_eq!(report.hir.imports.len(), 3);
    assert_eq!(report.hir.imports[1].targets, vec!["String", "Int"]);
    assert_eq!(report.hir.imports[2].source_path, "../../../base");

    let internal = report.hir.forms.get("AuthInternal").unwrap();
    assert_eq!(
        internal.fields.get("salt").unwrap().visibility,
        HirVisibility::Private
    );
    assert_eq!(
        internal.methods.get("verify").unwrap().visibility,
        HirVisibility::Public
    );

    let pub_mod = report.hir.forms.get("AuthModule").unwrap();
    assert!(!pub_mod.fields.contains_key("salt"));
    assert!(!pub_mod.methods.contains_key("hash"));
    assert!(pub_mod.methods.contains_key("verify"));

    let clean_mod = report.hir.forms.get("CleanModule").unwrap();
    assert!(!clean_mod.fields.contains_key("salt"));
    assert!(!clean_mod.methods.contains_key("hash"));
    assert!(clean_mod.methods.contains_key("verify"));
}

#[test]
fn test_specification_compact_has_zero_diagnostics() {
    let spec = std::fs::read_to_string("./specification.rl")
        .expect("Failed to read specification.rl");
    let report = analyze_source(&spec);

    assert!(
        report.diagnostics.is_empty(),
        "specification.rl должна проходить анализ с 0 ошибок и 0 предупреждений, но получено:\n{:#?}",
        report.diagnostics
    );
}

#[test]
fn test_analyzer_generic_form_methods_and_parenthesized_union_constraints() {
    let code = r#"
        Order      { id String, amount Int }
        Reserved   { reserved_at Int }
        Paid       { transaction_id String }
        OutOfStock { reason String } & Bypass
        NoMoney    { reason String } & Bypass

        Map(K, V) {
            (K) V
            get(m Map(K, V), key K) [ m ]
            bad_method(x UnknownTypeInMethod) [ x ]
        }

        Comparable(Other) {
            < (other Other) Bool
        }

        Functor(F(_)) {
            fmap(A, B, fa F(A), f (A) [B]) F(B)
        }

        Monad(M(_)) & Functor(M) {
            pure(A, a A) M(A)
            flat_map(A, B, ma M(A), f (A) [M(B)]) M(B)
        }

        Channel(T) {
            send(val T) [ val ]
        }

        audit_any_outcome(outcome (Order & Reserved & Paid) | OutOfStock | NoMoney) [
            "Audit: {outcome}"
        ]
    "#;

    let report = analyze_source(code);

    // Единственной ошибкой должен быть `UnknownTypeInMethod` в `Map.bad_method`
    assert_eq!(
        report.diagnostics.len(),
        1,
        "Неожиданные диагностики: {:#?}",
        report.diagnostics
    );
    assert_eq!(report.diagnostics[0].code, "E11_UNKNOWN_TYPE");
    assert_eq!(
        report.diagnostics[0].context_symbol.as_deref(),
        Some("Map.bad_method")
    );
    assert_eq!(
        report.diagnostics[0].highlight_token.as_deref(),
        Some("UnknownTypeInMethod")
    );
}

#[test]
fn test_analyzer_law1_form_field_keys_vs_field_values() {
    let code = r#"
        CounterState { count Int }
        Increment    { by Int }
        HttpRequest  { method Symbol, body String }
        HttpResponse { status Int, body String }
        AgentState   { retries Int, err String }

        // Корректно: ключи полей `count`, `body`, `retries`, `err` не являются чтением переменных
        update_counter_good(s CounterState, inc Increment) [
            CounterState { count (s.count + inc.by) }
        ]

        api_router_good(req HttpRequest) [
            HttpResponse { status 200, body "OK" }
        ]

        agent_step_good(state AgentState) [
            state & { retries (state.retries - 1), err state.err }
        ]

        // Ошибка Закона 1: в значении поля написано `count` вместо `s.count`
        update_counter_bad(s CounterState) [
            CounterState { count (count + 1) }
        ]
    "#;

    let report = analyze_source(code);
    let warnings: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("W001_LAW1"))
        .collect();

    assert_eq!(
        warnings.len(),
        1,
        "Ожидалось ровно 1 предупреждение Закона 1 (в update_counter_bad), получено: {:#?}",
        report.diagnostics
    );
    assert_eq!(
        warnings[0].context_symbol.as_deref(),
        Some("update_counter_bad")
    );
}
