// src/analyzer.rs
// =============================================================================
// RAZOMY V5.7 — HIR-POWERED STATIC ANALYZER & IDE ENGINE
// Реализует Законы 1–4 и каталог ошибок компилятора (E01..E07) + UFCS Autocomplete.
// =============================================================================

use serde_json::{json, Value as JsonValue};
use std::collections::{HashMap, HashSet};

use crate::hir::{
    infer_hir_type_with_locals, is_known_type, is_pascal_case, lower_to_hir,
    lower_to_hir_located, pascal_to_snake_case, substitute_type_vars, HirForm, HirFunction,
    HirModule, HirParam,
};
use crate::parser::{
    classify_sentence, has_pipe_op, parse_code, parse_located_with_imports_and_memory,
    recover_source_for_ide, split_pipeline, Literal, PipeOp, SemanticKind, Sentence, Token,
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
    pub highlight_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: &'static str,
    pub detail: String,
    pub insert_text: String,
}

#[derive(Debug, Clone, Default)]
pub struct ScopeSnapshot {
    pub context_name: String,
    pub enclosing_form: Option<String>,
    pub sentence_index: usize,
    pub var_types: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct AnalysisReport {
    pub hir: HirModule,
    pub diagnostics: Vec<Diagnostic>,
    pub scopes: HashMap<String, ScopeSnapshot>,
    pub sentence_scopes: HashMap<usize, ScopeSnapshot>,
    pub imported_sentence_offset: usize,
}

pub fn format_fn_signature(hir_fn: &HirFunction) -> String {
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
        .effective_return_type()
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
        self.push_diag_with_token(severity, code, message, ctx, idx, None);
    }

    pub fn push_diag_with_token(
        &mut self,
        severity: Severity,
        code: &'static str,
        message: String,
        ctx: Option<&str>,
        idx: usize,
        highlight_token: Option<&str>,
    ) {
        self.diagnostics.push(Diagnostic {
            severity,
            code,
            message,
            context_symbol: ctx.map(str::to_string),
            sentence_index: Some(idx),
            highlight_token: highlight_token.map(str::to_string),
        });
    }

    pub fn scope_for_sentence(&self, sentence_idx: Option<usize>) -> Option<&ScopeSnapshot> {
        let idx = sentence_idx?;
        if let Some(sc) = self.sentence_scopes.get(&idx) {
            return Some(sc);
        }
        if self.scopes.len() == 1 {
            return self.scopes.values().next();
        }
        None
    }

    /// Преобразует любое выражение типа (`User`, `MyGarageCar`, `User & Cart`, `Order / Cancelled`,
    /// `Atom(CounterState)`, `Some(User)`, `User | NotFound`) в разрешенную структуру `HirForm`.
    pub fn resolve_type_to_form(&self, type_expr: &str) -> Option<HirForm> {
        let trimmed = type_expr.trim();
        if trimmed.is_empty() || matches!(trimmed, "Any" | "_" | "None") {
            return None;
        }

        // 1. Прямое совпадение по имени формы
        if let Some(hf) = self.hir.forms.get(trimmed) {
            return Some(hf.clone());
        }

        // 2. Инстанцирование дженерика `Atom(CounterState)`, `Some(String)`, `Channel(Int)`
        if let Some((base_name, rest)) = trimmed.split_once('(') {
            let base_clean = base_name.trim();
            if let Some(args_str) = rest.strip_suffix(')') {
                if let Some(base_hf) = self.hir.forms.get(base_clean) {
                    let mut instantiated = base_hf.clone();
                    let type_args: Vec<&str> = args_str.split(',').map(str::trim).collect();
                    let mut subst = HashMap::new();
                    for (tp, targ) in base_hf.type_params.iter().zip(type_args.iter()) {
                        let tp_clean = tp.split_whitespace().next().unwrap_or(tp);
                        subst.insert(tp_clean.to_string(), (*targ).to_string());
                    }
                    for field in instantiated.fields.values_mut() {
                        field.inferred_type = substitute_type_vars(&field.inferred_type, &subst);
                    }
                    for method in instantiated.methods.values_mut() {
                        if let Some(ret) = &mut method.return_type {
                            *ret = substitute_type_vars(ret, &subst);
                        }
                        for p in &mut method.params {
                            p.type_constraint = substitute_type_vars(&p.type_constraint, &subst);
                        }
                    }
                    return Some(instantiated);
                }
            }
        }

        // 3. Union тип `User | NotFound` или `Circle | Rect`:
        // Если одна из веток не является Bypass, берем её или объединяем поля не-Bypass веток
        if trimmed.contains('|') {
            let branches: Vec<&str> = trimmed.split('|').map(str::trim).collect();
            let mut non_bypass_forms = Vec::new();
            for b in &branches {
                if let Some(bf) = self.resolve_type_to_form(b) {
                    if !bf.is_bypass && bf.name != "None" {
                        non_bypass_forms.push(bf);
                    }
                }
            }
            if non_bypass_forms.len() == 1 {
                let mut single = non_bypass_forms.remove(0);
                single.union_variants = branches.iter().map(|s| (*s).to_string()).collect();
                return Some(single);
            } else if !non_bypass_forms.is_empty() {
                let mut combined = non_bypass_forms[0].clone();
                combined.name = trimmed.to_string();
                combined.union_variants = branches.iter().map(|s| (*s).to_string()).collect();
                return Some(combined);
            }
        }

        // 4. Составная алгебра типов `User & Cart`, `Order & Reserved & Paid`, `Order / Cancelled`, `User & Cloud / Cloud.name`
        if trimmed.contains('&') || trimmed.contains('/') {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            let first = parts.first().copied()?;
            let mut synth = self.resolve_type_to_form(first)?;
            synth.name = trimmed.to_string();
            if !synth.ancestors.contains(&first.to_string()) {
                synth.ancestors.insert(0, first.to_string());
            }

            let mut i = 1;
            while i < parts.len() {
                match parts[i] {
                    "&" => {
                        if let Some(&rhs_name) = parts.get(i + 1) {
                            if rhs_name == "Bypass" {
                                synth.is_bypass = true;
                            }
                            if !synth.ancestors.contains(&rhs_name.to_string()) {
                                synth.ancestors.push(rhs_name.to_string());
                            }
                            if let Some(rhs_form) = self.resolve_type_to_form(rhs_name) {
                                for anc in &rhs_form.ancestors {
                                    if !synth.ancestors.contains(anc) {
                                        synth.ancestors.push(anc.clone());
                                    }
                                }
                                for fname in &rhs_form.field_order {
                                    if let Some(rf) = rhs_form.fields.get(fname) {
                                        if let Some(existing) = synth.fields.get_mut(fname) {
                                            if existing.origin_form != rf.origin_form {
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
                                                    .contains(&rf.origin_form)
                                                {
                                                    existing
                                                        .conflicting_origins
                                                        .push(rf.origin_form.clone());
                                                }
                                            }
                                        } else {
                                            synth.field_order.push(fname.clone());
                                            synth.fields.insert(fname.clone(), rf.clone());
                                        }
                                    }
                                }
                                for mname in &rhs_form.method_order {
                                    if let Some(rm) = rhs_form.methods.get(mname) {
                                        if !synth.methods.contains_key(mname) {
                                            synth.method_order.push(mname.clone());
                                            synth.methods.insert(mname.clone(), rm.clone());
                                        }
                                    }
                                }
                            }
                        }
                        i += 2;
                    }
                    "/" => {
                        if let Some(&sub_target) = parts.get(i + 1) {
                            let clean_sub = sub_target.trim_matches('\'').trim_matches('"');
                            if let Some((anc, fld)) = clean_sub.split_once('.') {
                                if let Some(f) = synth.fields.get_mut(fld) {
                                    if f.is_ambiguous {
                                        f.conflicting_origins.retain(|o| o != anc);
                                        if f.conflicting_origins.len() == 1 {
                                            f.is_ambiguous = false;
                                            f.origin_form = f.conflicting_origins[0].clone();
                                        }
                                    } else {
                                        synth.fields.remove(fld);
                                        synth.field_order.retain(|k| k != fld);
                                    }
                                }
                            } else {
                                synth.ancestors.retain(|a| a != clean_sub);
                                synth.fields.remove(clean_sub);
                                synth.fields.retain(|_, f| f.origin_form != clean_sub);
                                synth.field_order.retain(|k| synth.fields.contains_key(k));
                            }
                        }
                        i += 2;
                    }
                    _ => {
                        i += 1;
                    }
                }
            }
            return Some(synth);
        }

        None
    }

    /// Вычисляет тип произвольного выражения или цепочки `a.b.c` / `a.b().0` в заданном скоупе
    pub fn resolve_expr_type(
        &self,
        expr: &str,
        scope: Option<&ScopeSnapshot>,
    ) -> Option<String> {
        let trimmed = expr.trim().trim_end_matches('.');
        if trimmed.is_empty() {
            return None;
        }

        let segments: Vec<&str> = trimmed.split('.').collect();
        let first_raw = segments[0];
        let (first_ident, first_is_call) = first_raw
            .strip_suffix("()")
            .map(|s| (s, true))
            .unwrap_or((first_raw, false));

        // 1. Разрешаем базовый идентификатор
        let mut current_type = if first_is_call {
            self.hir
                .functions
                .get(first_ident)
                .and_then(|f| f.effective_return_type().map(str::to_string))
                .or_else(|| {
                    self.hir
                        .forms
                        .contains_key(first_ident)
                        .then(|| first_ident.to_string())
                })?
        } else if let Some(ty) = scope.and_then(|s| s.var_types.get(first_ident)) {
            ty.clone()
        } else if let Some(hf) = self.hir.forms.get(first_ident) {
            hf.name.clone()
        } else if let Some(g) = self.hir.globals.get(first_ident) {
            g.inferred_type.clone()
        } else if let Some(f) = self.hir.functions.get(first_ident) {
            f.effective_return_type()
                .map(str::to_string)
                .unwrap_or_else(|| "Stream".into())
        } else {
            // Fallback: ищем однозначную локальную переменную или параметр в скоупах функций
            self.scopes
                .values()
                .find_map(|s| s.var_types.get(first_ident).cloned())?
        };

        // 2. Проходим по цепочке `.segment`
        for &seg_raw in &segments[1..] {
            let (seg, is_call) = seg_raw
                .strip_suffix("()")
                .map(|s| (s, true))
                .unwrap_or((seg_raw, false));

            if seg == "to_float" {
                current_type = "Float".into();
                continue;
            }
            if seg == "round_to_int" || seg == "length" {
                current_type = "Int".into();
                continue;
            }

            // Позиционный индекс `.0`, `.1`, `.-1` на Tuple/List или форме
            if let Ok(idx) = seg.parse::<i32>() {
                if let Some(elem_ty) = extract_collection_element_type(&current_type, idx) {
                    current_type = elem_ty;
                    continue;
                }
                if let Some(hf) = self.resolve_type_to_form(&current_type) {
                    if hf.ancestors.contains(&"Tuple".to_string()) && !hf.type_args.is_empty() {
                        let i = if idx < 0 {
                            (hf.type_args.len() as i32 + idx) as usize
                        } else {
                            idx as usize
                        };
                        if let Some(targ) = hf.type_args.get(i).or_else(|| hf.type_args.first()) {
                            current_type = targ.clone();
                            continue;
                        }
                    }
                    let i = if idx < 0 {
                        (hf.field_order.len() as i32 + idx) as usize
                    } else {
                        idx as usize
                    };
                    if let Some(fname) = hf.field_order.get(i) {
                        if let Some(f) = hf.fields.get(fname) {
                            current_type = f.inferred_type.clone();
                            continue;
                        }
                    }
                }
                return None;
            }

            let hf = self.resolve_type_to_form(&current_type)?;
            if is_call {
                if let Some(m) = hf.methods.get(seg) {
                    current_type = m
                        .effective_return_type()
                        .unwrap_or("Any")
                        .to_string();
                    continue;
                }
            }
            if hf.ancestors.contains(&seg.to_string())
                || hf.ancestor_fields.contains_key(seg)
            {
                current_type = seg.to_string();
                continue;
            }
            if hf.submodules.contains_key(seg) {
                current_type = seg.to_string();
                continue;
            }
            if let Some(f) = hf.fields.get(seg) {
                current_type = f.inferred_type.clone();
                continue;
            }
            if let Some(m) = hf.methods.get(seg) {
                current_type = m
                    .effective_return_type()
                    .unwrap_or("Any")
                    .to_string();
                continue;
            }
            return None;
        }

        Some(current_type)
    }

    pub fn resolve_target_form_in_scope(
        &self,
        expr: &str,
        scope: Option<&ScopeSnapshot>,
    ) -> Option<(HirForm, String)> {
        let resolved_ty = self.resolve_expr_type(expr, scope)?;
        let hf = self.resolve_type_to_form(&resolved_ty)?;
        let base_ty = resolved_ty
            .split('(')
            .next()
            .unwrap_or(&resolved_ty)
            .trim()
            .to_string();
        Some((hf, base_ty))
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
        self.hover_at(symbol_name, None)
    }

    pub fn hover_at(&self, symbol_name: &str, sentence_idx: Option<usize>) -> Option<String> {
        let scope = self.scope_for_sentence(sentence_idx);
        let clean = symbol_name.trim().trim_matches('.');
        if clean.is_empty() {
            return None;
        }

        // 1. Проверяем функцию верхнего уровня
        if !clean.contains('.') {
            if let Some(f) = self.hir.functions.get(clean) {
                let mut md = format!("```razomy\n{}\n```\n", format_fn_signature(f));
                if f.auto_unpacks {
                    let unpacks: Vec<String> = f
                        .params
                        .iter()
                        .filter(|p| p.is_anonymous_type_slot)
                        .map(|p| format!("`{}`: `{}`", p.name, p.type_constraint))
                        .collect();
                    if !unpacks.is_empty() {
                        md.push_str(&format!(
                            "**Law 1c Auto-Bind:** {}\n",
                            unpacks.join(", ")
                        ));
                    }
                }
                return Some(md);
            }
        }

        // 2. Проверяем составной путь `obj.member` (поле или метод на форме или экземпляре)
        if let Some((base, member)) = clean.rsplit_once('.') {
            if let Some((hf, _)) = self.resolve_target_form_in_scope(base, scope) {
                if let Some(field) = hf.fields.get(member) {
                    let kind_badge = if field.is_ambiguous {
                        " ⚠️ [КОЛЛИЗИЯ LAW 2]"
                    } else if field.is_reactive {
                        " ⚡ [Reactive Stream Getter]"
                    } else {
                        ""
                    };
                    return Some(format!(
                        "```razomy\n{}.{}: {} (from {}){}\n```",
                        hf.name, field.name, field.inferred_type, field.origin_form, kind_badge
                    ));
                }
                if let Some(method) = hf.methods.get(member) {
                    let params_str = method
                        .params
                        .iter()
                        .filter(|p| !p.is_type_param)
                        .map(|p| {
                            if p.type_constraint.is_empty() {
                                p.name.clone()
                            } else {
                                format!("{} {}", p.name, p.type_constraint)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let ret = method.effective_return_type().unwrap_or("Any");
                    return Some(format!(
                        "```razomy\n{}.{}({}) -> {}\n```",
                        hf.name, method.name, params_str, ret
                    ));
                }
            }
        }

        // 3. Проверяем локальную переменную или параметр в текущем скоупе (или глобальную привязку)
        let local_or_global_ty = scope
            .and_then(|s| s.var_types.get(clean).cloned())
            .or_else(|| {
                if !self.hir.forms.contains_key(clean) {
                    self.hir.globals.get(clean).map(|g| g.inferred_type.clone())
                } else {
                    None
                }
            })
            .or_else(|| {
                if !self.hir.forms.contains_key(clean) {
                    self.scopes
                        .values()
                        .find_map(|s| s.var_types.get(clean).cloned())
                } else {
                    None
                }
            });

        if let Some((hf, _)) = self.resolve_target_form_in_scope(clean, scope) {
            let header = if let Some(var_ty) = &local_or_global_ty {
                format!(
                    "```razomy\n{} : {}\nForm {} {{ {} fields, {} methods }}\n```\n",
                    clean,
                    var_ty,
                    hf.name,
                    hf.field_order.len(),
                    hf.method_order.len()
                )
            } else {
                format!(
                    "```razomy\nForm {} {{ {} fields, {} methods }}\n```\n",
                    hf.name,
                    hf.field_order.len(),
                    hf.method_order.len()
                )
            };
            let mut md = header;
            if !hf.ancestors.is_empty() {
                let lineage_str = if !hf.type_args.is_empty() && hf.ancestors.len() == 1 {
                    format!("{}({})", hf.ancestors[0], hf.type_args.join(", "))
                } else {
                    hf.ancestors.join(" & ")
                };
                md.push_str(&format!("**Lineage (Предки):** `{lineage_str}`\n\n"));
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

        if let Some(var_ty) = local_or_global_ty {
            return Some(format!("```razomy\n{} : {}\n```\n", clean, var_ty));
        }

        None
    }

    /// Закон 4 (UFCS Autocomplete): по `Target.` выдает поля, собственные методы, предков Lineage
    /// И все внешние функции в области видимости, первый параметр которых совместим с `Target`!
    pub fn completions(&self, query: &str) -> Vec<CompletionItem> {
        self.completions_at(query, None)
    }

    pub fn completions_at(&self, query: &str, sentence_idx: Option<usize>) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen = HashSet::new();
        let scope = self.scope_for_sentence(sentence_idx);

        if let Some((base_raw, prefix)) = query.rsplit_once('.') {
            let base = base_raw.trim();
            if let Some((hf, target_type)) = self.resolve_target_form_in_scope(base, scope) {
                for fname in &hf.field_order {
                    if fname.starts_with(prefix) && seen.insert(fname.clone()) {
                        let f = &hf.fields[fname];
                        items.push(CompletionItem {
                            label: fname.clone(),
                            kind: if f.is_reactive {
                                "ReactiveField"
                            } else {
                                "Field"
                            },
                            detail: format!(
                                "{}: {} (from {})",
                                fname, f.inferred_type, f.origin_form
                            ),
                            insert_text: fname.clone(),
                        });
                    }
                }
                for m in &hf.method_order {
                    if m.starts_with(prefix) && seen.insert(m.clone()) {
                        let ret_detail = hf
                            .methods
                            .get(m)
                            .and_then(|mf| mf.effective_return_type())
                            .unwrap_or("Any");
                        let has_params = hf
                            .methods
                            .get(m)
                            .map(|mf| mf.params.iter().any(|p| !p.is_type_param))
                            .unwrap_or(false);
                        items.push(CompletionItem {
                            label: m.clone(),
                            kind: "Method",
                            detail: format!("{}.{}(...) -> {}", hf.name, m, ret_detail),
                            insert_text: if has_params {
                                m.clone()
                            } else {
                                format!("{m}()")
                            },
                        });
                    }
                }
                for sub_name in hf.submodules.keys() {
                    if sub_name.starts_with(prefix) && seen.insert(sub_name.clone()) {
                        items.push(CompletionItem {
                            label: sub_name.clone(),
                            kind: "Form",
                            detail: format!("Submodule {}.{}", hf.name, sub_name),
                            insert_text: sub_name.clone(),
                        });
                    }
                }
                for anc in &hf.ancestors {
                    if anc.starts_with(prefix) && seen.insert(anc.clone()) {
                        items.push(CompletionItem {
                            label: anc.clone(),
                            kind: "AncestorSlice",
                            detail: format!("Lineage ancestor of {}", hf.name),
                            insert_text: anc.clone(),
                        });
                    }
                }
                for fn_name in &self.hir.function_order {
                    if fn_name.starts_with(prefix) {
                        if let Some(fn_hir) = self.hir.functions.get(fn_name) {
                            if fn_hir.is_prelude {
                                continue;
                            }
                            if let Some(first_param) =
                                fn_hir.params.iter().find(|p| !p.is_type_param)
                            {
                                if is_ufcs_compatible(
                                    &first_param.type_constraint,
                                    &target_type,
                                    &hf,
                                ) && seen.insert(fn_name.clone())
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

            // Примитивные типы, строки и коллекции (String, Tuple, List, Range, Int, Float)
            if let Some(resolved_ty) = self.resolve_expr_type(base, scope) {
                let base_ty = resolved_ty.split('(').next().unwrap_or(&resolved_ty).trim();
                let is_str = base_ty == "String"
                    || self
                        .hir
                        .forms
                        .get(base_ty)
                        .map_or(false, |f| f.ancestors.contains(&"String".to_string()));
                let is_col = matches!(base_ty, "Tuple" | "List" | "Range" | "Cons")
                    || self.hir.forms.get(base_ty).map_or(false, |f| {
                        f.ancestors
                            .iter()
                            .any(|a| matches!(a.as_str(), "Tuple" | "List" | "Range" | "Cons"))
                    });
                let is_num = matches!(base_ty, "Int" | "Float")
                    || self.hir.forms.get(base_ty).map_or(false, |f| {
                        f.ancestors
                            .iter()
                            .any(|a| matches!(a.as_str(), "Int" | "Float"))
                    });

                if is_str {
                    for s_fn in ["trim", "to_upper", "split", "length"] {
                        if s_fn.starts_with(prefix) && seen.insert(s_fn.to_string()) {
                            if let Some(f) = self.hir.functions.get(s_fn) {
                                items.push(CompletionItem {
                                    label: s_fn.to_string(),
                                    kind: "UFCSFunction",
                                    detail: format_fn_signature(f),
                                    insert_text: s_fn.to_string(),
                                });
                            }
                        }
                    }
                }
                if is_col {
                    for c_fn in [
                        "map",
                        "filter",
                        "fold",
                        "flat_map",
                        "zip",
                        "enumerate",
                        "for_each",
                        "find",
                        "take_while",
                        "append",
                        "contains",
                        "join",
                        "length",
                    ] {
                        if c_fn.starts_with(prefix) && seen.insert(c_fn.to_string()) {
                            if let Some(f) = self.hir.functions.get(c_fn) {
                                items.push(CompletionItem {
                                    label: c_fn.to_string(),
                                    kind: "UFCSFunction",
                                    detail: format_fn_signature(f),
                                    insert_text: c_fn.to_string(),
                                });
                            }
                        }
                    }
                }
                if is_num {
                    for (prop, detail) in [
                        ("to_float", "Convert to Float"),
                        ("round_to_int", "Round Float to Int"),
                        ("bit_and", "bit_and(a Int, b Int) Int"),
                        ("bit_or", "bit_or(a Int, b Int) Int"),
                        ("bit_xor", "bit_xor(a Int, b Int) Int"),
                    ] {
                        if prop.starts_with(prefix) && seen.insert(prop.to_string()) {
                            items.push(CompletionItem {
                                label: prop.to_string(),
                                kind: "Method",
                                detail: detail.to_string(),
                                insert_text: prop.to_string(),
                            });
                        }
                    }
                }
                for fn_name in &self.hir.function_order {
                    if fn_name.starts_with(prefix) {
                        if let Some(fn_hir) = self.hir.functions.get(fn_name) {
                            if fn_hir.is_prelude {
                                continue;
                            }
                            if let Some(first_param) =
                                fn_hir.params.iter().find(|p| !p.is_type_param)
                            {
                                let c_base = first_param
                                    .type_constraint
                                    .split('(')
                                    .next()
                                    .unwrap_or(&first_param.type_constraint)
                                    .trim();
                                if (c_base == base_ty
                                    || (is_col && matches!(c_base, "Tuple" | "List")))
                                    && seen.insert(fn_name.clone())
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
            // 1. Локальные переменные и параметры текущего скоупа
            if let Some(sc) = scope {
                let mut local_names: Vec<_> = sc.var_types.keys().collect();
                local_names.sort();
                for vname in local_names {
                    if vname.starts_with(query) && seen.insert(vname.clone()) {
                        let vty = &sc.var_types[vname];
                        items.push(CompletionItem {
                            label: vname.clone(),
                            kind: "Variable",
                            detail: format!("{vname} : {vty}"),
                            insert_text: vname.clone(),
                        });
                    }
                }
            }

            // 2. Формы, функции и глобальные привязки
            for name in &self.hir.form_order {
                if is_pascal_case(name) && name.starts_with(query) && seen.insert(name.clone()) {
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
                if name.starts_with(query) && seen.insert(name.clone()) {
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
                if name.starts_with(query)
                    && !self.hir.forms.contains_key(name)
                    && seen.insert(name.clone())
                {
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

fn is_ufcs_compatible(param_constraint: &str, target_type: &str, hf: &HirForm) -> bool {
    if param_constraint.is_empty() {
        return false;
    }
    let clean = param_constraint.trim();
    if clean == target_type || hf.ancestors.contains(&clean.to_string()) {
        return true;
    }
    let base_constraint = clean.split_whitespace().next().unwrap_or("");
    let base_no_generic = base_constraint.split('(').next().unwrap_or(base_constraint);
    if base_no_generic == target_type || hf.ancestors.contains(&base_no_generic.to_string()) {
        return true;
    }
    if clean.contains('|') {
        for branch in clean.split('|').map(str::trim) {
            if branch == target_type || hf.ancestors.contains(&branch.to_string()) {
                return true;
            }
        }
    }
    false
}

pub fn extract_collection_element_type(col_ty: &str, tuple_idx: i32) -> Option<String> {
    let trimmed = col_ty.trim();
    if let Some(inner) = trimmed.strip_prefix("Tuple(").and_then(|s| s.strip_suffix(')')) {
        let elems: Vec<&str> = inner.split(',').map(str::trim).collect();
        if elems.len() == 1 {
            return Some(elems[0].to_string());
        }
        let i = if tuple_idx < 0 {
            (elems.len() as i32 + tuple_idx) as usize
        } else {
            tuple_idx as usize
        };
        return elems
            .get(i)
            .or_else(|| elems.first())
            .map(|s| (*s).to_string());
    }
    for prefix in ["List(", "Option(", "Some(", "Cons(", "Set("] {
        if let Some(inner) = trimmed.strip_prefix(prefix).and_then(|s| s.strip_suffix(')')) {
            return Some(inner.trim().to_string());
        }
    }
    if trimmed == "Range" {
        return Some("Int".into());
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner = &trimmed[1..trimmed.len() - 1];
        let elems: Vec<&str> = inner.split(',').map(str::trim).collect();
        let i = if tuple_idx < 0 {
            (elems.len() as i32 + tuple_idx) as usize
        } else {
            tuple_idx as usize
        };
        return elems.get(i).map(|s| (*s).to_string());
    }
    None
}

pub fn analyze_source(source: &str) -> AnalysisReport {
    let mut report = AnalysisReport::default();
    match parse_code(source) {
        Ok(sentences) => analyze_from_hir(lower_to_hir(&sentences), &mut report),
        Err(err) => {
            report.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                code: "E00_SYNTAX",
                message: format!("Синтаксическая ошибка парсера:\n{err}"),
                context_symbol: None,
                sentence_index: None,
                highlight_token: None,
            });
            let recovered = recover_source_for_ide(source);
            if let Ok(sentences) = parse_code(&recovered) {
                analyze_from_hir(lower_to_hir(&sentences), &mut report);
            }
        }
    }
    report
}

pub fn analyze_source_with_path(
    source: &str,
    file_path: Option<&std::path::Path>,
) -> AnalysisReport {
    analyze_source_with_path_and_memory(source, file_path, &HashMap::new())
}

pub fn analyze_source_with_path_and_memory(
    source: &str,
    file_path: Option<&std::path::Path>,
    in_memory_files: &HashMap<std::path::PathBuf, String>,
) -> AnalysisReport {
    let mut report = AnalysisReport::default();
    let base_dir = file_path.and_then(|p| p.parent());

    if let Err(err) = parse_code(source) {
        report.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: "E00_SYNTAX",
            message: format!("Синтаксическая ошибка парсера:\n{err}"),
            context_symbol: None,
            sentence_index: None,
            highlight_token: None,
        });
    }

    match parse_located_with_imports_and_memory(source, base_dir, in_memory_files) {
        Ok(located) => {
            let hir = lower_to_hir_located(&located, base_dir, in_memory_files);
            analyze_from_hir(hir, &mut report);
        }
        Err(err) => {
            if report.diagnostics.is_empty() {
                report.diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    code: "E00_SYNTAX",
                    message: err,
                    context_symbol: None,
                    sentence_index: None,
                    highlight_token: None,
                });
            }
            let recovered = recover_source_for_ide(source);
            if let Ok(sentences) = parse_code(&recovered) {
                analyze_from_hir(lower_to_hir(&sentences), &mut report);
            }
        }
    }

    report
}

pub fn analyze_from_hir(hir: HirModule, report: &mut AnalysisReport) {
    report.hir = hir;

    // 0. Проверка дубликатов функций и форм (E10_DUPLICATE_DEFINITION)
    let duplicates = report.hir.duplicates.clone();
    for dup in duplicates {
        let kind_ru = if dup.kind == "Function" {
            "Функция"
        } else {
            "Форма"
        };
        report.push_diag_with_token(
            Severity::Error,
            "E10_DUPLICATE_DEFINITION",
            format!(
                "Ошибка E10 (#{}): {kind_ru} с именем '{}' уже объявлена ранее (предложение #{}).",
                dup.duplicate_origin.sentence_index, dup.name, dup.first_sentence_index
            ),
            Some(&dup.name),
            dup.duplicate_origin.sentence_index,
            Some(&dup.name),
        );
    }

    // 1. Формы + проверка Закона 1c (E02, E03) и Закона 2 (I002_LAW2_COLLISION) + регистрация скоупов форм
    let form_order = report.hir.form_order.clone();
    for form_name in &form_order {
        if let Some(hf) = report.hir.forms.get(form_name).cloned() {
            if !hf.is_prelude && hf.origin.file_path.is_none() {
                verify_form_slot_rules(&hf, report);

                let mut form_scope = ScopeSnapshot {
                    context_name: form_name.clone(),
                    enclosing_form: Some(form_name.clone()),
                    sentence_index: hf.origin.sentence_index,
                    var_types: HashMap::new(),
                };
                form_scope
                    .var_types
                    .insert("self".into(), form_name.clone());
                form_scope
                    .var_types
                    .insert("Self".into(), form_name.clone());
                for vp in &hf.value_params {
                    if !vp.type_constraint.is_empty() {
                        form_scope
                            .var_types
                            .insert(vp.name.clone(), vp.type_constraint.clone());
                    }
                }
                for fname in &hf.field_order {
                    if let Some(field) = hf.fields.get(fname) {
                        form_scope
                            .var_types
                            .insert(fname.clone(), field.inferred_type.clone());
                    }
                }
                report.scopes.insert(form_name.clone(), form_scope.clone());
                report
                    .sentence_scopes
                    .insert(hf.origin.sentence_index, form_scope);
            }
        }
    }

    // 2. Глобальные привязки + проверка E07_STATIC_ASSERT_FAILED для `Compile & [...]`
    let global_order = report.hir.global_order.clone();
    for g_name in &global_order {
        if let Some(g) = report.hir.globals.get(g_name).cloned() {
            if g.origin.file_path.is_none() {
                verify_compile_time_assertion(
                    g_name,
                    &g.value_expr,
                    g.origin.sentence_index,
                    report,
                );
            }
        }
    }

    // 3. Верификация параметров и тел функций и методов (Законы 1–3, E01..E09)
    let function_order = report.hir.function_order.clone();
    for fn_name in &function_order {
        if let Some(hir_fn) = report.hir.functions.get(fn_name).cloned() {
            if hir_fn.is_prelude || hir_fn.origin.file_path.is_some() {
                continue;
            }
            verify_function_slots(
                &hir_fn.name,
                &hir_fn.params,
                None,
                hir_fn.origin.sentence_index,
                report,
            );
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
            if hf.is_prelude || hf.origin.file_path.is_some() {
                continue;
            }
            for m_name in &hf.method_order {
                if let Some(m_fn) = hf.methods.get(m_name) {
                    let full_name = format!("{form_name}.{m_name}");
                    verify_function_slots(
                        &full_name,
                        &m_fn.params,
                        Some(&hf),
                        hf.origin.sentence_index,
                        report,
                    );
                    verify_function_body(
                        &full_name,
                        &m_fn.params,
                        m_fn.auto_unpacks,
                        &m_fn.body,
                        Some(&hf),
                        hf.origin.sentence_index,
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
        if crate::hir::is_type_hole(base_tp) {
            continue;
        }
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

    // Проверка существования предков формы (например, `Cat User`, когда `User` переименовали в `User2`)
    for anc in &hf.ancestors {
        let base_anc = anc.split('(').next().unwrap_or(anc).trim();
        if is_pascal_case(base_anc)
            && !crate::hir::is_type_hole(base_anc)
            && !is_known_type(base_anc, &report.hir)
            && !hf.type_params.iter().any(|tp| tp.starts_with(base_anc))
        {
            let hint = closest_member_hint(base_anc, &report.hir.form_order);
            report.push_diag_with_token(
                Severity::Error,
                "E11_UNKNOWN_TYPE",
                format!(
                    "Ошибка E11 (#{}): Базовый тип или форма '{base_anc}' для '{}' не найдена в области видимости.{hint}",
                    hf.origin.sentence_index, hf.name
                ),
                Some(&hf.name),
                hf.origin.sentence_index,
                Some(base_anc),
            );
        }
    }
}

fn extract_type_names_from_constraint(constraint: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for raw_part in constraint.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '(' | ')' | '[' | ']' | '{' | '}' | ',' | '&' | '|' | '/' | '!'
            )
    }) {
        let part = raw_part.trim();
        if part.is_empty()
            || part.starts_with('\'')
            || part.starts_with('"')
            || crate::hir::is_type_hole(part)
        {
            continue;
        }
        let base = part.split('.').next().unwrap_or(part);
        if is_pascal_case(base) {
            out.push(base);
        }
    }
    out
}

/// Проверка `E02_DUPLICATE_UNNAMED_TYPE` в слотах функций (`transfer_bad(Account, Account, Money)` или `finalize(User & Cart)`)
fn verify_function_slots(
    fn_name: &str,
    params: &[HirParam],
    enclosing_form: Option<&HirForm>,
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    let mut seen_anon_types = HashSet::new();
    let mut type_param_names: HashSet<&str> = params
        .iter()
        .filter(|p| p.is_type_param)
        .map(|p| p.name.as_str())
        .collect();

    if let Some(hf) = enclosing_form {
        for tp in &hf.type_params {
            let clean_tp = tp
                .split(|c: char| c.is_whitespace() || c == '(')
                .next()
                .unwrap_or(tp)
                .trim();
            if !clean_tp.is_empty() {
                type_param_names.insert(clean_tp);
            }
        }
    }

    let is_compile_macro = report
        .hir
        .functions
        .get(fn_name)
        .map_or(false, |f| f.is_compile_time);

    for p in params {
        if p.is_type_param {
            continue;
        }
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

        if !is_compile_macro && !p.type_constraint.is_empty() {
            for base_ty in extract_type_names_from_constraint(&p.type_constraint) {
                if !is_known_type(base_ty, &report.hir) && !type_param_names.contains(base_ty) {
                    let hint = closest_member_hint(base_ty, &report.hir.form_order);
                    report.push_diag_with_token(
                        Severity::Error,
                        "E11_UNKNOWN_TYPE",
                        format!(
                            "Ошибка E11 (#{sentence_idx}): Тип '{base_ty}' в параметре функции '{fn_name}' не найден в области видимости.{hint}"
                        ),
                        Some(fn_name),
                        sentence_idx,
                        Some(base_ty),
                    );
                }
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
                        if p == "Compile.error" || p == "Meta.error" {
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
    enclosing_form: Option<&HirForm>,
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

    if let Some(hf) = enclosing_form {
        var_types.insert("self".into(), hf.name.clone());
        var_types.insert("Self".into(), hf.name.clone());
        // Law 1c: Enclosing Form `FormName` auto-projects its `snake_case` name `form_name`!
        let form_snake = pascal_to_snake_case(&hf.name);
        local_vars.insert(form_snake.clone());
        var_types.insert(form_snake, hf.name.clone());

        for vp in &hf.value_params {
            local_vars.insert(vp.name.clone());
            if !vp.type_constraint.is_empty() {
                var_types.insert(vp.name.clone(), vp.type_constraint.clone());
            }
        }
        for fname in &hf.field_order {
            local_vars.insert(fname.clone());
            if let Some(f) = hf.fields.get(fname) {
                var_types.insert(fname.clone(), f.inferred_type.clone());
            }
        }
    }

    for p in params {
        local_vars.insert(p.name.clone());
        if !p.type_constraint.is_empty() && !p.is_type_param {
            var_types.insert(p.name.clone(), p.type_constraint.clone());
            if !var_types.contains_key("_") {
                var_types.insert("_".into(), p.type_constraint.clone());
                var_types.insert("_0".into(), p.type_constraint.clone());
            }
        }
        let base_param_form = report.resolve_type_to_form(&p.type_constraint);
        for df in &p.destructured_fields {
            local_vars.insert(df.clone());
            if let Some(hf) = &base_param_form {
                if let Some(field) = hf.fields.get(df) {
                    var_types.insert(df.clone(), field.inferred_type.clone());
                }
            }
        }

        if p.is_anonymous_type_slot {
            if let Some(hf) = &base_param_form {
                for f in &hf.field_order {
                    local_vars.insert(f.clone());
                    if let Some(field) = hf.fields.get(f) {
                        var_types.insert(f.clone(), field.inferred_type.clone());
                    }
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
                if let [Token::Path(form_name), Token::Op(op), Token::Group('{', field_sents), ..] =
                    expr.as_slice()
                {
                    if op == "&" && is_pascal_case(form_name) {
                        verify_struct_init_fields(
                            fn_name,
                            form_name,
                            field_sents,
                            sentence_idx,
                            report,
                        );
                    }
                }
                let inferred = infer_local_binding_type(&expr, &var_types, report)
                    .unwrap_or_else(|| {
                        expr.first()
                            .and_then(|t| match t {
                                Token::Path(b) => Some(b.clone()),
                                _ => None,
                            })
                            .unwrap_or_else(|| "Any".into())
                    });
                var_types.insert(name.clone(), inferred);
                local_vars.insert(name);
            }
            SemanticKind::Binding { key, value } => {
                // 1. Проверка `E01_AMBIGUOUS_CALL_OR_BIND`: `area (Circle { radius 5.0 })` (пробел перед `(` у имени функции!)
                if matches!(value.as_slice(), [Token::Group('(', _)]) && !local_vars.contains(&key)
                {
                    if let Some(f) = report.hir.functions.get(&key) {
                        if !f.params.is_empty() && !f.is_prelude {
                            report.push_diag_with_token(
                                Severity::Error,
                                "E01_AMBIGUOUS_CALL_OR_BIND",
                                format!(
                                    "Ошибка E01 (#{sentence_idx}): '{key}' является функцией в области видимости. Нельзя вызывать функцию с пробелом перед '(' или перепривязывать её имя."
                                ),
                                Some(fn_name),
                                sentence_idx,
                                Some(&key),
                            );
                        }
                    }
                }

                // 2. Проверка `E06_IMMUTABLE_TYPE_BINDING`: попытка мутировать `PascalCase` форму напрямую (`Car.max_speed 500`)
                let root_ident = key.split('.').next().unwrap_or(&key);
                if is_pascal_case(root_ident)
                    && (key.contains('.') || report.hir.forms.contains_key(root_ident))
                {
                    report.push_diag_with_token(
                        Severity::Error,
                        "E06_IMMUTABLE_TYPE_BINDING",
                        format!(
                            "Ошибка E06 (#{sentence_idx}): '{root_ident}' является неизменяемым PascalCase прототипом/типом формы. Создайте snake_case экземпляр перед мутацией."
                        ),
                        Some(fn_name),
                        sentence_idx,
                        Some(&key),
                    );
                }

                // 3. Проверка точечной мутации `my_car.unknown_field 123` на существование поля
                if key.contains('.') && !is_pascal_case(root_ident) {
                    verify_identifier(
                        &key,
                        false,
                        fn_name,
                        &local_vars,
                        &var_types,
                        &shadowed_named_fields,
                        &ambiguous_fields,
                        sentence_idx,
                        report,
                    );
                }

                if let Some(inferred) = infer_local_binding_type(&value, &var_types, report) {
                    if !key.contains('.') {
                        var_types.insert(key.clone(), inferred);
                    }
                }
                local_vars.insert(key);
            }
            SemanticKind::TupleBinding { tuple, value } => {
                let rhs_ty = infer_local_binding_type(&value, &var_types, report)
                    .unwrap_or_else(|| "Tuple".into());
                let mut rest_count = 0usize;
                for (i, t) in tuple.iter().enumerate() {
                    match t.as_slice() {
                        [Token::Path(v)] => {
                            local_vars.insert(v.clone());
                            if let Some(elem_ty) = extract_collection_element_type(&rhs_ty, i as i32)
                            {
                                var_types.insert(v.clone(), elem_ty);
                            }
                        }
                        [Token::Op(op), Token::Path(rest_var)] if op == ".." => {
                            rest_count += 1;
                            local_vars.insert(rest_var.clone());
                            var_types.insert(rest_var.clone(), rhs_ty.clone());
                        }
                        _ => {}
                    }
                }
                if rest_count > 1 {
                    report.push_diag(
                        Severity::Error,
                        "E10_MULTIPLE_REST_PATTERNS",
                        format!(
                            "Ошибка E10 (#{sentence_idx}): В деструктуризации слева не может быть более одного '..rest' паттерна."
                        ),
                        Some(fn_name),
                        sentence_idx,
                    );
                }
            }
            _ => {
                // Деструктуризация формы: `{ name, ..other_fields } user_form`
                if let [Token::Group('{', field_sents), rest @ ..] = sent.as_slice() {
                    if !rest.is_empty() && !has_pipe_op(rest) {
                        for fs in field_sents {
                            match fs.as_slice() {
                                [Token::Path(v)] | [Token::Op(_), Token::Path(v)] => {
                                    local_vars.insert(v.clone());
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
        check_tokens(
            fn_name,
            sent,
            &local_vars,
            &mut var_types,
            &shadowed_named_fields,
            &ambiguous_fields,
            sentence_idx,
            report,
        );
    }

    // Сохраняем собранный снимок скоупа функции/метода для IDE (Hover, Completion, GoToDef, InlayHints)
    let snapshot = ScopeSnapshot {
        context_name: fn_name.to_string(),
        enclosing_form: enclosing_form.map(|f| f.name.clone()),
        sentence_index: sentence_idx,
        var_types: var_types.clone(),
    };
    report.scopes.insert(fn_name.to_string(), snapshot.clone());
    report
        .sentence_scopes
        .entry(sentence_idx)
        .and_modify(|existing| {
            existing.var_types.extend(var_types.clone());
        })
        .or_insert(snapshot);
}

fn infer_local_binding_type(
    value: &[Token],
    var_types: &HashMap<String, String>,
    report: &AnalysisReport,
) -> Option<String> {
    // Typed Local Binding: `u1 User (uid1 . Db.find_user)`
    if let [Token::Path(explicit_ty), rest @ ..] = value {
        if is_pascal_case(explicit_ty)
            && !rest.is_empty()
            && !matches!(
                rest.first(),
                Some(Token::Op(_) | Token::Group(')' | '{', _))
            )
        {
            return Some(explicit_ty.clone());
        }
    }
    let inner = match value {
        [Token::Group('(', sents)] if sents.len() == 1 => sents[0].as_slice(),
        other => other,
    };
    match inner {
        [Token::Path(form_name), Token::Group('{', _), ..] => Some(form_name.clone()),
        [Token::Path(src_var), Token::Op(op), Token::Group('{', _), ..] if op == "&" => {
            var_types
                .get(src_var)
                .cloned()
                .or_else(|| Some(src_var.clone()))
        }
        [Token::Path(src_var), Token::Op(op), Token::Path(rhs), Token::Group('{', _), ..]
            if op == "&" =>
        {
            let base = var_types
                .get(src_var)
                .cloned()
                .unwrap_or_else(|| src_var.clone());
            Some(format!("{base} & {rhs}"))
        }
        [Token::Path(src_var), Token::Op(op), Token::Path(sub_target)] if op == "/" => {
            let base_ty = var_types.get(src_var)?;
            Some(format!("{base_ty} / {sub_target}"))
        }
        _ => {
            let ty = infer_hir_type_with_locals(inner, var_types, &report.hir);
            if matches!(ty.as_str(), "" | "Any" | "_") || ty.starts_with("_.") {
                None
            } else {
                Some(ty)
            }
        }
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
    //    и на обращение к несуществующему полю `u.unknown_field` (`E08_UNKNOWN_MEMBER`)
    if let Some((obj_var, field_part)) = ident.split_once('.') {
        if !field_part.contains('.') && !field_part.ends_with("()") {
            if let Some(ty_expr) = var_types.get(obj_var) {
                if let Some(conflicts) = check_type_expr_field_ambiguity(ty_expr, field_part, report)
                {
                    report.push_diag_with_token(
                        Severity::Error,
                        "E05_AMBIGUOUS_LINEAGE_FIELD",
                        format!(
                            "Нарушение Закона 2 (#{sentence_idx}): Поле '{field_part}' у переменной '{obj_var}' ({ty_expr}) неоднозначно (присутствует в {}). Используйте явный путь предка ({obj_var}.{}.{field_part}).",
                            conflicts.join(" и "),
                            conflicts.first().cloned().unwrap_or_default()
                        ),
                        Some(fn_name),
                        sentence_idx,
                        Some(ident),
                    );
                    return;
                }

                // Проверка E08_UNKNOWN_MEMBER для snake_case экземпляров конкретных форм
                if !is_pascal_case(obj_var)
                    && !matches!(obj_var, "_" | "_0" | "_1" | "_2" | "self" | "Self")
                    && field_part.parse::<i32>().is_err()
                    && !matches!(
                        field_part,
                        "length"
                            | "to_float"
                            | "round_to_int"
                            | "value"
                            | "read"
                            | "get"
                            | "set"
                            | "swap"
                            | "send"
                            | "recv"
                            | "to_json"
                    )
                    && !report.hir.functions.contains_key(field_part)
                    && !report.hir.forms.contains_key(field_part)
                {
                    if let Some(hf) = report.hir.forms.get(ty_expr.as_str()) {
                        let is_concrete_struct = (!hf.fields.is_empty() || !hf.methods.is_empty())
                            && hf.union_variants.is_empty()
                            && !hf.fields.keys().any(|k| k.starts_with('('))
                            && !matches!(hf.name.as_str(), "Form" | "Any" | "Tuple" | "Stream");
                        if is_concrete_struct
                            && !hf.fields.contains_key(field_part)
                            && !hf.methods.contains_key(field_part)
                            && !hf.ancestors.contains(&field_part.to_string())
                            && !hf.submodules.contains_key(field_part)
                        {
                            let hint = closest_member_hint(field_part, &hf.field_order);
                            report.push_diag_with_token(
                                Severity::Error,
                                "E08_UNKNOWN_MEMBER",
                                format!(
                                    "Ошибка E08 (#{sentence_idx}): Поле или метод '{field_part}' не существует в форме '{ty_expr}' (переменная '{obj_var}').{hint}"
                                ),
                                Some(fn_name),
                                sentence_idx,
                                Some(ident),
                            );
                        }
                    }
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
        report.push_diag_with_token(
            Severity::Error,
            "E05_AMBIGUOUS_LINEAGE_FIELD",
            format!(
                "Нарушение Закона 2 (#{sentence_idx}): В '{fn_name}' поле '{ident}' неоднозначно (предков: {}). Используйте {suggestion}.",
                origins.join(" & ")
            ),
            Some(fn_name),
            sentence_idx,
            Some(ident),
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
        report.push_diag_with_token(
            Severity::Warning,
            code,
            format!(
                "Закон 1 (#{sentence_idx}): В '{fn_name}' параметр '{param_var} {param_type}' именованный. Используйте '{fix}'."
            ),
            Some(fn_name),
            sentence_idx,
            Some(ident),
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
fn closest_member_hint(typo: &str, candidates: &[String]) -> String {
    let mut best: Option<(&str, usize)> = None;
    for cand in candidates {
        let dist = edit_distance(typo, cand);
        if dist <= 2 && best.map_or(true, |(_, d)| dist < d) {
            best = Some((cand.as_str(), dist));
        }
    }
    if let Some((cand, _)) = best {
        format!(" Возможно, вы имели в виду '{cand}'?")
    } else {
        String::new()
    }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let ac: Vec<char> = a.chars().collect();
    let bc: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; bc.len() + 1]; ac.len() + 1];
    for i in 0..=ac.len() {
        dp[i][0] = i;
    }
    for j in 0..=bc.len() {
        dp[0][j] = j;
    }
    for i in 1..=ac.len() {
        for j in 1..=bc.len() {
            let cost = usize::from(ac[i - 1] != bc[j - 1]);
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[ac.len()][bc.len()]
}

fn verify_struct_init_fields(
    fn_name: &str,
    form_name: &str,
    field_sents: &[Sentence],
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    let is_compile_macro = report
        .hir
        .functions
        .get(fn_name)
        .map_or(false, |f| f.is_compile_time);

    if form_name.contains('.') {
        return;
    }

    let Some(hf) = report.hir.forms.get(form_name).cloned() else {
        if !is_compile_macro {
            let hint = closest_member_hint(form_name, &report.hir.form_order);
            report.push_diag_with_token(
                Severity::Error,
                "E11_UNKNOWN_TYPE",
                format!(
                    "Ошибка E11 (#{sentence_idx}): Форма '{form_name}' не найдена в области видимости.{hint}"
                ),
                Some(fn_name),
                sentence_idx,
                Some(form_name),
            );
        }
        return;
    };
    if hf.fields.is_empty() || hf.fields.keys().any(|k| k.starts_with('(')) {
        return;
    }

    for fs in field_sents {
        if let SemanticKind::Binding { key, value } = classify_sentence(fs) {
            if let Some(field) = hf.fields.get(&key) {
                if let [Token::Lit(lit)] = value.as_slice() {
                    let lit_ty = match lit {
                        Literal::Int(_) => "Int",
                        Literal::Float(_) => "Float",
                        Literal::Bool(_) => "Bool",
                        Literal::Byte(_) => "Byte",
                        Literal::String(_) | Literal::RawString(_) => "String",
                        Literal::Symbol(_) => "Symbol",
                    };
                    let expected = field.inferred_type.as_str();
                    let mismatch = match expected {
                        "Int" => !matches!(lit_ty, "Int" | "Byte"),
                        "Float" => !matches!(lit_ty, "Float" | "Int"),
                        "Bool" => lit_ty != "Bool",
                        "String" => lit_ty != "String",
                        "Symbol" => lit_ty != "Symbol",
                        _ => false,
                    };
                    if mismatch {
                        report.push_diag_with_token(
                            Severity::Error,
                            "E09_TYPE_MISMATCH",
                            format!(
                                "Ошибка типа E09 (#{sentence_idx}): Поле '{form_name}.{key}' ожидает тип '{expected}', но передан литерал типа '{lit_ty}'."
                            ),
                            Some(fn_name),
                            sentence_idx,
                            Some(&key),
                        );
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_tokens(
    fn_name: &str,
    tokens: &[Token],
    local_vars: &HashSet<String>,
    var_types: &mut HashMap<String, String>,
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
            if let [Token::Path(target_fn), ..] = step.expr.as_slice() {
                if let Some(in_type) = &current_type {
                    if step.op == PipeOp::Dot {
                        verify_typestate_and_union_bypass(
                            fn_name,
                            in_type,
                            target_fn,
                            sentence_idx,
                            report,
                        );
                    }
                }
                if let Some(fn_hir) = report.hir.functions.get(target_fn) {
                    if let Some(ret) = fn_hir.effective_return_type() {
                        if step.op == PipeOp::Enrich {
                            if let Some(cur) = &current_type {
                                current_type = Some(enrich_type_expression(cur, ret, report));
                            }
                        } else if step.op != PipeOp::Tap {
                            current_type = Some(ret.to_string());
                        }
                    }
                } else if let Some((owner, mname)) = target_fn.rsplit_once('.') {
                    let owner_ty = var_types
                        .get(owner)
                        .cloned()
                        .unwrap_or_else(|| owner.to_string());
                    if let Some(hf) = report.resolve_type_to_form(&owner_ty) {
                        if let Some(m) = hf.methods.get(mname) {
                            if let Some(ret) = m.effective_return_type() {
                                if step.op == PipeOp::Enrich {
                                    if let Some(cur) = &current_type {
                                        current_type = Some(enrich_type_expression(cur, ret, report));
                                    }
                                } else if step.op != PipeOp::Tap {
                                    current_type = Some(ret.to_string());
                                }
                            }
                        }
                    }
                }
            }

            if let Some(in_ty) = &current_type {
                let elem_ty = extract_collection_element_type(in_ty, 0)
                    .unwrap_or_else(|| in_ty.clone());
                var_types.entry("_1".into()).or_insert_with(|| elem_ty.clone());
                var_types.entry("_2".into()).or_insert_with(|| elem_ty);
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
                        SemanticKind::LambdaOrTupleBranch { args, body } => {
                            let mut branch_locals = local_vars.clone();
                            for a in &args {
                                if let [Token::Path(p)] = a.as_slice() {
                                    if is_pascal_case(p) {
                                        let snake = pascal_to_snake_case(p);
                                        branch_locals.insert(snake.clone());
                                        var_types.insert(snake, p.clone());
                                    } else if p != "_" {
                                        branch_locals.insert(p.clone());
                                    }
                                }
                            }
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
                            let mut branch_ambiguous = ambiguous.clone();
                            if is_pascal_case(&name) {
                                let snake = pascal_to_snake_case(&name);
                                branch_locals.insert(snake.clone());
                                let full_branch_type = if let Some(in_ty) = &current_type {
                                    if let Some(inner) = extract_collection_element_type(in_ty, 0) {
                                        if report
                                            .hir
                                            .forms
                                            .get(&name)
                                            .map_or(false, |f| !f.type_params.is_empty())
                                        {
                                            format!("{name}({inner})")
                                        } else {
                                            name.clone()
                                        }
                                    } else {
                                        name.clone()
                                    }
                                } else {
                                    name.clone()
                                };
                                var_types.insert(snake, full_branch_type);
                            }
                            collect_ambiguous_fields(&name, &report.hir, &mut branch_ambiguous);
                            for s in &stream {
                                check_tokens(
                                    fn_name,
                                    s,
                                    &branch_locals,
                                    var_types,
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
                // Если шаг является вызовом с лямбдой `map((item Item) [ ... ])`, регистрируем параметры лямбды
                if let [Token::Path(_), Token::Group(')', call_args)] = step.expr.as_slice() {
                    for ca in call_args {
                        if let [Token::Group('(' | ')', l_args), Token::Group('[', _)] =
                            ca.as_slice()
                        {
                            for la in l_args {
                                if let [Token::Path(lp), Token::Path(lty)] = la.as_slice() {
                                    var_types.insert(lp.clone(), lty.clone());
                                } else if let [Token::Path(lp)] = la.as_slice() {
                                    if let Some(in_ty) = &current_type {
                                        if let Some(elem) = extract_collection_element_type(in_ty, 0)
                                        {
                                            var_types.insert(lp.clone(), elem);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
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

    // Проверка инициализации формы `FormName { field val }` на несовпадение типов (E09)
    for win in tokens.windows(2) {
        if let [Token::Path(form_name), Token::Group('{', field_sents)] = win {
            if is_pascal_case(form_name) {
                verify_struct_init_fields(fn_name, form_name, field_sents, sentence_idx, report);
            }
        }
        // Закон 4 (UFCS): Прямой вызов `target_fn(arg)` проверяется по Закону 3 и Typestate так же, как `arg . target_fn`!
        if let [Token::Path(target_fn), Token::Group(')', call_args)] = win {
            if call_args.len() == 1 {
                if let Some(in_type) = infer_static_token_type(&call_args[0], var_types, report) {
                    verify_typestate_and_union_bypass(
                        fn_name,
                        &in_type,
                        target_fn,
                        sentence_idx,
                        report,
                    );
                }
            }
            for ca in call_args {
                if let [Token::Group('(' | ')', l_args), Token::Group('[', _)] = ca.as_slice() {
                    for la in l_args {
                        if let [Token::Path(lp), Token::Path(lty)] = la.as_slice() {
                            var_types.insert(lp.clone(), lty.clone());
                        }
                    }
                }
            }
        }
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
                for seg in crate::parser::parse_interpolated_string_segments(raw) {
                    if let crate::parser::StringSegment::Interpolation { raw_expr, ast } = seg {
                        verify_identifier(
                            &raw_expr,
                            true,
                            fn_name,
                            local_vars,
                            var_types,
                            shadowed,
                            ambiguous,
                            sentence_idx,
                            report,
                        );
                        for interp_sent in &ast {
                            for interp_tok in interp_sent {
                                if let Token::Path(p) = interp_tok {
                                    verify_identifier(
                                        p,
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
                    }
                }
            }
            Token::Group('{', sents) => {
                // Law 1a: Внутри `{ ... }` ленивые потоки (`[...]`) и методы видны всем сиблингам,
                // а обычные поля `key value` добавляются в скоуп после проверки их `value`.
                let mut inner_locals = local_vars.clone();
                for s in sents {
                    match classify_sentence(s) {
                        SemanticKind::StreamOrBranch { name: key, .. }
                        | SemanticKind::FunctionDef { name: key, .. } => {
                            inner_locals.insert(key);
                        }
                        _ => {}
                    }
                }
                for s in sents {
                    match classify_sentence(s) {
                        SemanticKind::Binding { key, value } => {
                            check_tokens(
                                fn_name,
                                &value,
                                &inner_locals,
                                var_types,
                                shadowed,
                                ambiguous,
                                sentence_idx,
                                report,
                            );
                            inner_locals.insert(key);
                        }
                        SemanticKind::TupleBinding { value, .. }
                        | SemanticKind::AlgebraicDef { expr: value, .. } => {
                            check_tokens(
                                fn_name,
                                &value,
                                &inner_locals,
                                var_types,
                                shadowed,
                                ambiguous,
                                sentence_idx,
                                report,
                            );
                        }
                        SemanticKind::FormDef { body, tail, .. } => {
                            let nested = [Token::Group('{', body)];
                            check_tokens(
                                fn_name,
                                &nested,
                                &inner_locals,
                                var_types,
                                shadowed,
                                ambiguous,
                                sentence_idx,
                                report,
                            );
                            check_tokens(
                                fn_name,
                                &tail,
                                &inner_locals,
                                var_types,
                                shadowed,
                                ambiguous,
                                sentence_idx,
                                report,
                            );
                        }
                        SemanticKind::StreamOrBranch { name, stream } => {
                            let mut branch_locals = inner_locals.clone();
                            if is_pascal_case(&name) {
                                let snake = pascal_to_snake_case(&name);
                                branch_locals.insert(snake.clone());
                                var_types.insert(snake, name);
                            }
                            for st in &stream {
                                check_tokens(
                                    fn_name,
                                    st,
                                    &branch_locals,
                                    var_types,
                                    shadowed,
                                    ambiguous,
                                    sentence_idx,
                                    report,
                                );
                            }
                        }
                        SemanticKind::PatternBranch { pattern, body } => {
                            let mut branch_locals = inner_locals.clone();
                            extract_pattern_vars(&pattern, &mut branch_locals);
                            for st in &body {
                                check_tokens(
                                    fn_name,
                                    st,
                                    &branch_locals,
                                    var_types,
                                    shadowed,
                                    ambiguous,
                                    sentence_idx,
                                    report,
                                );
                            }
                        }
                        SemanticKind::LambdaOrTupleBranch { args, body }
                        | SemanticKind::FunctionDef { args, body, .. } => {
                            let mut branch_locals = inner_locals.clone();
                            for la in &args {
                                if let [Token::Path(lp), rest @ ..] = la.as_slice() {
                                    if is_pascal_case(lp) && rest.is_empty() {
                                        let snake = pascal_to_snake_case(lp);
                                        branch_locals.insert(snake.clone());
                                        var_types.insert(snake, lp.clone());
                                    } else {
                                        branch_locals.insert(lp.clone());
                                        if let [Token::Path(lty)] = rest {
                                            var_types.insert(lp.clone(), lty.clone());
                                        }
                                    }
                                }
                            }
                            for st in &body {
                                check_tokens(
                                    fn_name,
                                    st,
                                    &branch_locals,
                                    var_types,
                                    shadowed,
                                    ambiguous,
                                    sentence_idx,
                                    report,
                                );
                            }
                        }
                        _ => {
                            check_tokens(
                                fn_name,
                                s,
                                &inner_locals,
                                var_types,
                                shadowed,
                                ambiguous,
                                sentence_idx,
                                report,
                            );
                        }
                    }
                }
            }
            Token::Group(_, sents) => {
                let mut inner_locals = local_vars.clone();
                for s in sents {
                    if let [Token::Group('(' | ')', l_args), Token::Group('[', _)] = s.as_slice() {
                        for la in l_args {
                            if let [Token::Path(lp), rest @ ..] = la.as_slice() {
                                if is_pascal_case(lp) && rest.is_empty() {
                                    let snake = pascal_to_snake_case(lp);
                                    inner_locals.insert(snake.clone());
                                    var_types.insert(snake, lp.clone());
                                } else {
                                    inner_locals.insert(lp.clone());
                                    if let [Token::Path(lty)] = rest {
                                        var_types.insert(lp.clone(), lty.clone());
                                    }
                                }
                            }
                        }
                    }
                    check_tokens(
                        fn_name,
                        s,
                        &inner_locals,
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
        let expected_form = report.resolve_type_to_form(expected_base);
        for branch in input_type.split('|').map(str::trim) {
            if branch == expected_base || branch == constraint {
                continue;
            }
            if let Some(branch_form) = report.resolve_type_to_form(branch) {
                let mut is_subtype = branch_form.ancestors.contains(&expected_base.to_string());
                if !is_subtype {
                    if let Some(exp_f) = &expected_form {
                        if exp_f.is_algebraic_alias
                            && !exp_f.ancestors.is_empty()
                            && exp_f.union_variants.is_empty()
                        {
                            is_subtype = exp_f.ancestors.iter().all(|anc| {
                                branch_form.name == *anc || branch_form.ancestors.contains(anc)
                            });
                        }
                    }
                }
                if !is_subtype && !branch_form.is_bypass {
                    report.push_diag_with_token(
                        Severity::Error,
                        "E04_UNHANDLED_UNION_BRANCH",
                        format!(
                            "Нарушение Закона 3 (#{sentence_idx}): Ветка '{branch}' из '{input_type}' не совместима с параметром '{constraint}' функции '{target_fn}' и не помечена тегом '& Bypass'."
                        ),
                        Some(fn_name),
                        sentence_idx,
                        Some(target_fn),
                    );
                }
            }
        }
        return;
    }

    // 2. Проверка Typestate (`Order & Reserved & Paid` или `Order / Cancelled`)
    let Some(in_sym) = report.resolve_type_to_form(input_type) else {
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
                    report.push_diag_with_token(
                        Severity::Error,
                        "E003_TYPESTATE_VIOLATION",
                        format!(
                            "Нарушение Typestate (#{sentence_idx}): '{target_fn}' требует '{constraint}', но тип '{input_type}' не содержит состояния '{req_trait}'."
                        ),
                        Some(fn_name),
                        sentence_idx,
                        Some(target_fn),
                    );
                }
            }
            i += 2;
        } else if parts[i] == "/" {
            if let Some(&forbidden) = parts.get(i + 1) {
                if in_sym.name == forbidden || in_sym.ancestors.contains(&forbidden.to_string()) {
                    report.push_diag_with_token(
                        Severity::Error,
                        "E003_TYPESTATE_FORBIDDEN",
                        format!(
                            "Нарушение Typestate (#{sentence_idx}): '{target_fn}' запрещает состояние '{forbidden}' ('{constraint}'), но '{input_type}' включает его."
                        ),
                        Some(fn_name),
                        sentence_idx,
                        Some(target_fn),
                    );
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }
}

fn enrich_type_expression(cur: &str, ret: &str, report: &AnalysisReport) -> String {
    if !cur.contains('|') {
        return format!("{cur} & {ret}");
    }
    cur.split('|')
        .map(str::trim)
        .map(|branch| {
            let is_bypass = report
                .resolve_type_to_form(branch)
                .map_or(branch == "None", |f| f.is_bypass || f.name == "None");
            if is_bypass {
                branch.to_string()
            } else {
                format!("{branch} & {ret}")
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}