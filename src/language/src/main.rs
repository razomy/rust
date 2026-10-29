// src/main.rs
// =============================================================================
// RAZOMY LANGUAGE CLI (V5.7) — COMPILER, RUNNER, LIR DEBUGGER & LSP ENTRYPOINT
// =============================================================================

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use razomy_language::analyzer::{analyze_source_with_path, Severity};
use razomy_language::hir::lower_to_hir;
use razomy_language::lir::{lower_hir_to_lir, lower_tokens_to_expr, LirStmt};
use razomy_language::lsp::{format_razomy_source, RazomyLanguageServer};
use razomy_language::parser::parse_code_with_imports;
use razomy_language::runtime::{eval_lir_expr, eval_lir_stmts, run_file, Value};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("lsp") => {
            if matches!(args.get(2).map(String::as_str), Some("--http" | "http")) {
                let port = args.get(3).map(String::as_str).unwrap_or("8765");
                let addr = if port.contains(':') {
                    port.to_string()
                } else {
                    format!("127.0.0.1:{port}")
                };
                run_lsp_http_command(&addr)
            } else {
                run_lsp_command()
            }
        }

        Some("serve" | "studio") => {
            let port = args.get(2).map(String::as_str).unwrap_or("8765");
            let addr = if port.contains(':') {
                port.to_string()
            } else {
                format!("127.0.0.1:{port}")
            };
            run_lsp_http_command(&addr)
        }

        Some("run") => {
            let Some(file) = args.get(2) else {
                eprintln!("Ошибка: не указан файл.\nИспользование: razomy run <file.rl>");
                return ExitCode::FAILURE;
            };
            run_script_command(Path::new(file))
        }

        Some("debug") => {
            let Some(file) = args.get(2) else {
                eprintln!("Ошибка: не указан файл.\nИспользование: razomy debug <file.rl>");
                return ExitCode::FAILURE;
            };
            debug_file_command(Path::new(file))
        }

        Some("check") => {
            let Some(file) = args.get(2) else {
                eprintln!("Ошибка: не указан файл.\nИспользование: razomy check <file.rl>");
                return ExitCode::FAILURE;
            };
            check_file_command(Path::new(file))
        }

        Some("fmt") => {
            let Some(file) = args.get(2) else {
                eprintln!("Ошибка: не указан файл.\nИспользование: razomy fmt <file.rl>");
                return ExitCode::FAILURE;
            };
            format_file_command(Path::new(file))
        }

        Some("-v" | "--version" | "version") => {
            println!("razomy {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }

        Some("-h" | "--help" | "help") => {
            print_help();
            ExitCode::SUCCESS
        }

        // Прямой запуск файла без подкоманды: `razomy script.rl`
        Some(file) if file.ends_with(".rl") => run_script_command(Path::new(file)),

        Some(unknown) => {
            eprintln!("Неизвестная команда: '{unknown}'\n");
            print_help();
            ExitCode::FAILURE
        }

        None => {
            print_help();
            ExitCode::SUCCESS
        }
    }
}

fn print_help() {
    println!("Razomy Language CLI v{}", env!("CARGO_PKG_VERSION"));
    println!("Гомоиконичный язык программирования трёх скобок {{}} [] ()\n");
    println!("Использование:");
    println!("  razomy <file.rl>        — Проверить и запустить скрипт");
    println!("  razomy run <file.rl>    — Запустить файл (.rl)");
    println!("  razomy debug <file.rl>  — Интерактивная пошаговая отладка LIR");
    println!("  razomy check <file.rl>  — Статический анализ (HIR Laws 1–4, E01..E07)");
    println!("  razomy fmt <file.rl>    — Автоформатирование файла на месте");
    println!("  razomy lsp              — Запустить Language Server (JSON-RPC 2.0 stdio)");
    println!("  razomy lsp --http [port]— Запустить HTTP LSP сервер для parser.html (по умолч. 8765)");
    println!("  razomy serve [port]     — Запустить веб-студию parser.html + LSP сервер");
    println!("  razomy --version        — Показать версию компилятора");
}

fn run_lsp_http_command(addr: &str) -> ExitCode {
    let mut server = RazomyLanguageServer::new();
    match server.run_http(addr) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("[HTTP LSP Error]: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_lsp_command() -> ExitCode {
    let mut server = RazomyLanguageServer::new();
    match server.run_stdio() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("[LSP Error]: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_script_command(path: &Path) -> ExitCode {
    let src = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("[IO Error] Не удалось прочитать '{}': {err}", path.display());
            return ExitCode::FAILURE;
        }
    };

    // 1. Статический анализ перед запуском
    let report = analyze_source_with_path(&src, Some(path));
    let mut has_syntax_error = false;

    for d in &report.diagnostics {
        let line_info = d
            .sentence_index
            .map(|idx| format!(" (предложение #{idx})"))
            .unwrap_or_default();
        eprintln!("[{:?}] {}{}: {}", d.severity, d.code, line_info, d.message);
        if d.code == "E00_SYNTAX" || d.code == "E07_STATIC_ASSERT_FAILED" {
            has_syntax_error = true;
        }
    }

    if has_syntax_error {
        eprintln!("\n[Compilation Aborted]: Исправьте ошибки компиляции перед запуском.");
        return ExitCode::FAILURE;
    }

    // 2. Выполнение в рантайме
    match run_file(path) {
        Ok(res) => {
            if !matches!(res, Value::None) {
                println!("\n[Result]: {}", res.compact_dump());
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("\n[Runtime Error]: {err}");
            ExitCode::FAILURE
        }
    }
}

fn check_file_command(path: &Path) -> ExitCode {
    let src = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("[IO Error] Не удалось прочитать '{}': {err}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let report = analyze_source_with_path(&src, Some(path));
    if report.diagnostics.is_empty() {
        println!("OK: {} — ошибок не обнаружено.", path.display());
        return ExitCode::SUCCESS;
    }

    let mut error_count = 0;
    for d in &report.diagnostics {
        if d.severity == Severity::Error {
            error_count += 1;
        }
        let line_info = d
            .sentence_index
            .map(|idx| format!(" (#{idx})"))
            .unwrap_or_default();
        println!("[{:?}] {}{}: {}", d.severity, d.code, line_info, d.message);
    }

    if error_count > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn format_file_command(path: &Path) -> ExitCode {
    let src = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("[IO Error] Не удалось прочитать '{}': {err}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let formatted = format_razomy_source(&src);
    if formatted != src {
        if let Err(err) = fs::write(path, formatted) {
            eprintln!("[IO Error] Не удалось записать '{}': {err}", path.display());
            return ExitCode::FAILURE;
        }
        println!("Formatted: {}", path.display());
    } else {
        println!("Unchanged: {}", path.display());
    }
    ExitCode::SUCCESS
}

/// Интерактивный пошаговый отладчик LIR для функции `main()` или всего модуля (`globals` + `top_exprs`)
fn debug_file_command(path: &Path) -> ExitCode {
    let source = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("[IO Error] Не удалось прочитать '{}': {err}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let sentences = match parse_code_with_imports(&source, path.parent()) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("[Syntax Error]:\n{err}");
            return ExitCode::FAILURE;
        }
    };

    let hir = lower_to_hir(&sentences);
    let lir = lower_hir_to_lir(&hir);

    println!("=== RAZOMY INTERACTIVE DEBUGGER (V5.7) ===");
    println!("Файл: {}", path.display());
    println!("Команды:");
    println!("  [Enter / n / step] — выполнить следующий шаг LIR");
    println!("  [v / vars]         — показать переменные текущего Scope");
    println!("  [c / continue]     — продолжить выполнение до конца");
    println!("  [q / quit]         — выйти из отладчика");
    println!("  <выражение>        — вычислить любое выражение Razomy на лету (Watch/Eval)\n");

    let mut scope = razomy_language::runtime_ext::create_global_scope();

    // Собираем инструкции верхнего уровня из `lir.globals` и `lir.top_exprs`
    let top_level_stmts: Vec<LirStmt> = lir
        .globals
        .iter()
        .map(|g| LirStmt::Let {
            name: g.name.clone(),
            value: g.value.clone(),
        })
        .chain(lir.top_exprs.iter().cloned().map(LirStmt::Expr))
        .collect();

    // Если есть функция `main()`, сначала инициализируем глобальные переменные, а отлаживаем тело `main()`
    let stmts_to_debug: Vec<LirStmt> =
        if let Some(main_fn) = lir.functions.iter().find(|f| f.raw_name == "main") {
            let _ = eval_lir_stmts(&top_level_stmts, None, &mut scope);
            println!(
                "Точка входа: fn main() ({} инструкций LIR)",
                main_fn.body.len()
            );
            main_fn.body.clone()
        } else {
            if top_level_stmts.is_empty() {
                println!("Функция main() и top-level выражения не найдены, выполняем модуль...");
                let _ = run_file(path);
                return ExitCode::SUCCESS;
            }
            println!(
                "Функция main() не найдена, пошаговая отладка модуля ({} инструкций LIR)...",
                top_level_stmts.len()
            );
            top_level_stmts
        };

    let mut continuous = false;

    for (idx, stmt) in stmts_to_debug.iter().enumerate() {
        if !continuous {
            println!(
                "\n[Шаг #{}/{}] LIR: {:?}",
                idx + 1,
                stmts_to_debug.len(),
                stmt
            );
            loop {
                print!("(razomy-dbg) > ");
                let _ = io::stdout().flush();

                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(0) | Err(_) => return ExitCode::SUCCESS,
                    Ok(_) => {}
                }

                match input.trim() {
                    "" | "n" | "step" | "s" => break,
                    "c" | "continue" => {
                        continuous = true;
                        break;
                    }
                    "v" | "vars" => {
                        println!("--- Переменные в Scope ({}) ---", scope.bindings.len());
                        let mut keys: Vec<_> = scope.bindings.keys().collect();
                        keys.sort();
                        for k in keys {
                            if let Some(v) = scope.bindings.get(k) {
                                println!("  {k} = {}", v.compact_dump());
                            }
                        }
                    }
                    "q" | "quit" | "exit" => {
                        println!("[Отладка прервана пользователем]");
                        return ExitCode::SUCCESS;
                    }
                    expr_str => match parse_code_with_imports(expr_str, path.parent()) {
                        Ok(sents) => {
                            if let Some(first) = sents.first() {
                                let expr = lower_tokens_to_expr(first, &hir);
                                match eval_lir_expr(&expr, &mut scope) {
                                    Ok(val) => println!("  => {}", val.compact_dump()),
                                    Err(err) => println!("  [Eval Error]: {err}"),
                                }
                            }
                        }
                        Err(err) => println!("  [Parse Error]: {err}"),
                    },
                }
            }
        }

        let step_slice = std::slice::from_ref(stmt);
        if let Err(e) = eval_lir_stmts(step_slice, None, &mut scope) {
            eprintln!("\n[Runtime Error на шаге #{}]: {e}", idx + 1);
            return ExitCode::FAILURE;
        }
    }

    println!("\n[Отладка успешно завершена]");
    ExitCode::SUCCESS
}