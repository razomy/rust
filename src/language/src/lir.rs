// src/lir.rs
// =============================================================================
// RAZOMY V5.7 — UNIFIED CANONICAL LIR FOR RUNTIME & MULTI-TARGET CODEGEN
// Архитектура: AST -> HIR -> Canonical LIR -> (LIR Runtime VM | TypeScript | C#)
// Вся семантика параметров, дефолтов, Tuple Auto-Lifting и ограничений Закона 3
// предкомпилируется здесь в простые структуры без усложнения кодогенераторов!
// =============================================================================

use crate::hir::{
    is_known_type, is_pascal_case, pascal_to_snake_case, HirField, HirForm, HirFunction, HirModule,
    HirParam,
};
use crate::parser::{
    classify_sentence, extract_import, has_pipe_op, parse_code, split_pipeline, Literal, PipeOp,
    SemanticKind, Sentence, Token,
};

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
}

#[derive(Debug, Clone)]
pub struct LirStruct {
    pub name: String,
    pub is_prelude: bool,
    pub sentence_index: usize,
    pub type_params: Vec<String>,
    /// Предкомпилированные runtime-параметры конструктора `(name, ty, default_expr)`
    pub ctor_params: Vec<(String, String, Option<LirExpr>)>,
    pub value_params: Vec<Sentence>,
    pub ancestors: Vec<String>,
    pub fields: Vec<LirField>,
    pub reactive_getters: Vec<LirGetter>,
    pub methods: Vec<LirFunc>,
    pub submodules: Vec<LirStruct>,
    pub union_variants: Vec<String>,
    pub union_variant_exprs: Vec<LirExpr>,
    pub refinement_predicates: Vec<LirExpr>,
    pub is_bypass: bool,
}

#[derive(Debug, Clone)]
pub struct LirField {
    pub name: String,
    pub ty: String,
    pub origin: String,
    pub default_val: Option<LirExpr>,
    pub is_ambiguous: bool,
    pub conflicting_origins: Vec<String>,
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
    pub is_contract: bool,
    pub is_stream: bool,
    /// Отфильтрованные runtime-параметры `(name, ty, default_val)` без compile-time дженериков!
    pub params: Vec<(String, String, Option<LirExpr>)>,
    /// Полное ограничение первого слота значений (`Order / Cancelled`, `Order & Reserved`) для Закона 3
    pub first_param_constraint: String,
    /// Закон 1c/1d: Авто-привязанные имена (`@self_bind:age`, `@self_bind:user`) и распакованные поля (`name`)
    pub auto_unpacked_fields: Vec<String>,
    pub return_ty: String,
    pub body: Vec<LirStmt>,
    pub raw_args: Vec<Sentence>,
    pub raw_body: Vec<Sentence>,
}

#[derive(Debug, Clone)]
pub struct LirBinding {
    pub name: String,
    pub sentence_index: usize,
    pub ty: String,
    pub value: LirExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LirStmt {
    Let { name: String, value: LirExpr },
    AssignPath { path: String, value: LirExpr },
    DestructTuple { names: Vec<String>, value: LirExpr },
    DestructForm { names: Vec<String>, value: LirExpr },
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
    pub condition: LirExpr,
    pub bindings: Vec<(String, LirExpr)>,
    pub unpacked_fields: Vec<String>,
    pub body: Vec<LirStmt>,
}

// =============================================================================
// 2. ЕДИНЫЙ ПОНИЖАТЕЛЬ HIR -> LIR (DESUGARING & PARAMETER COMPILATION)
// =============================================================================

pub fn lower_hir_to_lir(hir: &HirModule) -> LirModule {
    let mut structs = Vec::new();
    let mut functions = Vec::new();
    let mut globals = Vec::new();
    let mut top_exprs = Vec::new();

    for form_name in &hir.form_order {
        if !is_pascal_case(form_name) && hir.globals.contains_key(form_name) {
            continue;
        }
        if let Some(hf) = hir.forms.get(form_name) {
            if hf.is_prelude
                && matches!(
                    hf.name.as_str(),
                    "Decimal"
                        | "Regex"
                        | "Json"
                        | "Http"
                        | "Port"
                        | "Ui"
                        | "Parse"
                        | "FieldMeta"
                        | "Meta"
                        | "Atom"
                        | "Signal"
                        | "Channel"
                        | "Map"
                )
            {
                continue;
            }
            structs.push(lower_form_to_lir(hf, hir));
        }
    }

    for g_name in &hir.global_order {
        if let Some(g) = hir.globals.get(g_name) {
            if g_name == "_"
                || (is_pascal_case(g_name)
                    && hir.forms.contains_key(g_name)
                    && (matches!(g.value_expr.as_slice(), [Token::Path(_)])
                        || matches!(
                            g.value_expr.as_slice(),
                            [Token::Path(p), Token::Group(')', _)] if p == "Tuple"
                        )))
            {
                continue;
            }
            if hir.imports.iter().any(|imp| imp.targets.contains(g_name)) {
                continue;
            }
            globals.push(LirBinding {
                name: g.key.clone(),
                sentence_index: g.origin.sentence_index,
                ty: g.inferred_type.clone(),
                value: lower_tokens_to_expr(&g.value_expr, hir),
            });
        }
    }

    for fn_name in &hir.function_order {
        if let Some(hf) = hir.functions.get(fn_name) {
            if hf.is_prelude {
                continue;
            }
            functions.push(lower_fn_to_lir(hf, hir));
        }
    }

    for sent in &hir.top_level_sentences {
        if extract_import(sent).is_some() {
            continue;
        }
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
    }
}

fn is_open_field_for_positional_ctor(f: &HirField, hf: &HirForm, hir: &HirModule) -> bool {
    if f.is_reactive {
        return false;
    }
    if f.default_expr.is_empty() {
        return true;
    }
    match f.default_expr.as_slice() {
        [Token::Path(p)] => {
            crate::hir::is_type_hole(p)
                || matches!(
                    p.as_str(),
                    "String" | "Symbol" | "Int" | "Float" | "Bool" | "Byte" | "Money" | "Any" | "Self"
                )
                || hf.type_params.contains(p)
                || (is_pascal_case(p) && hir.forms.contains_key(p))
        }
        [Token::Path(p), Token::Group(')', args)] if is_pascal_case(p) => {
            matches!(
                p.as_str(),
                "Tuple" | "Map" | "List" | "Set" | "Option" | "Tree"
            ) || args
                .iter()
                .all(|a| matches!(a.as_slice(), [Token::Path(tp)] if crate::hir::is_type_or_hole(tp)))
        }
        _ => f
            .default_expr
            .iter()
            .any(|t| matches!(t, Token::Op(op) if op == "|")),
    }
}

fn lower_form_to_lir(hf: &HirForm, hir: &HirModule) -> LirStruct {
    let mut fields = Vec::new();
    let mut reactive_getters = Vec::new();

    for fname in &hf.field_order {
        if hf.submodules.contains_key(fname) {
            continue;
        }
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
                fields.push(lower_field_to_lir(f, hf, hir));
            }
        }
    }

    let methods = hf
        .method_order
        .iter()
        .filter_map(|m| hf.methods.get(m).map(|mf| lower_fn_to_lir(mf, hir)))
        .collect();

    let submodules = hf
        .submodules
        .values()
        .map(|sub| lower_form_to_lir(sub, hir))
        .collect();

    // Form-Tuple Constructor Duality (V6.4):
    // Если у формы нет явных `value_params` в заголовке `Name(...)`, её открытые поля (без дефолтного значения)
    // автоматически образуют позиционные слоты конструктора `Name(arg0, arg1, ...)`!
    let ctor_params = if !hf.value_params.is_empty() {
        hf.value_params
            .iter()
            .map(|vp| {
                let def = if vp.default_expr.is_empty() {
                    None
                } else {
                    Some(lower_tokens_to_expr(&vp.default_expr, hir))
                };
                (vp.name.clone(), normalize_param_type(vp), def)
            })
            .collect()
    } else {
        hf.field_order
            .iter()
            .filter_map(|fname| hf.fields.get(fname))
            .filter(|f| is_open_field_for_positional_ctor(f, hf, hir))
            .map(|f| (f.name.clone(), f.inferred_type.clone(), None))
            .collect()
    };

    let value_params = hf
        .value_params
        .iter()
        .map(|vp| {
            if vp.is_anonymous_type_slot {
                vec![Token::Path(vp.type_constraint.clone())]
            } else {
                let mut s = vec![
                    Token::Path(vp.name.clone()),
                    Token::Path(vp.type_constraint.clone()),
                ];
                s.extend(vp.default_expr.clone());
                s
            }
        })
        .collect();

    let union_variant_exprs = hf
        .union_variants
        .iter()
        .filter_map(|v| parse_code(v).ok()?.into_iter().next())
        .map(|s| lower_tokens_to_expr(&s, hir))
        .collect();

    let refinement_predicates = hf
        .refinement_predicates
        .iter()
        .map(|tok| lower_single_token(tok, hir))
        .collect();

    let is_builtin = hf.is_prelude
        || matches!(
            hf.name.as_str(),
            "Shell" | "Compile" | "Bypass" | "Public" | "Private"
        );

    LirStruct {
        name: hf.name.clone(),
        is_prelude: is_builtin,
        sentence_index: hf.origin.sentence_index,
        type_params: hf.type_params.clone(),
        ctor_params,
        value_params,
        ancestors: hf.ancestors.clone(),
        fields,
        reactive_getters,
        methods,
        submodules,
        union_variants: hf.union_variants.clone(),
        union_variant_exprs,
        refinement_predicates,
        is_bypass: hf.is_bypass,
    }
}

fn lower_field_to_lir(f: &HirField, hf: &HirForm, hir: &HirModule) -> LirField {
    let is_type_only = is_open_field_for_positional_ctor(f, hf, hir);

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
        is_ambiguous: f.is_ambiguous,
        conflicting_origins: f.conflicting_origins.clone(),
    }
}

pub fn lower_fn_to_lir(hf: &HirFunction, hir: &HirModule) -> LirFunc {
    let mut params = Vec::new();
    let mut auto_unpacked_fields = Vec::new();
    let mut prologue_stmts = Vec::new();

    // Отфильтровываем чистые compile-time параметры типа (`A`, `B` в `list_map(A, B, l, action)`) прямо в LIR!
    let runtime_params: Vec<&HirParam> = hf
        .params
        .iter()
        .enumerate()
        .filter(|(idx, p)| {
            if p.is_type_param {
                return false;
            }
            if p.is_anonymous_type_slot
                && !is_known_type(&p.type_constraint, hir)
                && *idx + 1 < hf.params.len()
            {
                return false;
            }
            true
        })
        .map(|(_, p)| p)
        .collect();

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
            params.push((param_var.clone(), normalize_param_type(p), def));

            // Дешугаринг Закона 1c и 1d прямо в LIR!
            if p.is_anonymous_type_slot {
                let src_var = if idx == 0 {
                    "_".to_string()
                } else {
                    p.name.clone()
                };
                if p.name != "_" {
                    if idx == 0 && !auto_unpacked_fields.contains(&p.name) {
                        auto_unpacked_fields.push(format!("@self_bind:{}", p.name));
                    }
                    prologue_stmts.push(LirStmt::Let {
                        name: p.name.clone(),
                        value: LirExpr::Ident(src_var.clone()),
                    });
                }
                if is_pascal_case(&p.type_constraint) && !p.type_constraint.contains(' ') {
                    prologue_stmts.push(LirStmt::Let {
                        name: p.type_constraint.clone(),
                        value: LirExpr::Ident(src_var),
                    });
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
                prologue_stmts.push(LirStmt::Let {
                    name: df.clone(),
                    value: LirExpr::Ident(format!("_.{df}")),
                });
            }
        }
    }

    let kind = classify_sentence(&hf.origin.raw_ast);
    let is_stream = matches!(kind, SemanticKind::StreamOrBranch { .. });
    let raw_args = match kind {
        SemanticKind::FunctionDef { args, .. } | SemanticKind::GenericDef { params: args, .. } => {
            args
        }
        _ => Vec::new(),
    };

    let mut body = prologue_stmts;
    body.extend(lower_body_to_stmts(&hf.body, hir));

    LirFunc {
        name: sanitize_fn_name(&hf.name),
        raw_name: hf.name.clone(),
        is_contract: hf.is_contract,
        is_stream,
        params,
        first_param_constraint,
        auto_unpacked_fields,
        return_ty: hf.return_type.clone().unwrap_or_else(|| "Any".into()),
        body,
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
            SemanticKind::AlgebraicDef {
                name,
                expr: val_tokens,
            } => {
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
                // Typed Local Binding (Law 3): `u1 User (uid1 . Db.find_user)` -> strip leading Type annotation `User`
                let clean_value = match value.as_slice() {
                    [Token::Path(ty), rest @ ..]
                        if crate::hir::is_type_or_hole(ty)
                            && !rest.is_empty()
                            && !matches!(
                                rest.first(),
                                Some(Token::Op(_) | Token::Group(')' | '{', _))
                            ) =>
                    {
                        rest
                    }
                    other => other,
                };
                let expr = lower_tokens_to_expr(clean_value, hir);
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
                            [Token::Op(op), Token::Path(p)] if op == ".." => {
                                Some(format!("..{p}"))
                            }
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
                // Form Destructuring with optional rest: `{ name, ..other_fields } user_form`
                if let [Token::Group('{', field_sents), rest @ ..] = s.as_slice() {
                    if !rest.is_empty() && !has_pipe_op(rest) {
                        let names: Vec<String> = field_sents
                            .iter()
                            .filter_map(|fs| match fs.as_slice() {
                                [Token::Path(p)] => Some(p.clone()),
                                [Token::Op(op), Token::Path(p)] if op == ".." => {
                                    Some(format!("..{p}"))
                                }
                                _ => None,
                            })
                            .collect();
                        stmts.push(LirStmt::DestructForm {
                            names,
                            value: lower_tokens_to_expr(rest, hir),
                        });
                        continue;
                    }
                }
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
            } else if step.op == PipeOp::Parallel {
                if let [Token::Group('{', sents)] = step.expr.as_slice() {
                    current = LirExpr::Pipe {
                        input: Box::new(current),
                        op: PipeOp::Parallel,
                        step: Box::new(lower_struct_init("AnonymousForm", sents, hir)),
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

    if let [Token::Op(op), rest @ ..] = tokens {
        if matches!(op.as_str(), "!" | "-" | "..") && !rest.is_empty() {
            return LirExpr::Unary {
                op: op.clone(),
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
        if fn_name == "Meta.compose" {
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
        let is_named_call = !args.is_empty()
            && args
                .iter()
                .all(|a| matches!(classify_sentence(a), SemanticKind::Binding { .. }));
        if is_named_call {
            return LirExpr::Call {
                callee: fn_name.clone(),
                args: vec![lower_struct_init("__NamedArgs__", args, hir)],
            };
        }
        return LirExpr::Call {
            callee: fn_name.clone(),
            args: args.iter().map(|a| lower_tokens_to_expr(a, hir)).collect(),
        };
    }

    // 5. Инстанцирование структуры `FormName { ... }` или дешугаренной `FormName & { ... }`
    match tokens {
        [Token::Path(type_name), Token::Group('{', sents)] => {
            return lower_struct_init(type_name, sents, hir);
        }
        [Token::Path(type_name), Token::Op(op), Token::Group('{', sents)]
        | [Token::Path(type_name), Token::Group(')', _), Token::Op(op), Token::Group('{', sents)]
            if op == "&" && is_pascal_case(type_name) =>
        {
            return lower_struct_init(type_name, sents, hir);
        }
        _ => {}
    }

    // Анонимная лямбда `(args) [Body]`, включая деструктуризацию кортежей `(idx Int, (item Item, disc Int))`!
    if let [Token::Group('(' | ')', args), Token::Group('[', body)] = tokens {
        let mut params = Vec::new();
        let mut prologue = Vec::new();
        for (i, s) in args.iter().enumerate() {
            match s.first() {
                Some(Token::Path(p)) => params.push(p.clone()),
                Some(Token::Group('(', tuple_sents)) => {
                    let tmp_name = format!("_tuple_arg_{i}");
                    params.push(tmp_name.clone());
                    let names: Vec<String> = tuple_sents
                        .iter()
                        .filter_map(|ts| match ts.first() {
                            Some(Token::Path(tp)) => Some(tp.clone()),
                            _ => None,
                        })
                        .collect();
                    prologue.push(LirStmt::DestructTuple {
                        names,
                        value: LirExpr::Ident(tmp_name),
                    });
                }
                _ => {}
            }
        }
        prologue.extend(lower_body_to_stmts(body, hir));
        return LirExpr::Lambda {
            params,
            body: prologue,
            raw_args: args.clone(),
            raw_body: body.clone(),
        };
    }

    if tokens.len() == 1 {
        return lower_single_token(&tokens[0], hir);
    }

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
    let form_schema = hir.forms.get(type_name);

    for s in sents {
        match classify_sentence(s) {
            SemanticKind::Binding { key, value } => {
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
                let mut lowered = lower_tokens_to_expr(val_slice, hir);
                let expects_tuple = form_schema
                    .and_then(|fs| fs.fields.get(&key))
                    .map(|f| f.inferred_type.starts_with("Tuple("))
                    .unwrap_or(key == "departments");
                if expects_tuple && !matches!(lowered, LirExpr::Tuple(_)) {
                    lowered = LirExpr::Tuple(vec![lowered]);
                }
                fields.push((key, lowered));
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
                    SemanticKind::FunctionDef { body, tail, .. } => {
                        tail.is_empty() && !body.is_empty()
                    }
                    _ => false,
                });
            if is_router {
                LirExpr::Router {
                    input: Box::new(LirExpr::None),
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
                let mut bindings = Vec::new();
                let condition = if name == "_" {
                    LirExpr::Bool(true)
                } else {
                    if is_pascal_case(&name) {
                        bindings.push((pascal_to_snake_case(&name), LirExpr::Ident("_".into())));
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
                    LirExpr::Binary {
                        left: Box::new(LirExpr::Ident("_".into())),
                        op: "is".into(),
                        right: Box::new(LirExpr::Ident(name)),
                    }
                };
                out.push(LirBranch {
                    condition,
                    bindings,
                    unpacked_fields,
                    body: lower_body_to_stmts(&stream, hir),
                });
            }
            SemanticKind::FunctionDef {
                name, args, body, ..
            } => {
                // Ветка алгебраического конструктора в роутере: `Some(val) [ ... ]`, `Cons(hd, tl) [ ... ]`, `Add(l, r) [ ... ]`
                let mut conds = vec![LirExpr::Binary {
                    left: Box::new(LirExpr::Ident("_".into())),
                    op: "is".into(),
                    right: Box::new(LirExpr::Ident(name.clone())),
                }];
                let mut bindings = Vec::new();
                if is_pascal_case(&name) {
                    bindings.push((pascal_to_snake_case(&name), LirExpr::Ident("_".into())));
                }
                let field_names: Vec<String> = hir
                    .forms
                    .get(&name)
                    .map(|hf| hf.field_order.clone())
                    .unwrap_or_default();
                for (idx, arg_sent) in args.iter().enumerate() {
                    let field_accessor = field_names
                        .get(idx)
                        .map(|f| format!("_.{f}"))
                        .unwrap_or_else(|| format!("_.{idx}"));
                    if let [Token::Path(var_name)] = arg_sent.as_slice() {
                        if var_name != "_" {
                            bindings.push((var_name.clone(), LirExpr::Ident(field_accessor)));
                        }
                    } else {
                        conds.push(LirExpr::Binary {
                            left: Box::new(LirExpr::Ident(field_accessor)),
                            op: "==".into(),
                            right: Box::new(lower_tokens_to_expr(arg_sent, hir)),
                        });
                    }
                }
                out.push(LirBranch {
                    condition: combine_and(conds),
                    bindings,
                    unpacked_fields: vec![],
                    body: lower_body_to_stmts(&body, hir),
                });
            }
            SemanticKind::LambdaOrTupleBranch { args, body } => {
                let mut conds = Vec::new();
                let mut bindings = Vec::new();
                lower_tuple_pattern("_", &args, hir, &mut conds, &mut bindings);
                out.push(LirBranch {
                    condition: combine_and(conds),
                    bindings,
                    unpacked_fields: vec![],
                    body: lower_body_to_stmts(&body, hir),
                });
            }
            SemanticKind::PatternBranch { pattern, body } => {
                let mut bindings = Vec::new();
                let condition = match pattern {
                    Token::Range(a, b) => LirExpr::Binary {
                        left: Box::new(LirExpr::Binary {
                            left: Box::new(LirExpr::Ident("_".into())),
                            op: ">=".into(),
                            right: Box::new(lower_single_token(&a, hir)),
                        }),
                        op: "&&".into(),
                        right: Box::new(LirExpr::Binary {
                            left: Box::new(LirExpr::Ident("_".into())),
                            op: "<=".into(),
                            right: Box::new(lower_single_token(&b, hir)),
                        }),
                    },
                    Token::Group('[', pred_sents) => pred_sents
                        .first()
                        .map(|s| lower_tokens_to_expr(s, hir))
                        .unwrap_or(LirExpr::Bool(true)),
                    Token::Group('{', field_pats) => {
                        let mut conds = Vec::new();
                        for fp in field_pats {
                            if let SemanticKind::Binding { key, value } = classify_sentence(&fp) {
                                let field_path = format!("_.{key}");
                                let field_ident = LirExpr::Ident(field_path.clone());
                                match value.as_slice() {
                                    [Token::Group('(' | ')', t_pats)] => {
                                        lower_tuple_pattern(
                                            &field_path,
                                            t_pats,
                                            hir,
                                            &mut conds,
                                            &mut bindings,
                                        );
                                    }
                                    [Token::Path(var_bind)] if !is_pascal_case(var_bind) => {
                                        conds.push(LirExpr::Binary {
                                            left: Box::new(field_ident.clone()),
                                            op: "!=".into(),
                                            right: Box::new(LirExpr::None),
                                        });
                                        bindings.push((var_bind.clone(), field_ident));
                                    }
                                    _ => {
                                        conds.push(LirExpr::Binary {
                                            left: Box::new(field_ident),
                                            op: "==".into(),
                                            right: Box::new(lower_tokens_to_expr(&value, hir)),
                                        });
                                    }
                                }
                            }
                        }
                        combine_and(conds)
                    }
                    other => LirExpr::Binary {
                        left: Box::new(LirExpr::Ident("_".into())),
                        op: "==".into(),
                        right: Box::new(lower_single_token(&other, hir)),
                    },
                };
                out.push(LirBranch {
                    condition,
                    bindings,
                    unpacked_fields: vec![],
                    body: lower_body_to_stmts(&body, hir),
                });
            }
            _ => {}
        }
    }
    out
}

fn lower_tuple_pattern(
    target_path: &str,
    pat_sents: &[Sentence],
    hir: &HirModule,
    conds: &mut Vec<LirExpr>,
    bindings: &mut Vec<(String, LirExpr)>,
) {
    conds.push(LirExpr::Binary {
        left: Box::new(LirExpr::Ident(target_path.into())),
        op: "tuple_len_eq".into(),
        right: Box::new(LirExpr::Int(pat_sents.len() as i64)),
    });
    for (i, s) in pat_sents.iter().enumerate() {
        let elem_ident = LirExpr::Ident(format!("{target_path}.{i}"));
        match s.as_slice() {
            [Token::Path(v)] if v == "_" => {}
            [Token::Path(v)] if is_pascal_case(v) => {
                conds.push(LirExpr::Binary {
                    left: Box::new(elem_ident.clone()),
                    op: "is".into(),
                    right: Box::new(LirExpr::Ident(v.clone())),
                });
                bindings.push((pascal_to_snake_case(v), elem_ident));
            }
            [Token::Path(v)] => {
                bindings.push((v.clone(), elem_ident));
            }
            other => {
                conds.push(LirExpr::Binary {
                    left: Box::new(elem_ident),
                    op: "==".into(),
                    right: Box::new(lower_tokens_to_expr(other, hir)),
                });
            }
        }
    }
}

fn combine_and(conds: Vec<LirExpr>) -> LirExpr {
    let mut iter = conds.into_iter();
    let Some(mut acc) = iter.next() else {
        return LirExpr::Bool(true);
    };
    for next in iter {
        acc = LirExpr::Binary {
            left: Box::new(acc),
            op: "&&".into(),
            right: Box::new(next),
        };
    }
    acc
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
                ) || tokens
                    .iter()
                    .take(i)
                    .any(|t| matches!(t, Token::Op(o) if o == "&"));
                if is_form {
                    2
                } else {
                    7
                }
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
