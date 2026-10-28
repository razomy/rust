// tests/codegen_typescript.rs
// =============================================================================
// RAZOMY V5.0 — TYPESCRIPT CODEGEN & LIR LOWERING TEST SUITE
// =============================================================================

use razomy_language::codegen::transpile;
use razomy_language::lir::Target;
use std::fs;

#[test]
fn ts_01_forms_lineage_and_subtraction() {
    let code = r#"
        Identifiable { Id String }
        User Identifiable {
            Name String
            Age  25
            PasswordHash String
        }
        PublicUser User / User.PasswordHash
    "#;

    let ts = transpile(code, Target::TypeScript).expect("TS transpile failed");

    assert!(ts.contains("export interface Identifiable"));
    assert!(ts.contains("export interface User"));
    assert!(ts.contains("Id: string; // from Identifiable"));
    assert!(ts.contains("Age: number; // from User"));
    assert!(ts.contains("Age: init.Age ?? 25"));

    // PublicUser не должен содержать вычтенное поле PasswordHash
    let pub_user_block = ts
        .split("export interface PublicUser")
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap();
    assert!(pub_user_block.contains("Name: string;"));
    assert!(!pub_user_block.contains("PasswordHash"));
}

#[test]
fn ts_02_reactive_getters_excel_style() {
    let code = r#"
        Invoice {
            Price    100
            Quantity 3
            TaxRate  0.2

            Subtotal [ Price * Quantity ]
            Tax      [ Subtotal * TaxRate ]
            Total    [ Subtotal + Tax ]
        }
    "#;

    let ts = transpile(code, Target::TypeScript).expect("TS transpile failed");

    assert!(ts.contains("readonly Subtotal: any;"));
    assert!(ts.contains("get Subtotal() {"));
    assert!(ts.contains("const Price = this.Price;"));
    assert!(ts.contains("return (Price * Quantity);"));
    assert!(ts.contains("get Total() {"));
    assert!(ts.contains("const Subtotal = this.Subtotal;"));
    assert!(ts.contains("return (Subtotal + Tax);"));
}

#[test]
fn ts_03_law1c_auto_unpacking_and_default_params() {
    let code = r#"
        User { Id String, Name String }
        Cart { CartId String, Amount Int }

        GreetAnon(User) [
            "Hello, {Name}!"
        ]

        GreetNamed(U User) [
            "Hello, {U.Name}!"
        ]

        FinalizeBill(User & Cart) [
            "{Name}: {Amount}"
        ]

        Connect(Host "localhost", Port 8080, Tls false) [
            "{Host}:{Port}"
        ]
    "#;

    let ts = transpile(code, Target::TypeScript).expect("TS transpile failed");

    // 1. Анонимный слот `GreetAnon(User)` распаковывает `{ Id, Name }`
    assert!(ts.contains("export function GreetAnon(_: User): any"));
    assert!(ts.contains("const { Id, Name } = _ as any;"));
    assert!(ts.contains("return `Hello, ${Name}!`;"));

    // 2. Именованный слот `GreetNamed(U User)` НЕ распаковывает поля
    assert!(ts.contains("export function GreetNamed(U: User): any"));
    assert!(ts.contains("return `Hello, ${U.Name}!`;"));

    // 3. Пересеченный слот `FinalizeBill(User & Cart)` распаковывает поля обеих форм
    assert!(ts.contains("const { Id, Name, CartId, Amount } = _ as any;"));

    // 4. Параметры по умолчанию
    assert!(ts.contains(
        "export function Connect(Host: string = \"localhost\", Port: number = 8080, Tls: boolean = false)"
    ));
}

#[test]
fn ts_04_pipelines_and_pattern_routers() {
    let code = r#"
        Circle { Radius Float }
        Rect   { Width Float, Height Float }
        Shape  Circle | Rect

        Area(S Shape) [
            S . {
                Circle [ 3.14159 * Radius * Radius ]
                Rect   [ Width * Height ]
                _      [ 0.0 ]
            }
        ]

        Process(X Int) [
            X
            .? [ _ > 0 ]
            .> [ _ . Shell.Log ]
        ]
    "#;

    let ts = transpile(code, Target::TypeScript).expect("TS transpile failed");

    assert!(ts.contains("export type Shape = Circle | Rect;"));
    assert!(ts.contains("if (_?.__type === \"Circle\") {"));
    assert!(ts.contains("const { Radius } = _ as any;"));
    assert!(ts.contains("return ((3.14159 * Radius) * Radius);"));
    assert!(ts.contains("if (_?.__type === \"Rect\") {"));
    assert!(ts.contains("const { Width, Height } = _ as any;"));
    assert!(ts.contains("console.log"));
}

#[test]
fn ts_05_operator_overloading_and_methods_in_forms() {
    let code = r#"
        Vec2 {
            X Float
            Y Float

            +(Other Vec2) [ Vec2 { X (X + Other.X), Y (Y + Other.Y) } ]
        }
    "#;

    let ts = transpile(code, Target::TypeScript).expect("TS transpile failed");

    assert!(ts.contains("op_add(Other: Vec2): any;"));
    assert!(ts.contains("op_add(Other: Vec2) {"));
    assert!(ts.contains("createVec2({ X: (X + Other.X), Y: (Y + Other.Y) })"));
}

#[test]
fn ts_06_full_spec_transpiles_to_typescript() {
    let spec = fs::read_to_string("./specification.rl").expect("Failed to read tests/specification.rl");
    let ts = transpile(&spec, Target::TypeScript).expect("Full spec TS transpile failed");

    assert!(ts.contains("// Generated by Razomy Multi-Target Transpiler (Target: TypeScript)"));
    assert!(ts.contains("export interface Invoice"));
    assert!(ts.contains("export function FinalizeBill"));
    assert!(ts.contains("export function Main"));
}