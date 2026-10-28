// src/hir.rs
// =============================================================================
// RAZOMY V4.7 — HIGH-LEVEL INTERMEDIATE REPRESENTATION (HIR)
// Переводит динамический синтаксис AST в разрешенную статическую модель,
// сохраняя 100% обратную связь с исходным AST (SourceOrigin).
// =============================================================================

use std::collections::HashMap;

use crate::parser::{
    classify_sentence, has_algebra_op, has_pipe_op, Literal, SemanticKind, Sentence, Token,
};

// =============================================================================
// 1. ОБРАТНАЯ СВЯЗЬ С AST И ИСХОДНЫМ КОДОМ (SOURCE ORIGIN)
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct SourceOrigin {
    /// Порядковый индекс предложения в файле (или блоке)
    pub sentence_index: usize,
    /// Имя символа/контекста, породившего этот узел HIR
    pub symbol_context: String,
    /// Исходный слепок AST для точной диагностики и обратной трассировки
    pub raw_ast: Sentence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirVisibility {
    Default,
    Public,
    Private,
}

// =============================================================================
// 2. УЗЛЫ HIR (СТАТИЧЕСКИЕ СХЕМЫ ФОРМ, ПОЛЕЙ, МЕТОДОВ, ИМПОРТОВ И ФУНКЦИЙ)
// =============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct HirField {
    pub name: String,
    pub inferred_type: String,
    /// Из какой формы пришло активное значение этого поля (Lineage)
    pub origin_form: String,
    /// Статически сохраненное дефолтное выражение поля (или тело реактивного потока)
    pub default_expr: Vec<Token>,
    pub is_reactive: bool,
    /// Закон 2: Флаг коллизии при слиянии двух форм с одинаковым именем поля
    pub is_ambiguous: bool,
    pub conflicting_origins: Vec<String>,
    /// Видимость поля (`& Public` / `& Private`)
    pub visibility: HirVisibility,
    /// Прямая ссылка на место в AST, где поле было объявлено
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirForm {
    pub name: String,
    /// Параметры типа для дженериков (`Map(K, V)`, `Cons(T)`, `Comparable(Other Self)`)
    pub type_params: Vec<String>,
    /// Аргументы типа при инстанцировании дженерика (`Atom(CounterState)`, `Succ(Zero)`)
    pub type_args: Vec<String>,
    /// Полностью разрешенная таблица активных полей (после применения всех `&` и `/`)
    pub fields: HashMap<String, HirField>,
    pub field_order: Vec<String>,
    /// История полей каждого предка для Lineage-доступа (`A.Parent.Field`, Law 2)
    pub ancestor_fields: HashMap<String, HashMap<String, HirField>>,
    /// Методы, контракты и перегруженные операторы внутри формы (`Vec2.+`, `Db.FindUser`)
    pub methods: HashMap<String, HirFunction>,
    pub method_order: Vec<String>,
    /// Вложенные подмодули/подформы (`ComplexSystem.SubModule.Nested`)
    pub submodules: HashMap<String, HirForm>,
    /// Полный список предков (Lineage)
    pub ancestors: Vec<String>,
    /// Ветки объединения (`Circle | Rect`, `'draft' | 'published'`, `"a".."z" | "A".."Z"`)
    pub union_variants: Vec<String>,
    /// Предикаты уточнения (`User & IsAdult`, `CompileError & [ _.Code >= 500 ]`)
    pub refinement_predicates: Vec<String>,
    pub is_algebraic_alias: bool,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirParam {
    pub name: String,
    pub type_constraint: String,
    pub default_expr: Vec<Token>,
    pub is_anonymous_type_slot: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirFunction {
    pub name: String,
    pub params: Vec<HirParam>,
    /// Закон 1c: true для `Fn(User)` или `Fn(User & Cart)`, false для `Fn(U User)`
    pub auto_unpacks: bool,
    pub body: Vec<Sentence>,
    /// Возвращаемый тип для контрактов методов (`<(Other) Bool`)
    pub return_type: Option<String>,
    pub is_contract: bool,
    pub visibility: HirVisibility,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirGlobalBinding {
    pub key: String,
    pub inferred_type: String,
    pub value_expr: Vec<Token>,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirImport {
    /// Имена импортируемых символов (`vec!["_"]`, `vec!["Base"]`, `vec!["String", "Int"]`)
    pub targets: Vec<String>,
    /// Источник (`root.base`, `%../../../base`, `std.net.http`)
    pub source_path: String,
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
    pub imports: Vec<HirImport>,
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
                // Если `Name` уже определялась ранее в этом же файле (Axiom 2 -> Axiom 3 в Разделе 0),
                // передаем предыдущую версию как неявную базу, чтобы сохранить историю!
                let prev_base = if hir.forms.contains_key(&name) {
                    Some(name.as_str())
                } else {
                    None
                };
                let form = lower_form(
                    &name,
                    prev_base,
                    vec![],
                    vec![],
                    &body,
                    &tail,
                    origin,
                    &hir,
                );
                register_form(&mut hir, name, form);
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
                // Извлекаем аргументы типа, если база была дженериком: `GlobalCounter Atom(CounterState) { ... }`
                let type_args = match sentence.as_slice() {
                    [Token::Path(_), Token::Path(_), Token::Group(')', args), ..] => {
                        extract_type_params(args)
                    }
                    _ => vec![],
                };
                let form = lower_form(
                    &name,
                    Some(&base),
                    vec![],
                    type_args,
                    &body,
                    &tail,
                    origin,
                    &hir,
                );
                register_form(&mut hir, name, form);
            }

            SemanticKind::GenericDef { name, params, body } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: name.clone(),
                    raw_ast: sentence.clone(),
                };
                let type_params = extract_type_params(&params);
                let form = match body.as_slice() {
                    [Token::Group('{', form_body), tail @ ..] => lower_form(
                        &name,
                        None,
                        type_params,
                        vec![],
                        form_body,
                        tail,
                        origin,
                        &hir,
                    ),
                    [Token::Path(base), Token::Group('{', form_body), tail @ ..] => lower_form(
                        &name,
                        Some(base),
                        type_params,
                        vec![],
                        form_body,
                        tail,
                        origin,
                        &hir,
                    ),
                    _ => {
                        let mut f = lower_algebraic_form(&name, &body, origin, &hir);
                        f.type_params = type_params;
                        f
                    }
                };
                register_form(&mut hir, name, form);
            }

            SemanticKind::AlgebraicDef { name, expr } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: name.clone(),
                    raw_ast: sentence.clone(),
                };

                // Проверяем, является ли это композицией потоков/функций: `CombinedPipeline Step1 & Step2` (Раздел 17.1)
                if is_stream_composition(&expr, &hir) {
                    let composed_steps: Vec<Token> = expr
                        .iter()
                        .map(|t| match t {
                            Token::Op(op) if op == "&" => Token::Op(".".into()),
                            other => other.clone(),
                        })
                        .collect();
                    let func = HirFunction {
                        name: name.clone(),
                        params: vec![],
                        auto_unpacks: true,
                        body: vec![composed_steps],
                        return_type: None,
                        is_contract: false,
                        visibility: HirVisibility::Default,
                        origin: origin.clone(),
                    };
                    if !hir.function_order.contains(&name) {
                        hir.function_order.push(name.clone());
                    }
                    hir.functions.insert(name.clone(), func);
                }

                let form = lower_algebraic_form(&name, &expr, origin, &hir);
                register_form(&mut hir, name, form);
            }

            SemanticKind::FunctionDef {
                name,
                args,
                body,
                tail,
            } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: name.clone(),
                    raw_ast: sentence.clone(),
                };
                let (params, auto_unpacks) = lower_fn_params(&args, &hir);
                let visibility = parse_visibility_from_tokens(&tail);
                let func = HirFunction {
                    name: name.clone(),
                    params,
                    auto_unpacks,
                    body,
                    return_type: None,
                    is_contract: false,
                    visibility,
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
                    return_type: None,
                    is_contract: false,
                    visibility: HirVisibility::Default,
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

                // 1. Проверяем, является ли это импортом (Раздел 18.1: `_ root.base`, `_ %../../../base`, `Http std.net.http`)
                if let Some(import_source) = detect_import_source(&key, &value) {
                    hir.imports.push(HirImport {
                        targets: vec![key.clone()],
                        source_path: import_source,
                        origin: origin.clone(),
                    });
                }

                // 2. Если правая часть — это известная форма (`JonnDoy Jonn`) или инстанцирование дженерика (`One Succ(Zero)`),
                // также регистрируем её в `hir.forms`!
                match value.as_slice() {
                    [Token::Path(base_type)] if hir.forms.contains_key(base_type) => {
                        let form = lower_algebraic_form(&key, &value, origin.clone(), &hir);
                        register_form(&mut hir, key.clone(), form);
                    }
                    [Token::Path(generic_base), Token::Group(')', args)]
                    if hir.forms.contains_key(generic_base) =>
                        {
                            let type_args = extract_type_params(args);
                            let form = lower_form(
                                &key,
                                Some(generic_base),
                                vec![],
                                type_args,
                                &[],
                                &[],
                                origin.clone(),
                                &hir,
                            );
                            register_form(&mut hir, key.clone(), form);
                        }
                    // Композиция потоков в скобках: `CleanAndUpper (Trim & ToUpper)` (Раздел 3)
                    [Token::Group('(', inner)]
                    if inner.len() == 1 && has_algebra_op(&inner[0]) =>
                        {
                            let composed_steps: Vec<Token> = inner[0]
                                .iter()
                                .map(|t| match t {
                                    Token::Op(op) if op == "&" => Token::Op(".".into()),
                                    other => other.clone(),
                                })
                                .collect();
                            let func = HirFunction {
                                name: key.clone(),
                                params: vec![],
                                auto_unpacks: true,
                                body: vec![composed_steps],
                                return_type: None,
                                is_contract: false,
                                visibility: HirVisibility::Default,
                                origin: origin.clone(),
                            };
                            if !hir.function_order.contains(&key) {
                                hir.function_order.push(key.clone());
                            }
                            hir.functions.insert(key.clone(), func);
                        }
                    _ => {}
                }

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

            SemanticKind::TupleBinding { tuple, value } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: "<tuple_binding>".into(),
                    raw_ast: sentence.clone(),
                };
                let inferred_type = infer_hir_type(&value, &hir);
                let mut targets = Vec::new();
                for elem_sent in tuple {
                    if let [Token::Path(var_name)] = elem_sent.as_slice() {
                        targets.push(var_name.clone());
                        let binding = HirGlobalBinding {
                            key: var_name.clone(),
                            inferred_type: inferred_type.clone(),
                            value_expr: value.clone(),
                            origin: origin.clone(),
                        };
                        if !hir.global_order.contains(var_name) {
                            hir.global_order.push(var_name.clone());
                        }
                        hir.globals.insert(var_name.clone(), binding);
                    }
                }
                if !targets.is_empty() {
                    if let Some(src) = detect_import_source(&targets[0], &value) {
                        hir.imports.push(HirImport {
                            targets,
                            source_path: src,
                            origin,
                        });
                    }
                }
            }

            // Поддержка постфиксных макросов на формах: `Product { Id Int, Title String } . DeriveJson` (Раздел 17.4)
            SemanticKind::Pipeline(tokens) => {
                if let [Token::Path(name), Token::Group('{', body), Token::Op(op), Token::Path(macro_name)] =
                    tokens.as_slice()
                {
                    if op == "." {
                        let origin = SourceOrigin {
                            sentence_index: idx,
                            symbol_context: name.clone(),
                            raw_ast: sentence.clone(),
                        };
                        // Если макрос (например, `DeriveJson`) определен как `TargetForm & { ... }`,
                        // извлекаем его хвост `& { ... }` и статически накладываем на форму!
                        let mut macro_tail = Vec::new();
                        if let Some(macro_fn) = hir.functions.get(macro_name) {
                            if let Some(first_sent) = macro_fn.body.first() {
                                if first_sent.len() >= 3 {
                                    if let Token::Op(amp) = &first_sent[1] {
                                        if amp == "&" {
                                            macro_tail.extend_from_slice(&first_sent[1..]);
                                        }
                                    }
                                }
                            }
                        }
                        let form = lower_form(
                            name,
                            None,
                            vec![],
                            vec![],
                            body,
                            &macro_tail,
                            origin,
                            &hir,
                        );
                        register_form(&mut hir, name.clone(), form);
                    }
                }
            }

            _ => {}
        }
    }

    hir
}

fn is_stream_composition(expr: &[Token], hir: &HirModule) -> bool {
    if let Some(Token::Path(first)) = expr.first() {
        if hir.functions.contains_key(first)
            && expr
            .iter()
            .any(|t| matches!(t, Token::Op(op) if op == "&"))
        {
            return true;
        }
    }
    false
}

fn detect_import_source(key: &str, value: &[Token]) -> Option<String> {
    match value {
        [Token::Lit(Literal::File(f))] if key == "_" => Some(f.clone()),
        [Token::Path(p)]
        if key == "_"
            || p.starts_with("root.")
            || p.starts_with("std.")
            || p == "base" =>
            {
                Some(p.clone())
            }
        _ => None,
    }
}

fn register_form(hir: &mut HirModule, name: String, form: HirForm) {
    if !hir.form_order.contains(&name) {
        hir.form_order.push(name.clone());
    }
    hir.forms.insert(name, form);
}

fn extract_type_params(params: &[Sentence]) -> Vec<String> {
    params
        .iter()
        .filter_map(|s| match s.as_slice() {
            [Token::Path(p), ..] => Some(p.clone()),
            _ => None,
        })
        .collect()
}

fn parse_visibility_from_tokens(tokens: &[Token]) -> HirVisibility {
    let mut vis = HirVisibility::Default;
    let mut i = 0;
    while i + 1 < tokens.len() {
        if let (Token::Op(op), Token::Path(trait_name)) = (&tokens[i], &tokens[i + 1]) {
            if op == "&" {
                if trait_name == "Private" {
                    vis = HirVisibility::Private;
                } else if trait_name == "Public" {
                    vis = HirVisibility::Public;
                }
            }
        }
        i += 1;
    }
    vis
}

fn strip_visibility_tail(tokens: &[Token]) -> (Vec<Token>, HirVisibility) {
    let vis = parse_visibility_from_tokens(tokens);
    if vis == HirVisibility::Default {
        return (tokens.to_vec(), vis);
    }
    let mut cleaned = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if i + 1 < tokens.len() {
            if let (Token::Op(op), Token::Path(trait_name)) = (&tokens[i], &tokens[i + 1]) {
                if op == "&" && (trait_name == "Private" || trait_name == "Public") {
                    i += 2;
                    continue;
                }
            }
        }
        cleaned.push(tokens[i].clone());
        i += 1;
    }
    (cleaned, vis)
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
            visibility: HirVisibility::Public,
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
            visibility: HirVisibility::Public,
            origin: dummy_origin.clone(),
        },
    );

    hir.form_order.push("Shell".into());
    hir.forms.insert(
        "Shell".into(),
        HirForm {
            name: "Shell".into(),
            type_params: vec![],
            type_args: vec![],
            fields,
            field_order: vec!["Log".into(), "Exec".into()],
            ancestor_fields: HashMap::new(),
            methods: HashMap::new(),
            method_order: vec![],
            submodules: HashMap::new(),
            ancestors: vec![],
            union_variants: vec![],
            refinement_predicates: vec![],
            is_algebraic_alias: false,
            origin: dummy_origin,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn lower_form(
    name: &str,
    base: Option<&str>,
    type_params: Vec<String>,
    type_args: Vec<String>,
    body: &[Sentence],
    tail: &[Token],
    origin: SourceOrigin,
    hir: &HirModule,
) -> HirForm {
    let mut fields = HashMap::new();
    let mut field_order = Vec::new();
    let mut ancestor_fields: HashMap<String, HashMap<String, HirField>> = HashMap::new();
    let mut methods = HashMap::new();
    let mut method_order = Vec::new();
    let mut submodules = HashMap::new();
    let mut ancestors = Vec::new();
    let mut union_variants = Vec::new();
    let mut refinement_predicates = Vec::new();

    // 1. Статическое копирование полей, истории предков и методов базовой формы (Axiom 4)
    if let Some(base_name) = base {
        if base_name != name {
            ancestors.push(base_name.to_string());
        }
        if let Some(base_form) = hir.forms.get(base_name) {
            ancestor_fields.insert(base_name.to_string(), base_form.fields.clone());
            for (anc_k, anc_map) in &base_form.ancestor_fields {
                ancestor_fields.insert(anc_k.clone(), anc_map.clone());
            }

            // Карта подстановки параметров дженерика (`T -> CounterState`, `N -> Zero`)
            let mut subst = HashMap::new();
            for (param_raw, arg_val) in base_form.type_params.iter().zip(type_args.iter()) {
                let param_name = param_raw.split_whitespace().next().unwrap_or(param_raw);
                subst.insert(param_name.to_string(), arg_val.clone());
            }

            for k in &base_form.field_order {
                if let Some(f) = base_form.fields.get(k) {
                    let mut cloned = f.clone();
                    if let Some(concrete_type) = subst.get(&cloned.inferred_type) {
                        cloned.inferred_type = concrete_type.clone();
                    }
                    fields.insert(k.clone(), cloned);
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

    // 2. Собственные поля, вложенные подформы, реактивные потоки, методы и контракты внутри `{ ... }`
    for (f_idx, s) in body.iter().enumerate() {
        let member_origin = SourceOrigin {
            sentence_index: f_idx,
            symbol_context: format!("{name}.{f_idx}"),
            raw_ast: s.clone(),
        };

        match classify_sentence(s) {
            SemanticKind::FormDef {
                name: sub_name,
                body: sub_body,
                tail: sub_tail,
            } => {
                // Вложенная форма/модуль: `ComplexSystem { SubModule { Nested { ... } } }` (Раздел 17.5)
                let sub_form = lower_form(
                    &sub_name,
                    None,
                    vec![],
                    vec![],
                    &sub_body,
                    &sub_tail,
                    member_origin.clone(),
                    hir,
                );
                if !fields.contains_key(&sub_name) {
                    field_order.push(sub_name.clone());
                }
                fields.insert(
                    sub_name.clone(),
                    HirField {
                        name: sub_name.clone(),
                        inferred_type: "Form".into(),
                        origin_form: name.to_string(),
                        default_expr: vec![Token::Group('{', sub_body)],
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                        visibility: HirVisibility::Default,
                        origin: member_origin,
                    },
                );
                submodules.insert(sub_name, sub_form);
            }
            SemanticKind::Binding { key, value } => {
                let (clean_val, visibility) = strip_visibility_tail(&value);
                let inferred_type = infer_hir_type(&clean_val, hir);
                if !fields.contains_key(&key) {
                    field_order.push(key.clone());
                }
                fields.insert(
                    key.clone(),
                    HirField {
                        name: key,
                        inferred_type,
                        origin_form: name.to_string(),
                        default_expr: clean_val,
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                        visibility,
                        origin: member_origin,
                    },
                );
            }
            SemanticKind::TupleBinding { tuple, value } => {
                // Параметризованный ключ карты: `Map(K, V) { (K) V }`
                let key_str = tuple
                    .first()
                    .and_then(|sent| sent.first())
                    .map(|t| match t {
                        Token::Path(p) => format!("({p})"),
                        Token::Lit(Literal::String(s)) => s.clone(),
                        _ => "()".to_string(),
                    })
                    .unwrap_or_else(|| "()".to_string());
                let (clean_val, visibility) = strip_visibility_tail(&value);
                let inferred_type = infer_hir_type(&clean_val, hir);
                if !fields.contains_key(&key_str) {
                    field_order.push(key_str.clone());
                }
                fields.insert(
                    key_str.clone(),
                    HirField {
                        name: key_str,
                        inferred_type,
                        origin_form: name.to_string(),
                        default_expr: clean_val,
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                        visibility,
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
                        visibility: HirVisibility::Default,
                        origin: member_origin,
                    },
                );
            }
            SemanticKind::FunctionDef {
                name: method_name,
                args,
                body: fn_body,
                tail: fn_tail,
            } => {
                let (params, auto_unpacks) = lower_fn_params(&args, hir);
                let visibility = parse_visibility_from_tokens(&fn_tail);
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
                        return_type: None,
                        is_contract: false,
                        visibility,
                        origin: member_origin,
                    },
                );
            }
            SemanticKind::GenericDef {
                name: method_name,
                params: args,
                body: ret_tokens,
            } => {
                // Контракт метода внутри формы: `Find(Id String) User | None`
                let (params, auto_unpacks) = lower_fn_params(&args, hir);
                let ret_type = infer_hir_type(&ret_tokens, hir);
                if !methods.contains_key(&method_name) {
                    method_order.push(method_name.clone());
                }
                methods.insert(
                    method_name.clone(),
                    HirFunction {
                        name: method_name,
                        params,
                        auto_unpacks,
                        body: vec![],
                        return_type: Some(ret_type),
                        is_contract: true,
                        visibility: HirVisibility::Default,
                        origin: member_origin,
                    },
                );
            }
            SemanticKind::OperatorMethod { op, args, rest } => {
                let (params, auto_unpacks) = lower_fn_params(&args, hir);
                let (fn_body, return_type, is_contract) = match rest.first() {
                    Some(Token::Group('[', b)) => (b.clone(), None, false),
                    _ => (vec![], Some(infer_hir_type(&rest, hir)), true),
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
                        return_type,
                        is_contract,
                        visibility: HirVisibility::Default,
                        origin: member_origin,
                    },
                );
            }
            _ => {
                // Поле с алгебраическим типом (`Role 'admin' | 'manager'` или `Salt "x9f2a" & Private`)
                if let [Token::Path(field_name), rest @ ..] = s.as_slice() {
                    if !rest.is_empty() && !has_pipe_op(rest) {
                        let (clean_val, visibility) = strip_visibility_tail(rest);
                        let inferred_type = infer_hir_type(&clean_val, hir);
                        if !fields.contains_key(field_name) {
                            field_order.push(field_name.clone());
                        }
                        fields.insert(
                            field_name.clone(),
                            HirField {
                                name: field_name.clone(),
                                inferred_type,
                                origin_form: name.to_string(),
                                default_expr: clean_val,
                                is_reactive: false,
                                is_ambiguous: false,
                                conflicting_origins: vec![],
                                visibility,
                                origin: member_origin,
                            },
                        );
                    }
                }
            }
        }
    }

    // Сохраняем снимок собственных полей до наложения `& { ... }` (для случая `JonnDoyName2 JonnName { ... } & { ... }`)
    ancestor_fields.insert(name.to_string(), fields.clone());

    // 3. Разрешение алгебры типов `&`, `|` и `/` в статику!
    if !tail.is_empty() {
        resolve_type_algebra_into_fields(
            name,
            &mut fields,
            &mut field_order,
            &mut ancestor_fields,
            &mut methods,
            &mut method_order,
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
        type_params,
        type_args,
        fields,
        field_order,
        ancestor_fields,
        methods,
        method_order,
        submodules,
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
    let mut ancestor_fields: HashMap<String, HashMap<String, HirField>> = HashMap::new();
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
                    ancestor_fields.insert(first_type.clone(), base_form.fields.clone());
                    for (anc_k, anc_map) in &base_form.ancestor_fields {
                        ancestor_fields.insert(anc_k.clone(), anc_map.clone());
                    }
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
                    if !base_form.union_variants.is_empty() {
                        union_variants.extend(base_form.union_variants.clone());
                    }
                } else {
                    union_variants.push(first_type.clone());
                }
            }
            Token::Lit(Literal::Symbol(sym)) => {
                union_variants.push(format!("'{sym}'"));
            }
            Token::Range(start, end) => {
                union_variants.push(format!(
                    "{}..{}",
                    format_single_token(start),
                    format_single_token(end)
                ));
            }
            _ => {}
        }

        if expr.len() > 1 {
            // Если первый токен был вызовом конструктора дженерика `Num(Int) | Add(Expr, Expr)`
            let start_idx = if matches!(expr.get(1), Some(Token::Group(')', _))) {
                if let Token::Path(first_type) = first_tok {
                    if !union_variants.contains(first_type) {
                        union_variants.push(first_type.clone());
                    }
                }
                2
            } else {
                1
            };

            if start_idx < expr.len() {
                resolve_type_algebra_into_fields(
                    name,
                    &mut fields,
                    &mut field_order,
                    &mut ancestor_fields,
                    &mut methods,
                    &mut method_order,
                    &mut ancestors,
                    &mut union_variants,
                    &mut refinement_predicates,
                    &expr[start_idx..],
                    &origin,
                    hir,
                );
            }
        }
    }

    HirForm {
        name: name.to_string(),
        type_params: vec![],
        type_args: vec![],
        fields,
        field_order,
        ancestor_fields,
        methods,
        method_order,
        submodules: HashMap::new(),
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
    ancestor_fields: &mut HashMap<String, HashMap<String, HirField>>,
    methods: &mut HashMap<String, HirFunction>,
    method_order: &mut Vec<String>,
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
                        Token::Path(rhs_type) if rhs_type == "Public" => {
                            // Инкапсуляция модуля `AuthModule AuthInternal & Public`:
                            // оставляем только поля и методы с видимостью `Public`
                            fields.retain(|_, f| f.visibility == HirVisibility::Public);
                            field_order.retain(|k| fields.contains_key(k));
                            methods.retain(|_, m| m.visibility == HirVisibility::Public);
                            method_order.retain(|k| methods.contains_key(k));
                        }
                        Token::Path(rhs_type) => {
                            if let Some(rhs_form) = hir.forms.get(rhs_type) {
                                if !ancestors.contains(rhs_type) {
                                    ancestors.push(rhs_type.clone());
                                }
                                ancestor_fields.insert(rhs_type.clone(), rhs_form.fields.clone());
                                for (anc_k, anc_map) in &rhs_form.ancestor_fields {
                                    ancestor_fields.insert(anc_k.clone(), anc_map.clone());
                                }
                                for anc in &rhs_form.ancestors {
                                    if !ancestors.contains(anc) {
                                        ancestors.push(anc.clone());
                                    }
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
                                for m in &rhs_form.method_order {
                                    if let Some(rhs_m) = rhs_form.methods.get(m) {
                                        if !methods.contains_key(m) {
                                            method_order.push(m.clone());
                                            methods.insert(m.clone(), rhs_m.clone());
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
                                match classify_sentence(s) {
                                    SemanticKind::Binding { key, value } => {
                                        let (clean_val, visibility) = strip_visibility_tail(&value);
                                        let t = infer_hir_type(&clean_val, hir);
                                        if !fields.contains_key(&key) {
                                            field_order.push(key.clone());
                                        }
                                        fields.insert(
                                            key.clone(),
                                            HirField {
                                                name: key,
                                                inferred_type: t,
                                                origin_form: owner_name.to_string(),
                                                default_expr: clean_val,
                                                is_reactive: false,
                                                is_ambiguous: false,
                                                conflicting_origins: vec![],
                                                visibility,
                                                origin: owner_origin.clone(),
                                            },
                                        );
                                    }
                                    SemanticKind::FunctionDef {
                                        name: method_name,
                                        args,
                                        body: fn_body,
                                        tail: fn_tail,
                                    } => {
                                        let (params, auto_unpacks) = lower_fn_params(&args, hir);
                                        let visibility = parse_visibility_from_tokens(&fn_tail);
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
                                                return_type: None,
                                                is_contract: false,
                                                visibility,
                                                origin: owner_origin.clone(),
                                            },
                                        );
                                    }
                                    _ => {}
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
                            if !union_variants.contains(variant) {
                                union_variants.push(variant.clone());
                            }
                            if matches!(tail.get(i + 2), Some(Token::Group(')', _))) {
                                i += 3;
                                continue;
                            }
                        }
                        Token::Lit(Literal::Symbol(sym)) => {
                            let formatted = format!("'{sym}'");
                            if !union_variants.contains(&formatted) {
                                union_variants.push(formatted);
                            }
                        }
                        Token::Range(start, end) => {
                            let formatted = format!(
                                "{}..{}",
                                format_single_token(start),
                                format_single_token(end)
                            );
                            if !union_variants.contains(&formatted) {
                                union_variants.push(formatted);
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
                            methods.remove(sym_key);
                            method_order.retain(|m| m != sym_key);
                        }
                        Token::Path(path_str) => {
                            if let Some((anc, field_key)) = path_str.split_once('.') {
                                if let Some(f) = fields.get_mut(field_key) {
                                    if f.is_ambiguous {
                                        f.conflicting_origins.retain(|o| o != anc);
                                        if f.conflicting_origins.len() == 1 {
                                            let winning_anc = f.conflicting_origins[0].clone();
                                            f.is_ambiguous = false;
                                            f.origin_form = winning_anc.clone();
                                            if let Some(anc_map) = ancestor_fields.get(&winning_anc)
                                            {
                                                if let Some(winning_field) = anc_map.get(field_key)
                                                {
                                                    *f = winning_field.clone();
                                                }
                                            }
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
                                union_variants.retain(|v| v != path_str);
                                fields.retain(|_, f| f.origin_form != *path_str);
                                for (fname, f) in fields.iter_mut() {
                                    if f.is_ambiguous {
                                        f.conflicting_origins.retain(|o| o != path_str);
                                        if f.conflicting_origins.len() == 1 {
                                            let winning_anc = f.conflicting_origins[0].clone();
                                            f.is_ambiguous = false;
                                            f.origin_form = winning_anc.clone();
                                            if let Some(anc_map) = ancestor_fields.get(&winning_anc)
                                            {
                                                if let Some(winning_field) = anc_map.get(fname) {
                                                    *f = winning_field.clone();
                                                }
                                            }
                                        }
                                    }
                                }
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
            // Закон 1c: Анонимный пересеченный слот `FinalizeBill(User & Cart)` или `ReserveStock(Order / Cancelled)`
            [Token::Path(first_type), Token::Op(op), ..] if op == "&" || op == "|" || op == "/" => {
                let constraint = format_tokens_as_type(sent);
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
            // Именованный параметр с типом `Fn(U User)`, `Fn(O Order / Cancelled)` или с дефолтным значением `Connect(Host "localhost")`
            [Token::Path(param_name), rest @ ..] => {
                let is_default_lit =
                    matches!(rest.first(), Some(Token::Lit(_) | Token::Group(_, _)));
                let constraint = if is_default_lit {
                    infer_hir_type(rest, hir)
                } else {
                    format_tokens_as_type(rest)
                };
                out.push(HirParam {
                    name: param_name.clone(),
                    type_constraint: constraint,
                    default_expr: if is_default_lit {
                        rest.to_vec()
                    } else {
                        vec![]
                    },
                    is_anonymous_type_slot: false,
                });
            }
            _ => {}
        }
    }

    (out, auto_unpacks)
}

fn format_single_token(tok: &Token) -> String {
    match tok {
        Token::Lit(Literal::String(s)) => format!("\"{s}\""),
        Token::Lit(Literal::Symbol(s)) => format!("'{s}'"),
        Token::Lit(Literal::Int(i)) => i.to_string(),
        Token::Path(p) => p.clone(),
        _ => "...".into(),
    }
}

fn format_tokens_as_type(tokens: &[Token]) -> String {
    let mut out = String::new();
    for tok in tokens {
        match tok {
            Token::Path(p) => {
                if !out.is_empty() && !out.ends_with('(') {
                    out.push(' ');
                }
                out.push_str(p);
            }
            Token::Op(o) => {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(o);
            }
            Token::Group(')', inner) => {
                let inner_str = inner
                    .iter()
                    .map(|s| format_tokens_as_type(s))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push('(');
                out.push_str(&inner_str);
                out.push(')');
            }
            Token::Lit(Literal::Symbol(s)) => {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(&format!("'{s}'"));
            }
            Token::Range(a, b) => {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(&format!(
                    "{}..{}",
                    format_single_token(a),
                    format_single_token(b)
                ));
            }
            _ => {}
        }
    }
    out
}

pub fn infer_hir_type(tokens: &[Token], hir: &HirModule) -> String {
    if tokens.is_empty() {
        return "Any".into();
    }

    if has_algebra_op(tokens) && !has_pipe_op(tokens) {
        if tokens
            .iter()
            .any(|t| matches!(t, Token::Op(op) if op == "|"))
        {
            return format_tokens_as_type(tokens);
        }
    }

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
            if matches!(tokens.get(1), Some(Token::Group(')', _))) {
                format_tokens_as_type(tokens)
            } else if hir.forms.contains_key(p) {
                p.clone()
            } else {
                p.clone()
            }
        }
        Some(Token::Range(_, _)) => "Range".into(),
        Some(Token::Group('(', inner)) => {
            if inner.len() == 1 {
                let sent = &inner[0];
                if sent.iter().any(|t| {
                    matches!(
                        t,
                        Token::Op(op)
                            if matches!(
                                op.as_str(),
                                "==" | "!=" | ">" | "<" | ">=" | "<=" | "&&" | "||" | "!"
                            )
                    )
                }) {
                    return "Bool".into();
                }
            }
            "Tuple".into()
        }
        Some(Token::Group('{', _)) => "Form".into(),
        Some(Token::Group('[', _)) => "Stream".into(),
        _ => "Any".into(),
    }
}
