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
    pub refinement_predicates: Vec<String>,
    /// Закон 3: Помечена ли форма тегом `& Bypass` (`None`, `NotFound`, `IoError`)
    pub is_bypass: bool,
    pub is_algebraic_alias: bool,
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
                let type_args = match sentence.as_slice() {
                    [Token::Path(_), Token::Path(_), Token::Group(')', args), ..] => {
                        extract_type_params(args)
                    }
                    _ => vec![],
                };
                // В Razomy v5.7 `global_counter Atom(CounterState) { ... }` (snake_case) — это также глобальный экземпляр!
                let is_snake_instance = !is_pascal_case(&name);
                let form = lower_form(
                    &name,
                    Some(&base),
                    vec![],
                    vec![],
                    type_args,
                    &body,
                    &tail,
                    origin.clone(),
                    &hir,
                );
                register_form(&mut hir, name.clone(), form);

                if is_snake_instance {
                    let binding = HirGlobalBinding {
                        key: name.clone(),
                        inferred_type: base,
                        value_expr: sentence[1..].to_vec(),
                        origin,
                    };
                    if !hir.global_order.contains(&name) {
                        hir.global_order.push(name.clone());
                    }
                    hir.globals.insert(name, binding);
                }
            }

            SemanticKind::GenericDef { name, params, body } => {
                let origin = SourceOrigin {
                    sentence_index: idx,
                    symbol_context: name.clone(),
                    raw_ast: sentence.clone(),
                };
                let (type_params, value_params) = extract_form_slots(&params, &hir);
                let form = match body.as_slice() {
                    [Token::Group('{', form_body), tail @ ..] => lower_form(
                        &name,
                        None,
                        type_params,
                        value_params,
                        vec![],
                        form_body,
                        tail,
                        origin,
                        &hir,
                    ),
                    // `Monad(M(_)) & Functor(M) { ... }` или `Element(id_attr ProceduralId) Cuboid & StartNode { ... }`
                    _ if body.iter().any(|t| matches!(t, Token::Group('{', _))) => {
                        let brace_idx = body
                            .iter()
                            .position(|t| matches!(t, Token::Group('{', _)))
                            .unwrap();
                        let Token::Group('{', form_body) = &body[brace_idx] else {
                            unreachable!()
                        };
                        let prefix = &body[..brace_idx];
                        let suffix = &body[brace_idx + 1..];
                        let mut combined_tail = Vec::new();
                        let mut first_base: Option<&str> = None;
                        if let Some(Token::Path(b)) = prefix.first() {
                            first_base = Some(b.as_str());
                            if prefix.len() > 1 {
                                combined_tail.extend_from_slice(&prefix[1..]);
                            }
                        } else {
                            combined_tail.extend_from_slice(prefix);
                        }
                        combined_tail.extend_from_slice(suffix);
                        lower_form(
                            &name,
                            first_base,
                            type_params,
                            value_params,
                            vec![],
                            form_body,
                            &combined_tail,
                            origin,
                            &hir,
                        )
                    }
                    _ => {
                        let mut f = lower_algebraic_form(&name, &body, origin, &hir);
                        f.type_params = type_params;
                        f.value_params = value_params;
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

                // 1. Композиция потоков: `clean_str trim_str & upper_str` или `combined_pipeline step_1 & step_2`
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
                        auto_unpacks: false,
                        body: vec![composed_steps],
                        return_type: None,
                        is_contract: false,
                        is_compile_time: false,
                        visibility: HirVisibility::Default,
                        origin: origin.clone(),
                    };
                    if !hir.function_order.contains(&name) {
                        hir.function_order.push(name.clone());
                    }
                    hir.functions.insert(name.clone(), func);
                }

                // 2. Если `name` в `snake_case` (например, `secret_key "xyz987" & Private`, `secure_connect connect & { ... }`, `order_1 Invoice & { ... }`),
                // также регистрируем как глобальную переменную!
                if !is_pascal_case(&name) {
                    let (clean_expr, _) = strip_visibility_and_compile_tail(&expr);
                    let inferred_type = infer_hir_type(&clean_expr, &hir);
                    let binding = HirGlobalBinding {
                        key: name.clone(),
                        inferred_type,
                        value_expr: clean_expr,
                        origin: origin.clone(),
                    };
                    if !hir.global_order.contains(&name) {
                        hir.global_order.push(name.clone());
                    }
                    hir.globals.insert(name.clone(), binding);
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
                let (ret_tokens, visibility, is_compile_time) =
                    parse_fn_tail_metadata(&tail, &hir);
                let return_type = if ret_tokens.is_empty() {
                    None
                } else {
                    Some(format_tokens_as_type(&ret_tokens))
                };
                let func = HirFunction {
                    name: name.clone(),
                    params,
                    auto_unpacks,
                    body,
                    return_type,
                    is_contract: false,
                    is_compile_time,
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
                    auto_unpacks: false,
                    body: stream,
                    return_type: None,
                    is_contract: false,
                    is_compile_time: false,
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
                    if is_pascal_case(&key)
                        && (hir.forms.contains_key(base_type)
                        || matches!(
                                    base_type.as_str(),
                                    "String" | "Int" | "Float" | "Bool" | "Byte" | "Symbol" | "Any"
                                )) =>
                        {
                            let form = lower_algebraic_form(&key, &value, origin.clone(), &hir);
                            register_form(&mut hir, key.clone(), form);
                        }
                    [Token::Path(generic_base), Token::Group(')', args)]
                    if is_pascal_case(&key) && hir.forms.contains_key(generic_base) =>
                        {
                            let type_args = extract_type_params(args);
                            let form = lower_form(
                                &key,
                                Some(generic_base),
                                vec![],
                                vec![],
                                type_args,
                                &[],
                                &[],
                                origin.clone(),
                                &hir,
                            );
                            register_form(&mut hir, key.clone(), form);
                        }
                    [Token::Group('(', inner)] if inner.len() == 1 => {
                        let inner_sent = &inner[0];
                        // `Product (Compile & [ { id Int, title String } . derive_json ])`
                        if let [Token::Path(c), Token::Op(amp), Token::Group('[', stream_sents)] =
                            inner_sent.as_slice()
                        {
                            if c == "Compile" && amp == "&" {
                                if let Some(first_s) = stream_sents.first() {
                                    if let [Token::Group('{', body), Token::Op(dot), Token::Path(macro_name)] =
                                        first_s.as_slice()
                                    {
                                        if dot == "." {
                                            let macro_tail = extract_macro_tail(macro_name, &hir);
                                            let form = lower_form(
                                                &key,
                                                None,
                                                vec![],
                                                vec![],
                                                vec![],
                                                body,
                                                &macro_tail,
                                                origin.clone(),
                                                &hir,
                                            );
                                            register_form(&mut hir, key.clone(), form);
                                        }
                                    }
                                }
                            }
                        } else if has_algebra_op(inner_sent) {
                            if is_pascal_case(&key) {
                                let form =
                                    lower_algebraic_form(&key, inner_sent, origin.clone(), &hir);
                                register_form(&mut hir, key.clone(), form);
                            } else if is_stream_composition(inner_sent, &hir) {
                                let composed_steps: Vec<Token> = inner_sent
                                    .iter()
                                    .map(|t| match t {
                                        Token::Op(op) if op == "&" => Token::Op(".".into()),
                                        other => other.clone(),
                                    })
                                    .collect();
                                let func = HirFunction {
                                    name: key.clone(),
                                    params: vec![],
                                    auto_unpacks: false,
                                    body: vec![composed_steps],
                                    return_type: None,
                                    is_contract: false,
                                    is_compile_time: false,
                                    visibility: HirVisibility::Default,
                                    origin: origin.clone(),
                                };
                                if !hir.function_order.contains(&key) {
                                    hir.function_order.push(key.clone());
                                }
                                hir.functions.insert(key.clone(), func);
                            }
                        }
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
                        let macro_tail = extract_macro_tail(macro_name, &hir);
                        let form = lower_form(
                            name,
                            None,
                            vec![],
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
    let dummy_origin = SourceOrigin {
        sentence_index: 0,
        symbol_context: "Shell".into(),
        raw_ast: vec![],
    };

    for builtin in ["Compile", "Bypass", "Public", "Private"] {
        hir.form_order.push(builtin.into());
        hir.forms.insert(
            builtin.into(),
            HirForm {
                name: builtin.into(),
                type_params: vec![],
                value_params: vec![],
                type_args: vec![],
                fields: HashMap::new(),
                field_order: vec![],
                ancestor_fields: HashMap::new(),
                methods: HashMap::new(),
                method_order: vec![],
                submodules: HashMap::new(),
                ancestors: vec![],
                union_variants: vec![],
                refinement_predicates: vec![],
                is_bypass: builtin == "Bypass",
                is_algebraic_alias: false,
                origin: dummy_origin.clone(),
            },
        );
    }

    let mut fields = HashMap::new();
    fields.insert(
        "log".into(),
        HirField {
            name: "log".into(),
            inferred_type: "Fn(msg String) -> None".into(),
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
        "exec".into(),
        HirField {
            name: "exec".into(),
            inferred_type: "Fn(cmd String) -> String".into(),
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
            value_params: vec![],
            type_args: vec![],
            fields,
            field_order: vec!["log".into(), "exec".into()],
            ancestor_fields: HashMap::new(),
            methods: HashMap::new(),
            method_order: vec![],
            submodules: HashMap::new(),
            ancestors: vec![],
            union_variants: vec![],
            refinement_predicates: vec![],
            is_bypass: false,
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
    value_params: Vec<HirParam>,
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
    let mut is_bypass = false;

    // 0. Закон 1a/1c: Слоты значений конструктора `Form(a Int, b Int)` автоматически становятся полями на `self`!
    for vp in &value_params {
        if !fields.contains_key(&vp.name) {
            field_order.push(vp.name.clone());
        }
        fields.insert(
            vp.name.clone(),
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

    // 1. Статическое копирование полей, истории предков и методов базовой формы (Axiom 4)
    if let Some(base_name) = base {
        if base_name != name {
            ancestors.push(base_name.to_string());
        }
        if base_name == "Bypass" {
            is_bypass = true;
        }
        if let Some(base_form) = hir.forms.get(base_name) {
            if base_form.is_bypass {
                is_bypass = true;
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
                    fields.insert(k.clone(), cloned);
                    if !field_order.contains(k) {
                        field_order.push(k.clone());
                    }
                }
            }
            for m in &base_form.method_order {
                if let Some(mf) = base_form.methods.get(m) {
                    methods.insert(m.clone(), mf.clone());
                    if !method_order.contains(m) {
                        method_order.push(m.clone());
                    }
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
                let sub_form = lower_form(
                    &sub_name,
                    None,
                    vec![],
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
                let (clean_val, visibility) = strip_visibility_and_compile_tail(&value);
                let (inferred_type, def_expr) = split_type_and_default(&clean_val, hir);
                if !fields.contains_key(&key) {
                    field_order.push(key.clone());
                }
                fields.insert(
                    key.clone(),
                    HirField {
                        name: key,
                        inferred_type,
                        origin_form: name.to_string(),
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
                let (ret_tokens, visibility, is_compile_time) =
                    parse_fn_tail_metadata(&fn_tail, hir);
                let return_type = if ret_tokens.is_empty() {
                    None
                } else {
                    Some(format_tokens_as_type(&ret_tokens))
                };
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
                        return_type,
                        is_contract: false,
                        is_compile_time,
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
                let (params, auto_unpacks) = lower_fn_params(&args, hir);
                let ret_type = format_tokens_as_type(&ret_tokens);
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
                        is_compile_time: false,
                        visibility: HirVisibility::Default,
                        origin: member_origin,
                    },
                );
            }
            SemanticKind::OperatorMethod { op, args, rest } => {
                let (params, auto_unpacks) = lower_fn_params(&args, hir);
                let (fn_body, return_type, is_contract) = match rest.first() {
                    Some(Token::Group('[', b)) => (b.clone(), None, false),
                    _ => (vec![], Some(format_tokens_as_type(&rest)), true),
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
                        is_compile_time: false,
                        visibility: HirVisibility::Default,
                        origin: member_origin,
                    },
                );
            }
            SemanticKind::CallExpr {
                name: field_name,
                args,
            } => {
                // Поле с вызовом конструктора или Tagged String `price Money"100.00"` (если оно распарсилось как CallExpr только при отсутствии пробела)
                let val_tokens = vec![Token::Path(field_name.clone()), Token::Group(')', args)];
                let inferred_type = infer_hir_type(&val_tokens, hir);
                let _ = inferred_type;
            }
            _ => {
                if let [Token::Path(field_name), rest @ ..] = s.as_slice() {
                    if !rest.is_empty() && !has_pipe_op(rest) {
                        let (clean_val, visibility) = strip_visibility_and_compile_tail(rest);
                        let (inferred_type, def_expr) = split_type_and_default(&clean_val, hir);
                        if !fields.contains_key(field_name) {
                            field_order.push(field_name.clone());
                        }
                        fields.insert(
                            field_name.clone(),
                            HirField {
                                name: field_name.clone(),
                                inferred_type,
                                origin_form: name.to_string(),
                                default_expr: def_expr,
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
            &mut is_bypass,
            tail,
            &origin,
            hir,
        );
    }

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
    let mut is_bypass = false;

    // Если выражение начинается со скобки `(PlayButton / Cuboid) & { ... }`, разворачиваем её:
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
        match first_tok {
            Token::Path(first_type) => {
                if first_type == "Bypass" {
                    is_bypass = true;
                }
                if let Some(base_form) = hir.forms.get(first_type) {
                    if base_form.is_bypass {
                        is_bypass = true;
                    }
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

        if expr_slice.len() > 1 {
            let start_idx = if matches!(expr_slice.get(1), Some(Token::Group(')', _))) {
                if let Token::Path(first_type) = first_tok {
                    if !union_variants.contains(first_type) {
                        union_variants.push(first_type.clone());
                    }
                }
                2
            } else {
                1
            };

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
    }

    let has_own_body_fields = expr_slice.iter().any(|t| matches!(t, Token::Group('{', _)));

    HirForm {
        name: name.to_string(),
        type_params: vec![],
        value_params: vec![],
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
        is_bypass,
        is_algebraic_alias: !has_own_body_fields,
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
                apply_brace_overrides(
                    owner_name,
                    fields,
                    field_order,
                    methods,
                    method_order,
                    override_sents,
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
                                refinement_predicates.push(rhs_type.clone());
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
                            apply_brace_overrides(
                                owner_name,
                                fields,
                                field_order,
                                methods,
                                method_order,
                                override_sents,
                                owner_origin,
                                hir,
                            );
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

#[allow(clippy::too_many_arguments)]
fn apply_brace_overrides(
    owner_name: &str,
    fields: &mut HashMap<String, HirField>,
    field_order: &mut Vec<String>,
    methods: &mut HashMap<String, HirFunction>,
    method_order: &mut Vec<String>,
    override_sents: &[Sentence],
    owner_origin: &SourceOrigin,
    hir: &HirModule,
) {
    for s in override_sents {
        match classify_sentence(s) {
            SemanticKind::Binding { key, value } => {
                let (clean_val, visibility) = strip_visibility_and_compile_tail(&value);
                let (t, def_expr) = split_type_and_default(&clean_val, hir);
                if !fields.contains_key(&key) {
                    field_order.push(key.clone());
                }
                fields.insert(
                    key.clone(),
                    HirField {
                        name: key,
                        inferred_type: t,
                        origin_form: owner_name.to_string(),
                        default_expr: def_expr,
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                        visibility,
                        origin: owner_origin.clone(),
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
                        origin_form: owner_name.to_string(),
                        default_expr: vec![Token::Group('[', stream)],
                        is_reactive: true,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                        visibility: HirVisibility::Default,
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
                let (ret_tokens, visibility, is_compile_time) =
                    parse_fn_tail_metadata(&fn_tail, hir);
                let return_type = if ret_tokens.is_empty() {
                    None
                } else {
                    Some(format_tokens_as_type(&ret_tokens))
                };
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
                        return_type,
                        is_contract: false,
                        is_compile_time,
                        visibility,
                        origin: owner_origin.clone(),
                    },
                );
            }
            SemanticKind::OperatorMethod { op, args, rest } => {
                let (params, auto_unpacks) = lower_fn_params(&args, hir);
                let (fn_body, return_type, is_contract) = match rest.first() {
                    Some(Token::Group('[', b)) => (b.clone(), None, false),
                    _ => (vec![], Some(format_tokens_as_type(&rest)), true),
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
                        is_compile_time: false,
                        visibility: HirVisibility::Default,
                        origin: owner_origin.clone(),
                    },
                );
            }
            _ => {}
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
                // Конструктор типа или Type"..." сахар (`Money("199.99")`, `Url"https://..."`, `Tuple(T)`)
                if matches!(
                    p.as_str(),
                    "Money"
                        | "Usd"
                        | "Date"
                        | "Time"
                        | "Duration"
                        | "Pair"
                        | "Hex"
                        | "Ip"
                        | "Version"
                        | "Url"
                        | "Path"
                        | "Signal"
                ) {
                    p.clone()
                } else {
                    format_tokens_as_type(tokens)
                }
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