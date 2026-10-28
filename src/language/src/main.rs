pub mod analyzer;
pub mod runtime;

// Подключаем ваш парсер из корня (или lib.rs)
pub use razomy_parser::*;

use analyzer::analyze_source;
use runtime::run_program;

fn main() {
    let source_code = r#"
// 1. Определение структуры User и Lineage
User {
  Name "Alice"
  Age 25
  Role 'admin'
}

// 2. Глобальная константа
AdultThreshold 18

// 3. Функция классификации (Закон 1: Именованный слот `U User`)
Classify(U User) [
  U.Age . {
    [ _ >= AdultThreshold ] [ "Adult: {U.Name} (Role: {U.Role})" . Shell.Log ]
    _                       [ "Minor: {U.Name}" . Shell.Log ]
  }
]

// 4. Пайплайн обработки с вызовом
Main [
  "--- Старт программы Razomy ---" . Shell.Log

  CurrentUser User {
    Name "Alice"
    Age 22
  }

  // Пайплайн с Tap .> (Закон 1c: анонимный поток распаковывает Name) и вызовом Classify
  CurrentUser
  .> [ "Пользователь создан: {Name}" . Shell.Log ]
  .  Classify

  "Программа успешно завершена!" . Shell.Log
]
"#;

    println!("=== 1. СТАТИЧЕСКИЙ АНАЛИЗ (LSP REPORT) ===");
    let report = analyze_source(source_code);
    if report.diagnostics.is_empty() {
        println!("Диагностика: 0 ошибок, 0 предупреждений.");
    } else {
        for diag in &report.diagnostics {
            println!("[{:?}] {}: {}", diag.severity, diag.code, diag.message);
        }
    }

    if let Some(hover_md) = report.hover("User") {
        println!("\nHover для `User`:\n{}", hover_md.trim());
    }

    println!("\n=== 2. ВЫПОЛНЕНИЕ ПРОГРАММЫ (RUNTIME) ===");
    match run_program(source_code) {
        Ok(_) => println!("\n[Успешное завершение]"),
        Err(err) => eprintln!("\n[Ошибка выполнения]: {err}"),
    }
}