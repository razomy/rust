// src/analyzer.rs
// =============================================================================
// RAZOMY V4.7 — HIR-POWERED STATIC ANALYZER & IDE ENGINE
// Меньше бойлерплейта, больше сути: Законы 1, 2, 3 (Typestate) + Autocomplete.
// =============================================================================

use serde_json::{json, Value as JsonValue};
use std::collections::{HashMap, HashSet};

use crate::hir::{lower_to_hir, HirField, HirFunction, HirModule, HirParam};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Form,
    Function,
    Stream,
    Variable,
    TypeAlgebra,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub signature: String,
    pub documentation: String,
    /// Напрямую используем HirField вместо дублирующей структуры!
    pub fields: HashMap<String, HirField>,
    pub field_order: Vec<String>,
    pub methods: Vec<String>,
    pub ancestors: Vec<String>,
    pub union_variants: Vec<String>,
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
                    "unionVariants": sym.union_variants,
                    "methods": sym.methods,
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
            md.push_str(&format!("**Lineage (Предки):** `{}`\n\n", sym.ancestors.join(" & ")));
        }
        if !sym.union_variants.is_empty() {
            md.push_str(&format!("**Union (Варианты):** `{}`\n\n", sym.union_variants.join(" | ")));
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
        if !sym.methods.is_empty() {
            md.push_str("\n**Методы и операторы:**\n");
            for m in &sym.methods {
                md.push_str(&format!("- `{m}`\n"));
            }
        }
        Some(md)
    }

    /// Умное автодополнение для IDE: по `Target.` выдает поля, методы и предков Lineage,
    /// а без точки — глобальные символы.
    pub fn completions(&self, query: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        if let Some((base, prefix)) = query.rsplit_once('.') {
            if let Some(sym) = self.symbols.get(base) {
                for fname in &sym.field_order {
                    if fname.starts_with(prefix) {
                        let f = &sym.fields[fname];
                        items.push(CompletionItem {
                            label: fname.clone(),
                            kind: if f.is_reactive { "ReactiveField" } else { "Field" },
                            detail: format!("{}: {} (from {})", fname, f.inferred_type, f.origin_form),
                            insert_text: fname.clone(),
                        });
                    }
                }
                for m in &sym.methods {
                    if m.starts_with(prefix) {
                        items.push(CompletionItem {
                            label: m.clone(),
                            kind: "Method",
                            detail: format!("{base}.{m}(...)"),
                            insert_text: format!("{m}()"),
                        });
                    }
                }
                for anc in &sym.ancestors {
                    if anc.starts_with(prefix) {
                        items.push(CompletionItem {
                            label: anc.clone(),
                            kind: "AncestorSlice",
                            detail: format!("Lineage ancestor of {base}"),
                            insert_text: anc.clone(),
                        });
                    }
                }
            }
        } else {
            for name in &self.symbol_order {
                if name.starts_with(query) {
                    let sym = &self.symbols[name];
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: match sym.kind {
                            SymbolKind::Form => "Form",
                            SymbolKind::Function => "Function",
                            SymbolKind::Stream => "Stream",
                            SymbolKind::Variable => "Variable",
                            SymbolKind::TypeAlgebra => "TypeAlgebra",
                        },
                        detail: sym.signature.clone(),
                        insert_text: name.clone(),
                    });
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
            code: "E001_SYNTAX",
            message: format!("Синтаксическая ошибка парсера:\n{err}"),
            context_symbol: None,
            sentence_index: None,
        }),
    }
    report
}

pub fn analyze_from_hir(hir: HirModule, report: &mut AnalysisReport) {
    // 1. Формы + проверка коллизий Закона 2 + проверка методов внутри форм
    for form_name in &hir.form_order {
        if let Some(hf) = hir.forms.get(form_name) {
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

            report.symbol_order.push(form_name.clone());
            report.symbols.insert(
                form_name.clone(),
                SymbolInfo {
                    name: form_name.clone(),
                    kind: if hf.is_algebraic_alias { SymbolKind::TypeAlgebra } else { SymbolKind::Form },
                    signature: format!("Form {form_name} {{ {} fields, {} methods }}", hf.field_order.len(), hf.method_order.len()),
                    documentation: format!("HIR Form (предложение #{})", hf.origin.sentence_index),
                    fields: hf.fields.clone(),
                    field_order: hf.field_order.clone(),
                    methods: hf.method_order.clone(),
                    ancestors: hf.ancestors.clone(),
                    union_variants: hf.union_variants.clone(),
                    params: vec![],
                    auto_unpacks_first_arg: false,
                },
            );
        }
    }

    // 2. Глобальные привязки
    for g_name in &hir.global_order {
        if let Some(g) = hir.globals.get(g_name) {
            let base = report.symbols.get(&g.inferred_type).cloned();
            report.symbol_order.push(g_name.clone());
            report.symbols.insert(
                g_name.clone(),
                SymbolInfo {
                    name: g.key.clone(),
                    kind: SymbolKind::Variable,
                    signature: format!("{} : {}", g.key, g.inferred_type),
                    documentation: format!("Global Binding (предложение #{})", g.origin.sentence_index),
                    fields: base.as_ref().map(|b| b.fields.clone()).unwrap_or_default(),
                    field_order: base.as_ref().map(|b| b.field_order.clone()).unwrap_or_default(),
                    methods: base.as_ref().map(|b| b.methods.clone()).unwrap_or_default(),
                    ancestors: base.as_ref().map(|b| b.ancestors.clone()).unwrap_or_default(),
                    union_variants: vec![],
                    params: vec![],
                    auto_unpacks_first_arg: false,
                },
            );
        }
    }

    // 3. Функции верхнего уровня и методы внутри форм
    for fn_name in &hir.function_order {
        if let Some(hir_fn) = hir.functions.get(fn_name) {
            register_and_verify_function(hir_fn, None, report);
        }
    }
    for form_name in &hir.form_order {
        if let Some(hf) = hir.forms.get(form_name) {
            for m_name in &hf.method_order {
                if let Some(m_fn) = hf.methods.get(m_name) {
                    verify_function_body(
                        &format!("{form_name}.{m_name}"),
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

    report.hir = hir;
}

fn register_and_verify_function(
    hir_fn: &HirFunction,
    enclosing_fields: Option<&Vec<String>>,
    report: &mut AnalysisReport,
) {
    let params_tuple: Vec<(String, String)> = hir_fn
        .params
        .iter()
        .map(|p| (p.name.clone(), p.type_constraint.clone()))
        .collect();

    let param_str = params_tuple
        .iter()
        .map(|(n, t)| if t.is_empty() || n == t { n.clone() } else { format!("{n} {t}") })
        .collect::<Vec<_>>()
        .join(", ");

    report.symbol_order.push(hir_fn.name.clone());
    report.symbols.insert(
        hir_fn.name.clone(),
        SymbolInfo {
            name: hir_fn.name.clone(),
            kind: if hir_fn.params.is_empty() { SymbolKind::Stream } else { SymbolKind::Function },
            signature: format!("{}({param_str}) [ ... ]", hir_fn.name),
            documentation: format!(
                "HIR Function (предложение #{}, auto_unpacks: {})",
                hir_fn.origin.sentence_index, hir_fn.auto_unpacks
            ),
            fields: HashMap::new(),
            field_order: vec![],
            methods: vec![],
            ancestors: vec![],
            union_variants: vec![],
            params: params_tuple,
            auto_unpacks_first_arg: hir_fn.auto_unpacks,
        },
    );

    verify_function_body(
        &hir_fn.name,
        &hir_fn.params,
        hir_fn.auto_unpacks,
        &hir_fn.body,
        enclosing_fields,
        hir_fn.origin.sentence_index,
        report,
    );
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
    let mut local_vars: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();

    if let Some(fields) = enclosing_fields {
        local_vars.extend(fields.iter().cloned());
        local_vars.insert("Self".into());
    }

    if !auto_unpacks {
        for p in params {
            let base = p.type_constraint.split_whitespace().next().unwrap_or("");
            if let Some(sym) = report.symbols.get(base) {
                for f in &sym.field_order {
                    shadowed_named_fields.insert(f.clone(), (p.name.clone(), base.to_string()));
                }
            }
        }
    } else if let Some(first) = params.first() {
        collect_ambiguous_fields(&first.name, &report.symbols, &mut ambiguous_fields);
    }

    for sent in body {
        match classify_sentence(sent) {
            SemanticKind::Binding { key, .. } => {
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
            &shadowed_named_fields,
            &ambiguous_fields,
            sentence_idx,
            report,
        );
    }
}

fn collect_ambiguous_fields(
    type_name: &str,
    symbols: &HashMap<String, SymbolInfo>,
    out: &mut HashMap<String, Vec<String>>,
) {
    if let Some(sym) = symbols.get(type_name) {
        for (fname, f) in &sym.fields {
            if f.is_ambiguous {
                out.insert(fname.clone(), f.conflicting_origins.clone());
            }
        }
    }
}

/// Единая проверка идентификатора (и для `Token::Path`, и для интерполяции `{Field}`)
fn verify_identifier(
    ident: &str,
    is_interp: bool,
    fn_name: &str,
    local_vars: &HashSet<String>,
    shadowed: &HashMap<String, (String, String)>,
    ambiguous: &HashMap<String, Vec<String>>,
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    if ident.contains('.') || local_vars.contains(ident) {
        return;
    }
    if let Some((param_var, param_type)) = shadowed.get(ident) {
        let code = if is_interp { "W001_LAW1_NAMED_SLOT_INTERP" } else { "W001_LAW1_NAMED_SLOT" };
        let fix = if is_interp { format!("{{{param_var}.{ident}}}") } else { format!("{param_var}.{ident}") };
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
    if let Some(origins) = ambiguous.get(ident) {
        let suggestion = origins
            .iter()
            .map(|o| if is_interp { format!("{{{o}.{ident}}}") } else { format!("{o}.{ident}") })
            .collect::<Vec<_>>()
            .join(" или ");
        report.push_diag(
            Severity::Error,
            "E002_LAW2_AMBIGUOUS_FIELD_ACCESS",
            format!(
                "Нарушение Закона 2 (#{sentence_idx}): В '{fn_name}' поле '{ident}' неоднозначно (предков: {}). Используйте {suggestion}.",
                origins.join(" & ")
            ),
            Some(fn_name),
            sentence_idx,
        );
    }
}

fn check_tokens(
    fn_name: &str,
    tokens: &[Token],
    local_vars: &HashSet<String>,
    shadowed: &HashMap<String, (String, String)>,
    ambiguous: &HashMap<String, Vec<String>>,
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    if has_pipe_op(tokens) {
        let (head, steps) = split_pipeline(tokens);
        check_tokens(fn_name, &head, local_vars, shadowed, ambiguous, sentence_idx, report);

        // Статическая проверка Typestate в пайплайнах (например, `RawOrder . ShipToCustomer`)
        let head_type = infer_static_token_type(&head, report);
        for step in steps {
            if let (Some(in_type), [Token::Path(target_fn)]) = (&head_type, step.expr.as_slice()) {
                verify_typestate_step(fn_name, in_type, target_fn, sentence_idx, report);
            }

            if let Some(Token::Group('{', branches)) = step.expr.first() {
                for branch in branches {
                    match classify_sentence(branch) {
                        SemanticKind::PatternBranch { pattern, body } => {
                            let mut branch_locals = local_vars.clone();
                            extract_pattern_vars(&pattern, &mut branch_locals);
                            for s in &body {
                                check_tokens(fn_name, s, &branch_locals, shadowed, ambiguous, sentence_idx, report);
                            }
                        }
                        SemanticKind::StreamOrBranch { name, stream } => {
                            let mut branch_ambiguous = ambiguous.clone();
                            collect_ambiguous_fields(&name, &report.symbols, &mut branch_ambiguous);
                            for s in &stream {
                                check_tokens(fn_name, s, local_vars, shadowed, &branch_ambiguous, sentence_idx, report);
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                check_tokens(fn_name, &step.expr, local_vars, shadowed, ambiguous, sentence_idx, report);
            }
        }
        return;
    }

    for tok in tokens {
        match tok {
            Token::Path(p) => verify_identifier(p, false, fn_name, local_vars, shadowed, ambiguous, sentence_idx, report),
            Token::Lit(
                Literal::String(raw)
                | Literal::Url(raw)
                | Literal::File(raw)
                | Literal::Email(raw)
            ) => {
                for chunk in raw.split('{').skip(1) {
                    if let Some(expr) = chunk.split('}').next() {
                        verify_identifier(expr.trim(), true, fn_name, local_vars, shadowed, ambiguous, sentence_idx, report);
                    }
                }
            }
            Token::Group(_, sents) => {
                for s in sents {
                    check_tokens(fn_name, s, local_vars, shadowed, ambiguous, sentence_idx, report);
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
                            locals.insert(var.clone());
                        }
                    }
                }
            }
        }
    }
}

fn infer_static_token_type(tokens: &[Token], report: &AnalysisReport) -> Option<String> {
    match tokens {
        [Token::Path(name), Token::Group('{', _)] => Some(name.clone()),
        [Token::Path(var)] => report.symbols.get(var).and_then(|s| {
            if s.kind == SymbolKind::Form || s.kind == SymbolKind::TypeAlgebra {
                Some(var.clone())
            } else {
                s.signature.split(':').nth(1).map(|t| t.trim().to_string())
            }
        }),
        _ => None,
    }
}

/// Проверка Typestate (Раздел 10): проверяет, что входная форма обладает всеми требуемыми `&` предками
/// и не содержит запрещенных `/` предков.
fn verify_typestate_step(
    fn_name: &str,
    input_type: &str,
    target_fn: &str,
    sentence_idx: usize,
    report: &mut AnalysisReport,
) {
    let (Some(in_sym), Some(fn_sym)) = (report.symbols.get(input_type).cloned(), report.symbols.get(target_fn).cloned()) else {
        return;
    };
    let Some((_, constraint)) = fn_sym.params.first() else {
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