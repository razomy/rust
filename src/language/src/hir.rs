// src/hir.rs
// =============================================================================
// RAZOMY V4.7 — HIGH-LEVEL INTERMEDIATE REPRESENTATION (HIR)
// Переводит динамический синтаксис AST в разрешенную статическую модель,
// сохраняя 100% обратную связь с исходным AST (SourceOrigin).
// =============================================================================

use std::collections::HashMap;

use crate::{classify_sentence, Literal, SemanticKind, Sentence, Token};

// =============================================================================
// 1. ОБРАТНАЯ СВЯЗЬ С AST И ИСХОДНЫМ КОДОМ (SOURCE ORIGIN)
// =============================================================================

#[derive(Debug, Clone)]
pub struct SourceOrigin {
    /// Порядковый индекс предложения в файле (или блоке)
    pub sentence_index: usize,
    /// Имя символа/контекста, породившего этот узел HIR
    pub symbol_context: String,
    /// Исходный слепок AST для точной диагностики и обратной трассировки
    pub raw_ast: Sentence,
}

// =============================================================================
// 2. УЗЛЫ HIR (СТАТИЧЕСКИЕ СХЕМЫ ФОРМ, ПОЛЕЙ, МЕТОДОВ И ФУНКЦИЙ)
// =============================================================================

#[derive(Debug, Clone)]
pub struct HirField {
    pub name: String,
    pub inferred_type: String,
    /// Из какой формы пришло это поле (Lineage)
    pub origin_form: String,
    /// Статически сохраненное дефолтное выражение поля (или тело реактивного потока)
    pub default_expr: Vec<Token>,
    pub is_reactive: bool,
    /// Закон 2: Флаг коллизии при слиянии двух форм с одинаковым именем поля
    pub is_ambiguous: bool,
    pub conflicting_origins: Vec<String>,
    /// Прямая ссылка на место в AST, где поле было объявлено
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone)]
pub struct HirForm {
    pub name: String,
    /// Полностью разрешенная таблица полей (после применения всех `&` и `/`)
    pub fields: HashMap<String, HirField>,
    pub field_order: Vec<String>,
    /// Методы и перегруженные операторы внутри формы (`Vec2.+`, `Db.FindUser`)
    pub methods: HashMap<String, HirFunction>,
    pub method_order: Vec<String>,
    /// Полный список предков (Lineage)
    pub ancestors: Vec<String>,
    /// Ветки объединения (`Circle | Rect`)
    pub union_variants: Vec<String>,
    /// Предикаты уточнения (`User & IsAdult`)
    pub refinement_predicates: Vec<String>,
    pub is_algebraic_alias: bool,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone)]
pub struct HirParam {
    pub name: String,
    pub type_constraint: String,
    pub default_expr: Vec<Token>,
    pub is_anonymous_type_slot: bool,
}

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub name: String,
    pub params: Vec<HirParam>,
    /// Закон 1c: true для `Fn(User)` или `Fn(User & Cart)`, false для `Fn(U User)`
    pub auto_unpacks: bool,
    pub body: Vec<Sentence>,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone)]
pub struct HirGlobalBinding {
    pub key: String,
    pub inferred_type: String,
    pub value_expr: Vec<Token>,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, Default)]
pub struct HirModule {
    pub forms: HashMap<String, HirForm>,
    pub form_order: Vec<String>,
    pub functions: HashMap<String, HirFunction>,
    pub function_order: Vec<String>,
    pub globals: HashMap<String, HirGlobalBinding>,
    pub global_order: Vec<String>,
    /// Все предложения верхнего уровня в порядке следования
    pub top_level_sentences: Vec<Sentence>,
}

// =============================================================================
// 3. ПОСТРОИТЕЛЬ HIR (LOWERING: AST -> HIR)
// =============================================================================

pub fn lower_to_hir(sentences: &[Sentence]) -> HirModule {
    let mut hir = HirModule {
        top_level_sentences: sentences.to_vec(),
        ..Default::default()
    };

    register_stdlib_hir(&mut hir);

    for (idx, sentence) in sentences.iter().enumerate() {
        let kind = classify_sentence(sentence);
        match kind {
            SemanticKind::FormDef { name, body, tail } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: name.clone(),
                    raw_ast: sentence.clone(),
                };
                let form = lower_form(&name, None, &body, &tail, origin, &hir);
                if !hir.form_order.contains(&name) {
                    hir.form_order.push(name.clone());
                }
                hir.forms.insert(name, form);
            }

            SemanticKind::SubtypeFormDef {
                name,
                base,
                body,
                tail,
            } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: name.clone(),
                    raw_ast: sentence.clone(),
                };
                let form = lower_form(&name, Some(&base), &body, &tail, origin, &hir);
                if !hir.form_order.contains(&name) {
                    hir.form_order.push(name.clone());
                }
                hir.forms.insert(name, form);
            }

            SemanticKind::AlgebraicDef { name, expr } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: name.clone(),
                    raw_ast: sentence.clone(),
                };
                let form = lower_algebraic_form(&name, &expr, origin, &hir);
                if !hir.form_order.contains(&name) {
                    hir.form_order.push(name.clone());
                }
                hir.forms.insert(name, form);
            }

            SemanticKind::FunctionDef {
                name, args, body, ..
            } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: name.clone(),
                    raw_ast: sentence.clone(),
                };
                let (params, auto_unpacks) = lower_fn_params(&args, &hir);
                let func = HirFunction {
                    name: name.clone(),
                    params,
                    auto_unpacks,
                    body,
                    origin,
                };
                if !hir.function_order.contains(&name) {
                    hir.function_order.push(name.clone());
                }
                hir.functions.insert(name, func);
            }

            SemanticKind::StreamOrBranch { name, stream } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: name.clone(),
                    raw_ast: sentence.clone(),
                };
                let func = HirFunction {
                    name: name.clone(),
                    params: vec![],
                    auto_unpacks: true,
                    body: stream,
                    origin,
                };
                if !hir.function_order.contains(&name) {
                    hir.function_order.push(name.clone());
                }
                hir.functions.insert(name, func);
            }

            SemanticKind::Binding { key, value } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: key.clone(),
                    raw_ast: sentence.clone(),
                };
                let inferred_type = infer_hir_type(&value, &hir);
                let binding = HirGlobalBinding {
                    key: key.clone(),
                    inferred_type,
                    value_expr: value,
                    origin,
                };
                if !hir.global_order.contains(&key) {
                    hir.global_order.push(key.clone());
                }
                hir.globals.insert(key, binding);
            }

            _ => {}
        }
    }

    hir
}

fn register_stdlib_hir(hir: &mut HirModule) {
    let dummy_origin = SourceOrigin {
        sentence_index: 0,
        symbol_context: "Shell".into(),
        raw_ast: vec![],
    };
    let mut fields = HashMap::new();
    fields.insert(
        "Log".into(),
        HirField {
            name: "Log".into(),
            inferred_type: "Fn(Msg Any) -> None".into(),
            origin_form: "Shell".into(),
            default_expr: vec![],
            is_reactive: false,
            is_ambiguous: false,
            conflicting_origins: vec![],
            origin: dummy_origin.clone(),
        },
    );
    fields.insert(
        "Exec".into(),
        HirField {
            name: "Exec".into(),
            inferred_type: "Fn(Cmd String) -> String".into(),
            origin_form: "Shell".into(),
            default_expr: vec![],
            is_reactive: false,
            is_ambiguous: false,
            conflicting_origins: vec![],
            origin: dummy_origin.clone(),
        },
    );

    hir.form_order.push("Shell".into());
    hir.forms.insert(
        "Shell".into(),
        HirForm {
            name: "Shell".into(),
            fields,
            field_order: vec!["Log".into(), "Exec".into()],
            methods: HashMap::new(),
            method_order: vec![],
            ancestors: vec![],
            union_variants: vec![],
            refinement_predicates: vec![],
            is_algebraic_alias: false,
            origin: dummy_origin,
        },
    );
}

fn lower_form(
    name: &str,
    base: Option<&str>,
    body: &[Sentence],
    tail: &[Token],
    origin: SourceOrigin,
    hir: &HirModule,
) -> HirForm {
    let mut fields = HashMap::new();
    let mut field_order = Vec::new();
    let mut methods = HashMap::new();
    let mut method_order = Vec::new();
    let mut ancestors = Vec::new();
    let mut union_variants = Vec::new();
    let mut refinement_predicates = Vec::new();

    // 1. Статическое копирование полей и методов базовой формы (Axiom 4)
    if let Some(base_name) = base {
        ancestors.push(base_name.to_string());
        if let Some(base_form) = hir.forms.get(base_name) {
            for k in &base_form.field_order {
                if let Some(f) = base_form.fields.get(k) {
                    fields.insert(k.clone(), f.clone());
                    field_order.push(k.clone());
                }
            }
            for m in &base_form.method_order {
                if let Some(mf) = base_form.methods.get(m) {
                    methods.insert(m.clone(), mf.clone());
                    method_order.push(m.clone());
                }
            }
            for anc in &base_form.ancestors {
                if !ancestors.contains(anc) {
                    ancestors.push(anc.clone());
                }
            }
        }
    }

    // 2. Собственные поля, реактивные потоки, методы и перегруженные операторы внутри `{ ... }`
    for (f_idx, s) in body.iter().enumerate() {
        let member_origin = SourceOrigin {
            sentence_index: f_idx,
            symbol_context: format!("{name}.{f_idx}"),
            raw_ast: s.clone(),
        };

        match classify_sentence(s) {
            SemanticKind::Binding { key, value } => {
                let inferred_type = infer_hir_type(&value, hir);
                if !fields.contains_key(&key) {
                    field_order.push(key.clone());
                }
                fields.insert(
                    key.clone(),
                    HirField {
                        name: key,
                        inferred_type,
                        origin_form: name.to_string(),
                        default_expr: value,
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                        origin: member_origin,
                    },
                );
            }
            SemanticKind::StreamOrBranch {
                name: field_name,
                stream,
            } => {
                if !fields.contains_key(&field_name) {
                    field_order.push(field_name.clone());
                }
                fields.insert(
                    field_name.clone(),
                    HirField {
                        name: field_name,
                        inferred_type: "ReactiveStream".into(),
                        origin_form: name.to_string(),
                        default_expr: vec![Token::Group('[', stream)],
                        is_reactive: true,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                        origin: member_origin,
                    },
                );
            }
            SemanticKind::FunctionDef {
                name: method_name,
                args,
                body: fn_body,
                ..
            } => {
                let (params, auto_unpacks) = lower_fn_params(&args, hir);
                if !methods.contains_key(&method_name) {
                    method_order.push(method_name.clone());
                }
                methods.insert(
                    method_name.clone(),
                    HirFunction {
                        name: method_name,
                        params,
                        auto_unpacks,
                        body: fn_body,
                        origin: member_origin,
                    },
                );
            }
            SemanticKind::OperatorMethod { op, args, rest } => {
                let (params, auto_unpacks) = lower_fn_params(&args, hir);
                let fn_body = match rest.first() {
                    Some(Token::Group('[', b)) => b.clone(),
                    _ => vec![rest],
                };
                if !methods.contains_key(&op) {
                    method_order.push(op.clone());
                }
                methods.insert(
                    op.clone(),
                    HirFunction {
                        name: op,
                        params,
                        auto_unpacks,
                        body: fn_body,
                        origin: member_origin,
                    },
                );
            }
            _ => {}
        }
    }

    // 3. Разрешение алгебры типов `&`, `|` и `/` в статику!
    if !tail.is_empty() {
        resolve_type_algebra_into_fields(
            name,
            &mut fields,
            &mut field_order,
            &mut ancestors,
            &mut union_variants,
            &mut refinement_predicates,
            tail,
            &origin,
            hir,
        );
    }

    HirForm {
        name: name.to_string(),
        fields,
        field_order,
        methods,
        method_order,
        ancestors,
        union_variants,
        refinement_predicates,
        is_algebraic_alias: false,
        origin,
    }
}

fn lower_algebraic_form(
    name: &str,
    expr: &[Token],
    origin: SourceOrigin,
    hir: &HirModule,
) -> HirForm {
    let mut fields = HashMap::new();
    let mut field_order = Vec::new();
    let mut methods = HashMap::new();
    let mut method_order = Vec::new();
    let mut ancestors = Vec::new();
    let mut union_variants = Vec::new();
    let mut refinement_predicates = Vec::new();

    if let Some(first_tok) = expr.first() {
        match first_tok {
            Token::Path(first_type) => {
                if let Some(base_form) = hir.forms.get(first_type) {
                    ancestors.push(first_type.clone());
                    for anc in &base_form.ancestors {
                        if !ancestors.contains(anc) {
                            ancestors.push(anc.clone());
                        }
                    }
                    for k in &base_form.field_order {
                        if let Some(f) = base_form.fields.get(k) {
                            fields.insert(k.clone(), f.clone());
                            field_order.push(k.clone());
                        }
                    }
                    for m in &base_form.method_order {
                        if let Some(mf) = base_form.methods.get(m) {
                            methods.insert(m.clone(), mf.clone());
                            method_order.push(m.clone());
                        }
                    }
                } else {
                    union_variants.push(first_type.clone());
                }
            }
            Token::Lit(Literal::Symbol(sym)) => {
                union_variants.push(format!("'{sym}'"));
            }
            _ => {}
        }

        if expr.len() > 1 {
            resolve_type_algebra_into_fields(
                name,
                &mut fields,
                &mut field_order,
                &mut ancestors,
                &mut union_variants,
                &mut refinement_predicates,
                &expr[1..],
                &origin,
                hir,
            );
        }
    }

    HirForm {
        name: name.to_string(),
        fields,
        field_order,
        methods,
        method_order,
        ancestors,
        union_variants,
        refinement_predicates,
        is_algebraic_alias: true,
        origin,
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_type_algebra_into_fields(
    owner_name: &str,
    fields: &mut HashMap<String, HirField>,
    field_order: &mut Vec<String>,
    ancestors: &mut Vec<String>,
    union_variants: &mut Vec<String>,
    refinement_predicates: &mut Vec<String>,
    tail: &[Token],
    owner_origin: &SourceOrigin,
    hir: &HirModule,
) {
    let mut i = 0;
    while i < tail.len() {
        match &tail[i] {
            Token::Op(op) if op == "&" => {
                if let Some(next_tok) = tail.get(i + 1) {
                    match next_tok {
                        Token::Path(rhs_type) => {
                            if let Some(rhs_form) = hir.forms.get(rhs_type) {
                                if !ancestors.contains(rhs_type) {
                                    ancestors.push(rhs_type.clone());
                                }
                                for k in &rhs_form.field_order {
                                    if let Some(rhs_f) = rhs_form.fields.get(k) {
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
                                            }
                                        } else {
                                            field_order.push(k.clone());
                                            fields.insert(k.clone(), rhs_f.clone());
                                        }
                                    }
                                }
                            } else if hir.functions.contains_key(rhs_type) {
                                // Предикат уточнения типа (например, `Adult User & IsAdult`)
                                refinement_predicates.push(rhs_type.clone());
                            } else if !ancestors.contains(rhs_type) {
                                ancestors.push(rhs_type.clone());
                            }
                        }
                        Token::Group('{', override_sents) => {
                            for s in override_sents {
                                if let SemanticKind::Binding { key, value } = classify_sentence(s) {
                                    let t = infer_hir_type(&value, hir);
                                    if !fields.contains_key(&key) {
                                        field_order.push(key.clone());
                                    }
                                    fields.insert(
                                        key.clone(),
                                        HirField {
                                            name: key,
                                            inferred_type: t,
                                            origin_form: owner_name.to_string(),
                                            default_expr: value,
                                            is_reactive: false,
                                            is_ambiguous: false,
                                            conflicting_origins: vec![],
                                            origin: owner_origin.clone(),
                                        },
                                    );
                                }
                            }
                        }
                        Token::Group('[', _) => {
                            refinement_predicates.push("<inline_predicate>".into());
                        }
                        _ => {}
                    }
                }
                i += 2;
            }
            Token::Op(op) if op == "|" => {
                if let Some(next_tok) = tail.get(i + 1) {
                    match next_tok {
                        Token::Path(variant) => {
                            if union_variants.is_empty() && !ancestors.is_empty() {
                                union_variants.extend(ancestors.clone());
                            }
                            union_variants.push(variant.clone());
                        }
                        Token::Lit(Literal::Symbol(sym)) => {
                            union_variants.push(format!("'{sym}'"));
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

pub fn lower_fn_params(args: &[Sentence], hir: &HirModule) -> (Vec<HirParam>, bool) {
    let mut out = Vec::new();
    let mut auto_unpacks = false;

    for (idx, sent) in args.iter().enumerate() {
        match sent.as_slice() {
            // Закон 1c: Одиночный идентификатор типа `Fn(User)`
            [Token::Path(single)] => {
                let is_type = hir.forms.contains_key(single)
                    || single
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false);
                if idx == 0 && is_type {
                    auto_unpacks = true;
                }
                out.push(HirParam {
                    name: single.clone(),
                    type_constraint: if is_type { single.clone() } else { String::new() },
                    default_expr: vec![],
                    is_anonymous_type_slot: is_type,
                });
            }
            // Закон 1c: Анонимный пересеченный слот `FinalizeBill(User & Cart)` или `ShipToCustomer(Order & Reserved & Paid)`
            [Token::Path(first_type), Token::Op(op), ..] if op == "&" || op == "|" || op == "/" => {
                let constraint = sent
                    .iter()
                    .map(|t| match t {
                        Token::Path(p) => p.clone(),
                        Token::Op(o) => o.clone(),
                        _ => "...".into(),
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                if idx == 0 {
                    auto_unpacks = true;
                }
                out.push(HirParam {
                    name: first_type.clone(),
                    type_constraint: constraint,
                    default_expr: vec![],
                    is_anonymous_type_slot: true,
                });
            }
            // Именованный параметр с типом `Fn(U User)` или с дефолтным значением `Connect(Host "localhost")`
            [Token::Path(param_name), rest @ ..] => {
                let is_default_lit = matches!(rest.first(), Some(Token::Lit(_) | Token::Group(_, _)));
                let constraint = if is_default_lit {
                    infer_hir_type(rest, hir)
                } else {
                    rest.iter()
                        .map(|t| match t {
                            Token::Path(p) => p.clone(),
                            Token::Op(o) => o.clone(),
                            _ => "...".into(),
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                out.push(HirParam {
                    name: param_name.clone(),
                    type_constraint: constraint,
                    default_expr: if is_default_lit { rest.to_vec() } else { vec![] },
                    is_anonymous_type_slot: false,
                });
            }
            _ => {}
        }
    }

    (out, auto_unpacks)
}

pub fn infer_hir_type(tokens: &[Token], hir: &HirModule) -> String {
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
            Literal::Url(_) => "Url".into(),
            Literal::File(_) => "File".into(),
            _ => "Literal".into(),
        },
        Some(Token::Path(p)) => {
            if hir.forms.contains_key(p) {
                p.clone()
            } else {
                p.clone()
            }
        }
        Some(Token::Group('(', _)) => "Tuple".into(),
        Some(Token::Group('{', _)) => "Form".into(),
        Some(Token::Group('[', _)) => "Stream".into(),
        _ => "Any".into(),
    }
}