// src/runtime.rs
// =============================================================================
// RAZOMY V5.7 — COMPACT LIR RUNTIME ENGINE
// Весь разбор параметров, дженериков, деструктуризации и Tuple Auto-Lifting
// уже выполнен в `lir.rs`. Рантайм только исполняет LirStmt и LirExpr!
// =============================================================================

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use crate::hir::{is_pascal_case, lower_to_hir, pascal_to_snake_case, HirModule};
use crate::lir::{
    lower_hir_to_lir, lower_tokens_to_expr, LirBranch, LirExpr, LirInterpPart, LirModule, LirStmt,
    LirStruct,
};
use crate::parser::{parse_code, PipeOp, Sentence, Token};
use crate::runtime_ext::{create_global_scope, eval_compose_call, make_empty_form, make_some};

#[derive(Clone)]
pub enum Value {
    None,
    Int(i64),
    Float(f64),
    Bool(bool),
    Byte(u8),
    String(String),
    Symbol(String),
    Range(i64, i64),
    Tuple(Vec<Value>),
    Form(Arc<RwLock<FormInstance>>),
    Stream {
        stmts: Vec<LirStmt>,
        raw: Vec<Sentence>,
    },
    Router(Vec<LirBranch>),
    Function(Arc<FunctionInstance>),
    Atom(Arc<Mutex<Value>>),
    Channel(Arc<Mutex<VecDeque<Value>>>),
    Done(Box<Value>),
}

impl Value {
    pub fn compact_dump(&self) -> String {
        match self {
            Value::None => "None".into(),
            Value::Int(i) => format!("Int({i})"),
            Value::Float(f) => format!("Float({f})"),
            Value::Bool(b) => format!("Bool({b})"),
            Value::Byte(b) => format!("Byte(0x{b:02X})"),
            Value::String(s) => format!("Str({s:?})"),
            Value::Symbol(s) => format!("Sym('{s}')"),
            Value::Range(a, b) => format!("Range({a}..{b})"),
            Value::Tuple(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.compact_dump()).collect();
                format!("Tuple[{}]({})", items.len(), inner.join(", "))
            }
            Value::Form(f_arc) => {
                let f = f_arc.read().unwrap();
                let flds: Vec<String> = f
                    .field_order
                    .iter()
                    .filter_map(|k| f.fields.get(k).map(|v| format!("{k}={}", v.compact_dump())))
                    .collect();
                format!("Form({}){{{}}}", f.type_name, flds.join(", "))
            }
            Value::Stream { raw, .. } => format!("Stream(steps={})", raw.len()),
            Value::Router(b) => format!("Router(branches={})", b.len()),
            Value::Function(f) => format!("Fn({})", f.name),
            Value::Atom(a) => format!("Atom({})", a.lock().unwrap().compact_dump()),
            Value::Channel(c) => format!("Chan(queued={})", c.lock().unwrap().len()),
            Value::Done(v) => format!("Break({})", v.compact_dump()),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::None, Value::None) => true,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < 1e-9,
            (Value::Int(a), Value::Float(b)) => (*a as f64 - b).abs() < 1e-9,
            (Value::Float(a), Value::Int(b)) => (a - *b as f64).abs() < 1e-9,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Byte(a), Value::Byte(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Range(a1, a2), Value::Range(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Form(f1), Value::Form(f2)) => {
                let (r1, r2) = (f1.read().unwrap(), f2.read().unwrap());
                r1.type_name == r2.type_name && r1.fields == r2.fields
            }
            (Value::Done(a), Value::Done(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct FormInstance {
    pub type_name: String,
    pub ctor_params: Vec<(String, String, Option<LirExpr>)>,
    pub eager_fields: Vec<(String, LirExpr)>,
    pub fields: HashMap<String, Value>,
    pub field_order: Vec<String>,
    pub field_origins: HashMap<String, String>,
    pub methods: HashMap<String, Arc<FunctionInstance>>,
    pub ancestors: HashMap<String, FormInstance>,
    pub ambiguous_fields: HashMap<String, Vec<String>>,
    pub predicates: Vec<Value>,
    pub union_variants: Vec<Value>,
}

impl FormInstance {
    pub fn new(type_name: &str) -> Self {
        Self {
            type_name: type_name.to_string(),
            ctor_params: Vec::new(),
            eager_fields: Vec::new(),
            fields: HashMap::new(),
            field_order: Vec::new(),
            field_origins: HashMap::new(),
            methods: HashMap::new(),
            ancestors: HashMap::new(),
            ambiguous_fields: HashMap::new(),
            predicates: Vec::new(),
            union_variants: Vec::new(),
        }
    }

    pub fn set_field(&mut self, key: &str, val: Value) {
        let origin = self.type_name.clone();
        self.set_field_with_origin(key, val, &origin);
    }

    pub fn set_field_with_origin(&mut self, key: &str, val: Value, origin: &str) {
        if !self.fields.contains_key(key) {
            self.field_order.push(key.to_string());
        }
        self.fields.insert(key.to_string(), val);
        self.field_origins.insert(key.to_string(), origin.to_string());
        self.ambiguous_fields.remove(key);
    }

    pub fn remove_field(&mut self, key: &str) {
        self.fields.remove(key);
        self.field_order.retain(|k| k != key);
        self.field_origins.remove(key);
        self.ambiguous_fields.remove(key);
    }

    pub fn get_field(&self, key: &str) -> Option<Value> {
        self.fields.get(key).cloned()
    }

    pub fn has_lineage(&self, target_type: &str) -> bool {
        self.type_name == target_type
            || self.ancestors.contains_key(target_type)
            || self.ancestors.values().any(|a| a.has_lineage(target_type))
    }
}

pub type NativeFn = Arc<dyn Fn(Vec<Value>, &mut Scope) -> Result<Value, String> + Send + Sync>;

#[derive(Clone)]
pub enum FunctionBody {
    LirDefined {
        params: Vec<(String, String, Option<LirExpr>)>,
        first_param_constraint: String,
        stmts: Vec<LirStmt>,
        raw_body: Vec<Sentence>,
        captured_env: Arc<RwLock<Scope>>,
        overrides: HashMap<String, Value>,
        partial_args: Vec<Value>,
    },
    Composed(Value, Value),
    Native(NativeFn),
}

#[derive(Clone)]
pub struct FunctionInstance {
    pub name: String,
    pub body: FunctionBody,
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::None => write!(f, "None"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(fl) => write!(f, "{fl}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Byte(b) => write!(f, "0x{b:02X}"),
            Value::String(s) => write!(f, "\"{s}\""),
            Value::Symbol(s) => write!(f, "'{s}'"),
            Value::Range(a, b) => write!(f, "{a}..{b}"),
            Value::Tuple(items) => {
                let s: Vec<String> = items.iter().map(|it| format!("{it:?}")).collect();
                write!(f, "({})", s.join(", "))
            }
            Value::Form(form) => {
                let form = form.read().unwrap();
                let pairs: Vec<String> = form
                    .field_order
                    .iter()
                    .filter_map(|k| form.fields.get(k).map(|v| format!("{k}: {v:?}")))
                    .collect();
                write!(f, "{} {{ {} }}", form.type_name, pairs.join(", "))
            }
            Value::Stream { .. } => write!(f, "[Stream]"),
            Value::Router(_) => write!(f, "{{Router}}"),
            Value::Function(func) => write!(f, "Fn({})", func.name),
            Value::Atom(a) => write!(f, "Atom({:?})", a.lock().unwrap()),
            Value::Channel(_) => write!(f, "Channel"),
            Value::Done(v) => write!(f, "Break({v:?})"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "{s}"),
            _ => write!(f, "{self:?}"),
        }
    }
}

#[derive(Clone, Default)]
pub struct Scope {
    pub parent: Option<Arc<RwLock<Scope>>>,
    pub bindings: HashMap<String, Value>,
    pub stream_depth: usize,
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child(parent: Arc<RwLock<Scope>>) -> Self {
        let parent_depth = parent.read().unwrap().stream_depth;
        Self {
            parent: Some(parent),
            bindings: HashMap::new(),
            stream_depth: parent_depth,
        }
    }

    pub fn child_stream(parent: Arc<RwLock<Scope>>, input: Value) -> Self {
        let next_depth = parent.read().unwrap().stream_depth + 1;
        let mut s = Self {
            parent: Some(parent),
            bindings: HashMap::new(),
            stream_depth: next_depth,
        };
        s.set("_", input.clone());
        s.set(&format!("_{next_depth}"), input);
        s
    }

    pub fn has_local(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    pub fn has_in_chain(&self, name: &str) -> bool {
        if self.bindings.contains_key(name) {
            return true;
        }
        if let Some(parent_arc) = &self.parent {
            let parent = parent_arc.read().unwrap();
            if parent.parent.is_some() {
                return parent.has_in_chain(name);
            }
        }
        false
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.bindings
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref()?.read().unwrap().get(name))
    }

    pub fn set(&mut self, name: &str, val: Value) {
        self.bindings.insert(name.to_string(), val);
    }

    pub fn unpack_form(&mut self, form_arc: &Arc<RwLock<FormInstance>>, eval_reactive: bool) {
        let form = form_arc.read().unwrap().clone();
        if is_pascal_case(&form.type_name) && form.type_name != "AnonymousForm" {
            let snake_var = pascal_to_snake_case(&form.type_name);
            if !self.has_in_chain(&snake_var) {
                self.set(&snake_var, Value::Form(form_arc.clone()));
            }
        }
        for fname in &form.field_order {
            if form.ambiguous_fields.contains_key(fname) || self.has_in_chain(fname) {
                continue;
            }
            if let Some(val) = form.fields.get(fname) {
                if eval_reactive {
                    if let Value::Stream { stmts, raw } = val {
                        if !stream_uses_free_underscore(raw) {
                            let mut r_scope = Scope::child(Arc::new(RwLock::new(self.clone())));
                            r_scope.set("self", Value::Form(form_arc.clone()));
                            r_scope.set("Self", Value::Form(form_arc.clone()));
                            if let Ok(comp) = eval_lir_stmts(stmts, None, &mut r_scope) {
                                self.set(fname, unwrap_done(comp));
                                continue;
                            }
                        }
                    }
                }
                self.set(fname, val.clone());
            }
        }
        for (anc_name, anc_inst) in &form.ancestors {
            let anc_arc = Arc::new(RwLock::new(anc_inst.clone()));
            self.set(anc_name, Value::Form(anc_arc.clone()));
            if is_pascal_case(anc_name) {
                let anc_snake = pascal_to_snake_case(anc_name);
                if !self.has_local(&anc_snake) {
                    self.set(&anc_snake, Value::Form(anc_arc));
                }
            }
        }
    }
}

#[inline]
fn unwrap_done(v: Value) -> Value {
    match v {
        Value::Done(inner) => *inner,
        other => other,
    }
}

pub fn stream_uses_free_underscore(sents: &[Sentence]) -> bool {
    sents.iter().any(|s| {
        s.iter()
            .any(|tok| matches!(tok, Token::Path(p) if p == "_" || p.starts_with("_.")))
    })
}

// =============================================================================
// ИСПОЛНЕНИЕ LIR-ИНСТРУКЦИЙ И ВЫРАЖЕНИЙ
// =============================================================================

pub fn eval_lir_stmts(
    stmts: &[LirStmt],
    input: Option<Value>,
    scope: &mut Scope,
) -> Result<Value, String> {
    if let Some(inp) = input {
        scope.set("_", inp.clone());
        let d = scope.stream_depth;
        scope.set(&format!("_{d}"), inp);
    }
    let mut last = Value::None;
    for stmt in stmts {
        last = match stmt {
            LirStmt::Let { name, value } => {
                let val = eval_lir_expr(value, scope)?;
                scope.set(name, val.clone());
                val
            }
            LirStmt::AssignPath { path, value } => {
                let val = eval_lir_expr(value, scope)?;
                mutate_path_in_place(path, val.clone(), scope)?;
                val
            }
            LirStmt::DestructTuple { names, value } => {
                let val = eval_lir_expr(value, scope)?;
                if let Value::Tuple(items) = &val {
                    for (n, it) in names.iter().zip(items.iter()) {
                        if n != "_" {
                            scope.set(n, it.clone());
                        }
                    }
                }
                val
            }
            LirStmt::Expr(e) | LirStmt::Return(e) => eval_lir_expr(e, scope)?,
        };

        if let Value::Done(_) = last {
            return Ok(last);
        }
        if let Value::Form(f_arc) = &last {
            let f = f_arc.read().unwrap();
            if f.type_name == "Break" {
                if let Some(v) = f.get_field("value") {
                    return Ok(Value::Done(Box::new(v)));
                }
            }
        }
    }
    Ok(last)
}

fn mutate_path_in_place(path: &str, new_val: Value, scope: &mut Scope) -> Result<(), String> {
    let segs: Vec<&str> = path.split('.').collect();
    if segs.len() < 2 {
        scope.set(path, new_val);
        return Ok(());
    }
    let root_name = segs[0];
    let Some(root_val) = scope.get(root_name) else {
        return Err(format!("Переменная '{root_name}' не найдена для мутации '{path}'"));
    };
    let updated = mutate_value_recursive(root_val, &segs[1..], new_val)?;
    scope.set(root_name, updated);
    Ok(())
}

fn mutate_value_recursive(
    current: Value,
    segs: &[&str],
    new_val: Value,
) -> Result<Value, String> {
    if segs.is_empty() {
        return Ok(new_val);
    }
    let seg = segs[0];
    match current {
        Value::Form(f_arc) => {
            let target_field = {
                let f = f_arc.read().unwrap();
                if let Ok(idx) = seg.parse::<i32>() {
                    let i = if idx < 0 {
                        (f.field_order.len() as i32 + idx) as usize
                    } else {
                        idx as usize
                    };
                    f.field_order.get(i).cloned().ok_or("Индекс вне диапазона")?
                } else {
                    seg.to_string()
                }
            };
            if segs.len() == 1 {
                f_arc.write().unwrap().set_field(&target_field, new_val);
            } else {
                let inner = f_arc.read().unwrap().get_field(&target_field).unwrap_or(Value::None);
                let updated_inner = mutate_value_recursive(inner, &segs[1..], new_val)?;
                f_arc.write().unwrap().set_field(&target_field, updated_inner);
            }
            Ok(Value::Form(f_arc))
        }
        Value::Tuple(mut items) => {
            let idx: i32 = seg.parse().unwrap_or(0);
            let i = if idx < 0 {
                (items.len() as i32 + idx) as usize
            } else {
                idx as usize
            };
            if i < items.len() {
                items[i] = if segs.len() == 1 {
                    new_val
                } else {
                    mutate_value_recursive(items[i].clone(), &segs[1..], new_val)?
                };
            }
            Ok(Value::Tuple(items))
        }
        other => Ok(other),
    }
}

pub fn eval_lir_expr(expr: &LirExpr, scope: &mut Scope) -> Result<Value, String> {
    match expr {
        LirExpr::None => Ok(Value::None),
        LirExpr::Int(i) => Ok(Value::Int(*i)),
        LirExpr::Float(f) => Ok(Value::Float(*f)),
        LirExpr::Bool(b) => Ok(Value::Bool(*b)),
        LirExpr::Byte(b) => Ok(Value::Byte(*b)),
        LirExpr::Str(s) => Ok(Value::String(s.clone())),
        LirExpr::Symbol(s) => Ok(Value::Symbol(s.clone())),
        LirExpr::InterpStr(parts) => {
            let mut out = String::new();
            for part in parts {
                match part {
                    LirInterpPart::Lit(s) => out.push_str(s),
                    LirInterpPart::Expr(key) => {
                        if let Ok(val) = eval_path(key, scope) {
                            out.push_str(&val.to_string());
                        } else if let Ok(sents) = parse_code(key) {
                            if let Some(first) = sents.first() {
                                let e = lower_tokens_to_expr(first, &HirModule::default());
                                if let Ok(val) = eval_lir_expr(&e, scope) {
                                    out.push_str(&val.to_string());
                                    continue;
                                }
                            }
                            out.push_str(&format!("{{{key}}}"));
                        }
                    }
                }
            }
            Ok(Value::String(out))
        }
        LirExpr::Ident(path) => eval_path(path, scope),
        LirExpr::Tuple(items) => Ok(Value::Tuple(
            items.iter().map(|e| eval_lir_expr(e, scope)).collect::<Result<Vec<_>, _>>()?,
        )),
        LirExpr::Range(a, b) => match (eval_lir_expr(a, scope)?, eval_lir_expr(b, scope)?) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Range(x, y)),
            (Value::String(x), Value::String(y)) => Ok(Value::Range(
                x.chars().next().unwrap_or('a') as i64,
                y.chars().next().unwrap_or('z') as i64,
            )),
            _ => Ok(Value::Range(0, 0)),
        },
        LirExpr::StructInit { type_name, fields } => {
            let mut form = match scope.get(type_name) {
                Some(Value::Form(proto_arc)) => {
                    let base = proto_arc.read().unwrap().clone();
                    let mut child = base.clone();
                    if base.type_name != "AnonymousForm" {
                        child.type_name = base.type_name.clone();
                        child.ancestors.insert(base.type_name.clone(), base);
                    } else {
                        child.type_name = type_name.clone();
                    }
                    child
                }
                _ => FormInstance::new(type_name),
            };
            for (k, v_expr) in fields {
                match v_expr {
                    LirExpr::Lambda {
                        params,
                        body,
                        raw_args,
                        raw_body,
                    } if raw_args.is_empty() && params.as_slice() == ["_"] => {
                        form.set_field(
                            k,
                            Value::Stream {
                                stmts: body.clone(),
                                raw: raw_body.clone(),
                            },
                        );
                    }
                    LirExpr::Lambda {
                        params,
                        body,
                        raw_body,
                        ..
                    } => {
                        let lir_p = params.iter().map(|p| (p.clone(), "Any".into(), None)).collect();
                        let func = Arc::new(FunctionInstance {
                            name: k.clone(),
                            body: FunctionBody::LirDefined {
                                params: lir_p,
                                first_param_constraint: String::new(),
                                stmts: body.clone(),
                                raw_body: raw_body.clone(),
                                captured_env: Arc::new(RwLock::new(scope.clone())),
                                overrides: HashMap::new(),
                                partial_args: Vec::new(),
                            },
                        });
                        form.methods.insert(k.clone(), func);
                    }
                    _ => {
                        let v = eval_lir_expr(v_expr, scope)?;
                        form.set_field(k, v);
                    }
                }
            }
            Ok(Value::Form(Arc::new(RwLock::new(form))))
        }
        LirExpr::Unary { op, expr: inner } => match (op.as_str(), eval_lir_expr(inner, scope)?) {
            ("!", Value::Bool(b)) => Ok(Value::Bool(!b)),
            _ => Err(format!("Унарный оператор '{op}' не применим")),
        },
        LirExpr::Binary { left, op, right } => {
            let lhs = eval_lir_expr(left, scope)?;
            if op == "is" {
                if let LirExpr::Ident(type_name) = right.as_ref() {
                    return Ok(Value::Bool(match_type_or_wildcard(type_name, &lhs, scope)));
                }
                return Ok(Value::Bool(false));
            }
            if op == "tuple_len_eq" {
                let LirExpr::Int(expected_len) = right.as_ref() else {
                    return Ok(Value::Bool(false));
                };
                let actual_len = match &lhs {
                    Value::Tuple(v) => v.len(),
                    Value::None => 0,
                    _ => 1,
                };
                return Ok(Value::Bool(actual_len == *expected_len as usize));
            }
            if op == "/" {
                if let (Value::Form(f_arc), LirExpr::Ident(path_str)) = (&lhs, right.as_ref()) {
                    let has_div_method = f_arc.read().unwrap().methods.contains_key("/");
                    if !has_div_method || is_pascal_case(path_str) || path_str.contains('.') {
                        let sub = subtract_ancestor_or_path(&f_arc.read().unwrap(), path_str);
                        return Ok(Value::Form(Arc::new(RwLock::new(sub))));
                    }
                }
            }
            let rhs = eval_lir_expr(right, scope)?;
            eval_binary_op_in_scope(lhs, op, rhs, scope)
        }
        LirExpr::Call { callee, args } => eval_lir_call(callee, args, None, scope),
        LirExpr::Pipe { input, op, step } => {
            let inp = eval_lir_expr(input, scope)?;
            eval_lir_pipe_step(inp, op.clone(), step, scope)
        }
        LirExpr::Router { input, branches } => {
            if matches!(input.as_ref(), LirExpr::Ident(id) if id == "_") && scope.get("_").is_none()
            {
                return Ok(Value::Router(branches.clone()));
            }
            let inp = eval_lir_expr(input, scope)?;
            eval_lir_router(branches, inp, scope)
        }
        LirExpr::Lambda {
            params,
            body,
            raw_args,
            raw_body,
        } => {
            if raw_args.is_empty() && params.as_slice() == ["_"] {
                Ok(Value::Stream {
                    stmts: body.clone(),
                    raw: raw_body.clone(),
                })
            } else {
                let lir_p = params.iter().map(|p| (p.clone(), "Any".into(), None)).collect();
                Ok(Value::Function(Arc::new(FunctionInstance {
                    name: if raw_args.is_empty() { "Method".into() } else { "Lambda".into() },
                    body: FunctionBody::LirDefined {
                        params: lir_p,
                        first_param_constraint: String::new(),
                        stmts: body.clone(),
                        raw_body: raw_body.clone(),
                        captured_env: Arc::new(RwLock::new(scope.clone())),
                        overrides: HashMap::new(),
                        partial_args: Vec::new(),
                    },
                })))
            }
        }
    }
}

fn eval_lir_pipe_step(
    input: Value,
    op: PipeOp,
    step: &LirExpr,
    scope: &mut Scope,
) -> Result<Value, String> {
    if is_bypass_value(&input) && op != PipeOp::Filter {
        return Ok(input);
    }

    match op {
        PipeOp::Dot => eval_lir_pipe_target(input, step, scope),
        PipeOp::Tap => {
            let _ = eval_lir_pipe_target(input.clone(), step, scope)?;
            Ok(input)
        }
        PipeOp::Enrich => {
            if let Ok(target) = eval_lir_expr(step, scope) {
                if !callable_accepts_input(&target, &input, scope) {
                    return Ok(input);
                }
            }
            let enriched = eval_lir_pipe_target(input.clone(), step, scope)?;
            if is_bypass_value(&enriched) {
                Ok(enriched)
            } else {
                eval_binary_op_in_scope(input, "&", enriched, scope)
            }
        }
        PipeOp::Filter => {
            if is_bypass_value(&input) {
                return Ok(input);
            }
            let pred = eval_lir_expr(step, scope)?;
            match apply_callable(pred, input.clone(), scope)? {
                Value::Bool(true) => Ok(input),
                _ => Ok(Value::None),
            }
        }
        PipeOp::Parallel => {
            let LirExpr::StructInit { fields, .. } = step else {
                return Err("Оператор .| ожидает блок {}".into());
            };
            let mut handles = Vec::new();
            for (branch_name, branch_expr) in fields {
                let (b_name, b_expr, in_clone) = (branch_name.clone(), branch_expr.clone(), input.clone());
                let parent = Arc::new(RwLock::new(scope.clone()));
                handles.push(thread::spawn(move || -> Result<(String, Value), String> {
                    let mut l_scope = Scope::child_stream(parent, in_clone.clone());
                    if let Value::Form(f) = &in_clone {
                        l_scope.unpack_form(f, true);
                    }
                    let res = match eval_lir_expr(&b_expr, &mut l_scope)? {
                        Value::Stream { stmts, .. } => {
                            unwrap_done(eval_lir_stmts(&stmts, Some(in_clone), &mut l_scope)?)
                        }
                        other => unwrap_done(other),
                    };
                    Ok((b_name, res))
                }));
            }
            let mut merged = FormInstance::new("ParallelResult");
            for h in handles {
                if let Ok(Ok((k, v))) = h.join() {
                    merged.set_field(&k, v);
                }
            }
            Ok(Value::Form(Arc::new(RwLock::new(merged))))
        }
    }
}

fn is_bypass_value(val: &Value) -> bool {
    match val {
        Value::None => true,
        Value::Form(f_arc) => {
            let f = f_arc.read().unwrap();
            f.type_name == "None" || f.has_lineage("Bypass")
        }
        _ => false,
    }
}

fn eval_lir_pipe_target(input: Value, step: &LirExpr, scope: &mut Scope) -> Result<Value, String> {
    if let LirExpr::Call { callee, args } = step {
        return eval_lir_call(callee, args, Some(input), scope);
    }
    if let LirExpr::Ident(prop) = step {
        match (&input, prop.as_str()) {
            (Value::Float(f), "round_to_int") => return Ok(Value::Int(f.round() as i64)),
            (Value::Int(i), "round_to_int") => return Ok(Value::Int(*i)),
            (Value::Int(i), "to_float") => return Ok(Value::Float(*i as f64)),
            (Value::Float(f), "to_float") => return Ok(Value::Float(*f)),
            _ => {}
        }
    }
    let mut t_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
    t_scope.set("_", input.clone());
    if let Value::Form(f_arc) = &input {
        t_scope.unpack_form(f_arc, true);
    }
    let target = eval_lir_expr(step, &mut t_scope)?;
    apply_callable(target, input, scope)
}

pub fn eval_lir_call(
    fn_name: &str,
    call_args: &[LirExpr],
    piped_first: Option<Value>,
    scope: &mut Scope,
) -> Result<Value, String> {
    if fn_name == "Meta.compose" {
        if let Some(LirExpr::Lambda { raw_body, .. }) = call_args.first() {
            return eval_compose_call(&[vec![Token::Group('[', raw_body.clone())]], scope);
        }
    }

    let mut evaluated = Vec::new();
    if let Some(first) = piped_first {
        evaluated.push(first);
    }
    for a in call_args {
        evaluated.push(eval_lir_expr(a, scope)?);
    }

    let arg = match evaluated.len() {
        0 => Value::None,
        1 => evaluated[0].clone(),
        _ => Value::Tuple(evaluated.clone()),
    };

    if let Some((obj_path, method_name)) = fn_name.rsplit_once('.') {
        if let Ok(obj_val) = eval_path(obj_path, scope) {
            match obj_val {
                Value::Form(f_arc) => {
                    let snap = f_arc.read().unwrap().clone();
                    if let Some(method) = find_method_in_form_or_ancestors(&snap, method_name) {
                        if let FunctionBody::Native(f) = &method.body {
                            return f(evaluated, scope);
                        }
                        return invoke_form_method(&snap, &method, arg, scope);
                    }
                    if let Some(Value::Function(func)) = snap.get_field(method_name) {
                        if let FunctionBody::Native(f) = &func.body {
                            return f(evaluated, scope);
                        }
                        return invoke_form_method(&snap, &func, arg, scope);
                    }
                    if snap.has_lineage("Atom") {
                        match method_name {
                            "swap" => {
                                let cur = snap.get_field("value").unwrap_or(Value::None);
                                let next = apply_callable(arg, cur, scope)?;
                                f_arc.write().unwrap().set_field("value", next.clone());
                                return Ok(next);
                            }
                            "read" | "get" => return Ok(snap.get_field("value").unwrap_or(Value::None)),
                            _ => {}
                        }
                    }
                }
                Value::Atom(atom_arc) => match method_name {
                    "swap" => {
                        let cur = atom_arc.lock().unwrap().clone();
                        let next = apply_callable(arg, cur, scope)?;
                        *atom_arc.lock().unwrap() = next.clone();
                        return Ok(next);
                    }
                    "read" | "get" => return Ok(atom_arc.lock().unwrap().clone()),
                    "set" => {
                        *atom_arc.lock().unwrap() = arg.clone();
                        return Ok(arg);
                    }
                    _ => {}
                },
                Value::Channel(chan_arc) => match method_name {
                    "send" => {
                        chan_arc.lock().unwrap().push_back(arg);
                        return Ok(Value::None);
                    }
                    "recv" => {
                        return Ok(match chan_arc.lock().unwrap().pop_front() {
                            Some(v) => make_some(v),
                            None => make_empty_form("None"),
                        });
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    if let Ok(func) = eval_path(fn_name, scope) {
        if let Value::Symbol(tag) = func {
            let mut node = FormInstance::new(&tag);
            node.set_field("args", arg);
            return Ok(Value::Form(Arc::new(RwLock::new(node))));
        }
        if let Value::Function(ref f_inst) = func {
            if let FunctionBody::Native(f) = &f_inst.body {
                return f(evaluated, scope);
            }
        }
        if let Value::Form(ref f_arc) = func {
            let snap = f_arc.read().unwrap().clone();
            if let Some(call_method) = snap.methods.get("__call__") {
                if let FunctionBody::Native(f) = &call_method.body {
                    return f(evaluated, scope);
                }
            }
        }
        apply_callable(func, arg, scope)
    } else {
        Err(format!("Функция '{fn_name}' не найдена"))
    }
}

fn find_method_in_form_or_ancestors(
    form: &FormInstance,
    method_name: &str,
) -> Option<Arc<FunctionInstance>> {
    form.methods.get(method_name).cloned().or_else(|| {
        form.ancestors
            .values()
            .find_map(|anc| find_method_in_form_or_ancestors(anc, method_name))
    })
}

pub fn eval_lir_router(
    branches: &[LirBranch],
    arg: Value,
    scope: &mut Scope,
) -> Result<Value, String> {
    for branch in branches {
        let mut b_scope = Scope::child_stream(Arc::new(RwLock::new(scope.clone())), arg.clone());
        if let Value::Form(f) = &arg {
            b_scope.unpack_form(f, true);
        }
        if matches!(eval_lir_expr(&branch.condition, &mut b_scope), Ok(Value::Bool(true))) {
            for (var, val_expr) in &branch.bindings {
                let v = eval_lir_expr(val_expr, &mut b_scope)?;
                b_scope.set(var, v);
            }
            return eval_lir_stmts(&branch.body, Some(arg), &mut b_scope);
        }
    }
    if is_bypass_value(&arg) {
        return Ok(arg);
    }
    Ok(Value::None)
}

pub fn eval_path(path: &str, scope: &mut Scope) -> Result<Value, String> {
    if path == "_" {
        return scope.get("_").ok_or_else(|| "Контекст `_` не определен".into());
    }
    if path.starts_with('_') && path.len() == 2 && path.chars().nth(1).unwrap().is_ascii_digit() {
        return scope.get(path).or_else(|| scope.get("_")).ok_or_else(|| format!("Контекст `{path}` не определен"));
    }

    let segments: Vec<&str> = path.split('.').collect();
    let (root_name, root_call) = segments[0].strip_suffix("()").map(|s| (s, true)).unwrap_or((segments[0], false));

    let mut current = if let Some(val) = scope.get(root_name).or_else(|| match root_name {
        "self" => scope.get("Self"),
        "Self" => scope.get("self"),
        r if r.starts_with('_') && r.len() == 2 => scope.get("_"),
        _ => None,
    }) {
        if root_call {
            apply_callable(val, Value::None, scope)?
        } else {
            val
        }
    } else if segments.len() == 1 {
        return Ok(Value::Symbol(root_name.to_string()));
    } else {
        return Err(format!("Неопределенный идентификатор: '{root_name}'"));
    };

    for seg_raw in &segments[1..] {
        let (seg, is_call) = seg_raw.strip_suffix("()").map(|s| (s, true)).unwrap_or((*seg_raw, false));

        current = match &current {
            Value::Form(form_arc) => {
                let snap = form_arc.read().unwrap().clone();
                if let Some(origins) = snap.ambiguous_fields.get(seg) {
                    return Err(format!(
                        "Нарушение Закона 2 (Lineage): Поле '{seg}' в '{}' неоднозначно ({}).",
                        snap.type_name,
                        origins.join(" & ")
                    ));
                }
                if is_call {
                    if let Some(m) = find_method_in_form_or_ancestors(&snap, seg) {
                        invoke_form_method(&snap, &m, Value::None, scope)?
                    } else if seg == "read" || seg == "get" {
                        snap.get_field("value").unwrap_or(Value::None)
                    } else {
                        return Err(format!("Метод '{seg}()' не найден в {}", snap.type_name));
                    }
                } else if let Some(val) = snap.get_field(seg) {
                    if let Value::Stream { stmts, raw } = val {
                        eval_reactive_field(&snap, &stmts, &raw, scope)?
                    } else {
                        val
                    }
                } else if let Some(m) = find_method_in_form_or_ancestors(&snap, seg) {
                    Value::Function(m)
                } else if let Ok(idx) = seg.parse::<i32>() {
                    let i = if idx < 0 { (snap.field_order.len() as i32 + idx) as usize } else { idx as usize };
                    let fname = snap.field_order.get(i).ok_or("Индекс поля вне диапазона")?;
                    let v = snap.fields.get(fname).unwrap().clone();
                    if let Value::Stream { stmts, raw } = v {
                        eval_reactive_field(&snap, &stmts, &raw, scope)?
                    } else {
                        v
                    }
                } else if let Some(anc) = find_ancestor_recursive(&snap, seg) {
                    Value::Form(Arc::new(RwLock::new(anc)))
                } else {
                    return Err(format!("Поле или предок '{seg}' не найден в {}", snap.type_name));
                }
            }
            Value::Atom(a) if matches!(seg, "read" | "value" | "get") => a.lock().unwrap().clone(),
            Value::Channel(q) if seg == "recv" && is_call => match q.lock().unwrap().pop_front() {
                Some(v) => make_some(v),
                None => make_empty_form("None"),
            },
            Value::Tuple(items) => {
                if seg == "length" {
                    Value::Int(items.len() as i64)
                } else {
                    let idx: i32 = seg.parse().map_err(|_| format!("Ожидался индекс, получено '{seg}'"))?;
                    let i = if idx < 0 { (items.len() as i32 + idx) as usize } else { idx as usize };
                    items.get(i).cloned().ok_or("Индекс вне границ кортежа")?
                }
            }
            Value::String(s) if seg == "length" => Value::Int(s.chars().count() as i64),
            Value::Int(i) if seg == "to_float" => Value::Float(*i as f64),
            Value::Float(f) if seg == "round_to_int" => Value::Int(f.round() as i64),
            _ => return Err(format!("Нельзя получить '{seg}' из {current:?}")),
        };
    }
    Ok(current)
}

fn find_ancestor_recursive(form: &FormInstance, target: &str) -> Option<FormInstance> {
    form.ancestors
        .get(target)
        .cloned()
        .or_else(|| form.ancestors.values().find_map(|a| find_ancestor_recursive(a, target)))
}

fn eval_reactive_field(
    snap: &FormInstance,
    stmts: &[LirStmt],
    raw_sents: &[Sentence],
    scope: &mut Scope,
) -> Result<Value, String> {
    if stream_uses_free_underscore(raw_sents) {
        return Ok(Value::Stream {
            stmts: stmts.to_vec(),
            raw: raw_sents.to_vec(),
        });
    }
    let mut r_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
    let mut resolved_self = snap.clone();

    for fname in &snap.field_order {
        if let Some(fval) = snap.fields.get(fname) {
            if let Value::Stream { stmts: dep_stmts, raw: dep_raw } = fval {
                if dep_raw == raw_sents {
                    break;
                }
                if !stream_uses_free_underscore(dep_raw) {
                    let s_arc = Arc::new(RwLock::new(resolved_self.clone()));
                    r_scope.set("self", Value::Form(s_arc.clone()));
                    r_scope.set("Self", Value::Form(s_arc));
                    if let Ok(comp) = eval_lir_stmts(dep_stmts, None, &mut r_scope) {
                        let unwrapped = unwrap_done(comp);
                        r_scope.set(fname, unwrapped.clone());
                        resolved_self.set_field(fname, unwrapped);
                    }
                }
            } else {
                r_scope.set(fname, fval.clone());
            }
        }
    }
    let resolved_arc = Arc::new(RwLock::new(resolved_self));
    r_scope.set("self", Value::Form(resolved_arc.clone()));
    r_scope.set("Self", Value::Form(resolved_arc));
    Ok(unwrap_done(eval_lir_stmts(stmts, None, &mut r_scope)?))
}

pub fn invoke_form_method(
    snap: &FormInstance,
    method: &Arc<FunctionInstance>,
    arg: Value,
    scope: &mut Scope,
) -> Result<Value, String> {
    match &method.body {
        FunctionBody::Native(f) => f(if matches!(arg, Value::None) { vec![] } else { vec![arg] }, scope),
        FunctionBody::LirDefined {
            params,
            stmts,
            captured_env,
            overrides,
            ..
        } => {
            let mut merged_env = scope.clone();
            for (k, v) in &captured_env.read().unwrap().bindings {
                merged_env.set(k, v.clone());
            }
            let mut call_scope = Scope::child(Arc::new(RwLock::new(merged_env)));
            let self_arc = Arc::new(RwLock::new(snap.clone()));
            call_scope.set("self", Value::Form(self_arc.clone()));
            call_scope.set("Self", Value::Form(self_arc.clone()));
            call_scope.unpack_form(&self_arc, true);
            for (k, v) in overrides {
                call_scope.set(k, v.clone());
            }
            bind_compiled_params(params, &arg, &mut call_scope)?;
            Ok(unwrap_done(eval_lir_stmts(stmts, Some(arg), &mut call_scope)?))
        }
        FunctionBody::Composed(f1, f2) => {
            let mid = apply_callable(f1.clone(), arg, scope)?;
            apply_callable(f2.clone(), mid, scope)
        }
    }
}

pub fn eval_binary_op_in_scope(
    left: Value,
    op: &str,
    right: Value,
    scope: &mut Scope,
) -> Result<Value, String> {
    if op == "&" {
        if matches!(right, Value::None) {
            return Ok(left);
        }
        if matches!(left, Value::None) {
            return Ok(right);
        }
    }
    if let Value::Form(ref f_arc) = left {
        let snap = f_arc.read().unwrap().clone();
        if let Some(m) = find_method_in_form_or_ancestors(&snap, op) {
            return invoke_form_method(&snap, &m, right, scope);
        }
    }
    if op == "==" {
        if let Value::Range(start, end) = &right {
            return match &left {
                Value::Int(v) => Ok(Value::Bool(*v >= *start && *v <= *end)),
                Value::String(s) => {
                    let ch = s.chars().next().unwrap_or('\0') as i64;
                    Ok(Value::Bool(ch >= *start && ch <= *end))
                }
                _ => Ok(Value::Bool(false)),
            };
        }
        if let Value::Form(r_arc) = &right {
            let rf = r_arc.read().unwrap();
            if !rf.union_variants.is_empty() {
                return Ok(Value::Bool(rf.union_variants.iter().any(|v| v == &left)));
            }
        }
        return Ok(Value::Bool(left == right));
    }
    if op == "!=" {
        return Ok(Value::Bool(left != right));
    }

    match (left, op, right) {
        (Value::Int(a), op, Value::Int(b)) => match op {
            "+" => Ok(Value::Int(a + b)),
            "-" => Ok(Value::Int(a - b)),
            "*" => Ok(Value::Int(a * b)),
            "/" => (b != 0).then(|| Value::Int(a / b)).ok_or_else(|| "Деление на ноль".into()),
            "%" => Ok(Value::Int(a % b)),
            "**" => Ok(Value::Int(a.pow(b.max(0) as u32))),
            "<<" => Ok(Value::Int(a << b)),
            ">>" => Ok(Value::Int(a >> b)),
            ">" => Ok(Value::Bool(a > b)),
            "<" => Ok(Value::Bool(a < b)),
            ">=" => Ok(Value::Bool(a >= b)),
            "<=" => Ok(Value::Bool(a <= b)),
            _ => Err(format!("Неизвестный оператор Int: {op}")),
        },
        (Value::Float(a), op, Value::Float(b)) => eval_float_op(a, op, b),
        (Value::Int(a), op, Value::Float(b)) => eval_float_op(a as f64, op, b),
        (Value::Float(a), op, Value::Int(b)) => eval_float_op(a, op, b as f64),
        (Value::Bool(a), "&&", Value::Bool(b)) => Ok(Value::Bool(a && b)),
        (Value::Bool(a), "||", Value::Bool(b)) => Ok(Value::Bool(a || b)),
        (Value::String(a), "+", Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),

        (Value::Form(f1), "&", Value::Form(f2)) => {
            let (r1, r2) = (f1.read().unwrap(), f2.read().unwrap());
            if r1.type_name == "Compile" {
                return Ok(Value::Form(f2.clone()));
            }
            if matches!(r2.type_name.as_str(), "Public" | "Private" | "Compile") {
                return Ok(Value::Form(f1.clone()));
            }
            if r2.type_name == "Bypass" {
                let mut m = r1.clone();
                m.ancestors.insert("Bypass".into(), r2.clone());
                return Ok(Value::Form(Arc::new(RwLock::new(m))));
            }
            let is_override = r2.type_name == "AnonymousForm";
            let new_name = if is_override {
                r1.type_name.clone()
            } else {
                format!("{}+{}", r1.type_name, r2.type_name)
            };

            let mut merged = FormInstance::new(&new_name);
            merged.ctor_params = r1.ctor_params.clone();
            merged.eager_fields = r1.eager_fields.clone();
            merged.ancestors = r1.ancestors.clone();
            merged.ancestors.extend(r2.ancestors.clone());
            merged.ancestors.insert(r1.type_name.clone(), r1.clone());
            if !is_override {
                merged.ancestors.insert(r2.type_name.clone(), r2.clone());
            }
            merged.methods = r1.methods.clone();
            merged.methods.extend(r2.methods.clone());
            merged.predicates = r1.predicates.clone();
            merged.predicates.extend(r2.predicates.clone());
            merged.ambiguous_fields = r1.ambiguous_fields.clone();

            for k in &r1.field_order {
                let orig = r1.field_origins.get(k).cloned().unwrap_or_else(|| r1.type_name.clone());
                merged.set_field_with_origin(k, r1.fields[k].clone(), &orig);
            }
            for k in &r2.field_order {
                let rhs_orig = r2.field_origins.get(k).cloned().unwrap_or_else(|| r2.type_name.clone());
                let rhs_val = r2.fields[k].clone();
                if is_override {
                    merged.set_field_with_origin(k, rhs_val, &r1.type_name);
                } else if let Some(lhs_orig) = merged.field_origins.get(k).cloned() {
                    if lhs_orig != rhs_orig {
                        merged.ambiguous_fields.entry(k.clone()).or_insert_with(|| vec![lhs_orig]).push(rhs_orig);
                    }
                    merged.fields.insert(k.clone(), rhs_val);
                } else {
                    merged.set_field_with_origin(k, rhs_val, &rhs_orig);
                }
            }
            Ok(Value::Form(Arc::new(RwLock::new(merged))))
        }

        (Value::Form(f1), "&", Value::Stream { stmts, .. }) if f1.read().unwrap().type_name == "Compile" => {
            Ok(unwrap_done(eval_lir_stmts(&stmts, None, scope)?))
        }
        (Value::Form(f1), "&", pred @ (Value::Function(_) | Value::Stream { .. })) => {
            let mut refined = f1.read().unwrap().clone();
            refined.ancestors.insert(refined.type_name.clone(), f1.read().unwrap().clone());
            refined.predicates.push(pred);
            Ok(Value::Form(Arc::new(RwLock::new(refined))))
        }
        (Value::Symbol(base_sym), "&", pred @ (Value::Function(_) | Value::Stream { .. })) => {
            let mut refined = FormInstance::new(&base_sym);
            refined.predicates.push(pred);
            Ok(Value::Form(Arc::new(RwLock::new(refined))))
        }
        (Value::Function(func_arc), "&", Value::Form(ov_arc)) => {
            let ov = ov_arc.read().unwrap();
            if matches!(ov.type_name.as_str(), "Public" | "Private" | "Compile") {
                return Ok(Value::Function(func_arc));
            }
            if let FunctionBody::LirDefined {
                params,
                first_param_constraint,
                stmts,
                raw_body,
                captured_env,
                overrides,
                partial_args,
            } = &func_arc.body
            {
                let mut new_ov = overrides.clone();
                new_ov.extend(ov.fields.clone());
                Ok(Value::Function(Arc::new(FunctionInstance {
                    name: func_arc.name.clone(),
                    body: FunctionBody::LirDefined {
                        params: params.clone(),
                        first_param_constraint: first_param_constraint.clone(),
                        stmts: stmts.clone(),
                        raw_body: raw_body.clone(),
                        captured_env: captured_env.clone(),
                        overrides: new_ov,
                        partial_args: partial_args.clone(),
                    },
                })))
            } else {
                Ok(Value::Function(func_arc))
            }
        }
        (f1 @ (Value::Function(_) | Value::Stream { .. }), "&", f2 @ (Value::Function(_) | Value::Stream { .. })) => {
            Ok(Value::Function(Arc::new(FunctionInstance {
                name: "ComposedStream".into(),
                body: FunctionBody::Composed(f1, f2),
            })))
        }
        (val, "&", Value::Form(_)) => Ok(val),
        (left_val, "|", right_val) => {
            let mut u = FormInstance::new("UnionType");
            for v in [left_val, right_val] {
                if let Value::Form(f_arc) = &v {
                    let fr = f_arc.read().unwrap();
                    if !fr.union_variants.is_empty() {
                        u.union_variants.extend(fr.union_variants.clone());
                        continue;
                    }
                }
                u.union_variants.push(v);
            }
            Ok(Value::Form(Arc::new(RwLock::new(u))))
        }
        (Value::Form(f), "/", Value::Symbol(sym) | Value::String(sym)) => {
            let mut form = f.read().unwrap().clone();
            form.remove_field(&sym);
            form.methods.remove(&sym);
            for anc in form.ancestors.values_mut() {
                anc.remove_field(&sym);
            }
            Ok(Value::Form(Arc::new(RwLock::new(form))))
        }
        (Value::Form(f), "/", Value::Form(rhs_arc)) => {
            let rhs_name = rhs_arc.read().unwrap().type_name.clone();
            Ok(Value::Form(Arc::new(RwLock::new(subtract_ancestor_or_path(&f.read().unwrap(), &rhs_name)))))
        }
        (a, op, b) => Err(format!("Оператор '{op}' не поддерживается для {a:?} и {b:?}")),
    }
}

fn eval_float_op(a: f64, op: &str, b: f64) -> Result<Value, String> {
    match op {
        "+" => Ok(Value::Float(a + b)),
        "-" => Ok(Value::Float(a - b)),
        "*" => Ok(Value::Float(a * b)),
        "/" => Ok(Value::Float(a / b)),
        "**" => Ok(Value::Float(a.powf(b))),
        ">" => Ok(Value::Bool(a > b)),
        "<" => Ok(Value::Bool(a < b)),
        ">=" => Ok(Value::Bool(a >= b)),
        "<=" => Ok(Value::Bool(a <= b)),
        _ => Err(format!("Неизвестный оператор Float: {op}")),
    }
}

pub fn subtract_ancestor_or_path(source: &FormInstance, target_path: &str) -> FormInstance {
    let mut form = source.clone();
    if let Some((anc_name, field_name)) = target_path.split_once('.') {
        if let Some(anc) = form.ancestors.get_mut(anc_name) {
            anc.remove_field(field_name);
        }
        if let Some(origins) = form.ambiguous_fields.get_mut(field_name) {
            origins.retain(|o| o != anc_name);
            if origins.len() == 1 {
                let winner = origins[0].clone();
                form.ambiguous_fields.remove(field_name);
                form.field_origins.insert(field_name.to_string(), winner.clone());
                if let Some(val) = form.ancestors.get(&winner).and_then(|a| a.get_field(field_name)) {
                    form.fields.insert(field_name.to_string(), val);
                }
            } else if origins.is_empty() {
                form.remove_field(field_name);
            }
        } else {
            form.remove_field(field_name);
        }
    } else {
        form.ancestors.remove(target_path);
        for k in form.field_order.clone() {
            if let Some(origins) = form.ambiguous_fields.get_mut(&k) {
                origins.retain(|o| o != target_path);
                if origins.len() == 1 {
                    let winner = origins[0].clone();
                    form.ambiguous_fields.remove(&k);
                    form.field_origins.insert(k.clone(), winner.clone());
                    if let Some(val) = form.ancestors.get(&winner).and_then(|a| a.get_field(&k)) {
                        form.fields.insert(k.clone(), val);
                    }
                }
            } else if form.field_origins.get(&k).map(String::as_str) == Some(target_path) {
                form.remove_field(&k);
            }
        }
    }
    form
}

// =============================================================================
// 6. КОМПАКТНАЯ ПРОВЕРКА ЗАКОНА 3 ПО `first_param_constraint` ИЗ LIR (15 СТРОК!)
// =============================================================================

fn callable_accepts_input(callable: &Value, input: &Value, scope: &Scope) -> bool {
    let Value::Function(func_arc) = callable else { return true };
    let FunctionBody::LirDefined { first_param_constraint, partial_args, .. } = &func_arc.body else {
        return true;
    };
    if !partial_args.is_empty() || first_param_constraint.is_empty() {
        return true;
    }
    let Value::Form(val_arc) = input else { return true };
    let vf = val_arc.read().unwrap();

    let parts: Vec<&str> = first_param_constraint.split_whitespace().collect();
    let mut i = 0;
    while i < parts.len() {
        if parts[i] == "/" {
            if let Some(&forb) = parts.get(i + 1) {
                if vf.has_lineage(forb) {
                    return false;
                }
            }
            i += 2;
            continue;
        }
        let t_name = parts[i].split('(').next().unwrap_or(parts[i]);
        if is_pascal_case(t_name)
            && !matches!(t_name, "String" | "Symbol" | "Int" | "Float" | "Bool" | "Byte" | "Any" | "Tuple" | "Path" | "Url" | "List" | "Option")
            && (matches!(scope.get(t_name), Some(Value::Form(_))) || parts.contains(&"&") || parts.contains(&"/"))
            && !vf.has_lineage(t_name)
        {
            let matches_union = matches!(
                scope.get(t_name),
                Some(Value::Form(u_arc)) if u_arc.read().unwrap().union_variants.iter().any(|v| {
                    matches!(v, Value::Form(vf2) if vf.has_lineage(&vf2.read().unwrap().type_name))
                })
            );
            if !matches_union {
                return false;
            }
        }
        i += 1;
    }
    true
}

pub fn apply_callable(callable: Value, arg: Value, scope: &mut Scope) -> Result<Value, String> {
    if !callable_accepts_input(&callable, &arg, scope) {
        return Ok(arg);
    }

    match callable {
        Value::Function(func) => match &func.body {
            FunctionBody::Native(f) => f(if matches!(arg, Value::None) { vec![] } else { vec![arg] }, scope),
            FunctionBody::LirDefined {
                params,
                first_param_constraint,
                stmts,
                raw_body,
                captured_env,
                overrides,
                partial_args,
            } => {
                let mut combined = partial_args.clone();
                match &arg {
                    Value::Tuple(items) if params.len() > 1 => combined.extend(items.clone()),
                    Value::None => {}
                    other => combined.push(other.clone()),
                }

                if !combined.is_empty() && combined.len() < params.len() && !matches!(arg, Value::None) {
                    if params.iter().skip(combined.len()).any(|(_, _, def)| def.is_none()) {
                        return Ok(Value::Function(Arc::new(FunctionInstance {
                            name: format!("{}_partial", func.name),
                            body: FunctionBody::LirDefined {
                                params: params.clone(),
                                first_param_constraint: first_param_constraint.clone(),
                                stmts: stmts.clone(),
                                raw_body: raw_body.clone(),
                                captured_env: captured_env.clone(),
                                overrides: overrides.clone(),
                                partial_args: combined,
                            },
                        })));
                    }
                }

                let mut merged_env = scope.clone();
                for (k, v) in &captured_env.read().unwrap().bindings {
                    merged_env.set(k, v.clone());
                }
                let mut call_scope = Scope::child(Arc::new(RwLock::new(merged_env)));
                call_scope.stream_depth = if func.name == "Lambda" { scope.stream_depth + 1 } else { 0 };

                let packed_arg = match combined.len() {
                    0 => Value::None,
                    1 => combined.remove(0),
                    _ => Value::Tuple(combined),
                };
                if call_scope.stream_depth == 0 {
                    call_scope.set("_0", packed_arg.clone());
                }
                bind_compiled_params(params, &packed_arg, &mut call_scope)?;
                for (k, v) in overrides {
                    call_scope.set(k, v.clone());
                }
                let res = eval_lir_stmts(stmts, Some(packed_arg), &mut call_scope)?;
                Ok(if func.name == "Lambda" { res } else { unwrap_done(res) })
            }
            FunctionBody::Composed(f1, f2) => {
                let mid = apply_callable(f1.clone(), arg, scope)?;
                apply_callable(f2.clone(), mid, scope)
            }
        },
        Value::Stream { stmts, .. } => {
            let mut s_scope = Scope::child_stream(Arc::new(RwLock::new(scope.clone())), arg.clone());
            if let Value::Form(f_arc) = &arg {
                s_scope.unpack_form(f_arc, true);
            }
            eval_lir_stmts(&stmts, Some(arg), &mut s_scope)
        }
        Value::Router(branches) => eval_lir_router(&branches, arg, scope),
        Value::Range(start, end) => match arg {
            Value::Int(v) => Ok(Value::Bool(v >= start && v <= end)),
            Value::String(s) => {
                let ch = s.chars().next().unwrap_or('\0') as i64;
                Ok(Value::Bool(ch >= start && ch <= end))
            }
            _ => Ok(Value::Bool(false)),
        },
        Value::Form(router_arc) => {
            let router = router_arc.read().unwrap().clone();
            if let Some(call_method) = router.methods.get("__call__") {
                return invoke_form_method(&router, call_method, arg, scope);
            }
            if (!router.ctor_params.is_empty() || is_pascal_case(&router.type_name)) && router.union_variants.is_empty() {
                return instantiate_form_constructor(&router, arg, scope);
            }
            if !router.union_variants.is_empty() {
                let matched = router.union_variants.iter().any(|v| match (v, &arg) {
                    (Value::Range(a, b), Value::String(s)) => {
                        let ch = s.chars().next().unwrap_or('\0') as i64;
                        ch >= *a && ch <= *b
                    }
                    _ => v == &arg,
                });
                return Ok(Value::Bool(matched));
            }
            for key in &router.field_order {
                if let Some(Value::Stream { stmts, .. }) = router.fields.get(key) {
                    let mut b_scope = Scope::child_stream(Arc::new(RwLock::new(scope.clone())), arg.clone());
                    if match_type_or_wildcard(key, &arg, &mut b_scope) {
                        return eval_lir_stmts(stmts, Some(arg), &mut b_scope);
                    }
                }
            }
            Ok(Value::None)
        }
        _ => Err(format!("Значение {callable:?} не может быть вызвано")),
    }
}

fn instantiate_form_constructor(
    proto: &FormInstance,
    arg: Value,
    scope: &mut Scope,
) -> Result<Value, String> {
    let mut inst = proto.clone();
    inst.ancestors.insert(proto.type_name.clone(), proto.clone());

    let mut init_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
    bind_compiled_params(&proto.ctor_params, &arg, &mut init_scope)?;

    for (name, _, _) in &proto.ctor_params {
        if let Some(v) = init_scope.get(name) {
            inst.set_field(name, v);
        }
    }
    if !proto.ctor_params.is_empty() {
        for (fname, expr) in &inst.eager_fields.clone() {
            let val = eval_lir_expr(expr, &mut init_scope)?;
            init_scope.set(fname, val.clone());
            inst.set_field(fname, val);
        }
    }
    Ok(Value::Form(Arc::new(RwLock::new(inst))))
}

/// ВСЯ привязка аргументов теперь занимает 25 строк, так как LIR уже сгенерировал пролог и дефолты!
fn bind_compiled_params(
    params: &[(String, String, Option<LirExpr>)],
    incoming: &Value,
    scope: &mut Scope,
) -> Result<(), String> {
    scope.set("_", incoming.clone());
    if let Value::Form(f_arc) = incoming {
        if params.len() <= 1 {
            scope.unpack_form(f_arc, true);
        }
    }
    let items: Vec<Value> = match incoming {
        Value::Tuple(v) if params.len() > 1 => v.clone(),
        Value::None => vec![],
        other => vec![other.clone()],
    };
    for (idx, (name, _, def_expr)) in params.iter().enumerate() {
        if let Some(val) = items.get(idx).cloned() {
            scope.set(name, val.clone());
            if let Value::Form(f_arc) = &val {
                scope.unpack_form(f_arc, true);
            }
        } else if let Some(def) = def_expr {
            let v = eval_lir_expr(def, scope)?;
            scope.set(name, v);
        } else if name != "_" {
            scope.set(name, Value::None);
        }
    }
    Ok(())
}

fn match_type_or_wildcard(pattern_name: &str, val: &Value, scope: &mut Scope) -> bool {
    if pattern_name == "_" {
        scope.set("_", val.clone());
        return true;
    }
    if pattern_name == "None" {
        return matches!(val, Value::None)
            || matches!(val, Value::Form(f) if f.read().unwrap().type_name == "None");
    }
    if (pattern_name == "Form" && matches!(val, Value::Form(_)))
        || (pattern_name == "Tuple" && matches!(val, Value::Tuple(_)))
        || (pattern_name == "Stream" && matches!(val, Value::Stream { .. } | Value::Function(_)))
    {
        return true;
    }
    match val {
        Value::Form(form_arc) => {
            let form = form_arc.read().unwrap().clone();
            let mut matched = form.has_lineage(pattern_name);
            if let Some(Value::Form(target_arc)) = scope.get(pattern_name) {
                let target = target_arc.read().unwrap().clone();
                if !matched && !target.ancestors.is_empty() {
                    matched = target.ancestors.keys().all(|anc| form.has_lineage(anc));
                }
                if matched {
                    for pred in &target.predicates {
                        let mut p_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                        if !matches!(apply_callable(pred.clone(), val.clone(), &mut p_scope), Ok(Value::Bool(true))) {
                            return false;
                        }
                    }
                }
            }
            if matched {
                scope.set(&pascal_to_snake_case(pattern_name), val.clone());
                scope.unpack_form(form_arc, true);
                return true;
            }
            false
        }
        Value::Tuple(_) => true,
        _ => false,
    }
}

// =============================================================================
// 7. ЗАГРУЗКА И ЗАПУСК LIR-МОДУЛЯ
// =============================================================================

fn build_form_from_lir_struct(
    st: &LirStruct,
    top_env: &Arc<RwLock<Scope>>,
    scope: &mut Scope,
) -> Result<Value, String> {
    let mut form = FormInstance::new(&st.name);
    form.ctor_params = st.ctor_params.clone();

    for anc_name in &st.ancestors {
        if let Some(Value::Form(anc_arc)) = scope.get(anc_name) {
            form.ancestors.insert(anc_name.clone(), anc_arc.read().unwrap().clone());
        } else if anc_name == "Bypass" {
            form.ancestors.insert("Bypass".into(), FormInstance::new("Bypass"));
        }
    }
    if st.is_bypass {
        form.ancestors.entry("Bypass".into()).or_insert_with(|| FormInstance::new("Bypass"));
    }

    let mut init_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
    for f in &st.fields {
        let is_ctor_param = st.ctor_params.iter().any(|(n, _, _)| n == &f.name);
        let val = match &f.default_val {
            Some(expr) => {
                if !is_ctor_param {
                    form.eager_fields.push((f.name.clone(), expr.clone()));
                }
                eval_lir_expr(expr, &mut init_scope).unwrap_or_else(|_| Value::Symbol(f.ty.clone()))
            }
            None => Value::Symbol(f.ty.clone()),
        };
        init_scope.set(&f.name, val.clone());
        form.set_field_with_origin(&f.name, val, &f.origin);
        if f.is_ambiguous {
            form.ambiguous_fields.insert(f.name.clone(), f.conflicting_origins.clone());
        }
    }

    for g in &st.reactive_getters {
        form.set_field(
            &g.name,
            Value::Stream {
                stmts: g.body.clone(),
                raw: g.raw_stream.clone(),
            },
        );
    }
    for sub in &st.submodules {
        let sub_val = build_form_from_lir_struct(sub, top_env, scope)?;
        form.set_field(&sub.name, sub_val);
    }
    for m in &st.methods {
        if m.is_contract && m.body.is_empty() {
            continue;
        }
        let func = Arc::new(FunctionInstance {
            name: m.raw_name.clone(),
            body: FunctionBody::LirDefined {
                params: m.params.clone(),
                first_param_constraint: m.first_param_constraint.clone(),
                stmts: m.body.clone(),
                raw_body: m.raw_body.clone(),
                captured_env: top_env.clone(),
                overrides: HashMap::new(),
                partial_args: Vec::new(),
            },
        });
        form.methods.insert(m.raw_name.clone(), func);
    }
    for v_expr in &st.union_variant_exprs {
        if let Ok(v) = eval_lir_expr(v_expr, scope) {
            form.union_variants.push(v);
        }
    }
    for p_expr in &st.refinement_predicates {
        if let Ok(p) = eval_lir_expr(p_expr, scope) {
            form.predicates.push(p);
        }
    }
    Ok(Value::Form(Arc::new(RwLock::new(form))))
}

pub fn run_lir_module(lir: &LirModule) -> Result<Value, String> {
    let mut global_scope = create_global_scope();
    let top_env = Arc::new(RwLock::new(global_scope.clone()));

    for func in &lir.functions {
        let val = if func.is_stream {
            Value::Stream {
                stmts: func.body.clone(),
                raw: func.raw_body.clone(),
            }
        } else {
            Value::Function(Arc::new(FunctionInstance {
                name: func.raw_name.clone(),
                body: FunctionBody::LirDefined {
                    params: func.params.clone(),
                    first_param_constraint: func.first_param_constraint.clone(),
                    stmts: func.body.clone(),
                    raw_body: func.raw_body.clone(),
                    captured_env: top_env.clone(),
                    overrides: HashMap::new(),
                    partial_args: Vec::new(),
                },
            }))
        };
        global_scope.set(&func.raw_name, val);
    }
    *top_env.write().unwrap() = global_scope.clone();

    enum TopItem<'a> {
        Struct(&'a LirStruct),
        Global(&'a crate::lir::LirBinding),
    }

    let mut items: Vec<(u8, usize, TopItem)> = Vec::new();
    for st in &lir.structs {
        items.push((if st.is_prelude { 0 } else { 1 }, st.sentence_index, TopItem::Struct(st)));
    }
    for g in &lir.globals {
        items.push((1, g.sentence_index, TopItem::Global(g)));
    }
    items.sort_by_key(|(prio, idx, _)| (*prio, *idx));

    let mut last_val = Value::None;
    for (_, _, item) in items {
        match item {
            TopItem::Struct(st) => {
                if (st.name == "Map" || st.name == "Atom") && global_scope.get(&st.name).is_some() {
                    continue;
                }
                let val = build_form_from_lir_struct(st, &top_env, &mut global_scope)?;
                global_scope.set(&st.name, val.clone());
                top_env.write().unwrap().set(&st.name, val.clone());
                last_val = val;
            }
            TopItem::Global(g) => {
                let val = eval_lir_expr(&g.value, &mut global_scope)?;
                if let Value::Form(f_arc) = &val {
                    if is_pascal_case(&g.name) {
                        f_arc.write().unwrap().type_name = g.name.clone();
                    }
                }
                global_scope.set(&g.name, val.clone());
                top_env.write().unwrap().set(&g.name, val.clone());
                last_val = val;
            }
        }
    }

    for expr in &lir.top_exprs {
        last_val = eval_lir_expr(expr, &mut global_scope)?;
    }
    if let Some(main_fn) = global_scope.get("main").or_else(|| global_scope.get("Main")) {
        return apply_callable(main_fn, Value::None, &mut global_scope);
    }
    Ok(last_val)
}

pub fn run_hir_module(hir: &HirModule) -> Result<Value, String> {
    let lir = lower_hir_to_lir(hir);
    run_lir_module(&lir)
}

pub fn run_program(source: &str) -> Result<Value, String> {
    let sentences = parse_code(source).map_err(|e| format!("Ошибка парсинга:\n{e}"))?;
    let hir = lower_to_hir(&sentences);
    let lir = lower_hir_to_lir(&hir);
    run_lir_module(&lir)
}
