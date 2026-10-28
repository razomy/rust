// src/analyzer.rs
// =============================================================================
// RAZOMY V4.7 — HIR-POWERED STATIC ANALYZER & IDE ENGINE
// =============================================================================

use serde_json::{json, Value as JsonValue};
use std::collections::{HashMap, HashSet};

use crate::hir::{lower_to_hir, HirModule, HirParam};
use crate::{
    classify_sentence, has_pipe_op, parse_code, split_pipeline, Literal, SemanticKind, Sentence,
    Token,
};

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
    /// Связь с AST через HIR SourceOrigin!
    pub sentence_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Form,
    Function,
    Stream,
    Variable,
    TypeAlgebra,
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
    pub params: Vec<(String, String)>,
    pub auto_unpacks_first_arg: bool,
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
    pub hir: HirModule,
    pub symbols: HashMap<String, SymbolInfo>,
    pub symbol_order: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl AnalysisReport {
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
                    "sentenceIndex": d.sentence_index,
                })
            })
            .collect();

        json!({
            "language": "Razomy V4.7 (HIR Engine)",
            "symbols": symbols_json,
            "diagnostics": diags_json,
        })
    }

    pub fn hover(&self, symbol_name: &str) -> Option<String> {
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
            md.push_str("**Статически разрешенные поля (HIR):**\n");
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
}

pub fn analyze_source(source: &str) -> AnalysisReport {
    let mut report = AnalysisReport::default();

    let sentences = match parse_code(source) {
        Ok(s) => s,
        Err(err) => {
            report.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                code: "E001_SYNTAX",
                message: format!("Синтаксическая ошибка парсера:\n{err}"),
                context_symbol: None,
                sentence_index: None,
            });
            return report;
        }
    };

    // ШАГ 1: Строим статический HIR из сырого AST
    let hir = lower_to_hir(&sentences);
    analyze_from_hir(hir, &mut report);
    report
}

pub fn analyze_from_hir(hir: HirModule, report: &mut AnalysisReport) {
    // 1. Конвертируем разрешенные HIR-формы в таблицу символов IDE и проверяем коллизии Закона 2
    for form_name in &hir.form_order {
        if let Some(hir_form) = hir.forms.get(form_name) {
            let mut fields = HashMap::new();
            for fname in &hir_form.field_order {
                if let Some(hf) = hir_form.fields.get(fname) {
                    if hf.is_ambiguous {
                        report.diagnostics.push(Diagnostic {
                            severity: Severity::Information,
                            code: "I002_LAW2_COLLISION",
                            message: format!(
                                "Закон 2 (Lineage, предложение #{}): В типе '{}' поле '{}' унаследовано от: {}. Используйте явный путь предка.",
                                hir_form.origin.sentence_index,
                                form_name,
                                fname,
                                hf.conflicting_origins.join(" и ")
                            ),
                            context_symbol: Some(form_name.clone()),
                            sentence_index: Some(hir_form.origin.sentence_index),
                        });
                    }
                    fields.insert(
                        fname.clone(),
                        FieldSchema {
                            name: hf.name.clone(),
                            inferred_type: hf.inferred_type.clone(),
                            origin_form: hf.origin_form.clone(),
                            is_reactive: hf.is_reactive,
                            is_ambiguous: hf.is_ambiguous,
                            conflicting_origins: hf.conflicting_origins.clone(),
                        },
                    );
                }
            }

            let kind = if hir_form.is_algebraic_alias {
                SymbolKind::TypeAlgebra
            } else {
                SymbolKind::Form
            };

            report.symbol_order.push(form_name.clone());
            report.symbols.insert(
                form_name.clone(),
                SymbolInfo {
                    name: form_name.clone(),
                    kind,
                    signature: format!("Form {form_name} {{ {} fields }}", hir_form.field_order.len()),
                    documentation: format!(
                        "Статически разрешенная форма HIR (предложение #{})",
                        hir_form.origin.sentence_index
                    ),
                    fields,
                    field_order: hir_form.field_order.clone(),
                    ancestors: hir_form.ancestors.clone(),
                    params: vec![],
                    auto_unpacks_first_arg: false,
                },
            );
        }
    }

    // 2. Глобальные привязки из HIR
    for g_name in &hir.global_order {
        if let Some(g) = hir.globals.get(g_name) {
            let mut sym = SymbolInfo {
                name: g.key.clone(),
                kind: SymbolKind::Variable,
                signature: format!("{} : {}", g.key, g.inferred_type),
                documentation: format!("Глобальная привязка (предложение #{})", g.origin.sentence_index),
                fields: HashMap::new(),
                field_order: vec![],
                ancestors: vec![],
                params: vec![],
                auto_unpacks_first_arg: false,
            };
            if let Some(base_sym) = report.symbols.get(&g.inferred_type).cloned() {
                sym.fields = base_sym.fields;
                sym.field_order = base_sym.field_order;
                sym.ancestors = base_sym.ancestors;
            }
            report.symbol_order.push(g_name.clone());
            report.symbols.insert(g_name.clone(), sym);
        }
    }

    // 3. Функции и потоки из HIR + проверка Закона 1 и Закона 2 внутри их тел
    for fn_name in &hir.function_order {
        if let Some(hir_fn) = hir.functions.get(fn_name) {
            let params_tuple: Vec<(String, String)> = hir_fn
                .params
                .iter()
                .map(|p| (p.name.clone(), p.type_constraint.clone()))
                .collect();

            verify_hir_function_body(
                &hir_fn.name,
                &hir_fn.params,
                hir_fn.auto_unpacks,
                &hir_fn.body,
                hir_fn.origin.sentence_index,
                report,
            );

            let param_str = params_tuple
                .iter()
                .map(|(n, t)| if t.is_empty() { n.clone() } else { format!("{n} {t}") })
                .collect::<Vec<_>>()
                .join(", ");

            report.symbol_order.push(fn_name.clone());
            report.symbols.insert(
                fn_name.clone(),
                SymbolInfo {
                    name: fn_name.clone(),
                    kind: if hir_fn.params.is_empty() {
                        SymbolKind::Stream
                    } else {
                        SymbolKind::Function
                    },
                    signature: format!("{fn_name}({param_str}) [ ... ]"),
                    documentation: format!(
                        "HIR Function (предложение #{}, auto_unpacks: {})",
                        hir_fn.origin.sentence_index, hir_fn.auto_unpacks
                    ),
                    fields: HashMap::new(),
                    field_order: vec![],
                    ancestors: vec![],
                    params: params_tuple,
                    auto_unpacks_first_arg: hir_fn.auto_unpacks,
                },
            );
        }
    }

    report.hir = hir;
}

fn verify_hir_function_body(
    fn_name: &str,
    params: &[HirParam],
    auto_unpacks: bool,
    body: &[Sentence],
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    let mut shadowed_named_fields: HashMap<String, (String, String)> = HashMap::new();
    if !auto_unpacks {
        for p in params {
            if !p.type_constraint.is_empty() {
                let base_type = p
                    .type_constraint
                    .split_whitespace()
                    .next()
                    .unwrap_or(&p.type_constraint);
                if let Some(sym) = report.symbols.get(base_type) {
                    for f in &sym.field_order {
                        shadowed_named_fields
                            .insert(f.clone(), (p.name.clone(), base_type.to_string()));
                    }
                }
            }
        }
    }

    let mut local_vars = HashSet::new();
    for p in params {
        local_vars.insert(p.name.clone());
    }

    for sent in body {
        if let SemanticKind::Binding { key, .. } = classify_sentence(sent) {
            local_vars.insert(key);
        }
        check_tokens_for_laws(
            fn_name,
            sent,
            &local_vars,
            &shadowed_named_fields,
            sentence_idx,
            report,
        );
    }
}

fn check_tokens_for_laws(
    fn_name: &str,
    tokens: &[Token],
    local_vars: &HashSet<String>,
    shadowed_named_fields: &HashMap<String, (String, String)>,
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    if has_pipe_op(tokens) {
        let (head, steps) = split_pipeline(tokens);
        check_tokens_for_laws(
            fn_name,
            &head,
            local_vars,
            shadowed_named_fields,
            sentence_idx,
            report,
        );
        for step in steps {
            if let Some(Token::Group('{', branches)) = step.expr.first() {
                for branch in branches {
                    if let SemanticKind:: PatternBranch { body, .. } = classify_sentence(branch) {
                        for s in &body {
                            check_tokens_for_laws(
                                fn_name,
                                s,
                                local_vars,
                                shadowed_named_fields,
                                sentence_idx,
                                report,
                            );
                        }
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
                                "Закон 1 (предложение #{}): В '{}' параметр '{} {}' именованный. Используйте '{}.{}' вместо '{}'.",
                                sentence_idx, fn_name, param_var, param_type, param_var, p, p
                            ),
                            context_symbol: Some(fn_name.to_string()),
                            sentence_index: Some(sentence_idx),
                        });
                    }
                }
            }
            Token::Lit(Literal::String(raw)) => {
                for (field_name, (param_var, _)) in shadowed_named_fields {
                    if raw.contains(&format!("{{{field_name}}}")) && !local_vars.contains(field_name) {
                        report.diagnostics.push(Diagnostic {
                            severity: Severity::Warning,
                            code: "W001_LAW1_NAMED_SLOT_INTERP",
                            message: format!(
                                "Закон 1 (предложение #{}): В '{}' интерполяция '{{{}}}' обращается к нераспакованному полю. Используйте '{{{}.{}}}'.",
                                sentence_idx, fn_name, field_name, param_var, field_name
                            ),
                            context_symbol: Some(fn_name.to_string()),
                            sentence_index: Some(sentence_idx),
                        });
                    }
                }
            }
            _ => {}
        }
    }
}