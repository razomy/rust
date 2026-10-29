// src/hir.rs
// =============================================================================
// RAZOMY V5.7 — HIGH-LEVEL INTERMEDIATE REPRESENTATION (HIR)
// Переводит динамический синтаксис AST в разрешенную статическую модель,
// сохраняя 100% обратную связь с исходным AST (SourceOrigin).
// =============================================================================

use std::collections::HashMap;

use crate::parser::{
    classify_sentence, has_algebra_op, has_pipe_op, Literal, SemanticKind, Sentence, Token,
};

// =============================================================================
// 1. ОБРАТНАЯ СВЯЗЬ С AST И ИСХОДНЫМ КОДОМ (SOURCE ORIGIN) & КОНВЕРТАЦИЯ РЕГИСТРА
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

/// Закон 1c (V5.7): Преобразует `PascalCase` имя типа в `snake_case` имя переменной
/// (`Age` -> `age`, `WithName` -> `with_name`, `FileHandle` -> `file_handle`).
pub fn pascal_to_snake_case(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn is_pascal_case(s: &str) -> bool {
    s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
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
    /// Параметры типа для дженериков (`Map(K, V)`, `Functor(F(_))`, `CategorizedUser(TC Category, active_cat TC)`)
    pub type_params: Vec<String>,
    /// Слоты-значения конструктора формы (`MathOps(a Int, b Int)`, `UserWithAge(Age)`), которые становятся полями на `self` (Law 1a/1c)
    pub value_params: Vec<HirParam>,
    /// Аргументы типа при инстанцировании дженерика (`Atom(CounterState)`, `Succ(Zero)`)
    pub type_args: Vec<String>,
    /// Полностью разрешенная таблица активных полей (после применения всех `&` и `/`)
    pub fields: HashMap<String, HirField>,
    pub field_order: Vec<String>,
    /// История полей каждого предка для Lineage-доступа (`a.Parent.field`, Law 2)
    pub ancestor_fields: HashMap<String, HashMap<String, HirField>>,
    /// Методы, контракты и перегруженные операторы внутри формы (`Vec2.+`, `Db.find_user`)
    pub methods: HashMap<String, HirFunction>,
    pub method_order: Vec<String>,
    /// Вложенные подмодули/подформы (`ComplexSystem.SubModule.Nested`)
    pub submodules: HashMap<String, HirForm>,
    /// Полный список предков (Lineage)
    pub ancestors: Vec<String>,
    /// Ветки объединения (`Circle | Rect`, `'draft' | 'published'`, `"a".."z" | "A".."Z"`)
    pub union_variants: Vec<String>,
    /// Предикаты уточнения (`User & is_adult`, `CompileError & [ _.code >= 500 ]`)
    pub refinement_predicates: Vec<Token>,
    /// Закон 3: Помечена ли форма тегом `& Bypass` (`None`, `NotFound`, `IoError`)
    pub is_bypass: bool,
    pub is_algebraic_alias: bool,
    /// Помечена ли форма как встроенная из `src/prelude.rl` (не эмитится в пользовательский кодоген, если не переопределена)
    pub is_prelude: bool,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirParam {
    /// Имя связываемой переменной (для `fn(Age)` -> `age`, для `fn(u User)` -> `u`, для `fn({ name } User)` -> `_destruct_0`)
    pub name: String,
    pub type_constraint: String,
    pub default_expr: Vec<Token>,
    /// Закон 1c: true для `fn(Age)` или `UserWithAge(Age)` (одиночный известный тип в `PascalCase`)
    pub is_anonymous_type_slot: bool,
    /// Чистый compile-time параметр типа в функции: `fmap(A, B, fa F(A))`
    pub is_type_param: bool,
    /// Деструктурируемые поля параметра: `read_unpacked({ name } WithName)`
    pub destructured_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HirFunction {
    pub name: String,
    pub params: Vec<HirParam>,
    /// Закон 1c: true, если есть хотя бы один авто-привязанный слот типа `fn(User)` -> `user`
    pub auto_unpacks: bool,
    pub body: Vec<Sentence>,
    /// Явный возвращаемый тип (`find_user(id String) User | NotFound [...]` или контракт `< (other Other) Bool`)
    pub return_type: Option<String>,
    pub is_contract: bool,
    /// Помечена ли функция тегом `& Compile`
    pub is_compile_time: bool,
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
    /// Источник (`root.base`, `Path"../../../base"`, `std.net.http`)
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

pub const PRELUDE_SOURCE: &str = include_str!("prelude.rl");

pub fn lower_to_hir(sentences: &[Sentence]) -> HirModule {
    let mut hir = HirModule {
        top_level_sentences: sentences.to_vec(),
        ..Default::default()
    };

    register_stdlib_hir(&mut hir);

    for (idx, sentence) in sentences.iter().enumerate() {
        lower_sentence_into_hir(idx, sentence, &mut hir);
    }

    hir
}

fn lower_sentence_into_hir(idx: usize, sentence: &Sentence, hir: &mut HirModule) {
    let kind = classify_sentence(sentence);
    match kind {
        SemanticKind::FormDef { name, body, tail } => {
            let origin = SourceOrigin {
                sentence_index: idx,
                symbol_context: name.clone(),
                raw_ast: sentence.clone(),
            };
            let mut expr = vec![Token::Group('{', body)];
            expr.extend(tail);
            let form = lower_form_expr(&name, vec![], vec![], &expr, origin, hir);
            register_form(hir, name, form);
        }

        SemanticKind::GenericDef { name, params, body } => {
            let origin = SourceOrigin {
                sentence_index: idx,
                symbol_context: name.clone(),
                raw_ast: sentence.clone(),
            };
            let (type_params, value_params) = extract_form_slots(&params, hir);
            let form = lower_form_expr(&name, type_params, value_params, &body, origin, hir);
            register_form(hir, name, form);
        }

        SemanticKind::AlgebraicDef { name, expr } => {
            let origin = SourceOrigin {
                sentence_index: idx,
                symbol_context: name.clone(),
                raw_ast: sentence.clone(),
            };

            if is_stream_composition(&expr, hir) {
                register_composed_stream_fn(hir, &name, &expr, origin.clone());
            }

            if !is_pascal_case(&name) {
                let (clean_expr, _) = strip_visibility_and_compile_tail(&expr);
                let inferred_type = infer_hir_type(&clean_expr, hir);
                register_global(hir, name.clone(), inferred_type, clean_expr, origin.clone());
            }

            let form = lower_form_expr(&name, vec![], vec![], &expr, origin, hir);
            register_form(hir, name, form);
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
            let func = build_hir_function(&name, &args, body, &tail, false, origin, hir);
            register_function(hir, name, func);
        }

        SemanticKind::StreamOrBranch { name, stream } => {
            let origin = SourceOrigin {
                sentence_index: idx,
                symbol_context: name.clone(),
                raw_ast: sentence.clone(),
            };
            let func = build_hir_function(&name, &[], stream, &[], false, origin, hir);
            register_function(hir, name, func);
        }

        SemanticKind::Binding { key, value } => {
            let origin = SourceOrigin {
                sentence_index: idx,
                symbol_context: key.clone(),
                raw_ast: sentence.clone(),
            };

            // 1. Проверяем импорт (`_ root.base`, `_ Path"../../../base"`, `Http std.net.http`)
            if let Some(import_source) = detect_import_source(&key, &value) {
                hir.imports.push(HirImport {
                    targets: vec![key.clone()],
                    source_path: import_source,
                    origin: origin.clone(),
                });
            }

            // 2. Обработка `Product (Compile & [ { id Int, title String } . derive_json ])` или алиасов типов `Age Int`, `One Succ(Zero)`
            match value.as_slice() {
                [Token::Path(base_type)]
                    if is_pascal_case(&key) && is_known_type(base_type, hir) =>
                {
                    let form = lower_form_expr(&key, vec![], vec![], &value, origin.clone(), hir);
                    register_form(hir, key.clone(), form);
                }
                [Token::Path(generic_base), Token::Group(')', _)]
                    if is_pascal_case(&key) && hir.forms.contains_key(generic_base) =>
                {
                    let form = lower_form_expr(&key, vec![], vec![], &value, origin.clone(), hir);
                    register_form(hir, key.clone(), form);
                }
                [Token::Group('(', inner)] if inner.len() == 1 => {
                    let inner_sent = &inner[0];
                    if let [Token::Path(c), Token::Op(amp), Token::Group('[', stream_sents)] =
                        inner_sent.as_slice()
                    {
                        if c == "Compile" && amp == "&" {
                            if let Some([Token::Group('{', body), Token::Op(dot), Token::Path(macro_name)]) =
                                stream_sents.first().map(Vec::as_slice)
                            {
                                if dot == "." {
                                    let mut expr = vec![Token::Group('{', body.clone())];
                                    expr.extend(extract_macro_tail(macro_name, hir));
                                    let form = lower_form_expr(
                                        &key,
                                        vec![],
                                        vec![],
                                        &expr,
                                        origin.clone(),
                                        hir,
                                    );
                                    register_form(hir, key.clone(), form);
                                }
                            }
                        }
                    } else if has_algebra_op(inner_sent) {
                        if is_pascal_case(&key) {
                            let form = lower_form_expr(
                                &key,
                                vec![],
                                vec![],
                                inner_sent,
                                origin.clone(),
                                hir,
                            );
                            register_form(hir, key.clone(), form);
                        } else if is_stream_composition(inner_sent, hir) {
                            register_composed_stream_fn(hir, &key, inner_sent, origin.clone());
                        }
                    }
                }
                _ => {}
            }

            let inferred_type = infer_hir_type(&value, hir);
            register_global(hir, key, inferred_type, value, origin);
        }

        SemanticKind::TupleBinding { tuple, value } => {
            let origin = SourceOrigin {
                sentence_index: idx,
                symbol_context: "<tuple_binding>".into(),
                raw_ast: sentence.clone(),
            };
            let inferred_type = infer_hir_type(&value, hir);
            let mut targets = Vec::new();
            for elem_sent in tuple {
                if let [Token::Path(var_name)] = elem_sent.as_slice() {
                    targets.push(var_name.clone());
                    register_global(
                        hir,
                        var_name.clone(),
                        inferred_type.clone(),
                        value.clone(),
                        origin.clone(),
                    );
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
                    let mut expr = vec![Token::Group('{', body.clone())];
                    expr.extend(extract_macro_tail(macro_name, hir));
                    let form = lower_form_expr(name, vec![], vec![], &expr, origin, hir);
                    register_form(hir, name.clone(), form);
                }
            }
        }

        _ => {}
    }
}

fn register_function(hir: &mut HirModule, name: String, func: HirFunction) {
    if !hir.function_order.contains(&name) {
        hir.function_order.push(name.clone());
    }
    hir.functions.insert(name, func);
}

fn register_global(
    hir: &mut HirModule,
    key: String,
    inferred_type: String,
    value_expr: Vec<Token>,
    origin: SourceOrigin,
) {
    let binding = HirGlobalBinding {
        key: key.clone(),
        inferred_type,
        value_expr,
        origin,
    };
    if !hir.global_order.contains(&key) {
        hir.global_order.push(key.clone());
    }
    hir.globals.insert(key, binding);
}

fn register_composed_stream_fn(
    hir: &mut HirModule,
    name: &str,
    expr: &[Token],
    origin: SourceOrigin,
) {
    let mut composed_steps = vec![Token::Path("_".into()), Token::Op(".".into())];
    for t in expr {
        match t {
            Token::Op(op) if op == "&" => composed_steps.push(Token::Op(".".into())),
            other => composed_steps.push(other.clone()),
        }
    }
    let func = build_hir_function(name, &[], vec![composed_steps], &[], false, origin, hir);
    register_function(hir, name.to_string(), func);
}

fn build_hir_function(
    name: &str,
    args: &[Sentence],
    body: Vec<Sentence>,
    tail: &[Token],
    is_contract: bool,
    origin: SourceOrigin,
    hir: &HirModule,
) -> HirFunction {
    let (params, auto_unpacks) = lower_fn_params(args, hir);
    let (ret_tokens, visibility, is_compile_time) = parse_fn_tail_metadata(tail, hir);
    let return_type = if ret_tokens.is_empty() {
        None
    } else {
        Some(format_tokens_as_type(&ret_tokens))
    };
    HirFunction {
        name: name.to_string(),
        params,
        auto_unpacks,
        body,
        return_type,
        is_contract,
        is_compile_time,
        visibility,
        origin,
    }
}

fn extract_macro_tail(macro_name: &str, hir: &HirModule) -> Vec<Token> {
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
    macro_tail
}

fn is_stream_composition(expr: &[Token], hir: &HirModule) -> bool {
    if let Some(Token::Path(first)) = expr.first() {
        if hir.functions.contains_key(first)
            && expr
            .iter()
            .any(|t| matches!(t, Token::Op(op) if op == "&"))
            && !expr
            .iter()
            .any(|t| matches!(t, Token::Group('{', _)))
        {
            return true;
        }
    }
    false
}

fn detect_import_source(key: &str, value: &[Token]) -> Option<String> {
    match value {
        // `_ Path"../../../base"` desugars to `[Path("Path"), Group(')', [[Lit(String("../../../base"))]])]`
        [Token::Path(p), Token::Group(')', args)] if key == "_" && p == "Path" => {
            if let Some([Token::Lit(Literal::String(s) | Literal::RawString(s))]) =
                args.first().map(Vec::as_slice)
            {
                return Some(s.clone());
            }
            None
        }
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
    hir.form_order.retain(|n| n != &name);
    hir.form_order.push(name.clone());
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

/// Разделяет слоты параметризованной формы `Name(Slots) { ... }` по Universal Slot Rule (V5.7):
/// • Непривязанный `PascalCase` (`T`, `F(_)`, `TC Category`) -> Type Parameter
/// • Одиночный известный тип в скоупе (`Age` в `UserWithAge(Age)`) -> Value Param `age: Age` (Law 1c)
/// • `snake_case` слот (`a Int`, `raw String "0.00"`) -> Value Param (и поле на `self` по Law 1a)
fn extract_form_slots(params: &[Sentence], hir: &HirModule) -> (Vec<String>, Vec<HirParam>) {
    let mut type_params = Vec::new();
    let mut value_params = Vec::new();

    for sent in params {
        match sent.as_slice() {
            // HKT 2-го или 3-го порядка: `F(_)`, `T(_(_), _)` или `M(_) Monad`
            [Token::Path(name), Token::Group(')', _), ..] if is_pascal_case(name) => {
                type_params.push(name.clone());
            }
            // Одиночный идентификатор: `Box(T)` vs `UserWithAge(Age)`
            [Token::Path(single)] => {
                if is_pascal_case(single) {
                    if is_known_type(single, hir) {
                        value_params.push(HirParam {
                            name: pascal_to_snake_case(single),
                            type_constraint: single.clone(),
                            default_expr: vec![],
                            is_anonymous_type_slot: true,
                            is_type_param: false,
                            destructured_fields: vec![],
                        });
                    } else {
                        type_params.push(single.clone());
                    }
                } else {
                    value_params.push(HirParam {
                        name: single.clone(),
                        type_constraint: "Any".into(),
                        default_expr: vec![],
                        is_anonymous_type_slot: false,
                        is_type_param: false,
                        destructured_fields: vec![],
                    });
                }
            }
            // Ограниченный параметр типа `TC Category` (`PascalCase` + `PascalCase`)
            [Token::Path(tp), Token::Path(bound)]
            if is_pascal_case(tp) && is_pascal_case(bound) =>
                {
                    type_params.push(tp.clone());
                }
            // Явный слот значения `a Int` или с дефолтом `raw String "0.00"` (`snake_case`)
            [Token::Path(param_name), rest @ ..] => {
                let (constraint, def_expr) = split_type_and_default(rest, hir);
                value_params.push(HirParam {
                    name: param_name.clone(),
                    type_constraint: constraint,
                    default_expr: def_expr,
                    is_anonymous_type_slot: false,
                    is_type_param: false,
                    destructured_fields: vec![],
                });
            }
            _ => {}
        }
    }

    (type_params, value_params)
}

pub fn is_known_type(name: &str, hir: &HirModule) -> bool {
    hir.forms.contains_key(name)
        || matches!(
            name,
            "String"
                | "Symbol"
                | "Int"
                | "Float"
                | "Bool"
                | "Byte"
                | "Any"
                | "Tuple"
                | "Self"
                | "Form"
                | "Stream"
        )
}

/// Разделяет `Int 180` или `String "0.00"` или `'default'` на `(inferred_type, default_expr)`
fn split_type_and_default(tokens: &[Token], hir: &HirModule) -> (String, Vec<Token>) {
    match tokens {
        // Explicit type + default value: `max_speed Int 180`, `label Symbol 'default'`, `items Tuple(T) ()`
        [Token::Path(ty), rest @ ..]
        if is_pascal_case(ty)
            && !rest.is_empty()
            && !matches!(rest.first(), Some(Token::Op(_) | Token::Group(')', _))) =>
            {
                (ty.clone(), rest.to_vec())
            }
        [Token::Path(ty), Token::Group(')', targs), rest @ ..]
        if is_pascal_case(ty)
            && !rest.is_empty()
            && !matches!(rest.first(), Some(Token::Op(_))) =>
            {
                let type_slice = &tokens[..2];
                let _ = targs;
                (format_tokens_as_type(type_slice), rest.to_vec())
            }
        // Only type: `brand BrandName`, `id String`, `role 'admin' | 'user'`
        [Token::Path(ty)] if is_pascal_case(ty) && is_known_type(ty, hir) => {
            (ty.clone(), tokens.to_vec())
        }
        _ => {
            let is_lit_or_expr = matches!(
                tokens.first(),
                Some(Token::Lit(_) | Token::Group(_, _))
            ) || (tokens.len() >= 2
                && matches!(&tokens[0], Token::Path(p) if is_pascal_case(p))
                && matches!(&tokens[1], Token::Group(')', _)));
            if is_lit_or_expr {
                (infer_hir_type(tokens, hir), tokens.to_vec())
            } else {
                (format_tokens_as_type(tokens), tokens.to_vec())
            }
        }
    }
}

fn parse_fn_tail_metadata(
    tail: &[Token],
    _hir: &HirModule,
) -> (Vec<Token>, HirVisibility, bool) {
    let mut vis = HirVisibility::Default;
    let mut is_compile = false;
    let mut ret_tokens = Vec::new();

    let mut i = 0;
    while i < tail.len() {
        if i + 1 < tail.len() {
            if let (Token::Op(op), Token::Path(tag)) = (&tail[i], &tail[i + 1]) {
                if op == "&" {
                    match tag.as_str() {
                        "Private" => {
                            vis = HirVisibility::Private;
                            i += 2;
                            continue;
                        }
                        "Public" => {
                            vis = HirVisibility::Public;
                            i += 2;
                            continue;
                        }
                        "Compile" => {
                            is_compile = true;
                            i += 2;
                            continue;
                        }
                        _ => {}
                    }
                }
            }
        }
        ret_tokens.push(tail[i].clone());
        i += 1;
    }
    (ret_tokens, vis, is_compile)
}

fn strip_visibility_and_compile_tail(tokens: &[Token]) -> (Vec<Token>, HirVisibility) {
    let mut vis = HirVisibility::Default;
    let mut cleaned = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if i + 1 < tokens.len() {
            if let (Token::Op(op), Token::Path(trait_name)) = (&tokens[i], &tokens[i + 1]) {
                if op == "&" {
                    if trait_name == "Private" {
                        vis = HirVisibility::Private;
                        i += 2;
                        continue;
                    } else if trait_name == "Public" {
                        vis = HirVisibility::Public;
                        i += 2;
                        continue;
                    } else if trait_name == "Compile" {
                        i += 2;
                        continue;
                    }
                }
            }
        }
        cleaned.push(tokens[i].clone());
        i += 1;
    }
    (cleaned, vis)
}

fn register_stdlib_hir(hir: &mut HirModule) {
    if let Ok(prelude_sentences) = crate::parser::parse_code(PRELUDE_SOURCE) {
        for (idx, sentence) in prelude_sentences.iter().enumerate() {
            lower_sentence_into_hir(idx, sentence, hir);
        }
        for form in hir.forms.values_mut() {
            form.is_prelude = true;
        }
    }
}

/// Единый понижатель всех аксиом форм (`Name { ... }`, `Name(Slots) ...`, `Name Base & { ... }`, `Name A | B`, `Name A / B`)
fn lower_form_expr(
    name: &str,
    type_params: Vec<String>,
    value_params: Vec<HirParam>,
    expr: &[Token],
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
    let mut type_args = Vec::new();
    let mut is_bypass = name == "Bypass";

    // 0. Закон 1a/1c: Слоты значений конструктора `Form(a Int, b Int)` автоматически становятся полями на `self`!
    for vp in &value_params {
        insert_hir_field(
            &mut fields,
            &mut field_order,
            HirField {
                name: vp.name.clone(),
                inferred_type: vp.type_constraint.clone(),
                origin_form: name.to_string(),
                default_expr: vp.default_expr.clone(),
                is_reactive: false,
                is_ambiguous: false,
                conflicting_origins: vec![],
                visibility: HirVisibility::Default,
                origin: origin.clone(),
            },
        );
    }

    // Разворачиваем ведущую скобку `(PlayButton / Cuboid) & { ... }`, если она есть
    let expanded_expr: Vec<Token>;
    let expr_slice: &[Token] = if let Some(Token::Group('(', inner)) = expr.first() {
        if inner.len() == 1 {
            let mut v = inner[0].clone();
            v.extend_from_slice(&expr[1..]);
            expanded_expr = v;
            &expanded_expr
        } else {
            expr
        }
    } else {
        expr
    };

    if let Some(first_tok) = expr_slice.first() {
        let mut start_idx = 1;
        match first_tok {
            Token::Group('{', body) => {
                populate_hir_form_members(
                    name,
                    body,
                    &mut fields,
                    &mut field_order,
                    &mut methods,
                    &mut method_order,
                    Some(&mut submodules),
                    &origin,
                    hir,
                );
            }
            Token::Path(first_type) => {
                if let Some(Token::Group(')', args)) = expr_slice.get(1) {
                    type_args = extract_type_params(args);
                    start_idx = 2;
                }
                if first_type == "Bypass" {
                    is_bypass = true;
                }
                if let Some(base_form) = hir.forms.get(first_type) {
                    if first_type != name {
                        ancestors.push(first_type.clone());
                    }
                    inherit_from_base_form(
                        first_type,
                        base_form,
                        &type_args,
                        &mut fields,
                        &mut field_order,
                        &mut ancestor_fields,
                        &mut methods,
                        &mut method_order,
                        &mut ancestors,
                        &mut is_bypass,
                    );
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
            _ => {
                start_idx = 0;
            }
        }

        if start_idx < expr_slice.len() {
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
                &mut is_bypass,
                &expr_slice[start_idx..],
                &origin,
                hir,
            );
        }
    }

    ancestor_fields.insert(name.to_string(), fields.clone());
    let has_own_body_fields = expr_slice.iter().any(|t| matches!(t, Token::Group('{', _)));

    HirForm {
        name: name.to_string(),
        type_params,
        value_params,
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
        is_bypass,
        is_algebraic_alias: !has_own_body_fields,
        is_prelude: false,
        origin,
    }
}

#[allow(clippy::too_many_arguments)]
fn inherit_from_base_form(
    base_name: &str,
    base_form: &HirForm,
    type_args: &[String],
    fields: &mut HashMap<String, HirField>,
    field_order: &mut Vec<String>,
    ancestor_fields: &mut HashMap<String, HashMap<String, HirField>>,
    methods: &mut HashMap<String, HirFunction>,
    method_order: &mut Vec<String>,
    ancestors: &mut Vec<String>,
    is_bypass: &mut bool,
) {
    if base_form.is_bypass {
        *is_bypass = true;
    }
    ancestor_fields.insert(base_name.to_string(), base_form.fields.clone());
    for (anc_k, anc_map) in &base_form.ancestor_fields {
        ancestor_fields.insert(anc_k.clone(), anc_map.clone());
    }

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
            insert_hir_field(fields, field_order, cloned);
        }
    }
    for m in &base_form.method_order {
        if let Some(mf) = base_form.methods.get(m) {
            insert_hir_method(methods, method_order, mf.clone());
        }
    }
    for anc in &base_form.ancestors {
        if !ancestors.contains(anc) {
            ancestors.push(anc.clone());
        }
    }
}

fn insert_hir_field(
    fields: &mut HashMap<String, HirField>,
    field_order: &mut Vec<String>,
    field: HirField,
) {
    if !fields.contains_key(&field.name) {
        field_order.push(field.name.clone());
    }
    fields.insert(field.name.clone(), field);
}

fn insert_hir_method(
    methods: &mut HashMap<String, HirFunction>,
    method_order: &mut Vec<String>,
    func: HirFunction,
) {
    if !methods.contains_key(&func.name) {
        method_order.push(func.name.clone());
    }
    methods.insert(func.name.clone(), func);
}

/// Единый обработчик членов тела формы `{ ... }` (используется и при объявлении формы, и при `& { ... }` слиянии)
#[allow(clippy::too_many_arguments)]
fn populate_hir_form_members(
    owner_name: &str,
    sentences: &[Sentence],
    fields: &mut HashMap<String, HirField>,
    field_order: &mut Vec<String>,
    methods: &mut HashMap<String, HirFunction>,
    method_order: &mut Vec<String>,
    mut submodules: Option<&mut HashMap<String, HirForm>>,
    owner_origin: &SourceOrigin,
    hir: &HirModule,
) {
    for (f_idx, s) in sentences.iter().enumerate() {
        let member_origin = SourceOrigin {
            sentence_index: f_idx,
            symbol_context: format!("{owner_name}.{f_idx}"),
            raw_ast: s.clone(),
        };

        match classify_sentence(s) {
            SemanticKind::FormDef {
                name: sub_name,
                body: sub_body,
                tail: sub_tail,
            } => {
                let mut sub_expr = vec![Token::Group('{', sub_body.clone())];
                sub_expr.extend(sub_tail);
                let sub_form = lower_form_expr(
                    &sub_name,
                    vec![],
                    vec![],
                    &sub_expr,
                    member_origin.clone(),
                    hir,
                );
                insert_hir_field(
                    fields,
                    field_order,
                    HirField {
                        name: sub_name.clone(),
                        inferred_type: "Form".into(),
                        origin_form: owner_name.to_string(),
                        default_expr: vec![Token::Group('{', sub_body)],
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                        visibility: HirVisibility::Default,
                        origin: member_origin,
                    },
                );
                if let Some(subs) = submodules.as_deref_mut() {
                    subs.insert(sub_name, sub_form);
                }
            }
            SemanticKind::Binding { key, value } => {
                let (clean_val, visibility) = strip_visibility_and_compile_tail(&value);
                let (inferred_type, def_expr) = split_type_and_default(&clean_val, hir);
                insert_hir_field(
                    fields,
                    field_order,
                    HirField {
                        name: key,
                        inferred_type,
                        origin_form: owner_name.to_string(),
                        default_expr: def_expr,
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                        visibility,
                        origin: member_origin,
                    },
                );
            }
            SemanticKind::TupleBinding { tuple, value } => {
                let key_str = tuple
                    .first()
                    .and_then(|sent| sent.first())
                    .map(|t| match t {
                        Token::Path(p) => format!("({p})"),
                        Token::Lit(Literal::String(s) | Literal::RawString(s)) => s.clone(),
                        _ => "()".to_string(),
                    })
                    .unwrap_or_else(|| "()".to_string());
                let (clean_val, visibility) = strip_visibility_and_compile_tail(&value);
                let inferred_type = infer_hir_type(&clean_val, hir);
                insert_hir_field(
                    fields,
                    field_order,
                    HirField {
                        name: key_str,
                        inferred_type,
                        origin_form: owner_name.to_string(),
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
                insert_hir_field(
                    fields,
                    field_order,
                    HirField {
                        name: field_name,
                        inferred_type: "ReactiveStream".into(),
                        origin_form: owner_name.to_string(),
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
                let func = build_hir_function(
                    &method_name,
                    &args,
                    fn_body,
                    &fn_tail,
                    false,
                    member_origin,
                    hir,
                );
                insert_hir_method(methods, method_order, func);
            }
            SemanticKind::GenericDef {
                name: method_name,
                params: args,
                body: ret_tokens,
            } => {
                let func = build_hir_function(
                    &method_name,
                    &args,
                    vec![],
                    &ret_tokens,
                    true,
                    member_origin,
                    hir,
                );
                insert_hir_method(methods, method_order, func);
            }
            _ => {
                if let [Token::Path(field_name), rest @ ..] = s.as_slice() {
                    if !rest.is_empty() && !has_pipe_op(rest) {
                        let (clean_val, visibility) = strip_visibility_and_compile_tail(rest);
                        let (inferred_type, def_expr) = split_type_and_default(&clean_val, hir);
                        insert_hir_field(
                            fields,
                            field_order,
                            HirField {
                                name: field_name.clone(),
                                inferred_type,
                                origin_form: owner_name.to_string(),
                                default_expr: def_expr,
                                is_reactive: false,
                                is_ambiguous: false,
                                conflicting_origins: vec![],
                                visibility,
                                origin: member_origin,
                            },
                        );
                    }
                } else {
                    let _ = owner_origin;
                }
            }
        }
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
    refinement_predicates: &mut Vec<Token>,
    is_bypass: &mut bool,
    tail: &[Token],
    owner_origin: &SourceOrigin,
    hir: &HirModule,
) {
    let mut i = 0;
    while i < tail.len() {
        match &tail[i] {
            // Каноническая форма слияния в V5.7: `PrintableUser User & Printable { format() [...] }`
            // или `SortableUser User & Comparable(User) { < (other User) [...] }`, где `{...}` идет после типа без второго `&`!
            Token::Group('{', override_sents) => {
                populate_hir_form_members(
                    owner_name,
                    override_sents,
                    fields,
                    field_order,
                    methods,
                    method_order,
                    None,
                    owner_origin,
                    hir,
                );
                i += 1;
            }
            Token::Op(op) if op == "&" => {
                if let Some(next_tok) = tail.get(i + 1) {
                    match next_tok {
                        Token::Path(rhs_type) if rhs_type == "Bypass" => {
                            *is_bypass = true;
                            if !ancestors.contains(rhs_type) {
                                ancestors.push(rhs_type.clone());
                            }
                        }
                        Token::Path(rhs_type) if rhs_type == "Public" => {
                            fields.retain(|_, f| f.visibility == HirVisibility::Public);
                            field_order.retain(|k| fields.contains_key(k));
                            methods.retain(|_, m| m.visibility == HirVisibility::Public);
                            method_order.retain(|k| methods.contains_key(k));
                        }
                        Token::Path(rhs_type) => {
                            if let Some(rhs_form) = hir.forms.get(rhs_type) {
                                if rhs_form.is_bypass {
                                    *is_bypass = true;
                                }
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
                                refinement_predicates.push(Token::Path(rhs_type.clone()));
                            } else if !ancestors.contains(rhs_type) {
                                ancestors.push(rhs_type.clone());
                            }
                            // Пропускаем аргументы параметризованного трейта `Comparable(User)`
                            if matches!(tail.get(i + 2), Some(Token::Group(')', _))) {
                                i += 3;
                                continue;
                            }
                        }
                        Token::Group('{', override_sents) => {
                            populate_hir_form_members(
                                owner_name,
                                override_sents,
                                fields,
                                field_order,
                                methods,
                                method_order,
                                None,
                                owner_origin,
                                hir,
                            );
                        }
                        tok @ Token::Group('[', _) => {
                            refinement_predicates.push(tok.clone());
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
                        // Вычитание `/ Private` (Раздел 21.2): удаляет все поля и методы с видимостью `Private`!
                        Token::Path(path_str) if path_str == "Private" => {
                            fields.retain(|_, f| f.visibility != HirVisibility::Private);
                            field_order.retain(|k| fields.contains_key(k));
                            methods.retain(|_, m| m.visibility != HirVisibility::Private);
                            method_order.retain(|k| methods.contains_key(k));
                        }
                        Token::Lit(
                            Literal::Symbol(sym_key)
                            | Literal::String(sym_key)
                            | Literal::RawString(sym_key),
                        ) => {
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

/// Разбор параметров функции по Закону 1c и 1d (V5.7):
/// • Одиночный известный `PascalCase` тип `fn(Age)` или `fn(WithName)` -> авто-привязывает переменную `age` / `with_name` (`auto_unpacks = true`)
/// • Непривязанный `PascalCase` идентификатор в начале списка `fmap(A, B, fa F(A))` -> compile-time Type Param (`is_type_param = true`)
/// • Деструктуризация полей `read_unpacked({ name } WithName)` -> связывает поле `name` напрямую (Law 1d)
/// • Явный именованный слот `fn(u User)`, `fn(ctx User & Cart)`, `fn(host String "localhost")` -> связывает переменную `u`, `ctx`, `host`
pub fn lower_fn_params(args: &[Sentence], hir: &HirModule) -> (Vec<HirParam>, bool) {
    let mut out = Vec::new();
    let mut auto_unpacks = false;

    for (idx, sent) in args.iter().enumerate() {
        match sent.as_slice() {
            // HKT Type Param: `lift(M(_) Monad, A, ma M(A))`
            [Token::Path(tp), Token::Group(')', _), ..] if is_pascal_case(tp) => {
                out.push(HirParam {
                    name: tp.clone(),
                    type_constraint: tp.clone(),
                    default_expr: vec![],
                    is_anonymous_type_slot: false,
                    is_type_param: true,
                    destructured_fields: vec![],
                });
            }
            // Закон 1d: Деструктуризация полей `{ name } WithName`
            [Token::Group('{', field_sents), rest @ ..] => {
                let mut destructured_fields = Vec::new();
                for fs in field_sents {
                    if let [Token::Path(fname)] = fs.as_slice() {
                        destructured_fields.push(fname.clone());
                    }
                }
                let constraint = format_tokens_as_type(rest);
                out.push(HirParam {
                    name: format!("_destruct_{idx}"),
                    type_constraint: constraint,
                    default_expr: vec![],
                    is_anonymous_type_slot: false,
                    is_type_param: false,
                    destructured_fields,
                });
            }
            // Закон 1d: Деструктуризация кортежа в маппере `(idx Int, (item Item, disc Int))`
            [Token::Group('(', tuple_sents), ..] => {
                let mut destructured_fields = Vec::new();
                for ts in tuple_sents {
                    if let [Token::Path(vname), ..] = ts.as_slice() {
                        destructured_fields.push(vname.clone());
                    }
                }
                out.push(HirParam {
                    name: format!("_tuple_param_{idx}"),
                    type_constraint: "Tuple".into(),
                    default_expr: vec![],
                    is_anonymous_type_slot: false,
                    is_type_param: false,
                    destructured_fields,
                });
            }
            // Одиночный идентификатор: `fn(User)` vs `fmap(A, B, ...)` vs `fn(x)`
            [Token::Path(single)] => {
                let is_upper = is_pascal_case(single);
                if is_upper {
                    // Если этот `PascalCase` идентификатор используется далее в сигнатуре как тип другого параметра (`A` в `fmap(A, B, fa F(A))`),
                    // и его нет среди известных форм — это чистый Type Parameter!
                    let used_as_type_later = args.iter().skip(idx + 1).any(|later| {
                        later.iter().skip(1).any(|t| token_references_ident(t, single))
                    });
                    if used_as_type_later && !is_known_type(single, hir) {
                        out.push(HirParam {
                            name: single.clone(),
                            type_constraint: single.clone(),
                            default_expr: vec![],
                            is_anonymous_type_slot: false,
                            is_type_param: true,
                            destructured_fields: vec![],
                        });
                    } else {
                        // Закон 1c: Одиночный известный тип `fn(Age)` -> авто-привязывает `age` (или `TargetForm` для макроса `derive_json(TargetForm)`)
                        let var_name = if is_known_type(single, hir) {
                            pascal_to_snake_case(single)
                        } else {
                            single.clone()
                        };
                        auto_unpacks = true;
                        out.push(HirParam {
                            name: var_name,
                            type_constraint: single.clone(),
                            default_expr: vec![],
                            is_anonymous_type_slot: true,
                            is_type_param: false,
                            destructured_fields: vec![],
                        });
                    }
                } else {
                    out.push(HirParam {
                        name: single.clone(),
                        type_constraint: String::new(),
                        default_expr: vec![],
                        is_anonymous_type_slot: false,
                        is_type_param: false,
                        destructured_fields: vec![],
                    });
                }
            }
            // Безымянный составной слот типа `(User & Cart)` (фиксируется для диагностики E02_DUPLICATE_UNNAMED_TYPE)
            [Token::Path(first_type), Token::Op(op), ..]
            if is_pascal_case(first_type) && matches!(op.as_str(), "&" | "|" | "/") =>
                {
                    let constraint = format_tokens_as_type(sent);
                    auto_unpacks = true;
                    out.push(HirParam {
                        name: pascal_to_snake_case(first_type),
                        type_constraint: constraint,
                        default_expr: vec![],
                        is_anonymous_type_slot: true,
                        is_type_param: false,
                        destructured_fields: vec![],
                    });
                }
            // Именованный параметр с типом и/или дефолтным значением:
            // `connect(host String "localhost", port Int 8080, tls Bool false)` или `reserve_stock(o Order / Cancelled)`
            [Token::Path(param_name), rest @ ..] => {
                let (constraint, default_expr) = split_param_type_and_default(rest, hir);
                out.push(HirParam {
                    name: param_name.clone(),
                    type_constraint: constraint,
                    default_expr,
                    is_anonymous_type_slot: false,
                    is_type_param: false,
                    destructured_fields: vec![],
                });
            }
            _ => {}
        }
    }

    (out, auto_unpacks)
}

fn token_references_ident(tok: &Token, ident: &str) -> bool {
    match tok {
        Token::Path(p) => p == ident,
        Token::Group(_, sents) => sents
            .iter()
            .any(|s| s.iter().any(|t| token_references_ident(t, ident))),
        _ => false,
    }
}

fn split_param_type_and_default(rest: &[Token], hir: &HirModule) -> (String, Vec<Token>) {
    match rest {
        // `host String "localhost"` (Явный тип + дефолтное значение)
        [Token::Path(ty), def @ ..]
        if is_pascal_case(ty)
            && !def.is_empty()
            && !matches!(def.first(), Some(Token::Op(_) | Token::Group(')', _) | Token::Group('[', _))) =>
            {
                (ty.clone(), def.to_vec())
            }
        // `k 10` (Только дефолтное значение без типа)
        [Token::Lit(_)] | [Token::Group(_, _)] => (infer_hir_type(rest, hir), rest.to_vec()),
        // `o Order / Cancelled`, `ctx User & Cart`, `f (A) [B]` (Только ограничение типа)
        _ => (format_tokens_as_type(rest), vec![]),
    }
}

fn format_single_token(tok: &Token) -> String {
    match tok {
        Token::Lit(Literal::String(s) | Literal::RawString(s)) => format!("\"{s}\""),
        Token::Lit(Literal::Symbol(s)) => format!("'{s}'"),
        Token::Lit(Literal::Int(i)) => i.to_string(),
        Token::Path(p) => p.clone(),
        _ => "...".into(),
    }
}

pub fn format_tokens_as_type(tokens: &[Token]) -> String {
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
            Token::Group('(', inner) => {
                let inner_str = inner
                    .iter()
                    .map(|s| format_tokens_as_type(s))
                    .collect::<Vec<_>>()
                    .join(", ");
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push('(');
                out.push_str(&inner_str);
                out.push(')');
            }
            Token::Group('[', inner) => {
                let inner_str = inner
                    .iter()
                    .map(|s| format_tokens_as_type(s))
                    .collect::<Vec<_>>()
                    .join(", ");
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push('[');
                out.push_str(&inner_str);
                out.push(']');
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
            Literal::String(_) | Literal::RawString(_) => "String".into(),
            Literal::Symbol(s) => format!("'{s}'"),
        },
        Some(Token::Path(p)) => {
            if matches!(tokens.get(1), Some(Token::Group(')', _))) {
                // Конструктор типа, экземпляр дженерика `Atom(CounterState) & { ... }` или `Type"..."` сахар
                if hir
                    .forms
                    .get(p)
                    .map(|f| f.type_params.is_empty())
                    .unwrap_or(false)
                    || p == "Signal"
                    || matches!(tokens.get(2), Some(Token::Op(op)) if op == "&")
                {
                    p.clone()
                } else {
                    format_tokens_as_type(tokens)
                }
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