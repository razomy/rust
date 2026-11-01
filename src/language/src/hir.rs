// src/hir.rs
// =============================================================================
// RAZOMY V5.7 — HIGH-LEVEL INTERMEDIATE REPRESENTATION (HIR)
// Переводит динамический синтаксис AST в разрешенную статическую модель,
// сохраняя 100% обратную связь с исходным AST (SourceOrigin).
// =============================================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::parser::{
    classify_sentence, extract_import, has_algebra_op, has_pipe_op, resolve_import_fs_path,
    ImportDirective, ImportSource, Literal, LocatedSentence, SemanticKind, Sentence, Token,
};

// =============================================================================
// 1. ОБРАТНАЯ СВЯЗЬ С AST И ИСХОДНЫМ КОДОМ (SOURCE ORIGIN) & КОНВЕРТАЦИЯ РЕГИСТРА
// =============================================================================

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceOrigin {
    /// Порядковый индекс предложения внутри того файла, где оно объявлено
    pub sentence_index: usize,
    /// Путь к файлу, если символ пришел из импортированного модуля (`None` = текущий файл)
    pub file_path: Option<PathBuf>,
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

/// Закон 2 (V6.4): Проверяет, является ли идентификатор именованной типовой дыркой `_PascalCase` (`_T`, `_A`, `_Key`, `_M`)
pub fn is_type_hole(s: &str) -> bool {
    s.starts_with('_')
        && s.len() >= 2
        && s.chars().nth(1).map(|c| c.is_uppercase()).unwrap_or(false)
}

pub fn is_type_or_hole(s: &str) -> bool {
    is_pascal_case(s) || is_type_hole(s)
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
    /// Статически выведенный из тела функции тип возвращаемого значения (для IDE и анализатора)
    pub inferred_return_type: Option<String>,
    pub is_contract: bool,
    /// Помечена ли функция тегом `& Compile`
    pub is_compile_time: bool,
    pub is_prelude: bool,
    pub visibility: HirVisibility,
    pub origin: SourceOrigin,
}

impl HirFunction {
    pub fn effective_return_type(&self) -> Option<&str> {
        self.return_type
            .as_deref()
            .or(self.inferred_return_type.as_deref())
    }
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
    /// Разрешенный файловый путь к импортируемому модулю (для межфайловых переходов в LSP)
    pub resolved_path: Option<PathBuf>,
    pub origin: SourceOrigin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateSymbol {
    pub name: String,
    pub kind: &'static str, // "Function" | "Form"
    pub first_sentence_index: usize,
    pub duplicate_origin: SourceOrigin,
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
    pub duplicates: Vec<DuplicateSymbol>,
    /// Все предложения верхнего уровня в порядке следования
    pub top_level_sentences: Vec<Sentence>,
}
// =============================================================================
// 3. ПОСТРОИТЕЛЬ HIR (LOWERING: AST -> HIR)
// =============================================================================

pub const PRELUDE_SOURCE: &str = include_str!("prelude.rl");
pub const IDE_STDLIB_SOURCE: &str = include_str!("ide_stdlib.rl");

pub fn lower_to_hir(sentences: &[Sentence]) -> HirModule {
    let located: Vec<LocatedSentence> = sentences
        .iter()
        .enumerate()
        .map(|(idx, s)| LocatedSentence {
            sentence: s.clone(),
            local_index: idx,
            file_path: None,
        })
        .collect();
    lower_to_hir_located(&located, None, &HashMap::new())
}

pub fn lower_to_hir_located(
    located_sentences: &[LocatedSentence],
    base_dir: Option<&Path>,
    in_memory_files: &HashMap<PathBuf, String>,
) -> HirModule {
    let mut hir = HirModule {
        top_level_sentences: located_sentences
            .iter()
            .map(|ls| ls.sentence.clone())
            .collect(),
        ..Default::default()
    };

    register_stdlib_hir(&mut hir);

    for loc in located_sentences {
        lower_located_sentence_into_hir(
            loc.local_index,
            loc.file_path.clone(),
            &loc.sentence,
            base_dir,
            in_memory_files,
            &mut hir,
        );
    }

    hir
}

fn lower_sentence_into_hir(idx: usize, sentence: &Sentence, hir: &mut HirModule) {
    lower_located_sentence_into_hir(idx, None, sentence, None, &HashMap::new(), hir);
}

fn lower_located_sentence_into_hir(
    idx: usize,
    file_path: Option<PathBuf>,
    sentence: &Sentence,
    base_dir: Option<&Path>,
    in_memory_files: &HashMap<PathBuf, String>,
    hir: &mut HirModule,
) {
    // 0. Если предложение является директивой импорта (`_ models`, `(User, Cat) models`, `Base root.base`)
    if let Some(import_dir) = extract_import(sentence) {
        let (targets, imp_src) = match import_dir {
            ImportDirective::Wildcard { source } => (vec!["_".to_string()], source),
            ImportDirective::Destructured { names, source } => (names, source),
            ImportDirective::Namespaced { alias, source } => (vec![alias], source),
        };
        let source_path = match &imp_src {
            ImportSource::ModulePath(m) => m.clone(),
            ImportSource::FilePath(f) => f.clone(),
        };
        let effective_base = file_path
            .as_deref()
            .and_then(Path::parent)
            .or(base_dir);
        let resolved_path =
            effective_base.map(|dir| resolve_import_fs_path(dir, &imp_src, in_memory_files));
        let origin = SourceOrigin {
            sentence_index: idx,
            file_path,
            symbol_context: targets.first().cloned().unwrap_or_else(|| "_".into()),
            raw_ast: sentence.clone(),
        };
        hir.imports.push(HirImport {
            targets,
            source_path,
            resolved_path,
            origin,
        });
        return;
    }

    let kind = classify_sentence(sentence);
    match kind {
        SemanticKind::FormDef { name, body, tail } => {
            let origin = SourceOrigin {
                sentence_index: idx,
                file_path,
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
                file_path,
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
                file_path,
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
                file_path,
                symbol_context: name.clone(),
                raw_ast: sentence.clone(),
            };
            let func = build_hir_function(&name, &args, body, &tail, false, origin.clone(), hir);
            if is_pascal_case(&name) && !hir.forms.contains_key(&name) {
                let (_, value_params) = extract_form_slots(&args, hir);
                let form = lower_form_expr(&name, vec![], value_params, &[], origin, hir);
                register_form(hir, name.clone(), form);
            }
            register_function(hir, name, func);
        }

        SemanticKind::StreamOrBranch { name, stream } => {
            let origin = SourceOrigin {
                sentence_index: idx,
                file_path,
                symbol_context: name.clone(),
                raw_ast: sentence.clone(),
            };
            let func = build_hir_function(&name, &[], stream, &[], false, origin, hir);
            register_function(hir, name, func);
        }

        SemanticKind::Binding { key, value } => {
            let origin = SourceOrigin {
                sentence_index: idx,
                file_path,
                symbol_context: key.clone(),
                raw_ast: sentence.clone(),
            };

            // Обработка `Product (Compile & [ { id Int, title String } . derive_json ])` или алиасов типов `Age Int`, `One Succ(Zero)`
            match value.as_slice() {
                [Token::Path(base_type)]
                    if is_pascal_case(&key) && is_pascal_case(base_type) =>
                {
                    let form = lower_form_expr(&key, vec![], vec![], &value, origin.clone(), hir);
                    register_form(hir, key.clone(), form);
                }
                [Token::Path(generic_base), Token::Group(')', _)]
                    if is_pascal_case(&key) && is_pascal_case(generic_base) =>
                {
                    let form = lower_form_expr(&key, vec![], vec![], &value, origin.clone(), hir);
                    register_form(hir, key.clone(), form);
                }
                [Token::Group('(', inner)]
                    if is_pascal_case(&key)
                        && !inner.is_empty()
                        && inner.iter().all(|s| {
                            matches!(
                                s.first(),
                                Some(Token::Path(p)) if is_pascal_case(p) && p != "Compile"
                            ) && !has_algebra_op(s)
                                && !has_pipe_op(s)
                        }) =>
                {
                    let tuple_expr = vec![
                        Token::Path("Tuple".into()),
                        Token::Group(')', inner.clone()),
                    ];
                    let form =
                        lower_form_expr(&key, vec![], vec![], &tuple_expr, origin.clone(), hir);
                    register_form(hir, key.clone(), form);
                }
                [Token::Group('(', _), Token::Group('[', _)] if is_pascal_case(&key) => {
                    let stream_expr = vec![Token::Path("Stream".into())];
                    let form =
                        lower_form_expr(&key, vec![], vec![], &stream_expr, origin.clone(), hir);
                    register_form(hir, key.clone(), form);
                }
                [Token::Group('(', inner)] if inner.len() == 1 => {
                    let inner_sent = &inner[0];
                    if let [Token::Path(c), Token::Op(amp), Token::Group('[', stream_sents)] =
                        inner_sent.as_slice()
                    {
                        if c == "Compile" && amp == "&" {
                            if let Some(
                                [Token::Group('{', body), Token::Op(dot), Token::Path(macro_name)],
                            ) = stream_sents.first().map(Vec::as_slice)
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
                file_path,
                symbol_context: "<tuple_binding>".into(),
                raw_ast: sentence.clone(),
            };
            let inferred_type = infer_hir_type(&value, hir);
            for elem_sent in tuple {
                if let [Token::Path(var_name)] = elem_sent.as_slice() {
                    register_global(
                        hir,
                        var_name.clone(),
                        inferred_type.clone(),
                        value.clone(),
                        origin.clone(),
                    );
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
                        file_path,
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

fn fn_param_signature_key(func: &HirFunction) -> Vec<String> {
    func.params
        .iter()
        .filter(|p| !p.is_type_param)
        .map(|p| {
            if p.type_constraint.is_empty() {
                "Any".to_string()
            } else {
                p.type_constraint.clone()
            }
        })
        .collect()
}

fn register_function(hir: &mut HirModule, name: String, func: HirFunction) {
    if let Some(existing) = hir.functions.get(&name) {
        if !existing.is_prelude && !func.is_prelude && func.origin.file_path.is_none() {
            // Axiom II.2 & E11: Same-name overloads with distinct parameter signatures merge cleanly!
            // Only flag a duplicate when parameter signatures are identical.
            if fn_param_signature_key(existing) == fn_param_signature_key(&func) {
                hir.duplicates.push(DuplicateSymbol {
                    name: name.clone(),
                    kind: "Function",
                    first_sentence_index: existing.origin.sentence_index,
                    duplicate_origin: func.origin.clone(),
                });
            }
        }
    }
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
    let (ret_tokens, visibility, is_compile_time) = parse_fn_tail_metadata(tail, hir);
    let (params, auto_unpacks) = lower_fn_params_with_ret(args, &ret_tokens, hir);
    let return_type = if ret_tokens.is_empty() {
        None
    } else {
        Some(format_tokens_as_type(&ret_tokens))
    };
    let inferred_return_type = if return_type.is_none() && !body.is_empty() {
        infer_fn_body_return_type(&params, &body, None, None, hir)
    } else {
        None
    };
    HirFunction {
        name: name.to_string(),
        params,
        auto_unpacks,
        body,
        return_type,
        inferred_return_type,
        is_contract,
        is_compile_time,
        is_prelude: false,
        visibility,
        origin,
    }
}

fn infer_fn_body_return_type(
    params: &[HirParam],
    body: &[Sentence],
    enclosing_form_name: Option<&str>,
    enclosing_fields: Option<&HashMap<String, HirField>>,
    hir: &HirModule,
) -> Option<String> {
    let mut local_types: HashMap<String, String> = HashMap::new();
    if let Some(form_name) = enclosing_form_name {
        local_types.insert("self".into(), form_name.to_string());
        local_types.insert("Self".into(), form_name.to_string());
    }
    if let Some(fields) = enclosing_fields {
        for (k, f) in fields {
            local_types.insert(k.clone(), f.inferred_type.clone());
            local_types.insert(format!("self.{k}"), f.inferred_type.clone());
        }
    }
    for p in params {
        if !p.type_constraint.is_empty() {
            local_types.insert(p.name.clone(), p.type_constraint.clone());
            if local_types.get("_").is_none() && !p.is_type_param {
                local_types.insert("_".into(), p.type_constraint.clone());
            }
        }
    }
    for sent in body {
        match classify_sentence(sent) {
            SemanticKind::Binding { key, value }
            | SemanticKind::AlgebraicDef {
                name: key,
                expr: value,
            } => {
                let ty = infer_hir_type_with_locals(&value, &local_types, hir);
                if ty != "Any" {
                    local_types.insert(key, ty);
                }
            }
            _ => {}
        }
    }
    let last = body.last()?;

    if let [Token::Path(p)] = last.as_slice() {
        if p == "self" {
            if let Some(fields) = enclosing_fields {
                if let Some(domain_field) = fields.values().find(|f| {
                    !f.is_reactive
                        && !matches!(
                            f.inferred_type.as_str(),
                            "Any" | "ReactiveStream" | "self" | "Self" | "Float" | "Int" | "Bool"
                        )
                }) {
                    return Some(domain_field.inferred_type.clone());
                }
            }
        }
    }

    let mut leaf_types = Vec::new();
    collect_body_return_leaf_types(body, &local_types, hir, &mut leaf_types);

    if leaf_types.is_empty() {
        let inferred = infer_hir_type_with_locals(last, &local_types, hir);
        if !is_uninformative_return_type(&inferred) {
            push_unique_return_types(&inferred, &mut leaf_types);
        }
    }

    if leaf_types.is_empty() {
        None
    } else {
        Some(leaf_types.join(" | "))
    }
}

fn is_uninformative_return_type(ty: &str) -> bool {
    let trimmed = ty.trim();
    trimmed.is_empty()
        || matches!(trimmed, "Any" | "Stream" | "self" | "_" | "..")
        || trimmed.starts_with("_.")
}

fn push_unique_return_types(ty_expr: &str, out: &mut Vec<String>) {
    for part in ty_expr.split('|').map(str::trim) {
        if !is_uninformative_return_type(part) && !out.iter().any(|existing| existing == part) {
            out.push(part.to_string());
        }
    }
}

/// Рекурсивно обходит все терминальные листья возврата функции:
/// • Ранние выходы `Return(val)`, `Break(val)`, воротные операторы `.?` и `.!`
/// • Все ветки роутеров `. { Branch [ ... ] }` на любой глубине вложенности
/// • Последнее выражение тела функции или вложенного потока `[ ... ]`
fn collect_body_return_leaf_types(
    stmts: &[Sentence],
    locals: &HashMap<String, String>,
    hir: &HirModule,
    out: &mut Vec<String>,
) {
    let mut current_locals = locals.clone();
    for (idx, sent) in stmts.iter().enumerate() {
        let is_last = idx + 1 == stmts.len();

        match classify_sentence(sent) {
            SemanticKind::Binding { key, value }
            | SemanticKind::AlgebraicDef {
                name: key,
                expr: value,
            } => {
                // Guarded local binding `u1 User (uid1 . Db.find_user)`:
                // если правая часть возвращает Union с `& Bypass` веткой (`User | NotFound`),
                // эта Bypass-ветка автоматически пробрасывается в возвращаемый тип функции!
                if let [Token::Path(explicit_ty), rest @ ..] = value.as_slice() {
                    if is_pascal_case(explicit_ty) && !rest.is_empty() {
                        current_locals.insert(key.clone(), explicit_ty.clone());
                        let rhs_ty = infer_hir_type_with_locals(rest, &current_locals, hir);
                        for branch in rhs_ty.split('|').map(str::trim) {
                            if branch != explicit_ty
                                && hir.forms.get(branch).map_or(false, |f| f.is_bypass)
                            {
                                push_unique_return_types(branch, out);
                            }
                        }
                        if is_last {
                            push_unique_return_types(explicit_ty, out);
                        }
                        continue;
                    }
                }
                let ty = infer_hir_type_with_locals(&value, &current_locals, hir);
                if !is_uninformative_return_type(&ty) {
                    current_locals.insert(key, ty.clone());
                    if is_last {
                        push_unique_return_types(&ty, out);
                    }
                }
                collect_early_exits_from_tokens(&value, &current_locals, hir, out);
                continue;
            }
            _ => {}
        }

        collect_early_exits_from_tokens(sent, &current_locals, hir, out);

        if is_last {
            collect_terminal_expr_leaf_types(sent, &current_locals, hir, out);
        }
    }
}

fn collect_early_exits_from_tokens(
    tokens: &[Token],
    locals: &HashMap<String, String>,
    hir: &HirModule,
    out: &mut Vec<String>,
) {
    // 1. Явные вызовы `Return(val)` или `Break(val)` или `Break { value val }`
    for win in tokens.windows(2) {
        match win {
            [Token::Path(p), Token::Group(')', args)] if p == "Return" || p == "Break" => {
                if let Some(first_arg) = args.first() {
                    let ty = infer_hir_type_with_locals(first_arg, locals, hir);
                    push_unique_return_types(&ty, out);
                }
            }
            [Token::Path(p), Token::Group('{', field_sents)] if p == "Return" || p == "Break" => {
                for fs in field_sents {
                    if let SemanticKind::Binding { key, value } = classify_sentence(fs) {
                        if key == "value" {
                            let ty = infer_hir_type_with_locals(&value, locals, hir);
                            push_unique_return_types(&ty, out);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // 2. Пайплайны с воротами `.!` (Guard) и `.?` (Positive Gate / Early Exit)
    if has_pipe_op(tokens) {
        let (head, steps) = crate::parser::split_pipeline(tokens);
        collect_early_exits_from_tokens(&head, locals, hir, out);
        for step in steps {
            match step.op {
                crate::parser::PipeOp::Guard => {
                    let guard_ty = infer_hir_type_with_locals(&step.expr, locals, hir);
                    push_unique_return_types(&guard_ty, out);
                }
                crate::parser::PipeOp::Filter => {
                    collect_early_exits_from_tokens(&step.expr, locals, hir, out);
                    if let [Token::Path(p), ..] = step.expr.as_slice() {
                        if p != "Break" && p != "Return" && is_pascal_case(p) {
                            let ty = infer_hir_type_with_locals(&step.expr, locals, hir);
                            if hir.forms.get(&ty).map_or(false, |f| f.is_bypass) {
                                push_unique_return_types(&ty, out);
                            }
                        }
                    }
                }
                _ => {
                    collect_early_exits_from_tokens(&step.expr, locals, hir, out);
                }
            }
        }
        return;
    }

    for tok in tokens {
        if let Token::Group(_, sents) = tok {
            for s in sents {
                collect_early_exits_from_tokens(s, locals, hir, out);
            }
        }
    }
}

fn collect_terminal_expr_leaf_types(
    tokens: &[Token],
    locals: &HashMap<String, String>,
    hir: &HirModule,
    out: &mut Vec<String>,
) {
    if tokens.is_empty() {
        return;
    }

    // Ignore self-loop attractor step `. ..` as a terminal return type
    if matches!(tokens.last(), Some(Token::Op(op)) if op == "..") {
        return;
    }

    if let [Token::Group('[', stream_sents)] = tokens {
        collect_body_return_leaf_types(stream_sents, locals, hir, out);
        return;
    }

    if has_pipe_op(tokens) {
        let (head, steps) = crate::parser::split_pipeline(tokens);
        let mut cur_ty = infer_hir_type_with_locals(&head, locals, hir);

        for (s_idx, step) in steps.iter().enumerate() {
            let is_last_step = s_idx + 1 == steps.len();

            // Propagate bypass union branches from intermediate steps (e.g. `uid . db.find_user . finalize_bill`)
            if !is_last_step && cur_ty.contains('|') {
                for branch in cur_ty.split('|').map(str::trim) {
                    if hir.forms.get(branch).map_or(false, |f| f.is_bypass) {
                        push_unique_return_types(branch, out);
                    }
                }
            }

            if step.op == crate::parser::PipeOp::Dot {
                match step.expr.as_slice() {
                    [Token::Group('{', branches)] if is_last_step => {
                        for b in branches {
                            collect_router_branch_leaf_types(b, &cur_ty, locals, hir, out);
                        }
                        return;
                    }
                    [Token::Group('[', stream_sents)] if is_last_step => {
                        let mut s_locals = locals.clone();
                        if !is_uninformative_return_type(&cur_ty) {
                            s_locals.insert("_".into(), cur_ty.clone());
                        }
                        collect_body_return_leaf_types(stream_sents, &s_locals, hir, out);
                        return;
                    }
                    _ => {}
                }
            }

            let step_slice = &steps[..=s_idx];
            let mut prefix_tokens = head.clone();
            for st in step_slice {
                let op_str = match st.op {
                    crate::parser::PipeOp::Dot => ".",
                    crate::parser::PipeOp::Tap => ".>",
                    crate::parser::PipeOp::Enrich => ".&",
                    crate::parser::PipeOp::Subtract => "./",
                    crate::parser::PipeOp::Filter => ".?",
                    crate::parser::PipeOp::Guard => ".!",
                    crate::parser::PipeOp::Parallel => ".|",
                };
                prefix_tokens.push(Token::Op(op_str.into()));
                prefix_tokens.extend(st.expr.clone());
            }
            cur_ty = infer_hir_type_with_locals(&prefix_tokens, locals, hir);
        }

        push_unique_return_types(&cur_ty, out);
        return;
    }

    let inferred = infer_hir_type_with_locals(tokens, locals, hir);
    push_unique_return_types(&inferred, out);
}

fn collect_router_branch_leaf_types(
    branch: &Sentence,
    input_ty: &str,
    locals: &HashMap<String, String>,
    hir: &HirModule,
    out: &mut Vec<String>,
) {
    match classify_sentence(branch) {
        SemanticKind::StreamOrBranch { name, stream } => {
            let mut b_locals = locals.clone();
            if is_pascal_case(&name) {
                b_locals.insert(pascal_to_snake_case(&name), name.clone());
                b_locals.insert("_".into(), name.clone());
                if let Some(hf) = hir.forms.get(&name) {
                    for (fname, f) in &hf.fields {
                        b_locals.insert(fname.clone(), f.inferred_type.clone());
                    }
                }
            } else if !is_uninformative_return_type(input_ty) {
                b_locals.insert("_".into(), input_ty.to_string());
            }
            collect_body_return_leaf_types(&stream, &b_locals, hir, out);
        }
        SemanticKind::FunctionDef {
            name, args, body, ..
        } => {
            let mut b_locals = locals.clone();
            if is_pascal_case(&name) {
                b_locals.insert(pascal_to_snake_case(&name), name.clone());
                if let Some(hf) = hir.forms.get(&name) {
                    for (idx, arg_s) in args.iter().enumerate() {
                        if let [Token::Path(var_name)] = arg_s.as_slice() {
                            if let Some(fname) = hf.field_order.get(idx) {
                                if let Some(fld) = hf.fields.get(fname) {
                                    b_locals.insert(var_name.clone(), fld.inferred_type.clone());
                                }
                            }
                        }
                    }
                }
            }
            collect_body_return_leaf_types(&body, &b_locals, hir, out);
        }
        SemanticKind::PatternBranch { body, .. } => {
            collect_body_return_leaf_types(&body, locals, hir, out);
        }
        SemanticKind::LambdaOrTupleBranch { args, body } => {
            let mut b_locals = locals.clone();
            for a in &args {
                if let [Token::Path(p)] = a.as_slice() {
                    if is_pascal_case(p) {
                        b_locals.insert(pascal_to_snake_case(p), p.clone());
                    }
                }
            }
            collect_body_return_leaf_types(&body, &b_locals, hir, out);
        }
        _ => {}
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
            && expr.iter().any(|t| matches!(t, Token::Op(op) if op == "&"))
            && !expr.iter().any(|t| matches!(t, Token::Group('{', _)))
        {
            return true;
        }
    }
    false
}

fn register_form(hir: &mut HirModule, name: String, form: HirForm) {
    if let Some(existing) = hir.forms.get(&name) {
        if !existing.is_prelude && !form.is_prelude && form.origin.file_path.is_none() {
            hir.duplicates.push(DuplicateSymbol {
                name: name.clone(),
                kind: "Form",
                first_sentence_index: existing.origin.sentence_index,
                duplicate_origin: form.origin.clone(),
            });
        }
    }
    hir.form_order.retain(|n| n != &name);
    hir.form_order.push(name.clone());
    hir.forms.insert(name, form);
}

fn extract_type_params(params: &[Sentence]) -> Vec<String> {
    params
        .iter()
        .filter_map(|s| match s.as_slice() {
            [Token::Path(_), ..] => Some(format_tokens_as_type(s)),
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
            // HKT 2-го или 3-го порядка: `_F(_)`, `_T(_(_), _)`, `F(_)`, `M(_) Monad`
            [Token::Path(name), Token::Group(')', _), ..] if is_type_or_hole(name) => {
                type_params.push(name.clone());
            }
            // Одиночный идентификатор: `Set(_T)`, `Box(T)`, `Some(value)` vs `UserWithAge(Age)`
            [Token::Path(single)] => {
                if is_type_hole(single) {
                    type_params.push(single.clone());
                } else if is_pascal_case(single) {
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
            // Ограниченный параметр типа `_TC & Category` или `_TC Category` или `TC Category`
            [Token::Path(tp), Token::Path(bound)]
                if is_type_or_hole(tp) && is_pascal_case(bound) =>
            {
                type_params.push(tp.clone());
            }
            [Token::Path(tp), Token::Op(amp), Token::Path(bound), ..]
                if amp == "&" && is_type_or_hole(tp) && is_pascal_case(bound) =>
            {
                type_params.push(tp.clone());
            }
            // Явный слот значения `a Int` или с дефолтом `raw String "0.00"` (`snake_case`)
            [Token::Path(param_name), rest @ ..] => {
                let (constraint, def_expr) = split_type_and_default(rest, hir);
                if is_type_hole(&constraint) && !type_params.contains(&constraint) {
                    type_params.push(constraint.clone());
                }
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
        // Explicit type + default value: `max_speed Int 180`, `active _U current_user`, `items Tuple(_T) ()`
        [Token::Path(ty), rest @ ..]
            if is_type_or_hole(ty)
                && !rest.is_empty()
                && !matches!(rest.first(), Some(Token::Op(_) | Token::Group(')', _))) =>
        {
            (ty.clone(), rest.to_vec())
        }
        [Token::Path(ty), Token::Group(')', targs), rest @ ..]
            if is_type_or_hole(ty)
                && !rest.is_empty()
                && !matches!(rest.first(), Some(Token::Op(_))) =>
        {
            let type_slice = &tokens[..2];
            let _ = targs;
            (format_tokens_as_type(type_slice), rest.to_vec())
        }
        // Only type or type hole: `brand BrandName`, `start _T`, `id String`
        [Token::Path(ty)] if is_type_hole(ty) || (is_pascal_case(ty) && is_known_type(ty, hir)) => {
            (ty.clone(), tokens.to_vec())
        }
        _ => {
            let is_lit_or_expr = matches!(tokens.first(), Some(Token::Lit(_) | Token::Group(_, _)))
                || (tokens.len() >= 2
                    && matches!(&tokens[0], Token::Path(p) if is_pascal_case(p) && !matches!(p.as_str(), "Tuple" | "List" | "Map" | "Set" | "Option" | "Tree" | "RoseTree" | "Channel" | "Atom"))
                    && matches!(&tokens[1], Token::Group(')', _)));
            if is_lit_or_expr {
                (infer_hir_type(tokens, hir), tokens.to_vec())
            } else {
                (format_tokens_as_type(tokens), tokens.to_vec())
            }
        }
    }
}

fn parse_fn_tail_metadata(tail: &[Token], _hir: &HirModule) -> (Vec<Token>, HirVisibility, bool) {
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
    let mut next_idx = 0; // Единый счетчик для обоих источников

    if let Ok(prelude_sentences) = crate::parser::parse_code(PRELUDE_SOURCE) {
        for sentence in &prelude_sentences {
            lower_sentence_into_hir(next_idx, sentence, hir);
            next_idx += 1;
        }
    }

    if let Ok(ide_sentences) = crate::parser::parse_code(IDE_STDLIB_SOURCE) {
        for sentence in &ide_sentences {
            lower_sentence_into_hir(next_idx, sentence, hir);
            next_idx += 1;
        }
    }

    for form in hir.forms.values_mut() {
        form.is_prelude = true;
    }
    for func in hir.functions.values_mut() {
        func.is_prelude = true;
    }
}
/// Единый понижатель всех аксиом форм (`Name { ... }`, `Name(Slots) ...`, `Name Base & { ... }`, `Name A | B`, `Name A / B`)
fn collect_type_holes_from_tokens(tokens: &[Token], out: &mut Vec<String>) {
    for tok in tokens {
        match tok {
            Token::Path(p) if is_type_hole(p) => {
                if !out.contains(p) {
                    out.push(p.clone());
                }
            }
            Token::Group(_, sents) => {
                for s in sents {
                    collect_type_holes_from_tokens(s, out);
                }
            }
            _ => {}
        }
    }
}

fn lower_form_expr(
    name: &str,
    mut type_params: Vec<String>,
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
                    &mut type_params,
                    &mut fields,
                    &mut field_order,
                    &mut ancestor_fields,
                    &mut methods,
                    &mut method_order,
                    &mut ancestors,
                    &mut is_bypass,
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
                    if is_pascal_case(first_type) && first_type != name {
                        ancestors.push(first_type.clone());
                    }
                    if expr_slice[start_idx..]
                        .iter()
                        .any(|t| matches!(t, Token::Op(op) if op == "|"))
                    {
                        union_variants.push(first_type.clone());
                    }
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

    // Автоматически собираем все `_PascalCase` дырки типов (`_T`, `_K`, `_V`, `_N`) из полей формы в `type_params`
    collect_type_holes_from_tokens(expr_slice, &mut type_params);

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
            let substituted = apply_subst_to_method(mf, &subst);
            insert_hir_method(methods, method_order, substituted);
        }
    }
    for anc in &base_form.ancestors {
        if !ancestors.contains(anc) {
            ancestors.push(anc.clone());
        }
    }
}

pub fn substitute_type_vars(input: &str, subst: &HashMap<String, String>) -> String {
    if subst.is_empty() || input.is_empty() {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut current_word = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current_word.push(ch);
        } else {
            if !current_word.is_empty() {
                if let Some(rep) = subst.get(&current_word) {
                    out.push_str(rep);
                } else {
                    out.push_str(&current_word);
                }
                current_word.clear();
            }
            out.push(ch);
        }
    }
    if !current_word.is_empty() {
        if let Some(rep) = subst.get(&current_word) {
            out.push_str(rep);
        } else {
            out.push_str(&current_word);
        }
    }
    out
}

fn apply_subst_to_method(mf: &HirFunction, subst: &HashMap<String, String>) -> HirFunction {
    if subst.is_empty() {
        return mf.clone();
    }
    let mut cloned = mf.clone();
    for p in &mut cloned.params {
        if !p.type_constraint.is_empty() {
            p.type_constraint = substitute_type_vars(&p.type_constraint, subst);
        }
    }
    if let Some(ret) = &mut cloned.return_type {
        *ret = substitute_type_vars(ret, subst);
    }
    if let Some(inf_ret) = &mut cloned.inferred_return_type {
        *inf_ret = substitute_type_vars(inf_ret, subst);
    }
    cloned
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
    type_params: &mut Vec<String>,
    fields: &mut HashMap<String, HirField>,
    field_order: &mut Vec<String>,
    ancestor_fields: &mut HashMap<String, HashMap<String, HirField>>,
    methods: &mut HashMap<String, HirFunction>,
    method_order: &mut Vec<String>,
    ancestors: &mut Vec<String>,
    is_bypass: &mut bool,
    mut submodules: Option<&mut HashMap<String, HirForm>>,
    owner_origin: &SourceOrigin,
    hir: &HirModule,
) {
    for (f_idx, s) in sentences.iter().enumerate() {
        let member_origin = SourceOrigin {
            sentence_index: owner_origin.sentence_index,
            file_path: owner_origin.file_path.clone(),
            symbol_context: format!("{owner_name}.{f_idx}"),
            raw_ast: s.clone(),
        };

        // Axiom 3 (V6.4): Prefix Lineage Spread `..Ancestor` или `..Functor(_M)` внутри `{}`!
        if let [Token::Op(op), Token::Path(anc_name), rest @ ..] = s.as_slice() {
            if op == ".." {
                let spread_targs = if let Some(Token::Group(')', args)) = rest.first() {
                    extract_type_params(args)
                } else {
                    vec![]
                };
                if !ancestors.contains(anc_name) {
                    ancestors.push(anc_name.clone());
                }
                if let Some(base_form) = hir.forms.get(anc_name) {
                    let mut subst = HashMap::new();
                    for (param_raw, arg_val) in
                        base_form.type_params.iter().zip(spread_targs.iter())
                    {
                        let param_name = param_raw.split_whitespace().next().unwrap_or(param_raw);
                        let param_clean = param_name.split('(').next().unwrap_or(param_name);
                        subst.insert(param_clean.to_string(), arg_val.clone());
                    }
                    if base_form.is_bypass {
                        *is_bypass = true;
                    }
                    ancestor_fields.insert(anc_name.clone(), base_form.fields.clone());
                    for (anc_k, anc_map) in &base_form.ancestor_fields {
                        ancestor_fields.insert(anc_k.clone(), anc_map.clone());
                    }
                    for anc in &base_form.ancestors {
                        if !ancestors.contains(anc) {
                            ancestors.push(anc.clone());
                        }
                    }
                    for k in &base_form.field_order {
                        if let Some(rhs_f) = base_form.fields.get(k) {
                            let mut cloned_f = rhs_f.clone();
                            if !subst.is_empty() {
                                cloned_f.inferred_type =
                                    substitute_type_vars(&cloned_f.inferred_type, &subst);
                            }
                            if let Some(existing) = fields.get_mut(k) {
                                if existing.origin_form != cloned_f.origin_form
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
                                        .contains(&cloned_f.origin_form)
                                    {
                                        existing
                                            .conflicting_origins
                                            .push(cloned_f.origin_form.clone());
                                    }
                                }
                            } else {
                                field_order.push(k.clone());
                                fields.insert(k.clone(), cloned_f);
                            }
                        }
                    }
                    for m in &base_form.method_order {
                        if let Some(rhs_m) = base_form.methods.get(m) {
                            if !methods.contains_key(m) {
                                method_order.push(m.clone());
                                methods.insert(m.clone(), apply_subst_to_method(rhs_m, &subst));
                            }
                        }
                    }
                }
                continue;
            }
        }

        // Sugar #4 (V6.4): Auto-Generic Open Slot `Box { item }` -> `item _T`
        if let [Token::Path(open_slot)] = s.as_slice() {
            if !is_pascal_case(open_slot) && !is_type_hole(open_slot) && open_slot != "_" {
                if !type_params.contains(&"_T".to_string()) {
                    type_params.push("_T".into());
                }
                insert_hir_field(
                    fields,
                    field_order,
                    HirField {
                        name: open_slot.clone(),
                        inferred_type: "_T".into(),
                        origin_form: owner_name.to_string(),
                        default_expr: vec![],
                        is_reactive: false,
                        is_ambiguous: false,
                        conflicting_origins: vec![],
                        visibility: HirVisibility::Default,
                        origin: member_origin,
                    },
                );
                continue;
            }
        }

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
                let reactive_ty =
                    infer_fn_body_return_type(&[], &stream, Some(owner_name), Some(fields), hir)
                        .unwrap_or_else(|| "ReactiveStream".into());
                insert_hir_field(
                    fields,
                    field_order,
                    HirField {
                        name: field_name,
                        inferred_type: reactive_ty,
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
                let mut func = build_hir_function(
                    &method_name,
                    &args,
                    fn_body,
                    &fn_tail,
                    false,
                    member_origin,
                    hir,
                );
                if func.return_type.is_none() && func.inferred_return_type.is_none() {
                    func.inferred_return_type = infer_fn_body_return_type(
                        &func.params,
                        &func.body,
                        Some(owner_name),
                        Some(fields),
                        hir,
                    );
                }
                if func.inferred_return_type.is_none()
                    && matches!(method_name.as_str(), "+" | "-" | "*" | "/")
                    && is_pascal_case(owner_name)
                {
                    func.inferred_return_type = Some(owner_name.to_string());
                }
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
                let mut dummy_tparams = Vec::new();
                populate_hir_form_members(
                    owner_name,
                    override_sents,
                    &mut dummy_tparams,
                    fields,
                    field_order,
                    ancestor_fields,
                    methods,
                    method_order,
                    ancestors,
                    is_bypass,
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
                            let rhs_type_args = if let Some(Token::Group(')', args)) = tail.get(i + 2)
                            {
                                extract_type_params(args)
                            } else {
                                Vec::new()
                            };
                            if let Some(rhs_form) = hir.forms.get(rhs_type) {
                                let mut subst = HashMap::new();
                                for (param_raw, arg_val) in
                                    rhs_form.type_params.iter().zip(rhs_type_args.iter())
                                {
                                    let param_name =
                                        param_raw.split_whitespace().next().unwrap_or(param_raw);
                                    let param_clean =
                                        param_name.split('(').next().unwrap_or(param_name);
                                    subst.insert(param_clean.to_string(), arg_val.clone());
                                }
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
                                        let mut cloned_f = rhs_f.clone();
                                        if !subst.is_empty() {
                                            cloned_f.inferred_type =
                                                substitute_type_vars(&cloned_f.inferred_type, &subst);
                                        }
                                        if let Some(existing) = fields.get_mut(k) {
                                            if existing.origin_form != cloned_f.origin_form
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
                                                    .contains(&cloned_f.origin_form)
                                                {
                                                    existing
                                                        .conflicting_origins
                                                        .push(cloned_f.origin_form.clone());
                                                }
                                            }
                                        } else {
                                            field_order.push(k.clone());
                                            fields.insert(k.clone(), cloned_f);
                                        }
                                    }
                                }
                                for m in &rhs_form.method_order {
                                    if let Some(rhs_m) = rhs_form.methods.get(m) {
                                        if !methods.contains_key(m) {
                                            method_order.push(m.clone());
                                            methods.insert(
                                                m.clone(),
                                                apply_subst_to_method(rhs_m, &subst),
                                            );
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
                            let mut dummy_tparams = Vec::new();
                            populate_hir_form_members(
                                owner_name,
                                override_sents,
                                &mut dummy_tparams,
                                fields,
                                field_order,
                                ancestor_fields,
                                methods,
                                method_order,
                                ancestors,
                                is_bypass,
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
    lower_fn_params_with_ret(args, &[], hir)
}

pub fn lower_fn_params_with_ret(
    args: &[Sentence],
    ret_tokens: &[Token],
    hir: &HirModule,
) -> (Vec<HirParam>, bool) {
    let mut out = Vec::new();
    let mut auto_unpacks = false;

    for (idx, sent) in args.iter().enumerate() {
        match sent.as_slice() {
            // HKT Type Param: `lift(_M(_) Monad, ma _M(_A))` или `lift(M(_) Monad, A, ma M(A))`
            [Token::Path(tp), Token::Group(')', _), ..] if is_type_or_hole(tp) => {
                out.push(HirParam {
                    name: tp.clone(),
                    type_constraint: tp.clone(),
                    default_expr: vec![],
                    is_anonymous_type_slot: false,
                    is_type_param: true,
                    destructured_fields: vec![],
                });
            }
            // Variadic rest parameter: `..rest`
            [Token::Op(op), Token::Path(rest_name)] if op == ".." => {
                out.push(HirParam {
                    name: rest_name.clone(),
                    type_constraint: "Tuple".into(),
                    default_expr: vec![],
                    is_anonymous_type_slot: false,
                    is_type_param: false,
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
            // Одиночный идентификатор: `derive_json(_TargetForm)` vs `fn(User)` vs `fmap(A, B, ...)` vs `fn(x)`
            [Token::Path(single)] => {
                if is_type_hole(single) {
                    out.push(HirParam {
                        name: single.clone(),
                        type_constraint: "Form".into(),
                        default_expr: vec![],
                        is_anonymous_type_slot: false,
                        is_type_param: false,
                        destructured_fields: vec![],
                    });
                    continue;
                }
                let is_upper = is_pascal_case(single);
                if is_upper {
                    let used_as_type_later = args.iter().skip(idx + 1).any(|later| {
                        later
                            .iter()
                            .skip(1)
                            .any(|t| token_references_ident(t, single))
                    }) || ret_tokens
                        .iter()
                        .any(|t| token_references_ident(t, single));
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
                        // Закон 1c: Одиночный PascalCase тип `fn(User)` всегда авто-привязывает snake_case `user`
                        let var_name = pascal_to_snake_case(single);
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
                && !matches!(
                    def.first(),
                    Some(Token::Op(_) | Token::Group(')', _) | Token::Group('[', _))
                ) =>
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
    infer_hir_type_with_locals(tokens, &HashMap::new(), hir)
}

pub fn infer_hir_type_with_locals(
    tokens: &[Token],
    locals: &HashMap<String, String>,
    hir: &HirModule,
) -> String {
    if tokens.is_empty() {
        return "Any".into();
    }

    // 1. Пайплайн `head . step1 . step2`
    if has_pipe_op(tokens) {
        let (head, steps) = crate::parser::split_pipeline(tokens);
        let mut current = infer_hir_type_with_locals(&head, locals, hir);
        for step in steps {
            match step.op {
                crate::parser::PipeOp::Tap => {}
                crate::parser::PipeOp::Filter | crate::parser::PipeOp::Guard => {}
                crate::parser::PipeOp::Subtract => {
                    let sub_ty = format_tokens_as_type(&step.expr);
                    if !sub_ty.is_empty() {
                        current = format!("{current} / {sub_ty}");
                    }
                }
                crate::parser::PipeOp::Parallel => {
                    current = "Form".into();
                }
                crate::parser::PipeOp::Enrich => {
                    let step_ty = infer_hir_type_with_locals(&step.expr, locals, hir);
                    if step_ty != "Any" && step_ty != "Form" && step_ty != "Stream" {
                        current = format!("{current} & {step_ty}");
                    }
                }
                crate::parser::PipeOp::Dot => {
                    match step.expr.as_slice() {
                        [Token::Group('{', branches)] => {
                            let mut branch_types = Vec::new();
                            for b in branches {
                                collect_router_branch_leaf_types(
                                    b,
                                    &current,
                                    locals,
                                    hir,
                                    &mut branch_types,
                                );
                            }
                            if !branch_types.is_empty() {
                                current = branch_types.join(" | ");
                            }
                        }
                        [Token::Path(callee), ..] => {
                            if let Some(ret) = resolve_callee_return_type(callee, hir) {
                                current = ret;
                            }
                        }
                        [Token::Group('[', stream)] => {
                            if let Some(last) = stream.last() {
                                let mut s_locals = locals.clone();
                                s_locals.insert("_".into(), current.clone());
                                current = infer_hir_type_with_locals(last, &s_locals, hir);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        return current;
    }

    if has_algebra_op(tokens) {
        if tokens
            .iter()
            .any(|t| matches!(t, Token::Op(op) if op == "|"))
        {
            return format_tokens_as_type(tokens);
        }
        // `o & Reserved { ... }` или `my_car & { ... }` или `entity / Cloud.name`
        if let [Token::Path(lhs), Token::Op(op), Token::Path(rhs), Token::Group('{', _)] = tokens {
            if op == "&" && is_pascal_case(rhs) {
                let lhs_ty = locals.get(lhs).cloned().unwrap_or_else(|| lhs.clone());
                return format!("{lhs_ty} & {rhs}");
            }
        }
        if let [Token::Path(lhs), Token::Op(op), Token::Group('{', _)] = tokens {
            if op == "&" {
                return locals.get(lhs).cloned().unwrap_or_else(|| lhs.clone());
            }
        }
        if let [Token::Path(lhs), Token::Group(')', targs), Token::Op(op), Token::Group('{', _)] =
            tokens
        {
            if op == "&" && is_pascal_case(lhs) {
                let _ = targs;
                return format_tokens_as_type(&tokens[..2]);
            }
        }
        if let [Token::Path(lhs), Token::Op(op), Token::Path(rhs)] = tokens {
            if op == "/" {
                let lhs_ty = locals.get(lhs).cloned().unwrap_or_else(|| lhs.clone());
                return format!("{lhs_ty} / {rhs}");
            }
        }
    }

    // 2. Логические и сравнительные выражения
    if tokens.iter().any(|t| {
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

    // 3. Бинарные арифметические операции (`self.price * self.quantity`, `a + b`)
    if tokens.len() >= 3 {
        if let Some(Token::Op(op)) = tokens.get(1) {
            if matches!(op.as_str(), "+" | "-" | "*" | "/" | "%" | "**" | "<<" | ">>") {
                let lhs_ty = infer_hir_type_with_locals(&tokens[..1], locals, hir);
                let rhs_ty = infer_hir_type_with_locals(&tokens[2..], locals, hir);
                if let Some(hf) = hir.forms.get(&lhs_ty) {
                    if let Some(m) = hf.methods.get(op) {
                        if let Some(ret) = m.effective_return_type() {
                            return ret.to_string();
                        }
                    }
                    if is_pascal_case(&lhs_ty) && hf.methods.contains_key(op) {
                        return lhs_ty;
                    }
                }
                if lhs_ty == "Float" || rhs_ty == "Float" {
                    return "Float".into();
                }
                if lhs_ty == "Int" && rhs_ty == "Int" {
                    return "Int".into();
                }
                if lhs_ty == "String" && op == "+" {
                    return "String".into();
                }
            }
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
            // Инстанцирование формы `Order { id "o1", amount 100 }`
            if matches!(tokens.get(1), Some(Token::Group('{', _))) && is_pascal_case(p) {
                return p.clone();
            }
            if let Some(Token::Group(')', args)) = tokens.get(1) {
                // Вызов функции или метода `connect(...)` / `Money.from_scaled(...)`
                if let Some(ret) = resolve_callee_return_type(p, hir) {
                    return ret;
                }
                if hir
                    .forms
                    .get(p)
                    .map(|f| f.type_params.is_empty())
                    .unwrap_or(false)
                    || p == "Signal"
                {
                    return p.clone();
                }
                if let Some(hf) = hir.forms.get(p) {
                    if !hf.type_params.is_empty() {
                        if let Some(first_arg) = args.first() {
                            let inner_ty = infer_hir_type_with_locals(first_arg, locals, hir);
                            if inner_ty != "Any" && is_pascal_case(&inner_ty) {
                                return format!("{p}({inner_ty})");
                            }
                        }
                    }
                }
                format_tokens_as_type(tokens)
            } else {
                resolve_path_type_in_context(p, locals, hir)
            }
        }
        Some(Token::Range(_, _)) => "Range".into(),
        Some(Token::Group('(', inner)) => {
            if inner.len() == 1 {
                return infer_hir_type_with_locals(&inner[0], locals, hir);
            }
            "Tuple".into()
        }
        Some(Token::Group('{', _)) => "Form".into(),
        Some(Token::Group('[', _)) => "Stream".into(),
        _ => "Any".into(),
    }
}

fn resolve_callee_return_type(callee: &str, hir: &HirModule) -> Option<String> {
    if let Some(f) = hir.functions.get(callee) {
        return f.effective_return_type().map(str::to_string);
    }
    if let Some((owner, method)) = callee.rsplit_once('.') {
        let owner_type = hir
            .globals
            .get(owner)
            .map(|g| g.inferred_type.as_str())
            .unwrap_or(owner);
        let base_owner = owner_type.split('(').next().unwrap_or(owner_type);
        if let Some(hf) = hir.forms.get(base_owner) {
            if let Some(m) = hf.methods.get(method) {
                return m.effective_return_type().map(str::to_string);
            }
        }
    }
    None
}

fn resolve_path_type_in_context(
    path: &str,
    locals: &HashMap<String, String>,
    hir: &HirModule,
) -> String {
    if let Some(ty) = locals.get(path) {
        return ty.clone();
    }
    if let Some((root, rest)) = path.split_once('.') {
        let root_clean = root.strip_suffix("()").unwrap_or(root);
        let mut current_ty = locals
            .get(root_clean)
            .cloned()
            .or_else(|| hir.globals.get(root_clean).map(|g| g.inferred_type.clone()))
            .unwrap_or_else(|| root_clean.to_string());

        for seg in rest.split('.') {
            let (seg_name, is_call) = seg
                .strip_suffix("()")
                .map(|s| (s, true))
                .unwrap_or((seg, false));
            if seg_name == "to_float" {
                current_ty = "Float".into();
                continue;
            }
            if seg_name == "round_to_int" || seg_name == "length" {
                current_ty = "Int".into();
                continue;
            }
            let base_form = current_ty.split('(').next().unwrap_or(&current_ty);
            let type_arg = current_ty
                .split_once('(')
                .and_then(|(_, r)| r.strip_suffix(')'));
            if let Ok(idx) = seg_name.parse::<i32>() {
                if base_form == "Tuple" {
                    if let Some(args_str) = type_arg {
                        let elems: Vec<&str> = args_str.split(',').map(str::trim).collect();
                        let i = if idx < 0 {
                            (elems.len() as i32 + idx) as usize
                        } else {
                            idx as usize
                        };
                        if let Some(el) = elems.get(i).or_else(|| elems.first()) {
                            current_ty = (*el).to_string();
                            continue;
                        }
                    }
                }
                if let Some(hf) = hir.forms.get(base_form) {
                    if hf.ancestors.contains(&"Tuple".to_string()) && !hf.type_args.is_empty() {
                        let i = if idx < 0 {
                            (hf.type_args.len() as i32 + idx) as usize
                        } else {
                            idx as usize
                        };
                        if let Some(el) = hf.type_args.get(i).or_else(|| hf.type_args.first()) {
                            current_ty = el.clone();
                            continue;
                        }
                    }
                }
            }
            if let Some(hf) = hir.forms.get(base_form) {
                if is_call {
                    if let Some(m) = hf.methods.get(seg_name) {
                        if let Some(ret) = m.effective_return_type() {
                            current_ty = substitute_type_param(ret, &hf.type_params, type_arg);
                            continue;
                        }
                    }
                }
                if let Some(f) = hf.fields.get(seg_name) {
                    current_ty = substitute_type_param(&f.inferred_type, &hf.type_params, type_arg);
                    continue;
                }
                if let Some(m) = hf.methods.get(seg_name) {
                    if let Some(ret) = m.effective_return_type() {
                        current_ty = substitute_type_param(ret, &hf.type_params, type_arg);
                        continue;
                    }
                }
                if hf.ancestors.contains(&seg_name.to_string()) {
                    current_ty = seg_name.to_string();
                    continue;
                }
            }
        }
        return current_ty;
    }
    if let Some(g) = hir.globals.get(path) {
        return g.inferred_type.clone();
    }
    path.to_string()
}

fn substitute_type_param(raw_ty: &str, type_params: &[String], type_arg: Option<&str>) -> String {
    if let (Some(first_tp), Some(arg)) = (type_params.first(), type_arg) {
        if raw_ty == first_tp {
            return arg.to_string();
        }
        if raw_ty.contains(&format!("({first_tp})")) {
            return raw_ty.replace(&format!("({first_tp})"), &format!("({arg})"));
        }
    }
    raw_ty.to_string()
}
