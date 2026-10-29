// tests/analyzer.rs
// =============================================================================
// RAZOMY V5.7 — STATIC HIR ANALYZER, ERROR CATALOG (E01..E07) & IDE TEST SUITE
// =============================================================================

use razomy_language::analyzer::{analyze_source, Severity, SymbolKind};
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

    let anon_sym = report.symbols.get("greet_anon").expect("greet_anon symbol");
    assert!(anon_sym.auto_unpacks_first_arg);
    assert_eq!(anon_sym.params[0].0, "user");

    let named_sym = report.symbols.get("greet_named").expect("greet_named symbol");
    assert!(!named_sym.auto_unpacks_first_arg);

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
    let sym = report.symbols.get("User").unwrap();
    assert_eq!(sym.kind, SymbolKind::Form);

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
}
