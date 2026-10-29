// src/analyzer.rs
// =============================================================================
// RAZOMY V5.7 — HIR-POWERED STATIC ANALYZER & IDE ENGINE
// Реализует Законы 1–4 и каталог ошибок компилятора (E01..E07) + UFCS Autocomplete.
// =============================================================================

use serde_json::{json, Value as JsonValue};
use std::collections::{HashMap, HashSet};

use crate::hir::{
    is_known_type, is_pascal_case, lower_to_hir, pascal_to_snake_case, HirForm, HirFunction,
    HirModule, HirParam,
};
use crate::parser::{
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
    pub sentence_index: Option<usize>,
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
    pub diagnostics: Vec<Diagnostic>,
}

fn format_fn_signature(hir_fn: &HirFunction) -> String {
    let param_str = hir_fn
        .params
        .iter()
        .filter(|p| !p.is_type_param)
        .map(|p| {
            if p.type_constraint.is_empty() || p.name == p.type_constraint {
                p.name.clone()
            } else {
                format!("{} {}", p.name, p.type_constraint)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let ret_str = hir_fn
        .return_type
        .as_ref()
        .map(|r| format!(" {r}"))
        .unwrap_or_default();
    format!("{}({param_str}){ret_str} [ ... ]", hir_fn.name)
}

impl AnalysisReport {
    pub fn push_diag(
        &mut self,
        severity: Severity,
        code: &'static str,
        message: String,
        ctx: Option<&str>,
        idx: usize,
    ) {
        self.diagnostics.push(Diagnostic {
            severity,
            code,
            message,
            context_symbol: ctx.map(str::to_string),
            sentence_index: Some(idx),
        });
    }

    fn resolve_target_form(&self, name: &str) -> Option<(&HirForm, &str)> {
        if let Some(hf) = self.hir.forms.get(name) {
            return Some((hf, &hf.name));
        }
        if let Some(g) = self.hir.globals.get(name) {
            if let Some(hf) = self.hir.forms.get(&g.inferred_type) {
                return Some((hf, &g.inferred_type));
            }
        }
        None
    }

    pub fn to_ide_json(&self) -> JsonValue {
        let mut symbols_json = Vec::new();

        for name in &self.hir.form_order {
            if let Some(hf) = self.hir.forms.get(name) {
                let fields: Vec<JsonValue> = hf
                    .field_order
                    .iter()
                    .filter_map(|f| hf.fields.get(f))
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

                symbols_json.push(json!({
                    "name": hf.name,
                    "kind": if hf.is_algebraic_alias { "TypeAlgebra" } else { "Form" },
                    "signature": format!("Form {} {{ {} fields, {} methods }}", hf.name, hf.field_order.len(), hf.method_order.len()),
                    "doc": format!("HIR Form (предложение #{})", hf.origin.sentence_index),
                    "ancestors": hf.ancestors,
                    "unionVariants": hf.union_variants,
                    "methods": hf.method_order,
                    "autoUnpacks": false,
                    "isBypass": hf.is_bypass,
                    "fields": fields,
                }));
            }
        }

        for fn_name in &self.hir.function_order {
            if let Some(f) = self.hir.functions.get(fn_name) {
                symbols_json.push(json!({
                    "name": f.name,
                    "kind": if f.params.is_empty() { "Stream" } else { "Function" },
                    "signature": format_fn_signature(f),
                    "doc": format!("HIR Function (предложение #{})", f.origin.sentence_index),
                    "ancestors": [],
                    "unionVariants": [],
                    "methods": [],
                    "autoUnpacks": f.auto_unpacks,
                    "isBypass": false,
                    "fields": [],
                }));
            }
        }

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
            "language": "Razomy V5.7 (HIR Engine)",
            "symbols": symbols_json,
            "diagnostics": diags_json,
        })
    }

    pub fn hover(&self, symbol_name: &str) -> Option<String> {
        if let Some((hf, _)) = self.resolve_target_form(symbol_name) {
            let mut md = format!(
                "```razomy\nForm {} {{ {} fields, {} methods }}\n```\n",
                hf.name,
                hf.field_order.len(),
                hf.method_order.len()
            );
            if !hf.ancestors.is_empty() {
                md.push_str(&format!(
                    "**Lineage (Предки):** `{}`\n\n",
                    hf.ancestors.join(" & ")
                ));
            }
            if !hf.union_variants.is_empty() {
                md.push_str(&format!(
                    "**Union (Варианты):** `{}`\n\n",
                    hf.union_variants.join(" | ")
                ));
            }
            if !hf.field_order.is_empty() {
                md.push_str("**Статически разрешенные поля (HIR):**\n");
                for fname in &hf.field_order {
                    if let Some(f) = hf.fields.get(fname) {
                        let badge = if f.is_ambiguous {
                            "⚠️ [КОЛЛИЗИЯ LAW 2]"
                        } else if f.is_reactive {
                            "⚡ [Reactive Getter]"
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
            if !hf.method_order.is_empty() {
                md.push_str("\n**Методы и операторы:**\n");
                for m in &hf.method_order {
                    md.push_str(&format!("- `{m}`\n"));
                }
            }
            return Some(md);
        }

        if let Some(f) = self.hir.functions.get(symbol_name) {
            return Some(format!("```razomy\n{}\n```\n", format_fn_signature(f)));
        }

        if let Some(g) = self.hir.globals.get(symbol_name) {
            return Some(format!("```razomy\n{} : {}\n```\n", g.key, g.inferred_type));
        }

        None
    }

    /// Закон 4 (UFCS Autocomplete): по `Target.` выдает поля, собственные методы, предков Lineage
    /// И все внешние функции в области видимости, первый параметр которых совместим с `Target`!
    pub fn completions(&self, query: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        if let Some((base, prefix)) = query.rsplit_once('.') {
            if let Some((hf, target_type)) = self.resolve_target_form(base) {
                for fname in &hf.field_order {
                    if fname.starts_with(prefix) {
                        let f = &hf.fields[fname];
                        items.push(CompletionItem {
                            label: fname.clone(),
                            kind: if f.is_reactive {
                                "ReactiveField"
                            } else {
                                "Field"
                            },
                            detail: format!("{}: {} (from {})", fname, f.inferred_type, f.origin_form),
                            insert_text: fname.clone(),
                        });
                    }
                }
                for m in &hf.method_order {
                    if m.starts_with(prefix) {
                        items.push(CompletionItem {
                            label: m.clone(),
                            kind: "Method",
                            detail: format!("{base}.{m}(...)"),
                            insert_text: format!("{m}()"),
                        });
                    }
                }
                for anc in &hf.ancestors {
                    if anc.starts_with(prefix) {
                        items.push(CompletionItem {
                            label: anc.clone(),
                            kind: "AncestorSlice",
                            detail: format!("Lineage ancestor of {base}"),
                            insert_text: anc.clone(),
                        });
                    }
                }
                for fn_name in &self.hir.function_order {
                    if fn_name.starts_with(prefix) {
                        if let Some(fn_hir) = self.hir.functions.get(fn_name) {
                            if let Some(first_param) =
                                fn_hir.params.iter().find(|p| !p.is_type_param)
                            {
                                let base_constraint = first_param
                                    .type_constraint
                                    .split_whitespace()
                                    .next()
                                    .unwrap_or("");
                                if base_constraint == target_type
                                    || hf.ancestors.contains(&base_constraint.to_string())
                                {
                                    items.push(CompletionItem {
                                        label: fn_name.clone(),
                                        kind: "UFCSFunction",
                                        detail: format_fn_signature(fn_hir),
                                        insert_text: fn_name.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        } else {
            for name in &self.hir.form_order {
                if name.starts_with(query) {
                    if let Some(hf) = self.hir.forms.get(name) {
                        items.push(CompletionItem {
                            label: name.clone(),
                            kind: if hf.is_algebraic_alias {
                                "TypeAlgebra"
                            } else {
                                "Form"
                            },
                            detail: format!("Form {name}"),
                            insert_text: name.clone(),
                        });
                    }
                }
            }
            for name in &self.hir.function_order {
                if name.starts_with(query) {
                    if let Some(f) = self.hir.functions.get(name) {
                        items.push(CompletionItem {
                            label: name.clone(),
                            kind: if f.params.is_empty() {
                                "Stream"
                            } else {
                                "Function"
                            },
                            detail: format_fn_signature(f),
                            insert_text: name.clone(),
                        });
                    }
                }
            }
            for name in &self.hir.global_order {
                if name.starts_with(query) && !self.hir.forms.contains_key(name) {
                    if let Some(g) = self.hir.globals.get(name) {
                        items.push(CompletionItem {
                            label: name.clone(),
                            kind: "Variable",
                            detail: format!("{} : {}", g.key, g.inferred_type),
                            insert_text: name.clone(),
                        });
                    }
                }
            }
        }
        items
    }
}

pub fn analyze_source(source: &str) -> AnalysisReport {
    let mut report = AnalysisReport::default();
    match parse_code(source) {
        Ok(sentences) => analyze_from_hir(lower_to_hir(&sentences), &mut report),
        Err(err) => report.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: "E00_SYNTAX",
            message: format!("Синтаксическая ошибка парсера:\n{err}"),
            context_symbol: None,
            sentence_index: None,
        }),
    }
    report
}

pub fn analyze_from_hir(hir: HirModule, report: &mut AnalysisReport) {
    report.hir = hir;

    // 1. Формы + проверка Закона 1c (E02, E03) и Закона 2 (I002_LAW2_COLLISION)
    let form_order = report.hir.form_order.clone();
    for form_name in &form_order {
        if let Some(hf) = report.hir.forms.get(form_name).cloned() {
            if !hf.is_prelude {
                verify_form_slot_rules(&hf, report);

                for fname in &hf.field_order {
                    if let Some(field) = hf.fields.get(fname) {
                        if field.is_ambiguous {
                            report.push_diag(
                                Severity::Information,
                                "I002_LAW2_COLLISION",
                                format!(
                                    "Закон 2 (Lineage, #{}): В '{}' поле '{}' унаследовано от: {}. Используйте явный путь ({}.{fname}).",
                                    hf.origin.sentence_index,
                                    form_name,
                                    fname,
                                    field.conflicting_origins.join(" и "),
                                    field.conflicting_origins.first().cloned().unwrap_or_default()
                                ),
                                Some(form_name),
                                hf.origin.sentence_index,
                            );
                        }
                    }
                }
            }
        }
    }

    // 2. Глобальные привязки + проверка E07_STATIC_ASSERT_FAILED для `Compile & [...]`
    let global_order = report.hir.global_order.clone();
    for g_name in &global_order {
        if let Some(g) = report.hir.globals.get(g_name).cloned() {
            verify_compile_time_assertion(g_name, &g.value_expr, g.origin.sentence_index, report);
        }
    }

    // 3. Верификация параметров и тел функций и методов (Законы 1–3, E01..E06)
    let function_order = report.hir.function_order.clone();
    for fn_name in &function_order {
        if let Some(hir_fn) = report.hir.functions.get(fn_name).cloned() {
            verify_function_slots(&hir_fn.name, &hir_fn.params, hir_fn.origin.sentence_index, report);
            verify_function_body(
                &hir_fn.name,
                &hir_fn.params,
                hir_fn.auto_unpacks,
                &hir_fn.body,
                None,
                hir_fn.origin.sentence_index,
                report,
            );
        }
    }

    for form_name in &form_order {
        if let Some(hf) = report.hir.forms.get(form_name).cloned() {
            if hf.is_prelude {
                continue;
            }
            for m_name in &hf.method_order {
                if let Some(m_fn) = hf.methods.get(m_name) {
                    let full_name = format!("{form_name}.{m_name}");
                    verify_function_slots(&full_name, &m_fn.params, m_fn.origin.sentence_index, report);
                    verify_function_body(
                        &full_name,
                        &m_fn.params,
                        m_fn.auto_unpacks,
                        &m_fn.body,
                        Some(&hf.field_order),
                        m_fn.origin.sentence_index,
                        report,
                    );
                }
            }
        }
    }
}

/// Проверка `E02_DUPLICATE_UNNAMED_TYPE` и `E03_TYPE_PARAM_SHADOWS_TYPE` в слотах формы `Name(Slots) { ... }`
fn verify_form_slot_rules(hf: &HirForm, report: &mut AnalysisReport) {
    let mut seen_anon_types = HashSet::new();
    for vp in &hf.value_params {
        if vp.is_anonymous_type_slot {
            if !seen_anon_types.insert(vp.type_constraint.clone()) {
                report.push_diag(
                    Severity::Error,
                    "E02_DUPLICATE_UNNAMED_TYPE",
                    format!(
                        "Закон 1c (#{}): В форме '{}' тип '{}' указан несколько раз без явного snake_case имени.",
                        hf.origin.sentence_index, hf.name, vp.type_constraint
                    ),
                    Some(&hf.name),
                    hf.origin.sentence_index,
                );
            }
            for f in hf.fields.values() {
                if f.name != vp.name && f.inferred_type == vp.type_constraint {
                    report.push_diag(
                        Severity::Error,
                        "E03_TYPE_PARAM_SHADOWS_TYPE",
                        format!(
                            "Закон 1c (#{}): В '{}' параметр '{}' совпадает с уже существующим типом в скоупе и не может использоваться как Generic Type Parameter.",
                            hf.origin.sentence_index, hf.name, vp.type_constraint
                        ),
                        Some(&hf.name),
                        hf.origin.sentence_index,
                    );
                }
            }
        }
    }

    for tp in &hf.type_params {
        let base_tp = tp.split('(').next().unwrap_or(tp).trim();
        if is_known_type(base_tp, &report.hir) && base_tp != hf.name {
            report.push_diag(
                Severity::Error,
                "E03_TYPE_PARAM_SHADOWS_TYPE",
                format!(
                    "Закон 1c (#{}): Параметр типа '{}' в '{}' затеняет существующий тип в области видимости.",
                    hf.origin.sentence_index, base_tp, hf.name
                ),
                Some(&hf.name),
                hf.origin.sentence_index,
            );
        }
    }
}

/// Проверка `E02_DUPLICATE_UNNAMED_TYPE` в слотах функций (`transfer_bad(Account, Account, Money)` или `finalize(User & Cart)`)
fn verify_function_slots(
    fn_name: &str,
    params: &[HirParam],
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    let mut seen_anon_types = HashSet::new();
    for p in params {
        if p.is_anonymous_type_slot {
            if !seen_anon_types.insert(p.type_constraint.clone()) {
                report.push_diag(
                    Severity::Error,
                    "E02_DUPLICATE_UNNAMED_TYPE",
                    format!(
                        "Закон 1c (#{sentence_idx}): В '{fn_name}' тип '{}' повторяется без явного snake_case имени параметра.",
                        p.type_constraint
                    ),
                    Some(fn_name),
                    sentence_idx,
                );
            }
        }
    }
}

/// Проверка `E07_STATIC_ASSERT_FAILED`: если в `Compile & [ ... ]` вызывается `Compile.error(...)`
fn verify_compile_time_assertion(
    name: &str,
    value_expr: &[Token],
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    if let [Token::Group('(', inner)] = value_expr {
        if let Some([Token::Path(c), Token::Op(amp), Token::Group('[', sents)]) =
            inner.first().map(Vec::as_slice)
        {
            if c == "Compile" && amp == "&" {
                for s in sents {
                    if let [Token::Path(p), Token::Group(')', args)] = s.as_slice() {
                        if p == "Compile.error" {
                            let msg = args
                                .first()
                                .and_then(|a| a.first())
                                .map(|t| format!("{t:?}"))
                                .unwrap_or_else(|| "Static assert failed".into());
                            report.push_diag(
                                Severity::Error,
                                "E07_STATIC_ASSERT_FAILED",
                                format!("Статическое утверждение '{name}' не выполнено: {msg}"),
                                Some(name),
                                sentence_idx,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn verify_function_body(
    fn_name: &str,
    params: &[HirParam],
    auto_unpacks: bool,
    body: &[Sentence],
    enclosing_fields: Option<&Vec<String>>,
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    let mut shadowed_named_fields: HashMap<String, (String, String)> = HashMap::new();
    let mut ambiguous_fields: HashMap<String, Vec<String>> = HashMap::new();
    let mut local_vars: HashSet<String> = HashSet::new();
    let mut var_types: HashMap<String, String> = HashMap::new();

    // Встроенные контекстные переменные потоков и форм (Law 1a, 1b)
    for builtin_var in ["self", "Self", "_", "_0", "_1", "_2", "_3"] {
        local_vars.insert(builtin_var.into());
    }

    if let Some(fields) = enclosing_fields {
        local_vars.extend(fields.iter().cloned());
    }

    for p in params {
        local_vars.insert(p.name.clone());
        if !p.type_constraint.is_empty() {
            var_types.insert(p.name.clone(), p.type_constraint.clone());
        }
        for df in &p.destructured_fields {
            local_vars.insert(df.clone());
        }

        if p.is_anonymous_type_slot {
            let base = p.type_constraint.split_whitespace().next().unwrap_or("");
            if let Some(hf) = report.hir.forms.get(base) {
                for f in &hf.field_order {
                    local_vars.insert(f.clone());
                }
            }
            for part in p.type_constraint.split_whitespace() {
                if !matches!(part, "&" | "|" | "/") {
                    collect_ambiguous_fields(part, &report.hir, &mut ambiguous_fields);
                }
            }
        } else {
            for part in p.type_constraint.split_whitespace() {
                if !matches!(part, "&" | "|" | "/") {
                    collect_ambiguous_fields(part, &report.hir, &mut ambiguous_fields);
                }
            }
            if !auto_unpacks {
                let base = p.type_constraint.split_whitespace().next().unwrap_or("");
                if let Some(hf) = report.hir.forms.get(base) {
                    for f in &hf.field_order {
                        shadowed_named_fields
                            .insert(f.clone(), (p.name.clone(), base.to_string()));
                    }
                }
            }
        }
    }

    for sent in body {
        match classify_sentence(sent) {
            SemanticKind::AlgebraicDef { name, expr } => {
                if let Some(Token::Path(base)) = expr.first() {
                    var_types.insert(name.clone(), base.clone());
                }
                local_vars.insert(name);
            }
            SemanticKind::Binding { key, value } => {
                // 1. Проверка `E01_AMBIGUOUS_CALL_OR_BIND`: `area (Circle { radius 5.0 })` (пробел перед `(` у имени функции!)
                if matches!(value.as_slice(), [Token::Group('(', _)]) && !local_vars.contains(&key)
                {
                    if let Some(f) = report.hir.functions.get(&key) {
                        if !f.params.is_empty() {
                            report.push_diag(
                                Severity::Error,
                                "E01_AMBIGUOUS_CALL_OR_BIND",
                                format!(
                                    "Ошибка E01 (#{sentence_idx}): '{key}' является функцией в области видимости. Нельзя вызывать функцию с пробелом перед '(' или перепривязывать её имя."
                                ),
                                Some(fn_name),
                                sentence_idx,
                            );
                        }
                    }
                }

                // 2. Проверка `E06_IMMUTABLE_TYPE_BINDING`: попытка мутировать `PascalCase` форму напрямую (`Car.max_speed 500`)
                let root_ident = key.split('.').next().unwrap_or(&key);
                if is_pascal_case(root_ident)
                    && (key.contains('.') || report.hir.forms.contains_key(root_ident))
                {
                    report.push_diag(
                        Severity::Error,
                        "E06_IMMUTABLE_TYPE_BINDING",
                        format!(
                            "Ошибка E06 (#{sentence_idx}): '{root_ident}' является неизменяемым PascalCase прототипом/типом формы. Создайте snake_case экземпляр перед мутацией."
                        ),
                        Some(fn_name),
                        sentence_idx,
                    );
                }

                if let Some(inferred) = infer_local_binding_type(&value, &var_types) {
                    var_types.insert(key.clone(), inferred);
                }
                local_vars.insert(key);
            }
            SemanticKind::TupleBinding { tuple, .. } => {
                for t in tuple {
                    if let [Token::Path(v)] = t.as_slice() {
                        local_vars.insert(v.clone());
                    }
                }
            }
            _ => {}
        }
        check_tokens(
            fn_name,
            sent,
            &local_vars,
            &var_types,
            &shadowed_named_fields,
            &ambiguous_fields,
            sentence_idx,
            report,
        );
    }
}

fn infer_local_binding_type(
    value: &[Token],
    var_types: &HashMap<String, String>,
) -> Option<String> {
    let inner = match value {
        [Token::Group('(', sents)] if sents.len() == 1 => sents[0].as_slice(),
        other => other,
    };
    match inner {
        [Token::Path(form_name), Token::Group('{', _)] => Some(form_name.clone()),
        [Token::Path(src_var), Token::Op(op), Token::Path(sub_target)] if op == "/" => {
            let base_ty = var_types.get(src_var)?;
            Some(format!("{base_ty} / {sub_target}"))
        }
        _ => None,
    }
}

fn collect_ambiguous_fields(
    type_name: &str,
    hir: &HirModule,
    out: &mut HashMap<String, Vec<String>>,
) {
    if let Some(hf) = hir.forms.get(type_name) {
        for (fname, f) in &hf.fields {
            if f.is_ambiguous {
                out.insert(fname.clone(), f.conflicting_origins.clone());
            }
        }
    }
}

/// Проверка идентификатора и путей (`entity.name`, `{user_order.id}`, `{Name}`)
#[allow(clippy::too_many_arguments)]
fn verify_identifier(
    ident: &str,
    is_interp: bool,
    fn_name: &str,
    local_vars: &HashSet<String>,
    var_types: &HashMap<String, String>,
    shadowed: &HashMap<String, (String, String)>,
    ambiguous: &HashMap<String, Vec<String>>,
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    // 1. Проверка точечного доступа `entity.name` или `user_order.id` на коллизию Закона 2 (`E05_AMBIGUOUS_LINEAGE_FIELD`)
    if let Some((obj_var, field_part)) = ident.split_once('.') {
        if !field_part.contains('.') {
            if let Some(ty_expr) = var_types.get(obj_var) {
                if let Some(conflicts) = check_type_expr_field_ambiguity(ty_expr, field_part, report)
                {
                    report.push_diag(
                        Severity::Error,
                        "E05_AMBIGUOUS_LINEAGE_FIELD",
                        format!(
                            "Нарушение Закона 2 (#{sentence_idx}): Поле '{field_part}' у переменной '{obj_var}' ({ty_expr}) неоднозначно (присутствует в {}). Используйте явный путь предка ({obj_var}.{}.{field_part}).",
                            conflicts.join(" и "),
                            conflicts.first().cloned().unwrap_or_default()
                        ),
                        Some(fn_name),
                        sentence_idx,
                    );
                }
            }
        }
        return;
    }

    if let Some(origins) = ambiguous.get(ident) {
        let suggestion = origins
            .iter()
            .map(|o| {
                if is_interp {
                    format!("{{{o}.{ident}}}")
                } else {
                    format!("{o}.{ident}")
                }
            })
            .collect::<Vec<_>>()
            .join(" или ");
        report.push_diag(
            Severity::Error,
            "E05_AMBIGUOUS_LINEAGE_FIELD",
            format!(
                "Нарушение Закона 2 (#{sentence_idx}): В '{fn_name}' поле '{ident}' неоднозначно (предков: {}). Используйте {suggestion}.",
                origins.join(" & ")
            ),
            Some(fn_name),
            sentence_idx,
        );
        return;
    }

    if local_vars.contains(ident) {
        return;
    }

    if let Some((param_var, param_type)) = shadowed.get(ident) {
        let code = if is_interp {
            "W001_LAW1_NAMED_SLOT_INTERP"
        } else {
            "W001_LAW1_NAMED_SLOT"
        };
        let fix = if is_interp {
            format!("{{{param_var}.{ident}}}")
        } else {
            format!("{param_var}.{ident}")
        };
        report.push_diag(
            Severity::Warning,
            code,
            format!(
                "Закон 1 (#{sentence_idx}): В '{fn_name}' параметр '{param_var} {param_type}' именованный. Используйте '{fix}'."
            ),
            Some(fn_name),
            sentence_idx,
        );
    }
}

fn check_type_expr_field_ambiguity(
    ty_expr: &str,
    field_name: &str,
    report: &AnalysisReport,
) -> Option<Vec<String>> {
    if let Some(hf) = report.hir.forms.get(ty_expr) {
        if let Some(f) = hf.fields.get(field_name) {
            if f.is_ambiguous {
                return Some(f.conflicting_origins.clone());
            }
        }
        return None;
    }

    let mut origins = Vec::new();
    let mut subtracted = HashSet::new();
    let parts: Vec<&str> = ty_expr.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        if parts[i] == "/" {
            if let Some(&sub) = parts.get(i + 1) {
                subtracted.insert(sub.to_string());
            }
            i += 2;
            continue;
        }
        if parts[i] != "&" && parts[i] != "|" {
            let anc = parts[i];
            if !subtracted.contains(anc) && !subtracted.contains(&format!("{anc}.{field_name}")) {
                if let Some(hf) = report.hir.forms.get(anc) {
                    if hf.fields.contains_key(field_name) {
                        origins.push(anc.to_string());
                    }
                }
            }
        }
        i += 1;
    }
    // Перепроверяем вычитания, которые шли после `&`
    origins.retain(|anc| {
        !subtracted.contains(anc) && !subtracted.contains(&format!("{anc}.{field_name}"))
    });
    if origins.len() > 1 {
        Some(origins)
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn check_tokens(
    fn_name: &str,
    tokens: &[Token],
    local_vars: &HashSet<String>,
    var_types: &HashMap<String, String>,
    shadowed: &HashMap<String, (String, String)>,
    ambiguous: &HashMap<String, Vec<String>>,
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    if has_pipe_op(tokens) {
        let (head, steps) = split_pipeline(tokens);
        check_tokens(
            fn_name,
            &head,
            local_vars,
            var_types,
            shadowed,
            ambiguous,
            sentence_idx,
            report,
        );

        let mut current_type = infer_static_token_type(&head, var_types, report);
        for step in steps {
            if let [Token::Path(target_fn)] = step.expr.as_slice() {
                if let Some(in_type) = &current_type {
                    verify_typestate_and_union_bypass(
                        fn_name,
                        in_type,
                        target_fn,
                        sentence_idx,
                        report,
                    );
                }
                if let Some(fn_hir) = report.hir.functions.get(target_fn) {
                    if let Some(ret) = &fn_hir.return_type {
                        current_type = Some(ret.clone());
                    }
                }
            }

            if let Some(Token::Group('{', branches)) = step.expr.first() {
                for branch in branches {
                    match classify_sentence(branch) {
                        SemanticKind::PatternBranch { pattern, body } => {
                            let mut branch_locals = local_vars.clone();
                            extract_pattern_vars(&pattern, &mut branch_locals);
                            for s in &body {
                                check_tokens(
                                    fn_name,
                                    s,
                                    &branch_locals,
                                    var_types,
                                    shadowed,
                                    ambiguous,
                                    sentence_idx,
                                    report,
                                );
                            }
                        }
                        SemanticKind::StreamOrBranch { name, stream } => {
                            let mut branch_locals = local_vars.clone();
                            let mut branch_var_types = var_types.clone();
                            let mut branch_ambiguous = ambiguous.clone();
                            if is_pascal_case(&name) {
                                let snake = pascal_to_snake_case(&name);
                                branch_locals.insert(snake.clone());
                                branch_var_types.insert(snake, name.clone());
                            }
                            collect_ambiguous_fields(&name, &report.hir, &mut branch_ambiguous);
                            for s in &stream {
                                check_tokens(
                                    fn_name,
                                    s,
                                    &branch_locals,
                                    &branch_var_types,
                                    shadowed,
                                    &branch_ambiguous,
                                    sentence_idx,
                                    report,
                                );
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                check_tokens(
                    fn_name,
                    &step.expr,
                    local_vars,
                    var_types,
                    shadowed,
                    ambiguous,
                    sentence_idx,
                    report,
                );
            }
        }
        return;
    }

    for tok in tokens {
        match tok {
            Token::Path(p) => verify_identifier(
                p,
                false,
                fn_name,
                local_vars,
                var_types,
                shadowed,
                ambiguous,
                sentence_idx,
                report,
            ),
            Token::Lit(Literal::String(raw)) => {
                for chunk in raw.split('{').skip(1) {
                    if let Some(expr) = chunk.split('}').next() {
                        verify_identifier(
                            expr.trim(),
                            true,
                            fn_name,
                            local_vars,
                            var_types,
                            shadowed,
                            ambiguous,
                            sentence_idx,
                            report,
                        );
                    }
                }
            }
            Token::Group(_, sents) => {
                for s in sents {
                    check_tokens(
                        fn_name,
                        s,
                        local_vars,
                        var_types,
                        shadowed,
                        ambiguous,
                        sentence_idx,
                        report,
                    );
                }
            }
            _ => {}
        }
    }
}

fn extract_pattern_vars(pattern: &Token, locals: &mut HashSet<String>) {
    if let Token::Group('{', fields) | Token::Group('(' | ')', fields) = pattern {
        for f in fields {
            match classify_sentence(f) {
                SemanticKind::Binding { value, .. } => {
                    for v in value {
                        extract_pattern_vars(&v, locals);
                    }
                }
                _ => {
                    if let [Token::Path(var)] = f.as_slice() {
                        if var != "_" {
                            if is_pascal_case(var) {
                                locals.insert(pascal_to_snake_case(var));
                            } else {
                                locals.insert(var.clone());
                            }
                        }
                    }
                }
            }
        }
    }
}

fn infer_static_token_type(
    tokens: &[Token],
    var_types: &HashMap<String, String>,
    report: &AnalysisReport,
) -> Option<String> {
    match tokens {
        [Token::Path(name), Token::Group('{', _)] => Some(name.clone()),
        [Token::Path(var)] => var_types.get(var).cloned().or_else(|| {
            if report.hir.forms.contains_key(var) {
                Some(var.clone())
            } else {
                report.hir.globals.get(var).map(|g| g.inferred_type.clone())
            }
        }),
        _ => None,
    }
}

/// Закон 3 (Subtyping-Driven `Bypass` Outcome Short-Circuiting & Typestate Validation):
/// • Проверяет `E04_UNHANDLED_UNION_BRANCH`, если в пайплайн подается Union `A | B | NotFound`,
///   где ветка `B` не совместима с параметром функции и НЕ помечена тегом `& Bypass`!
/// • Проверяет Typestate ограничения (`Order & Reserved & Paid` и `Order / Cancelled`).
fn verify_typestate_and_union_bypass(
    fn_name: &str,
    input_type: &str,
    target_fn: &str,
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    let Some(constraint) = report
        .hir
        .functions
        .get(target_fn)
        .and_then(|f| f.params.iter().find(|p| !p.is_type_param))
        .map(|p| p.type_constraint.clone())
    else {
        return;
    };
    if constraint.is_empty() || constraint == "Any" {
        return;
    }

    // 1. Закон 3: Проверка Union-входа `User | Company | NotFound` -> `send_welcome_email(User)`
    if input_type.contains('|') && !constraint.contains('|') {
        let expected_base = constraint.split_whitespace().next().unwrap_or("");
        for branch in input_type.split('|').map(str::trim) {
            if branch == expected_base {
                continue;
            }
            if let Some(branch_form) = report.hir.forms.get(branch) {
                let is_subtype = branch_form.ancestors.contains(&expected_base.to_string());
                if !is_subtype && !branch_form.is_bypass {
                    report.push_diag(
                        Severity::Error,
                        "E04_UNHANDLED_UNION_BRANCH",
                        format!(
                            "Нарушение Закона 3 (#{sentence_idx}): Ветка '{branch}' из '{input_type}' не совместима с параметром '{constraint}' функции '{target_fn}' и не помечена тегом '& Bypass'."
                        ),
                        Some(fn_name),
                        sentence_idx,
                    );
                }
            }
        }
        return;
    }

    // 2. Проверка Typestate (`Order & Reserved & Paid` или `Order / Cancelled`)
    let Some(in_sym) = report.hir.forms.get(input_type).cloned() else {
        return;
    };
    if !constraint.contains('&') && !constraint.contains('/') {
        return;
    }

    let parts: Vec<&str> = constraint.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        if parts[i] == "&" {
            if let Some(&req_trait) = parts.get(i + 1) {
                if in_sym.name != req_trait && !in_sym.ancestors.contains(&req_trait.to_string()) {
                    report.push_diag(
                        Severity::Error,
                        "E003_TYPESTATE_VIOLATION",
                        format!(
                            "Нарушение Typestate (#{sentence_idx}): '{target_fn}' требует '{constraint}', но тип '{input_type}' не содержит состояния '{req_trait}'."
                        ),
                        Some(fn_name),
                        sentence_idx,
                    );
                }
            }
            i += 2;
        } else if parts[i] == "/" {
            if let Some(&forbidden) = parts.get(i + 1) {
                if in_sym.name == forbidden || in_sym.ancestors.contains(&forbidden.to_string()) {
                    report.push_diag(
                        Severity::Error,
                        "E003_TYPESTATE_FORBIDDEN",
                        format!(
                            "Нарушение Typestate (#{sentence_idx}): '{target_fn}' запрещает состояние '{forbidden}' ('{constraint}'), но '{input_type}' включает его."
                        ),
                        Some(fn_name),
                        sentence_idx,
                    );
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }
}