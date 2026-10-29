// src/lir.rs
// =============================================================================
// RAZOMY V5.7 — UNIFIED CANONICAL LIR FOR RUNTIME & MULTI-TARGET CODEGEN
// Архитектура: AST -> HIR -> Canonical LIR -> (LIR Runtime VM | TypeScript | C#)
// Вся семантика Razomy (Законы 1–4, Lineage, распаковка, пайплайны, роутеры)
// разрешается ОДИН РАЗ при понижении в LIR!
// =============================================================================

use crate::hir::{
    is_pascal_case, pascal_to_snake_case, HirField, HirForm, HirFunction, HirModule, HirParam,
};
use crate::parser::{
    classify_sentence, has_pipe_op, split_pipeline, Literal, PipeOp, SemanticKind, Sentence, Token,
};

// =============================================================================
// 1. КАНОНИЧЕСКИЙ LIR (ЯЗЫКОНЕЗАВИСИМОЕ ДЕРЕВО ИСПОЛНЕНИЯ И КОДОГЕНЕРАЦИИ)
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    TypeScript,
    CSharp,
}

#[derive(Debug, Clone)]
pub struct LirModule {
    pub structs: Vec<LirStruct>,
    pub functions: Vec<LirFunc>,
    pub globals: Vec<LirBinding>,
    pub top_exprs: Vec<LirExpr>,
    /// Сырые предложения верхнего уровня для гибридного выполнения макросов и мета-операций
    pub raw_sentences: Vec<Sentence>,
}

#[derive(Debug, Clone)]
pub struct LirStruct {
    pub name: String,
    pub type_params: Vec<String>,
    pub ancestors: Vec<String>,
    pub fields: Vec<LirField>,
    pub reactive_getters: Vec<LirGetter>,
    pub methods: Vec<LirFunc>,
    pub union_variants: Vec<String>,
    pub is_bypass: bool,
}

#[derive(Debug, Clone)]
pub struct LirField {
    pub name: String,
    pub ty: String,
    pub origin: String,
    pub default_val: Option<LirExpr>,
}

#[derive(Debug, Clone)]
pub struct LirGetter {
    pub name: String,
    pub body: Vec<LirStmt>,
    pub raw_stream: Vec<Sentence>,
}

#[derive(Debug, Clone)]
pub struct LirFunc {
    pub name: String,
    pub raw_name: String,
    pub params: Vec<(String, String, Option<LirExpr>)>,
    /// Полное ограничение первого слота значений (например, `Order / Cancelled` или `Order & Reserved`) для Law 3
    pub first_param_constraint: String,
    /// Закон 1c (V5.7): Авто-привязанное `snake_case` имя переменной (`age` для `fn(Age)`, `user` для `fn(User)`)
    /// + дополнительные распакованные поля из `{ name } WithName`
    pub auto_unpacked_fields: Vec<String>,
    pub return_ty: String,
    pub body: Vec<LirStmt>,
    pub raw_args: Vec<Sentence>,
    pub raw_body: Vec<Sentence>,
}

#[derive(Debug, Clone)]
pub struct LirBinding {
    pub name: String,
    pub ty: String,
    pub value: LirExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LirStmt {
    Let { name: String, value: LirExpr },
    /// Закон 1b / Раздел 3: In-place мутация пути или индекса `my_car.brand "Tesla"`, `my_car.-1 42`, `corp.departments.0.budget 2500`
    AssignPath { path: String, value: LirExpr },
    DestructTuple { names: Vec<String>, value: LirExpr },
    Expr(LirExpr),
    Return(LirExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LirExpr {
    None,
    Int(i64),
    Float(f64),
    Bool(bool),
    Byte(u8),
    Str(String),
    Symbol(String),
    InterpStr(Vec<LirInterpPart>),
    Ident(String),
    Tuple(Vec<LirExpr>),
    Range(Box<LirExpr>, Box<LirExpr>),
    StructInit {
        type_name: String,
        fields: Vec<(String, LirExpr)>,
    },
    Binary {
        left: Box<LirExpr>,
        op: String,
        right: Box<LirExpr>,
    },
    Unary {
        op: String,
        expr: Box<LirExpr>,
    },
    Call {
        callee: String,
        args: Vec<LirExpr>,
    },
    Pipe {
        input: Box<LirExpr>,
        op: PipeOp,
        step: Box<LirExpr>,
    },
    Router {
        input: Box<LirExpr>,
        branches: Vec<LirBranch>,
    },
    Lambda {
        params: Vec<String>,
        body: Vec<LirStmt>,
        raw_args: Vec<Sentence>,
        raw_body: Vec<Sentence>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LirInterpPart {
    Lit(String),
    Expr(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LirBranch {
    pub cond: LirPattern,
    /// Закон 1c: Имя авто-привязанной переменной в ветке роутера (`circle` для `Circle [...]`) и распакованные поля
    pub bound_type_var: Option<String>,
    pub unpacked_fields: Vec<String>,
    pub body: Vec<LirStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LirPattern {
    Wildcard,
    TypeMatch(String),
    LiteralMatch(LirExpr),
    RangeMatch(Box<LirExpr>, Box<LirExpr>),
    Predicate(LirExpr),
    TupleMatch(Vec<LirTuplePatElem>),
    FieldMatch(Vec<(String, LirFieldPatValue)>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LirTuplePatElem {
    Wildcard,
    /// Закон 1c в кортежах роутера `(Disconnected, ConnectCmd)`: матчит тип `Disconnected` и связывает `disconnected`!
    TypeBind { type_name: String, var_name: String },
    Bind(String),
    Exact(LirExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LirFieldPatValue {
    Expr(LirExpr),
    Bind(String),
    Tuple(Vec<LirTuplePatElem>),
}

// =============================================================================
// 2. ЕДИНЫЙ ПОНИЖАТЕЛЬ HIR -> LIR (РЕШАЕТ ВСЮ СЕМАНТИКУ RAZOMY ОДИН РАЗ!)
// =============================================================================

pub fn lower_hir_to_lir(hir: &HirModule) -> LirModule {
    let mut structs = Vec::new();
    let mut functions = Vec::new();
    let mut globals = Vec::new();
    let mut top_exprs = Vec::new();

    // 1. Формы (с уже разрешенными в HIR полями предков, реактивными геттерами и методами)
    for form_name in &hir.form_order {
        if matches!(
            form_name.as_str(),
            "Shell" | "Compile" | "Bypass" | "Public" | "Private"
        ) {
            continue;
        }
        // Пропускаем `snake_case` экземпляры форм (`global_counter`, `order_1`), которые уже в `globals`
        if !is_pascal_case(form_name) && hir.globals.contains_key(form_name) {
            continue;
        }
        if let Some(hf) = hir.forms.get(form_name) {
            structs.push(lower_form_to_lir(hf, hir));
        }
    }

    // 2. Глобальные константы и привязки
    for g_name in &hir.global_order {
        if let Some(g) = hir.globals.get(g_name) {
            // Пропускаем чистые алиасы типов (`BrandName String`, `Age Int`) и импорты (`_ root.base`)
            if g_name == "_"
                || (is_pascal_case(g_name)
                && hir.forms.contains_key(g_name)
                && matches!(g.value_expr.as_slice(), [Token::Path(_)]))
            {
                continue;
            }
            if hir.imports.iter().any(|imp| imp.targets.contains(g_name)) {
                continue;
            }
            globals.push(LirBinding {
                name: g.key.clone(),
                ty: g.inferred_type.clone(),
                value: lower_tokens_to_expr(&g.value_expr, hir),
            });
        }
    }

    // 3. Функции и потоки
    for fn_name in &hir.function_order {
        if let Some(hf) = hir.functions.get(fn_name) {
            functions.push(lower_fn_to_lir(hf, hir));
        }
    }

    // 4. Выражения верхнего уровня (пайплайны или вызовы)
    for sent in &hir.top_level_sentences {
        if matches!(
            classify_sentence(sent),
            SemanticKind::Pipeline(_) | SemanticKind::CallExpr { .. } | SemanticKind::Expression(_)
        ) {
            if matches!(
                sent.as_slice(),
                [Token::Path(_), Token::Group('{', _), Token::Op(op), Token::Path(_)] if op == "."
            ) {
                continue;
            }
            top_exprs.push(lower_tokens_to_expr(sent, hir));
        }
    }

    LirModule {
        structs,
        functions,
        globals,
        top_exprs,
        raw_sentences: hir.top_level_sentences.clone(),
    }
}

fn lower_form_to_lir(hf: &HirForm, hir: &HirModule) -> LirStruct {
    let mut fields = Vec::new();
    let mut reactive_getters = Vec::new();

    for fname in &hf.field_order {
        if let Some(f) = hf.fields.get(fname) {
            if f.is_reactive {
                if let [Token::Group('[', sents)] = f.default_expr.as_slice() {
                    reactive_getters.push(LirGetter {
                        name: f.name.clone(),
                        body: lower_body_to_stmts(sents, hir),
                        raw_stream: sents.clone(),
                    });
                }
            } else {
                fields.push(lower_field_to_lir(f, hir));
            }
        }
    }

    let methods = hf
        .method_order
        .iter()
        .filter_map(|m| hf.methods.get(m).map(|mf| lower_fn_to_lir(mf, hir)))
        .collect();

    LirStruct {
        name: hf.name.clone(),
        type_params: hf.type_params.clone(),
        ancestors: hf.ancestors.clone(),
        fields,
        reactive_getters,
        methods,
        union_variants: hf.union_variants.clone(),
        is_bypass: hf.is_bypass,
    }
}

fn lower_field_to_lir(f: &HirField, hir: &HirModule) -> LirField {
    let is_type_only = matches!(
        f.default_expr.as_slice(),
        [Token::Path(p)] if matches!(
            p.as_str(),
            "String" | "Symbol" | "Int" | "Float" | "Bool" | "Byte" | "Money" | "Any"
        ) || (is_pascal_case(p) && hir.forms.contains_key(p))
    ) || f
        .default_expr
        .iter()
        .any(|t| matches!(t, Token::Op(op) if op == "|"));

    let default_val = if f.default_expr.is_empty() || is_type_only {
        None
    } else {
        Some(lower_tokens_to_expr(&f.default_expr, hir))
    };

    LirField {
        name: f.name.clone(),
        ty: f.inferred_type.clone(),
        origin: f.origin_form.clone(),
        default_val,
    }
}

pub fn lower_fn_to_lir(hf: &HirFunction, hir: &HirModule) -> LirFunc {
    let mut params = Vec::new();
    let mut auto_unpacked_fields = Vec::new();

    // Отфильтровываем чистые compile-time параметры типа (`A`, `B` в `fmap(A, B, fa F(A))`) для рантайма
    let runtime_params: Vec<&HirParam> =
        hf.params.iter().filter(|p| !p.is_type_param).collect();

    let first_param_constraint = runtime_params
        .first()
        .map(|p| p.type_constraint.clone())
        .unwrap_or_default();

    if runtime_params.is_empty() {
        params.push(("_".into(), "Any".into(), None));
    } else {
        for (idx, p) in runtime_params.iter().enumerate() {
            let param_var =
                if idx == 0 && (p.is_anonymous_type_slot || !p.destructured_fields.is_empty()) {
                    "_".to_string()
                } else {
                    p.name.clone()
                };
            let def = if p.default_expr.is_empty() {
                None
            } else {
                Some(lower_tokens_to_expr(&p.default_expr, hir))
            };
            params.push((param_var, normalize_param_type(p), def));

            // Закон 1c: Если параметр `Age` авто-привязывает `age` (или `User` -> `user`),
            // а также сохраняем распаковку полей для обратной совместимости и `{ name } WithName` (Law 1d)
            if p.is_anonymous_type_slot {
                if idx == 0 && p.name != "_" && !auto_unpacked_fields.contains(&p.name) {
                    auto_unpacked_fields.push(format!("@self_bind:{}", p.name));
                }
                for part in p.type_constraint.split_whitespace() {
                    if matches!(part, "&" | "|" | "/") {
                        continue;
                    }
                    if let Some(form) = hir.forms.get(part) {
                        for fname in &form.field_order {
                            if let Some(f) = form.fields.get(fname) {
                                if !f.is_ambiguous && !auto_unpacked_fields.contains(fname) {
                                    auto_unpacked_fields.push(fname.clone());
                                }
                            }
                        }
                    }
                }
            }
            for df in &p.destructured_fields {
                if !auto_unpacked_fields.contains(df) {
                    auto_unpacked_fields.push(df.clone());
                }
            }
        }
    }

    let raw_args = match classify_sentence(&hf.origin.raw_ast) {
        SemanticKind::FunctionDef { args, .. } | SemanticKind::OperatorMethod { args, .. } => args,
        _ => Vec::new(),
    };

    LirFunc {
        name: sanitize_fn_name(&hf.name),
        raw_name: hf.name.clone(),
        params,
        first_param_constraint,
        auto_unpacked_fields,
        return_ty: hf.return_type.clone().unwrap_or_else(|| "Any".into()),
        body: lower_body_to_stmts(&hf.body, hir),
        raw_args,
        raw_body: hf.body.clone(),
    }
}

fn normalize_param_type(p: &HirParam) -> String {
    if p.type_constraint.is_empty() {
        "Any".into()
    } else {
        p.type_constraint
            .split_whitespace()
            .next()
            .unwrap_or("Any")
            .to_string()
    }
}

pub fn sanitize_fn_name(name: &str) -> String {
    match name {
        "+" => "op_add".into(),
        "-" => "op_sub".into(),
        "*" => "op_mul".into(),
        "/" => "op_div".into(),
        "%" => "op_mod".into(),
        "**" => "op_pow".into(),
        "==" => "op_eq".into(),
        "!=" => "op_neq".into(),
        "<" => "op_lt".into(),
        ">" => "op_gt".into(),
        "<=" => "op_lte".into(),
        ">=" => "op_gte".into(),
        other => other.to_string(),
    }
}

pub fn lower_body_to_stmts(sentences: &[Sentence], hir: &HirModule) -> Vec<LirStmt> {
    let mut stmts = Vec::new();
    let len = sentences.len();
    for (i, s) in sentences.iter().enumerate() {
        let is_last = i + 1 == len;
        match classify_sentence(s) {
            SemanticKind::SubtypeFormDef {
                name,
                base,
                body,
                tail,
            } => {
                // Внутри тела функции `raw_order Order { id "o1", amount 100 }` — это связывание локальной переменной `raw_order`!
                let mut val_tokens = vec![Token::Path(base), Token::Group('{', body)];
                val_tokens.extend(tail);
                let expr = lower_tokens_to_expr(&val_tokens, hir);
                stmts.push(LirStmt::Let {
                    name: name.clone(),
                    value: expr,
                });
                if is_last {
                    stmts.push(LirStmt::Return(LirExpr::Ident(name)));
                }
            }
            SemanticKind::AlgebraicDef { name, expr: val_tokens } => {
                let expr = lower_tokens_to_expr(&val_tokens, hir);
                stmts.push(LirStmt::Let {
                    name: name.clone(),
                    value: expr,
                });
                if is_last {
                    stmts.push(LirStmt::Return(LirExpr::Ident(name)));
                }
            }
            SemanticKind::Binding { key, value } => {
                let expr = lower_tokens_to_expr(&value, hir);
                if key.contains('.') {
                    stmts.push(LirStmt::AssignPath {
                        path: key.clone(),
                        value: expr,
                    });
                    if is_last {
                        let root = key.split('.').next().unwrap_or(&key).to_string();
                        stmts.push(LirStmt::Return(LirExpr::Ident(root)));
                    }
                } else {
                    stmts.push(LirStmt::Let {
                        name: key.clone(),
                        value: expr,
                    });
                    if is_last {
                        stmts.push(LirStmt::Return(LirExpr::Ident(key)));
                    }
                }
            }
            SemanticKind::TupleBinding { tuple, value } => {
                if matches!(value.first(), Some(Token::Op(_))) {
                    let expr = lower_tokens_to_expr(s, hir);
                    stmts.push(if is_last {
                        LirStmt::Return(expr)
                    } else {
                        LirStmt::Expr(expr)
                    });
                } else {
                    let names = tuple
                        .iter()
                        .filter_map(|t| match t.as_slice() {
                            [Token::Path(p)] => Some(p.clone()),
                            _ => None,
                        })
                        .collect();
                    stmts.push(LirStmt::DestructTuple {
                        names,
                        value: lower_tokens_to_expr(&value, hir),
                    });
                }
            }
            _ => {
                let expr = lower_tokens_to_expr(s, hir);
                stmts.push(if is_last {
                    LirStmt::Return(expr)
                } else {
                    LirStmt::Expr(expr)
                });
            }
        }
    }
    stmts
}

pub fn lower_tokens_to_expr(tokens: &[Token], hir: &HirModule) -> LirExpr {
    if tokens.is_empty() {
        return LirExpr::None;
    }

    // 1. Пайплайны (`.`, `.>`, `.&`, `.?`, `.|`)
    if has_pipe_op(tokens) {
        let (head, steps) = split_pipeline(tokens);
        let mut current = lower_tokens_to_expr(&head, hir);
        for step in steps {
            if step.op == PipeOp::Dot {
                if let [Token::Group('{', branches)] = step.expr.as_slice() {
                    current = LirExpr::Router {
                        input: Box::new(current),
                        branches: lower_router_branches(branches, hir),
                    };
                    continue;
                }
            }
            current = LirExpr::Pipe {
                input: Box::new(current),
                op: step.op,
                step: Box::new(lower_tokens_to_expr(&step.expr, hir)),
            };
        }
        return current;
    }

    // 2. Унарный `!`
    if let [Token::Op(op), rest @ ..] = tokens {
        if op == "!" && !rest.is_empty() {
            return LirExpr::Unary {
                op: "!".into(),
                expr: Box::new(lower_tokens_to_expr(rest, hir)),
            };
        }
    }

    // 3. Цепочка метода после скобки: `(self.scaled.to_float * factor).round_to_int`
    if tokens.len() >= 2 && find_lowest_precedence_op(tokens, hir).is_none() {
        if let Some(Token::Path(tail_method)) = tokens.last() {
            if tail_method.starts_with('.')
                && matches!(
                    tokens.get(tokens.len() - 2),
                    Some(Token::Group('(' | ')', _))
                )
            {
                let prefix_expr = lower_tokens_to_expr(&tokens[..tokens.len() - 1], hir);
                let method_name = tail_method.trim_start_matches('.').to_string();
                return LirExpr::Pipe {
                    input: Box::new(prefix_expr),
                    op: PipeOp::Dot,
                    step: Box::new(LirExpr::Ident(method_name)),
                };
            }
        }
    }

    // 4. Вызов функции или конструктора `fn(args)` / `Meta.compose([...])` / `Money"199.99"`
    if let [Token::Path(fn_name), Token::Group(')', args)] = tokens {
        if fn_name == "Compose" || fn_name == "Meta.compose" {
            if let Some([Token::Group('[', stream_sents)]) = args.first().map(Vec::as_slice) {
                return LirExpr::Call {
                    callee: fn_name.clone(),
                    args: vec![LirExpr::Lambda {
                        params: vec!["_".into()],
                        body: lower_body_to_stmts(stream_sents, hir),
                        raw_args: vec![],
                        raw_body: stream_sents.clone(),
                    }],
                };
            }
        }
        return LirExpr::Call {
            callee: fn_name.clone(),
            args: args.iter().map(|a| lower_tokens_to_expr(a, hir)).collect(),
        };
    }

    // 5. Инстанцирование структуры `FormName { ... }`
    if let [Token::Path(type_name), Token::Group('{', sents)] = tokens {
        return lower_struct_init(type_name, sents, hir);
    }

    // 6. Анонимная лямбда `(args) [Body]`
    if let [Token::Group('(' | ')', args), Token::Group('[', body)] = tokens {
        let params = args
            .iter()
            .filter_map(|s| match s.first() {
                Some(Token::Path(p)) => Some(p.clone()),
                _ => None,
            })
            .collect();
        return LirExpr::Lambda {
            params,
            body: lower_body_to_stmts(body, hir),
            raw_args: args.clone(),
            raw_body: body.clone(),
        };
    }

    // 7. Одиночный токен
    if tokens.len() == 1 {
        return lower_single_token(&tokens[0], hir);
    }

    // 8. Бинарные операторы с учетом приоритетов
    if let Some((idx, op)) = find_lowest_precedence_op(tokens, hir) {
        return LirExpr::Binary {
            left: Box::new(lower_tokens_to_expr(&tokens[..idx], hir)),
            op,
            right: Box::new(lower_tokens_to_expr(&tokens[idx + 1..], hir)),
        };
    }

    lower_single_token(&tokens[0], hir)
}

fn lower_struct_init(type_name: &str, sents: &[Sentence], hir: &HirModule) -> LirExpr {
    let mut fields = Vec::new();
    for s in sents {
        match classify_sentence(s) {
            SemanticKind::Binding { key, value } => {
                // Внутри `{}` поля с явным типом и дефолтом `max_speed Int 180` сохраняют дефолт `180`!
                let val_slice = match value.as_slice() {
                    [Token::Path(ty), def @ ..]
                    if is_pascal_case(ty)
                        && !def.is_empty()
                        && !matches!(
                                def.first(),
                                Some(Token::Op(_) | Token::Group(')', _))
                            ) =>
                        {
                            def
                        }
                    other => other,
                };
                fields.push((key, lower_tokens_to_expr(val_slice, hir)));
            }
            SemanticKind::StreamOrBranch { name, stream } => {
                fields.push((
                    name,
                    LirExpr::Lambda {
                        params: vec!["_".into()],
                        body: lower_body_to_stmts(&stream, hir),
                        raw_args: vec![],
                        raw_body: stream,
                    },
                ));
            }
            SemanticKind::FormDef { name, body, .. } => {
                fields.push((name.clone(), lower_struct_init(&name, &body, hir)));
            }
            SemanticKind::FunctionDef {
                name, args, body, ..
            } => {
                let params = args
                    .iter()
                    .filter_map(|s| match s.first() {
                        Some(Token::Path(p)) => Some(p.clone()),
                        _ => None,
                    })
                    .collect();
                fields.push((
                    name,
                    LirExpr::Lambda {
                        params,
                        body: lower_body_to_stmts(&body, hir),
                        raw_args: args,
                        raw_body: body,
                    },
                ));
            }
            _ => {}
        }
    }
    LirExpr::StructInit {
        type_name: type_name.to_string(),
        fields,
    }
}

pub fn lower_single_token(token: &Token, hir: &HirModule) -> LirExpr {
    match token {
        Token::Lit(lit) => match lit {
            Literal::Int(i) => LirExpr::Int(*i),
            Literal::Float(f) => LirExpr::Float(*f),
            Literal::Bool(b) => LirExpr::Bool(*b),
            Literal::Byte(b) => LirExpr::Byte(*b),
            Literal::String(s) => parse_interpolated_str(s),
            Literal::RawString(s) => LirExpr::Str(s.clone()),
            Literal::Symbol(s) => LirExpr::Symbol(s.clone()),
        },
        Token::Path(p) => LirExpr::Ident(p.clone()),
        Token::Range(a, b) => LirExpr::Range(
            Box::new(lower_single_token(a, hir)),
            Box::new(lower_single_token(b, hir)),
        ),
        Token::Group('(' | ')', sents) => match sents.as_slice() {
            [] => LirExpr::Tuple(vec![]),
            [single] => lower_tokens_to_expr(single, hir),
            many => LirExpr::Tuple(many.iter().map(|s| lower_tokens_to_expr(s, hir)).collect()),
        },
        Token::Group('{', sents) => {
            // ВАЖНО: В `{}` строковый ключ `"host" "localhost"` является `Binding` (не `PatternBranch`!).
            // Одиночный `{ ... }` вне пайплайна является роутером ТОЛЬКО если все его предложения — ветки `[...]`!
            let is_router = !sents.is_empty()
                && sents.iter().all(|s| match classify_sentence(s) {
                SemanticKind::PatternBranch { .. }
                | SemanticKind::LambdaOrTupleBranch { .. } => true,
                SemanticKind::StreamOrBranch { name, .. } => {
                    name == "_"
                        || is_pascal_case(&name)
                        || hir.forms.contains_key(&name)
                        || hir.functions.contains_key(&name)
                }
                _ => false,
            });
            if is_router {
                LirExpr::Router {
                    input: Box::new(LirExpr::Ident("_".into())),
                    branches: lower_router_branches(sents, hir),
                }
            } else {
                lower_struct_init("AnonymousForm", sents, hir)
            }
        }
        Token::Group('[', sents) => LirExpr::Lambda {
            params: vec!["_".into()],
            body: lower_body_to_stmts(sents, hir),
            raw_args: vec![],
            raw_body: sents.clone(),
        },
        _ => LirExpr::None,
    }
}

fn parse_interpolated_str(raw: &str) -> LirExpr {
    if !raw.contains('{') && !raw.contains('\\') {
        return LirExpr::Str(raw.to_string());
    }
    let mut parts = Vec::new();
    let mut current_lit = String::new();
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next_c) = chars.next() {
                current_lit.push(next_c);
            }
        } else if c == '{' {
            if !current_lit.is_empty() {
                parts.push(LirInterpPart::Lit(std::mem::take(&mut current_lit)));
            }
            let expr: String = chars.by_ref().take_while(|&ch| ch != '}').collect();
            parts.push(LirInterpPart::Expr(expr.trim().to_string()));
        } else {
            current_lit.push(c);
        }
    }
    if !current_lit.is_empty() {
        parts.push(LirInterpPart::Lit(current_lit));
    }
    if parts.len() == 1 {
        if let LirInterpPart::Lit(s) = &parts[0] {
            return LirExpr::Str(s.clone());
        }
    }
    LirExpr::InterpStr(parts)
}

pub fn lower_router_branches(branches: &[Sentence], hir: &HirModule) -> Vec<LirBranch> {
    let mut out = Vec::new();
    for b in branches {
        match classify_sentence(b) {
            SemanticKind::StreamOrBranch { name, stream } => {
                let mut unpacked_fields = Vec::new();
                let mut bound_type_var = None;
                let cond = if name == "_" {
                    LirPattern::Wildcard
                } else {
                    if is_pascal_case(&name) {
                        bound_type_var = Some(pascal_to_snake_case(&name));
                    }
                    if let Some(form) = hir.forms.get(&name) {
                        for fname in &form.field_order {
                            if let Some(f) = form.fields.get(fname) {
                                if !f.is_ambiguous {
                                    unpacked_fields.push(fname.clone());
                                }
                            }
                        }
                    }
                    LirPattern::TypeMatch(name)
                };
                out.push(LirBranch {
                    cond,
                    bound_type_var,
                    unpacked_fields,
                    body: lower_body_to_stmts(&stream, hir),
                });
            }
            SemanticKind::LambdaOrTupleBranch { args, body } => {
                let elems = lower_tuple_pat_elems(&args, hir);
                out.push(LirBranch {
                    cond: LirPattern::TupleMatch(elems),
                    bound_type_var: None,
                    unpacked_fields: vec![],
                    body: lower_body_to_stmts(&body, hir),
                });
            }
            SemanticKind::PatternBranch { pattern, body } => {
                let cond = match pattern {
                    Token::Range(a, b) => LirPattern::RangeMatch(
                        Box::new(lower_single_token(&a, hir)),
                        Box::new(lower_single_token(&b, hir)),
                    ),
                    Token::Group('[', pred_sents) => {
                        let pred_expr = pred_sents
                            .first()
                            .map(|s| lower_tokens_to_expr(s, hir))
                            .unwrap_or(LirExpr::Bool(true));
                        LirPattern::Predicate(pred_expr)
                    }
                    Token::Group('{', field_pats) => {
                        let mut matches = Vec::new();
                        for fp in field_pats {
                            if let SemanticKind::Binding { key, value } = classify_sentence(&fp) {
                                match value.as_slice() {
                                    [Token::Group('(' | ')', t_pats)] => {
                                        matches.push((
                                            key,
                                            LirFieldPatValue::Tuple(lower_tuple_pat_elems(
                                                t_pats, hir,
                                            )),
                                        ));
                                    }
                                    [Token::Path(var_bind)] if !is_pascal_case(var_bind) => {
                                        matches
                                            .push((key, LirFieldPatValue::Bind(var_bind.clone())));
                                    }
                                    _ => {
                                        matches.push((
                                            key,
                                            LirFieldPatValue::Expr(lower_tokens_to_expr(
                                                &value, hir,
                                            )),
                                        ));
                                    }
                                }
                            }
                        }
                        LirPattern::FieldMatch(matches)
                    }
                    other => LirPattern::LiteralMatch(lower_single_token(&other, hir)),
                };
                out.push(LirBranch {
                    cond,
                    bound_type_var: None,
                    unpacked_fields: vec![],
                    body: lower_body_to_stmts(&body, hir),
                });
            }
            _ => {}
        }
    }
    out
}

fn lower_tuple_pat_elems(pat_sents: &[Sentence], hir: &HirModule) -> Vec<LirTuplePatElem> {
    pat_sents
        .iter()
        .map(|s| match s.as_slice() {
            [Token::Path(v)] if v == "_" => LirTuplePatElem::Wildcard,
            [Token::Path(v)] if is_pascal_case(v) => LirTuplePatElem::TypeBind {
                type_name: v.clone(),
                var_name: pascal_to_snake_case(v),
            },
            [Token::Path(v)] => LirTuplePatElem::Bind(v.clone()),
            other => LirTuplePatElem::Exact(lower_tokens_to_expr(other, hir)),
        })
        .collect()
}

fn find_lowest_precedence_op(tokens: &[Token], hir: &HirModule) -> Option<(usize, String)> {
    let mut lowest_idx = None;
    let mut lowest_prec = u8::MAX;

    for (i, tok) in tokens.iter().enumerate().skip(1) {
        if let Token::Op(op) = tok {
            let prec = if op == "/" {
                let is_form = matches!(
                    tokens.first(),
                    Some(Token::Path(p)) if hir.forms.contains_key(p) || hir.globals.contains_key(p) || is_pascal_case(p)
                ) || tokens.iter().take(i).any(|t| matches!(t, Token::Op(o) if o == "&"));
                if is_form { 2 } else { 7 }
            } else {
                match op.as_str() {
                    "|" => 1,
                    "&" => 2,
                    "||" => 3,
                    "&&" => 4,
                    "==" | "!=" | ">" | "<" | ">=" | "<=" => 5,
                    "+" | "-" | "<<" | ">>" => 6,
                    "*" | "%" => 7,
                    "**" => 8,
                    _ => 0,
                }
            };
            if prec > 0 && prec <= lowest_prec {
                lowest_prec = prec;
                lowest_idx = Some((i, op.clone()));
            }
        }
    }
    lowest_idx
}
