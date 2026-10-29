// tests/codegen_typescript.rs
// =============================================================================
// RAZOMY V5.7 — TYPESCRIPT CODEGEN & LIR LOWERING TEST SUITE
// =============================================================================

use razomy_language::codegen::transpile;
use razomy_language::lir::Target;
use std::fs;

#[test]
fn ts_01_forms_lineage_and_subtraction() {
    let code = r#"
        Identifiable { id String }
        User Identifiable {
            name          String
            age           Int 25
            password_hash String
        }
        PublicUser User / User.password_hash
    "#;

    let ts = transpile(code, Target::TypeScript).expect("TS transpile failed");

    assert!(ts.contains("export interface Identifiable"));
    assert!(ts.contains("export interface User"));
    assert!(ts.contains("id: string; // from Identifiable"));
    assert!(ts.contains("age: number; // from User"));
    assert!(ts.contains("age: init.age ?? 25"));

    let pub_user_block = ts
        .split("export interface PublicUser")
        .nth(1)
        .unwrap()
        .split('}')
        .next()
        .unwrap();
    assert!(pub_user_block.contains("name: string;"));
    assert!(!pub_user_block.contains("password_hash"));
}

#[test]
fn ts_02_reactive_getters_excel_style() {
    let code = r#"
        Invoice {
            price    100
            quantity 3
            tax_rate 0.2

            subtotal [ self.price * self.quantity ]
            tax      [ self.subtotal * self.tax_rate ]
            total    [ self.subtotal + self.tax ]
        }
    "#;

    let ts = transpile(code, Target::TypeScript).expect("TS transpile failed");

    assert!(ts.contains("readonly subtotal: any;"));
    assert!(ts.contains("get subtotal() {"));
    assert!(ts.contains("const self = this;"));
    assert!(ts.contains("return (self.price * self.quantity);"));
    assert!(ts.contains("get total() {"));
    assert!(ts.contains("return (self.subtotal + self.tax);"));
}

#[test]
fn ts_03_law1c_auto_unpacking_and_default_params() {
    let code = r#"
        User { id String, name String }
        Cart { cart_id String, amount Int }

        greet_anon(User) [
            "Hello, {user.name}!"
        ]

        greet_named(u User) [
            "Hello, {u.name}!"
        ]

        finalize_bill(ctx User & Cart) [
            "{ctx.name}: {ctx.amount}"
        ]

        connect(host String "localhost", port Int 8080, tls Bool false) [
            "{host}:{port}"
        ]
    "#;

    let ts = transpile(code, Target::TypeScript).expect("TS transpile failed");

    // 1. Анонимный слот `greet_anon(User)` связывает `const user = _;` и распаковывает `{ id, name }`
    assert!(ts.contains("export function greet_anon(_: User): any"));
    assert!(ts.contains("const user = _;"));
    assert!(ts.contains("const { id, name } = _ as any;"));
    assert!(ts.contains("return `Hello, ${user.name}!`;"));

    // 2. Именованный слот `greet_named(u User)`
    assert!(ts.contains("export function greet_named(u: User): any"));
    assert!(ts.contains("return `Hello, ${u.name}!`;"));

    // 3. Параметры по умолчанию
    assert!(ts.contains(
        "export function connect(host: string = \"localhost\", port: number = 8080, tls: boolean = false)"
    ));
}

#[test]
fn ts_04_pipelines_and_pattern_routers() {
    let code = r#"
        Circle   { radius Float }
        Rect     { width Float, height Float }
        Geometry Circle | Rect

        area(g Geometry) [
            g . {
                Circle [ 3.14159 * circle.radius * circle.radius ]
                Rect   [ rect.width * rect.height ]
                _      [ 0.0 ]
            }
        ]

        process(x Int) [
            x
            .? [ _ > 0 ]
            .> [ _ . Shell.log ]
        ]
    "#;

    let ts = transpile(code, Target::TypeScript).expect("TS transpile failed");

    assert!(ts.contains("export type Geometry = Circle | Rect;"));
    assert!(ts.contains("if (_?.__type === \"Circle\") {"));
    assert!(ts.contains("const circle = _;"));
    assert!(ts.contains("return ((3.14159 * circle.radius) * circle.radius);"));
    assert!(ts.contains("if (_?.__type === \"Rect\") {"));
    assert!(ts.contains("const rect = _;"));
    assert!(ts.contains("console.log"));
}

#[test]
fn ts_05_operator_overloading_and_methods_in_forms() {
    let code = r#"
        Vec2 {
            x Float
            y Float

            + (other Vec2) [ Vec2 { x (self.x + other.x), y (self.y + other.y) } ]
        }
    "#;

    let ts = transpile(code, Target::TypeScript).expect("TS transpile failed");

    assert!(ts.contains("op_add(other: Vec2): any;"));
    assert!(ts.contains("op_add(other: Vec2) {"));
    assert!(ts.contains("createVec2({ x: (self.x + other.x), y: (self.y + other.y) })"));
}

#[test]
fn ts_06_full_spec_transpiles_to_typescript() {
    let spec =
        fs::read_to_string("./specification.rl").expect("Failed to read specification.rl");
    let ts = transpile(&spec, Target::TypeScript).expect("Full spec TS transpile failed");

    assert!(ts.contains("// Generated by Razomy Multi-Target Transpiler (Target: TypeScript)"));
    assert!(ts.contains("export interface Invoice"));
    assert!(ts.contains("export function finalize_bill"));
    assert!(ts.contains("export function main"));
}
