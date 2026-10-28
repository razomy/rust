// tests/v47_analyzer_suite.rs
// =============================================================================
// RAZOMY V4.7 — STATIC HIR ANALYZER & IDE ENGINE TEST SUITE
// Запуск: cargo test --test v47_analyzer_suite -- --nocapture
// =============================================================================

use razomy_language::analyzer::{analyze_source, Severity, SymbolKind};
use razomy_language::hir::HirVisibility;

#[test]
fn test_analyzer_law1_named_vs_anonymous_slots() {
    let code = r#"
        User { Id String, Name String, Age Int }

        GreetAnon(User) [
            "Hello, {Name}!"
        ]

        GreetNamed(U User) [
            "Hello, {Name}!"
        ]
    "#;

    let report = analyze_source(code);

    let anon_sym = report.symbols.get("GreetAnon").expect("GreetAnon symbol");
    assert!(anon_sym.auto_unpacks_first_arg);

    let named_sym = report.symbols.get("GreetNamed").expect("GreetNamed symbol");
    assert!(!named_sym.auto_unpacks_first_arg);

    let law1_warnings: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("W001_LAW1"))
        .collect();
    assert_eq!(law1_warnings.len(), 1);
    assert_eq!(law1_warnings[0].context_symbol.as_deref(), Some("GreetNamed"));
}

#[test]
fn test_analyzer_law2_lineage_collision_and_subtraction() {
    let code = r#"
        User  { Id String, Name String }
        Order { Id String, Amount Int }

        UserOrder User & Order
        UserOrderClean UserOrder / Order.Id

        BadAccess(UserOrder) [
            "Ambiguous ID: {Id}"
        ]

        GoodAccess(UserOrder) [
            "Explicit IDs: {User.Id} and {Order.Id}"
        ]
    "#;

    let report = analyze_source(code);

    let uo = report.symbols.get("UserOrder").unwrap();
    assert!(uo.fields.get("Id").unwrap().is_ambiguous);

    let uo_clean = report.symbols.get("UserOrderClean").unwrap();
    let clean_id = uo_clean.fields.get("Id").unwrap();
    assert!(!clean_id.is_ambiguous);
    assert_eq!(clean_id.origin_form, "User");

    let errors: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error && d.code == "E002_LAW2_AMBIGUOUS_FIELD_ACCESS")
        .collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].context_symbol.as_deref(), Some("BadAccess"));
}

#[test]
fn test_analyzer_law3_typestate_validation() {
    let code = r#"
        Order     { Id String, Amount Int }
        Reserved  { ReservedAt Int }
        Paid      { TransactionId String }
        Cancelled { CancelledAt Int }

        ShipToCustomer(Order & Reserved & Paid) [
            "Shipping {Id}"
        ]

        ReserveStock(O Order / Cancelled) [
            O.Amount
        ]

        RawOrder       Order { Id "o1", Amount 100 }
        CancelledOrder Order { Id "o2", Amount 100 } & Cancelled

        TestTypestateViolations() [
            RawOrder . ShipToCustomer
            CancelledOrder . ReserveStock
        ]
    "#;

    let report = analyze_source(code);

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
fn test_analyzer_hover_and_completions() {
    let code = r#"
        Identifiable { Id String }
        User Identifiable {
            Name String
            Age  Int
            Greet() [ "Hi {Name}" ]
        }
    "#;

    let report = analyze_source(code);
    let sym = report.symbols.get("User").unwrap();
    assert_eq!(sym.kind, SymbolKind::Form);

    let hover_md = report.hover("User").expect("Hover markdown");
    assert!(hover_md.contains("Identifiable"));
    assert!(hover_md.contains("Name"));
    assert!(hover_md.contains("Greet"));

    let completions = report.completions("User.");
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    assert!(labels.contains(&"Id"));
    assert!(labels.contains(&"Name"));
    assert!(labels.contains(&"Greet"));
    assert!(labels.contains(&"Identifiable"));
}

#[test]
fn test_analyzer_module_visibility_and_encapsulation() {
    let code = r#"
        Public  { }
        Private { }

        AuthInternal {
            Salt         "x9f2a" & Private
            Hash(Raw)    [ Raw ] & Private
            Verify(U, P) [ true ] & Public
        }

        AuthModule  AuthInternal & Public
        CleanModule AuthInternal / 'Salt' / 'Hash'
    "#;

    let report = analyze_source(code);

    let internal = report.hir.forms.get("AuthInternal").unwrap();
    assert_eq!(
        internal.fields.get("Salt").unwrap().visibility,
        HirVisibility::Private
    );
    assert_eq!(
        internal.methods.get("Verify").unwrap().visibility,
        HirVisibility::Public
    );

    // `AuthModule` (`& Public`) должен содержать только `Verify`, а `Salt` и `Hash` скрыты!
    let pub_mod = report.hir.forms.get("AuthModule").unwrap();
    assert!(!pub_mod.fields.contains_key("Salt"));
    assert!(!pub_mod.methods.contains_key("Hash"));
    assert!(pub_mod.methods.contains_key("Verify"));

    // `CleanModule` (`/ 'Salt' / 'Hash'`) также очищен от приватных членов
    let clean_mod = report.hir.forms.get("CleanModule").unwrap();
    assert!(!clean_mod.fields.contains_key("Salt"));
    assert!(!clean_mod.methods.contains_key("Hash"));
}

#[test]
fn test_analyzer_generics_imports_and_ast_traceability() {
    let code = r#"
        Base          root.base
        (String, Int) root.base
        _             %../../../base

        Atom(T) {
            Value T
        }

        CounterState { Count Int }
        GlobalCounter Atom(CounterState) {
            Value CounterState { Count 0 }
        }
    "#;

    let report = analyze_source(code);

    // 1. Проверяем импорты в HIR
    assert_eq!(report.hir.imports.len(), 3);
    assert_eq!(report.hir.imports[1].targets, vec!["String", "Int"]);
    assert_eq!(report.hir.imports[2].source_path, "%../../../base");

    // 2. Проверяем подстановку параметра типа `T -> CounterState` в `GlobalCounter`
    let gc = report.hir.forms.get("GlobalCounter").unwrap();
    assert_eq!(gc.type_args, vec!["CounterState"]);
    assert_eq!(gc.origin.symbol_context, "GlobalCounter");
    assert!(!gc.origin.raw_ast.is_empty());
}