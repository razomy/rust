// src/main.rs
// =============================================================================
// RAZOMY V5.7 — FULL SHOWCASE: HIR STATIC ANALYZER + LIR DATAFLOW RUNTIME
// =============================================================================

use razomy_language::analyzer::analyze_source;
use razomy_language::runtime::run_program;

fn main() {
    let source_code = r#"
// 1. Определение структур, Lineage и Алгебры типов (& и /) по Спецификации V5.7
Identifiable { id String }

User Identifiable {
  id            String
  name          String
  age           Int
  role          'admin' | 'manager' | 'user'
  password_hash String
}

Order { id String, amount Int }

// Слияние с коллизией на поле `id` (Закон 2) и ее разрешение через вычитание `/`
UserOrder      User & Order
UserOrderClean UserOrder / Order.id

// 2. Уточняющие типы (Refinement Types) и инфиксное вхождение в паттерн `==`
is_adult(User)   [ user.age >= 18 ]
is_minor(User)   [ user.age < 18 ]
is_manager(User) [ user.role == ('admin' | 'manager') ]

Adult   User & is_adult
Minor   User & is_minor
Manager User & is_manager

classify_account(account User) [
  account . {
    Manager [ "  -> [Router] Manager dashboard: {manager.name} (Role: {manager.role})" . Shell.log ]
    Adult   [ "  -> [Router] Adult account: {adult.name}" . Shell.log ]
    Minor   [ "  -> [Router] Minor account: {minor.name}" . Shell.log ]
    _       [ "  -> [Router] Unknown profile" . Shell.log ]
  }
]

// 3. Точный тип `Money` и Реактивные формы в стиле Excel (Закон 1a: `self.field`)
Money(raw String "0.00") {
  scaled (raw . Decimal.parse_scale4)
  from_scaled(val Int) [ Money { scaled val } ]
  +  (other Money)  [ Money.from_scaled(self.scaled + other.scaled) ]
  -  (other Money)  [ Money.from_scaled(self.scaled - other.scaled) ]
  *  (factor Float) [ Money.from_scaled((self.scaled.to_float * factor).round_to_int) ]
  == (other Money)  [ self.scaled == other.scaled ]
}

Invoice {
  price    Money"100.00"
  quantity 3.0
  tax_rate 0.20

  subtotal [ self.price * self.quantity ]
  tax      [ self.subtotal * self.tax_rate ]
  total    [ self.subtotal + self.tax ]
}

// 4. Перегрузка операторов внутри Формы (с пробелом перед скобкой параметров по V5.7)
Vec2 {
  x Float
  y Float

  + (other Vec2)  [ Vec2 { x (self.x + other.x), y (self.y + other.y) } ]
  * (scale Float) [ Vec2 { x (self.x * scale),   y (self.y * scale) } ]
}

// 5. Онтологическая типизация (Typestate) и Закон 3 (`& Bypass` Outcome Short-Circuiting)
Reserved   { reserved_at Int }
Paid       { transaction_id String }
Cancelled  { cancelled_at Int }
OutOfStock { reason String } & Bypass

reserve_stock(o Order / Cancelled) [
  o.amount . {
    [ _ > 0 ] [ o & Reserved { reserved_at 1700000000 } ]
    _         [ OutOfStock { reason "Warehouse empty" } ]
  }
]

// Ожидает `Order & Reserved`. Если со склада пришел `OutOfStock` (`& Bypass`), Закон 3 автоматически пропускает этот шаг!
charge_payment(o Order & Reserved) [
  o & Paid { transaction_id "tx_999" }
]

ship_to_customer(order Order & Reserved & Paid) [
  "  -> [Fulfillment] Shipping order {order.id}, tx: {order.transaction_id}" . Shell.log
]

execute_order(initial_order Order / Cancelled) [
  initial_order
  . reserve_stock
  . charge_payment
  . {
    OutOfStock [ "  -> [Fulfillment Bypassed via Law 3] OutOfStock: {out_of_stock.reason}" . Shell.log ]
    Paid       [ _ . ship_to_customer ]
  }
]

// 6. Функциональный ранний выход `Break` в `fold`
Break(T) { value T, label Symbol 'default' }

sum_until_limit(limit Int) [
  (1..10) . fold(0, (acc Int, n Int) [
    (acc + n) . {
      [ _ > limit ] [ Break { value _ } ]
      _             [ _ ]
    }
  ])
]

// 7. Глубокий паттерн-матчинг со структурной деструктуризацией кортежей
HttpRequest { method Symbol, path Tuple(String) }

api_router(req HttpRequest) [
  req . {
    { method 'GET', path ("users", user_id) } [
      "  -> [ApiRouter] Matched GET /users/{user_id}" . Shell.log
    ]
    _ [
      "  -> [ApiRouter] 404 Not Found" . Shell.log
    ]
  }
]

// 8. Точка входа main() с явными скобками `()`
main() [
  "=== [1] Реактивные формы и точный Money (Excel-style Dataflow) ===" . Shell.log
  order_1 (Invoice & { quantity 5.0 })
  "  Invoice Default Total (qty=3, scaled): {Invoice.total.scaled}" . Shell.log
  "  order_1 Reactive Total (qty=5, scaled): {order_1.total.scaled}" . Shell.log

  "=== [2] Перегрузка операторов (Vec2) ===" . Shell.log
  v1 Vec2 { x 1.5, y 2.5 }
  v2 Vec2 { x 3.5, y 4.5 }
  v3 ((v1 + v2) * 2.0)
  "  (v1 + v2) * 2.0 = ({v3.x}, {v3.y})" . Shell.log

  "=== [3] Уточняющие типы (Refinement Types) и Закон 1c ===" . Shell.log
  User { id "u1", name "Alice", age 25, role 'admin', password_hash "h" } . classify_account
  User { id "u2", name "Bobby", age 14, role 'user',  password_hash "h" } . classify_account

  "=== [4] Закон 3 (Subtyping-Driven Bypass Outcome) ===" . Shell.log
  Order { id "ord_100", amount 2500 } . execute_order
  Order { id "ord_101", amount 0 }    . execute_order

  "=== [5] Коллекции, диапазоны и ранний выход Break в fold ===" . Shell.log
  early_sum sum_until_limit(12)
  "  Sum of 1..10 halted via Break above 12 = {early_sum}" . Shell.log

  "=== [6] Глубокая маршрутизация и захват из кортежа ===" . Shell.log
  HttpRequest { method 'GET', path ("users", "42") } . api_router

  (order_1.total.scaled, v3.x, v3.y, early_sum)
]
"#;

    println!("=== 1. СТАТИЧЕСКИЙ АНАЛИЗ HIR (LSP REPORT V5.7) ===");
    let report = analyze_source(source_code);
    for diag in &report.diagnostics {
        println!("[{:?}] {}: {}", diag.severity, diag.code, diag.message);
    }

    if let Some(hover_md) = report.hover("UserOrder") {
        println!(
            "\nHover для `UserOrder` (с коллизией Закона 2):\n{}",
            hover_md.trim()
        );
    }
    if let Some(hover_md) = report.hover("UserOrderClean") {
        println!(
            "\nHover для `UserOrderClean` (после вычитания `/ Order.id`):\n{}",
            hover_md.trim()
        );
    }

    println!("\n=== 2. ВЫПОЛНЕНИЕ ПРОГРАММЫ (RUNTIME V5.7) ===");
    match run_program(source_code) {
        Ok(res) => {
            println!("\n[Компактный дамп возвращенной структуры]: {}", res.compact_dump());
            println!("[Успешное завершение]");
        }
        Err(err) => eprintln!("\n[Ошибка выполнения]: {err}"),
    }
}
