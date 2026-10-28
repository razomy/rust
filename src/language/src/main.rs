// src/main.rs
// =============================================================================
// RAZOMY V4.7 — FULL SHOWCASE: HIR STATIC ANALYZER + DATAFLOW RUNTIME
// =============================================================================

use razomy_parser::analyzer::analyze_source;
use razomy_parser::runtime::run_program;

fn main() {
    let source_code = r#"
// 1. Определение структур, Lineage и Алгебры типов (& и /)
Identifiable { Id String }

User Identifiable {
  Id           String
  Name         String
  Age          Int
  Role         'admin' | 'manager' | 'user'
  PasswordHash String
}

Order { Id String, Amount Int }

// Слияние с коллизией на поле `Id` (Закон 2) и ее разрешение через вычитание `/`
UserOrder      User & Order
UserOrderClean UserOrder / Order.Id

// 2. Уточняющие типы (Refinement Types)
IsAdult [ _.Age >= 18 ]
IsMinor [ _.Age < 18 ]
Adult   User & IsAdult
Minor   User & IsMinor

ClassifyAccount(Account) [
  Account . {
    Adult [ "  -> [Router] Adult account: {Name} (Role: {Role})" . Shell.Log ]
    Minor [ "  -> [Router] Minor account: {Name}" . Shell.Log ]
    _     [ "  -> [Router] Unknown profile" . Shell.Log ]
  }
]

// 3. Реактивные формы в стиле Excel (Закон 1a: каскадный пересчет)
Invoice {
  Price    $100.00
  Quantity 3
  TaxRate  20%

  Subtotal [ Price * Quantity ]
  Tax      [ Subtotal * TaxRate ]
  Total    [ Subtotal + Tax ]
}

// 4. Перегрузка операторов внутри Формы (Закон 1a)
Vec2 {
  X Float
  Y Float

  +(Other Vec2) [ Vec2 { X (X + Other.X), Y (Y + Other.Y) } ]
}

// 5. Онтологическая типизация (Typestate) и Закон 3 (Railway-Free Outcome Bypass)
Reserved   { ReservedAt Int }
Paid       { TransactionId String }
OutOfStock { Reason String }

ReserveStock(O Order) [
  O.Amount . {
    [ _ > 0 ] [ O & Reserved { ReservedAt 1700000000 } ]
    _         [ OutOfStock { Reason "Warehouse empty" } ]
  }
]

// Ожидает `Order & Reserved`. Если со склада пришел `OutOfStock`, Закон 3 автоматически пропускает этот шаг!
ChargePayment(O Order & Reserved) [
  O & Paid { TransactionId "tx_999" }
]

// Анонимный слот `(Order & Reserved & Paid)` автоматически распаковывает `Id` и `TransactionId` (Закон 1c)
ShipToCustomer(Order & Reserved & Paid) [
  "  -> [Fulfillment] Shipping order {Id}, tx: {TransactionId}" . Shell.Log
]

ExecuteOrder(InitialOrder Order) [
  InitialOrder
  . ReserveStock
  . ChargePayment
  . {
    OutOfStock [ "  -> [Fulfillment Bypassed via Law 3] OutOfStock: {Reason}" . Shell.Log ]
    Paid       [ _ . ShipToCustomer ]
  }
]

// 6. Функциональный ранний выход `Done` в `Fold`
Done { Value Int }

SumUntilLimit(Limit Int) [
  (1..10) . Fold(0, (Acc, N) [
    (Acc + N) . {
      [ _ > Limit ] [ Done { Value _ } ]
      _             [ _ ]
    }
  ])
]

// 7. Глубокий паттерн-матчинг со структурной деструктуризацией кортежей
HttpRequest { Method Symbol, Path Tuple }

ApiRouter(Req HttpRequest) [
  Req . {
    { Method 'GET', Path ("users", UserId) } [
      "  -> [ApiRouter] Matched GET /users/{UserId}" . Shell.Log
    ]
    _ [
      "  -> [ApiRouter] 404 Not Found" . Shell.Log
    ]
  }
]

// 8. Точка входа Main
Main [
  "=== [1] Реактивные формы (Excel-style Dataflow) ===" . Shell.Log
  Order1 (Invoice & { Quantity 5 })
  "  Invoice Default Total (Qty=3): {Invoice.Total}" . Shell.Log
  "  Order1 Reactive Total (Qty=5): {Order1.Total}" . Shell.Log

  "=== [2] Перегрузка операторов (Vec2) ===" . Shell.Log
  V1 Vec2 { X 1.5, Y 2.5 }
  V2 Vec2 { X 3.5, Y 4.5 }
  V3 (V1 + V2)
  "  V1 + V2 = ({V3.X}, {V3.Y})" . Shell.Log

  "=== [3] Уточняющие типы (Refinement Types) и Закон 1b ===" . Shell.Log
  User { Id "u1", Name "Alice", Age 25, Role 'admin', PasswordHash "h" } . ClassifyAccount
  User { Id "u2", Name "Bobby", Age 14, Role 'user',  PasswordHash "h" } . ClassifyAccount

  "=== [4] Закон 3 (Railway-Free Outcome Bypass) ===" . Shell.Log
  Order { Id "ord_100", Amount 2500 } . ExecuteOrder
  Order { Id "ord_101", Amount 0 }    . ExecuteOrder

  "=== [5] Коллекции, диапазоны и ранний выход Done в Fold ===" . Shell.Log
  EarlySum SumUntilLimit(12)
  "  Sum of 1..10 halted via Done above 12 = {EarlySum}" . Shell.Log

  "=== [6] Глубокая маршрутизация и захват из кортежа ===" . Shell.Log
  HttpRequest { Method 'GET', Path ("users", "42") } . ApiRouter
]
"#;

    println!("=== 1. СТАТИЧЕСКИЙ АНАЛИЗ HIR (LSP REPORT) ===");
    let report = analyze_source(source_code);
    for diag in &report.diagnostics {
        println!("[{:?}] {}: {}", diag.severity, diag.code, diag.message);
    }

    if let Some(hover_md) = report.hover("UserOrder") {
        println!("\nHover для `UserOrder` (с коллизией Закона 2):\n{}", hover_md.trim());
    }
    if let Some(hover_md) = report.hover("UserOrderClean") {
        println!(
            "\nHover для `UserOrderClean` (после вычитания `/ Order.Id`):\n{}",
            hover_md.trim()
        );
    }

    println!("\n=== 2. ВЫПОЛНЕНИЕ ПРОГРАММЫ (RUNTIME V4.7) ===");
    match run_program(source_code) {
        Ok(_) => println!("\n[Успешное завершение]"),
        Err(err) => eprintln!("\n[Ошибка выполнения]: {err}"),
    }
}