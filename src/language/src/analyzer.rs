// src/analyzer.rs
// =============================================================================
// RAZOMY V4.7 — STATIC ANALYZER & IDE INTELLIGENCE ENGINE (LSP READY)
// =============================================================================

use serde_json::{json, Value as JsonValue};
use std::collections::{HashMap, HashSet};

use crate::{
    classify_sentence, has_pipe_op, parse_code, split_pipeline, Literal, SemanticKind, Sentence,
    Token,
};

// =============================================================================
// 1. СТРУКТУРЫ ДАННЫХ ДЛЯ IDE (LSP СОВМЕСТИМЫЕ)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub context_symbol: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Form,
    Function,
    Stream,
    Variable,
    Generic,
    TypeAlgebra,
    OperatorMethod,
}

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub inferred_type: String,
    pub origin_form: String,
    pub is_reactive: bool,
    pub is_ambiguous: bool,
    pub conflicting_origins: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub signature: String,
    pub documentation: String,
    pub fields: HashMap<String, FieldSchema>,
    pub field_order: Vec<String>,
    pub ancestors: Vec<String>,
    pub params: Vec<(String, String)>, // (Имя/Тип слота, Тип/Дефолт)
    pub auto_unpacks_first_arg: bool,  // Закон 1c: true для `Fn(User)`, false для `Fn(U User)`
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: &'static str,
    pub detail: String,
    pub insert_text: String,
}

#[derive(Debug, Clone, Default)]
pub struct AnalysisReport {
    pub symbols: HashMap<String, SymbolInfo>,
    pub symbol_order: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl AnalysisReport {
    /// Экспорт полного отчета в JSON для плагинов VS Code / JetBrains
    pub fn to_ide_json(&self) -> JsonValue {
        let symbols_json: Vec<JsonValue> = self
            .symbol_order
            .iter()
            .filter_map(|name| self.symbols.get(name))
            .map(|sym| {
                let fields: Vec<JsonValue> = sym
                    .field_order
                    .iter()
                    .filter_map(|f| sym.fields.get(f))
                    .map(|field| {
                        json!({
                            "name": field.name,
                            "type": field.inferred_type,
                            "origin": field.origin_form,
                            "reactive": field.is_reactive,
                            "ambiguous": field.is_ambiguous,
                            "conflicts": field.conflicting_origins,
                        })
                    })
                    .collect();

                json!({
                    "name": sym.name,
                    "kind": format!("{:?}", sym.kind),
                    "signature": sym.signature,
                    "doc": sym.documentation,
                    "ancestors": sym.ancestors,
                    "autoUnpacks": sym.auto_unpacks_first_arg,
                    "fields": fields,
                })
            })
            .collect();

        let diags_json: Vec<JsonValue> = self
            .diagnostics
            .iter()
            .map(|d| {
                json!({
                    "severity": format!("{:?}", d.severity),
                    "code": d.code,
                    "message": d.message,
                    "symbol": d.context_symbol,
                })
            })
            .collect();

        json!({
            "language": "Razomy V4.7",
            "symbols": symbols_json,
            "diagnostics": diags_json,
        })
    }

    /// Получение подсказки при наведении мыши (LSP `textDocument/hover`)
    pub fn hover(&self, symbol_name: &str) -> Option<String> {
        if let Some((root, field)) = symbol_name.split_once('.') {
            let sym = self.symbols.get(root)?;
            if let Some(f) = sym.fields.get(field) {
                let mut md = format!(
                    "```razomy\n({}.{}) : {}\n```\n",
                    root, f.name, f.inferred_type
                );
                if f.is_reactive {
                    md.push_str("*⚡ Реактивное вычисляемое поле (Law 1a)*\n");
                }
                if f.is_ambiguous {
                    md.push_str(&format!(
                        "⚠️ **Коллизия имен (Law 2)**: присутствует в `{}`. Используйте явный путь предка.\n",
                        f.conflicting_origins.join("`, `")
                    ));
                } else {
                    md.push_str(&format!("Источник (Lineage): `{}`", f.origin_form));
                }
                return Some(md);
            }
        }

        let sym = self.symbols.get(symbol_name)?;
        let mut md = format!("```razomy\n{}\n```\n", sym.signature);
        if !sym.documentation.is_empty() {
            md.push_str(&format!("{}\n\n", sym.documentation));
        }
        if !sym.ancestors.is_empty() {
            md.push_str(&format!(
                "**Lineage (Предки):** `{}`\n\n",
                sym.ancestors.join(" & ")
            ));
        }
        if !sym.field_order.is_empty() {
            md.push_str("**Поля формы:**\n");
            for fname in &sym.field_order {
                if let Some(f) = sym.fields.get(fname) {
                    let badge = if f.is_ambiguous {
                        "⚠️ [КОЛЛИЗИЯ LAW 2]"
                    } else if f.is_reactive {
                        "⚡ [Reactive]"
                    } else {
                        ""
                    };
                    md.push_str(&format!(
                        "- `{}`: `{}` *(из {})* {}\n",
                        f.name, f.inferred_type, f.origin_form, badge
                    ));
                }
            }
        }
        Some(md)
    }

    /// Автодополнение (LSP `textDocument/completion`) для выражения перед точкой
    pub fn complete_dot(&self, receiver_type: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        if let Some(sym) = self.symbols.get(receiver_type) {
            for fname in &sym.field_order {
                if let Some(f) = sym.fields.get(fname) {
                    if f.is_ambiguous {
                        for origin in &f.conflicting_origins {
                            items.push(CompletionItem {
                                label: format!("{}.{}", origin, f.name),
                                kind: "Field (Lineage Disambiguated)",
                                detail: format!("Разрешение коллизии Law 2 из {}", origin),
                                insert_text: format!("{}.{}", origin, f.name),
                            });
                        }
                    } else {
                        items.push(CompletionItem {
                            label: f.name.clone(),
                            kind: if f.is_reactive {
                                "Property (Reactive)"
                            } else {
                                "Field"
                            },
                            detail: format!("{} (from {})", f.inferred_type, f.origin_form),
                            insert_text: f.name.clone(),
                        });
                    }
                }
            }

            for anc in &sym.ancestors {
                items.push(CompletionItem {
                    label: anc.clone(),
                    kind: "Class (Ancestor Slice)",
                    detail: format!("Доступ к предку {} (Law 2)", anc),
                    insert_text: anc.clone(),
                });
            }
        }

        for name in &self.symbol_order {
            if let Some(sym) = self.symbols.get(name) {
                if matches!(sym.kind, SymbolKind::Function | SymbolKind::Stream) {
                    items.push(CompletionItem {
                        label: sym.name.clone(),
                        kind: "Function (Pipe UFCS)",
                        detail: sym.signature.clone(),
                        insert_text: sym.name.clone(),
                    });
                }
            }
        }

        items
    }
}

// =============================================================================
// 2. ГЛАВНЫЙ ПРОХОД АНАЛИЗАТОРА
// =============================================================================

pub fn analyze_source(source: &str) -> AnalysisReport {
    let mut report = AnalysisReport::default();
    register_stdlib_symbols(&mut report);

    let sentences = match parse_code(source) {
        Ok(s) => s,
        Err(err) => {
            report.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                code: "E001_SYNTAX",
                message: format!("Синтаксическая ошибка парсера:\n{err}"),
                context_symbol: None,
            });
            return report;
        }
    };

    analyze_sentences(&sentences, &mut report);
    report
}

fn register_stdlib_symbols(report: &mut AnalysisReport) {
    let mut shell_fields = HashMap::new();
    shell_fields.insert(
        "Log".into(),
        FieldSchema {
            name: "Log".into(),
            inferred_type: "Fn(Msg Any) -> None".into(),
            origin_form: "Shell".into(),
            is_reactive: false,
            is_ambiguous: false,
            conflicting_origins: vec![],
        },
    );
    shell_fields.insert(
        "Exec".into(),
        FieldSchema {
            name: "Exec".into(),
            inferred_type: "Fn(Cmd String) -> String".into(),
            origin_form: "Shell".into(),
            is_reactive: false,
            is_ambiguous: false,
            conflicting_origins: vec![],
        },
    );
    report.symbol_order.push("Shell".into());
    report.symbols.insert(
        "Shell".into(),
        SymbolInfo {
            name: "Shell".into(),
            kind: SymbolKind::Form,
            signature: "Shell { Log(Msg), Exec(Cmd) }".into(),
            documentation: "Стандартный модуль ввода-вывода и системных команд".into(),
            fields: shell_fields,
            field_order: vec!["Log".into(), "Exec".into()],
            ancestors: vec![],
            params: vec![],
            auto_unpacks_first_arg: false,
        },
    );
}

pub fn analyze_sentences(sentences: &[Sentence], report: &mut AnalysisReport) {
    for sentence in sentences {
        let kind = classify_sentence(sentence);
        match kind {
            SemanticKind::FormDef { name, body, tail } => {
                let sym = build_form_symbol(&name, None, &body, &tail, report);
                register_symbol(sym, report);
            }
            SemanticKind::SubtypeFormDef {
                name,
                base,
                body,
                tail,
            } => {
                let sym = build_form_symbol(&name, Some(&base), &body, &tail, report);
                register_symbol(sym, report);
            }
            SemanticKind::AlgebraicDef { name, expr } => {
                let sym = build_algebraic_symbol(&name, &expr, report);
                register_symbol(sym, report);
            }
            SemanticKind::FunctionDef {
                name,
                args,
                body,
                tail: _,
            } => {
                let (params, auto_unpacks) = analyze_fn_args(&args);
                let param_str = params
                    .iter()
                    .map(|(n, t)| {
                        if t.is_empty() {
                            n.clone()
                        } else {
                            format!("{n} {t}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                let doc = if auto_unpacks {
                    "Закон 1c: Анонимный типизированный слот автоматически проецирует поля в скоуп `[]`.".into()
                } else {
                    "Закон 1: Именованные параметры предотвращают затенение (доступ через `Param.Field`).".into()
                };

                let sym = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Function,
                    signature: format!("{name}({param_str}) [ ... ]"),
                    documentation: doc,
                    fields: HashMap::new(),
                    field_order: vec![],
                    ancestors: vec![],
                    params: params.clone(),
                    auto_unpacks_first_arg: auto_unpacks,
                };

                verify_stream_body(&name, &params, auto_unpacks, &body, report);
                register_symbol(sym, report);
            }
            SemanticKind::StreamOrBranch { name, stream } => {
                verify_stream_body(&name, &[], true, &stream, report);
                let sym = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Stream,
                    signature: format!("{name} [ ... ]"),
                    documentation: "Поток данных / точка входа".into(),
                    fields: HashMap::new(),
                    field_order: vec![],
                    ancestors: vec![],
                    params: vec![],
                    auto_unpacks_first_arg: true,
                };
                register_symbol(sym, report);
            }
            SemanticKind::Binding { key, value } => {
                let inferred = infer_tokens_type(&value, report);
                let mut sym = SymbolInfo {
                    name: key.clone(),
                    kind: SymbolKind::Variable,
                    signature: format!("{key} : {inferred}"),
                    documentation: "Глобальная привязка (Axiom 1)".into(),
                    fields: HashMap::new(),
                    field_order: vec![],
                    ancestors: vec![],
                    params: vec![],
                    auto_unpacks_first_arg: false,
                };
                if let Some(base_sym) = report.symbols.get(&inferred).cloned() {
                    sym.fields = base_sym.fields;
                    sym.field_order = base_sym.field_order;
                    sym.ancestors = base_sym.ancestors;
                }
                register_symbol(sym, report);
            }
            SemanticKind::GenericDef { name, params, body } => {
                let p_names: Vec<String> = params.iter().map(|s| format_sentence(s)).collect();
                let sym = SymbolInfo {
                    name: name.clone(),
                    kind: SymbolKind::Generic,
                    signature: format!(
                        "{}({}) {}",
                        name,
                        p_names.join(", "),
                        format_sentence(&body)
                    ),
                    documentation: "Обобщенный тип / конструктор типа".into(),
                    fields: HashMap::new(),
                    field_order: vec![],
                    ancestors: vec![],
                    params: vec![],
                    auto_unpacks_first_arg: false,
                };
                register_symbol(sym, report);
            }
            _ => {}
        }
    }
}

fn register_symbol(sym: SymbolInfo, report: &mut AnalysisReport) {
    if !report.symbol_order.contains(&sym.name) {
        report.symbol_order.push(sym.name.clone());
    }
    report.symbols.insert(sym.name.clone(), sym);
}

// =============================================================================
// 3. ВЫВОД ПОЛЕЙ ФОРМЫ, LINEAGE И КОЛЛИЗИЙ (ЗАКОН 2)
// =============================================================================

fn build_form_symbol(
    name: &str,
    base: Option<&str>,
    body: &[Sentence],
    tail: &[Token],
    report: &mut AnalysisReport,
) -> SymbolInfo {
    let mut fields: HashMap<String, FieldSchema> = HashMap::new();
    let mut field_order = Vec::new();
    let mut ancestors = Vec::new();

    if let Some(base_name) = base {
        ancestors.push(base_name.to_string());
        if let Some(base_sym) = report.symbols.get(base_name) {
            for k in &base_sym.field_order {
                if let Some(f) = base_sym.fields.get(k) {
                    fields.insert(k.clone(), f.clone());
                    field_order.push(k.clone());
                }
            }
            for anc in &base_sym.ancestors {
                if !ancestors.contains(anc) {
                    ancestors.push(anc.clone());
                }
            }
        }
    }

    for s in body {
        match classify_sentence(s) {
            SemanticKind::Binding { key, value } => {
                let t = infer_tokens_type(&value, report);
                if !fields.contains_key(&key) {
                    field_order.push(key.clone());
                }
                fields.insert(
                    key.clone(),
                    FieldSchema {
                        name: key,
                        inferred_type: t,
                        origin_form: name.to_string(),
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                    },
                );
            }
            SemanticKind::StreamOrBranch {
                name: field_name, ..
            } => {
                if !fields.contains_key(&field_name) {
                    field_order.push(field_name.clone());
                }
                fields.insert(
                    field_name.clone(),
                    FieldSchema {
                        name: field_name,
                        inferred_type: "ReactiveStream".into(),
                        origin_form: name.to_string(),
                        is_reactive: true,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                    },
                );
            }
            SemanticKind::FunctionDef {
                name: method_name,
                args,
                ..
            } => {
                let (params, _) = analyze_fn_args(&args);
                let sig = format!(
                    "Fn({})",
                    params
                        .iter()
                        .map(|(n, t)| format!("{n} {t}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                if !fields.contains_key(&method_name) {
                    field_order.push(method_name.clone());
                }
                fields.insert(
                    method_name.clone(),
                    FieldSchema {
                        name: method_name,
                        inferred_type: sig,
                        origin_form: name.to_string(),
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                    },
                );
            }
            SemanticKind::OperatorMethod { op, args, .. } => {
                let (params, _) = analyze_fn_args(&args);
                let sig = format!(
                    "Op({})",
                    params
                        .iter()
                        .map(|(n, t)| format!("{n} {t}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                let key = format!("operator{op}");
                if !fields.contains_key(&key) {
                    field_order.push(key.clone());
                }
                fields.insert(
                    key.clone(),
                    FieldSchema {
                        name: key,
                        inferred_type: sig,
                        origin_form: name.to_string(),
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                    },
                );
            }
            _ => {}
        }
    }

    if !tail.is_empty() {
        apply_algebra_tail(
            name,
            &mut fields,
            &mut field_order,
            &mut ancestors,
            tail,
            report,
        );
    }

    SymbolInfo {
        name: name.to_string(),
        kind: SymbolKind::Form,
        signature: format!("Form {name} {{ {} fields }}", field_order.len()),
        documentation: "Определение структуры / формы (Axiom 2-4)".into(),
        fields,
        field_order,
        ancestors,
        params: vec![],
        auto_unpacks_first_arg: false,
    }
}

fn build_algebraic_symbol(
    name: &str,
    expr: &[Token],
    report: &mut AnalysisReport,
) -> SymbolInfo {
    let mut fields: HashMap<String, FieldSchema> = HashMap::new();
    let mut field_order = Vec::new();
    let mut ancestors = Vec::new();

    if let Some(Token::Path(first_type)) = expr.first() {
        if let Some(base_sym) = report.symbols.get(first_type) {
            ancestors.push(first_type.clone());
            for anc in &base_sym.ancestors {
                if !ancestors.contains(anc) {
                    ancestors.push(anc.clone());
                }
            }
            for k in &base_sym.field_order {
                if let Some(f) = base_sym.fields.get(k) {
                    fields.insert(k.clone(), f.clone());
                    field_order.push(k.clone());
                }
            }
        }
        if expr.len() > 1 {
            apply_algebra_tail(
                name,
                &mut fields,
                &mut field_order,
                &mut ancestors,
                &expr[1..],
                report,
            );
        }
    }

    SymbolInfo {
        name: name.to_string(),
        kind: SymbolKind::TypeAlgebra,
        signature: format!("{name} = {}", format_sentence(expr)),
        documentation: "Алгебраический тип (`&` пересечение, `|` объединение, `/` вычитание)".into(),
        fields,
        field_order,
        ancestors,
        params: vec![],
        auto_unpacks_first_arg: false,
    }
}

fn apply_algebra_tail(
    owner_name: &str,
    fields: &mut HashMap<String, FieldSchema>,
    field_order: &mut Vec<String>,
    ancestors: &mut Vec<String>,
    tail: &[Token],
    report: &mut AnalysisReport,
) {
    let mut i = 0;
    while i < tail.len() {
        match &tail[i] {
            Token::Op(op) if op == "&" => {
                if let Some(next_tok) = tail.get(i + 1) {
                    match next_tok {
                        Token::Path(rhs_type) => {
                            if !ancestors.contains(rhs_type) {
                                ancestors.push(rhs_type.clone());
                            }
                            if let Some(rhs_sym) = report.symbols.get(rhs_type).cloned() {
                                for k in &rhs_sym.field_order {
                                    if let Some(rhs_f) = rhs_sym.fields.get(k) {
                                        if let Some(existing) = fields.get_mut(k) {
                                            if existing.origin_form != rhs_f.origin_form
                                                && existing.origin_form != owner_name
                                            {
                                                existing.is_ambiguous = true;
                                                if !existing
                                                    .conflicting_origins
                                                    .contains(&existing.origin_form)
                                                {
                                                    existing
                                                        .conflicting_origins
                                                        .push(existing.origin_form.clone());
                                                }
                                                if !existing
                                                    .conflicting_origins
                                                    .contains(&rhs_f.origin_form)
                                                {
                                                    existing
                                                        .conflicting_origins
                                                        .push(rhs_f.origin_form.clone());
                                                }
                                                report.diagnostics.push(Diagnostic {
                                                    severity: Severity::Information,
                                                    code: "I002_LAW2_COLLISION",
                                                    message: format!(
                                                        "Закон 2 (Lineage): В типе '{}' поле '{}' унаследовано и от '{}', и от '{}'. Используйте '{}.{}' или '{}.{}'.",
                                                        owner_name, k, existing.origin_form, rhs_f.origin_form, existing.origin_form, k, rhs_f.origin_form, k
                                                    ),
                                                    context_symbol: Some(owner_name.to_string()),
                                                });
                                            }
                                        } else {
                                            field_order.push(k.clone());
                                            fields.insert(k.clone(), rhs_f.clone());
                                        }
                                    }
                                }
                            }
                        }
                        Token::Group('{', override_sents) => {
                            for s in override_sents {
                                if let SemanticKind::Binding { key, value } = classify_sentence(s) {
                                    let t = infer_tokens_type(&value, report);
                                    if !fields.contains_key(&key) {
                                        field_order.push(key.clone());
                                    }
                                    fields.insert(
                                        key.clone(),
                                        FieldSchema {
                                            name: key,
                                            inferred_type: t,
                                            origin_form: owner_name.to_string(),
                                            is_reactive: false,
                                            is_ambiguous: false,
                                            conflicting_origins: vec![],
                                        },
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
                i += 2;
            }
            Token::Op(op) if op == "/" => {
                if let Some(next_tok) = tail.get(i + 1) {
                    match next_tok {
                        Token::Lit(Literal::Symbol(sym_key))
                        | Token::Lit(Literal::String(sym_key)) => {
                            fields.remove(sym_key);
                            field_order.retain(|k| k != sym_key);
                        }
                        Token::Path(path_str) => {
                            if let Some((anc, field_key)) = path_str.split_once('.') {
                                if let Some(f) = fields.get_mut(field_key) {
                                    if f.is_ambiguous {
                                        f.conflicting_origins.retain(|o| o != anc);
                                        if f.conflicting_origins.len() == 1 {
                                            f.is_ambiguous = false;
                                            f.origin_form = f.conflicting_origins[0].clone();
                                        } else if f.conflicting_origins.is_empty() {
                                            fields.remove(field_key);
                                            field_order.retain(|k| k != field_key);
                                        }
                                    } else {
                                        fields.remove(field_key);
                                        field_order.retain(|k| k != field_key);
                                    }
                                }
                            } else {
                                ancestors.retain(|a| a != path_str);
                                fields.retain(|_, f| f.origin_form != *path_str);
                                field_order.retain(|k| fields.contains_key(k));
                            }
                        }
                        _ => {}
                    }
                }
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }
}

// =============================================================================
// 4. ПРОВЕРКА ЗАКОНА 1 И ЗАКОНА 2 ВНУТРИ ФУНКЦИЙ И ПОТОКОВ
// =============================================================================

pub fn analyze_fn_args(args: &[Sentence]) -> (Vec<(String, String)>, bool) {
    let mut out = Vec::new();
    let mut auto_unpacks = false;

    for (idx, sent) in args.iter().enumerate() {
        match sent.as_slice() {
            // Одно слово `Fn(User)` -> Закон 1c (авто-распаковка!)
            [Token::Path(single)] => {
                let is_type = single
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false);
                if idx == 0 && is_type {
                    auto_unpacks = true;
                }
                out.push((single.clone(), String::new()));
            }
            [Token::Path(_), Token::Op(op), ..] if op == "&" || op == "/" => {
                if idx == 0 {
                    auto_unpacks = true;
                }
                out.push((format_sentence(sent), String::new()));
            }
            // Именованный слот `Fn(U User)` -> НЕ распаковывает!
            [Token::Path(param_name), rest @ ..] => {
                out.push((param_name.clone(), format_sentence(rest)));
            }
            _ => {
                out.push((format_sentence(sent), String::new()));
            }
        }
    }

    (out, auto_unpacks)
}

fn verify_stream_body(
    fn_name: &str,
    params: &[(String, String)],
    auto_unpacks: bool,
    body: &[Sentence],
    report: &mut AnalysisReport,
) {
    let mut shadowed_named_fields: HashMap<String, (String, String)> = HashMap::new();
    if !auto_unpacks {
        for (p_name, p_type) in params {
            if !p_type.is_empty() {
                let base_type = p_type.split_whitespace().next().unwrap_or(p_type);
                if let Some(sym) = report.symbols.get(base_type) {
                    for f in &sym.field_order {
                        shadowed_named_fields
                            .insert(f.clone(), (p_name.clone(), base_type.to_string()));
                    }
                }
            }
        }
    }

    let mut local_vars = HashSet::new();
    for (p_name, _) in params {
        local_vars.insert(p_name.clone());
    }

    for sent in body {
        if let SemanticKind::Binding { key, .. } = classify_sentence(sent) {
            local_vars.insert(key);
        }
        check_tokens_for_laws(fn_name, sent, &local_vars, &shadowed_named_fields, report);
    }
}

fn check_tokens_for_laws(
    fn_name: &str,
    tokens: &[Token],
    local_vars: &HashSet<String>,
    shadowed_named_fields: &HashMap<String, (String, String)>,
    report: &mut AnalysisReport,
) {
    if has_pipe_op(tokens) {
        let (head, steps) = split_pipeline(tokens);
        check_tokens_for_laws(fn_name, &head, local_vars, shadowed_named_fields, report);
        for step in steps {
            if let Some(Token::Group('{', branches)) = step.expr.first() {
                for branch in branches {
                    match classify_sentence(branch) {
                        SemanticKind::StreamOrBranch {
                            name: matched_type,
                            stream,
                        } => {
                            if let Some(sym) = report.symbols.get(&matched_type).cloned() {
                                check_branch_for_law2_collision(
                                    fn_name,
                                    &matched_type,
                                    &sym,
                                    &stream,
                                    report,
                                );
                            } else {
                                // Дефолтная ветка `_ [ ... ]` наследует проверку именованных слотов
                                for s in &stream {
                                    check_tokens_for_laws(
                                        fn_name,
                                        s,
                                        local_vars,
                                        shadowed_named_fields,
                                        report,
                                    );
                                }
                            }
                        }
                        SemanticKind::PatternBranch { body, .. } => {
                            for s in &body {
                                check_tokens_for_laws(
                                    fn_name,
                                    s,
                                    local_vars,
                                    shadowed_named_fields,
                                    report,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        return;
    }

    for tok in tokens {
        match tok {
            Token::Path(p) => {
                if !p.contains('.') && !local_vars.contains(p) {
                    if let Some((param_var, param_type)) = shadowed_named_fields.get(p) {
                        report.diagnostics.push(Diagnostic {
                            severity: Severity::Warning,
                            code: "W001_LAW1_NAMED_SLOT",
                            message: format!(
                                "Закон 1 (Именованный слот): В функции '{}' параметр '{} {}' именованный и не распаковывает поле '{}' автоматически. Используйте '{}.{}'.",
                                fn_name, param_var, param_type, p, param_var, p
                            ),
                            context_symbol: Some(fn_name.to_string()),
                        });
                    }
                }
            }
            Token::Lit(Literal::String(raw)) => {
                for (field_name, (param_var, param_type)) in shadowed_named_fields {
                    if raw.contains(&format!("{{{field_name}}}")) && !local_vars.contains(field_name) {
                        report.diagnostics.push(Diagnostic {
                            severity: Severity::Warning,
                            code: "W001_LAW1_NAMED_SLOT_INTERP",
                            message: format!(
                                "Закон 1 (Именованный слот): В функции '{}' интерполяция '{{{}}}' обращается к нераспакованному полю '{} {}'. Используйте '{{{}.{}}}'.",
                                fn_name, field_name, param_var, param_type, param_var, field_name
                            ),
                            context_symbol: Some(fn_name.to_string()),
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

fn check_branch_for_law2_collision(
    fn_name: &str,
    matched_type: &str,
    sym: &SymbolInfo,
    stream: &[Sentence],
    report: &mut AnalysisReport,
) {
    for s in stream {
        for tok in s {
            match tok {
                Token::Path(p) => {
                    if let Some(field) = sym.fields.get(p) {
                        if field.is_ambiguous {
                            report.diagnostics.push(Diagnostic {
                                severity: Severity::Error,
                                code: "E002_LAW2_AMBIGUOUS_FIELD",
                                message: format!(
                                    "Нарушение Закона 2 в '{}': Поле '{}' неоднозначно в типе '{}' (присутствует в {}). Укажите путь предка: '{}.{}'.",
                                    fn_name, p, matched_type, field.conflicting_origins.join(" и "), field.conflicting_origins[0], p
                                ),
                                context_symbol: Some(fn_name.to_string()),
                            });
                        }
                    }
                }
                Token::Lit(Literal::String(raw)) => {
                    for f in sym.fields.values() {
                        if f.is_ambiguous && raw.contains(&format!("{{{}}}", f.name)) {
                            report.diagnostics.push(Diagnostic {
                                severity: Severity::Error,
                                code: "E002_LAW2_AMBIGUOUS_INTERP",
                                message: format!(
                                    "Нарушение Закона 2 в '{}': Интерполяция '{{{}}}' неоднозначна в '{}' (конфликт между {}). Используйте '{{{}.{}}}'.",
                                    fn_name, f.name, matched_type, f.conflicting_origins.join(" и "), f.conflicting_origins[0], f.name
                                ),
                                context_symbol: Some(fn_name.to_string()),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn infer_tokens_type(tokens: &[Token], report: &AnalysisReport) -> String {
    match tokens.first() {
        Some(Token::Lit(lit)) => match lit {
            Literal::Int(_) => "Int".into(),
            Literal::Float(_) => "Float".into(),
            Literal::Bool(_) => "Bool".into(),
            Literal::Byte(_) => "Byte".into(),
            Literal::String(_) => "String".into(),
            Literal::Symbol(s) => format!("'{s}'"),
            Literal::Money(_) => "Money".into(),
            Literal::Percent(_) => "Percent".into(),
            Literal::Date(_) => "Date".into(),
            Literal::Time(_) => "Time".into(),
            Literal::Duration(_) => "Duration".into(),
            Literal::Coord(_) => "Coord".into(),
            Literal::Color(_) => "Color".into(),
            Literal::IpAddr(_) => "IpAddr".into(),
            Literal::Version(_) => "Version".into(),
            Literal::Email(_) => "Email".into(),
            Literal::Url(_) => "Url".into(),
            Literal::File(_) => "File".into(),
        },
        Some(Token::Path(p)) => {
            if report.symbols.contains_key(p) {
                p.clone()
            } else {
                p.clone()
            }
        }
        Some(Token::Group('(', _)) => "Tuple".into(),
        Some(Token::Group('{', _)) => "Form".into(),
        Some(Token::Group('[', _)) => "Stream".into(),
        Some(Token::Range(_, _)) => "Range".into(),
        _ => "Any".into(),
    }
}

pub fn format_sentence(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|t| match t {
            Token::Path(p) => p.clone(),
            Token::Op(o) => o.clone(),
            Token::Lit(l) => format!("{l:?}"),
            Token::Range(a, b) => format!(
                "{}..{}",
                format_sentence(&[*a.clone()]),
                format_sentence(&[*b.clone()])
            ),
            Token::Group(c, _) => match c {
                '{' => "{...}".into(),
                '[' => "[...]".into(),
                _ => "(...)".into(),
            },
        })
        .collect::<Vec<_>>()
        .join(" ")
}