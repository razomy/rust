// src/runtime.rs
// =============================================================================
// RAZOMY V5.7 — LIR-POWERED RUNTIME ENGINE (SYSTEM KERNEL)
// Архитектура: AST -> HIR -> Canonical LIR -> LIR VM Execution
// Потокобезопасное ядро: Value, Scope, Законы 1–4, Lineage, LIR-выражения и Роутеры.
// =============================================================================

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use crate::hir::{is_pascal_case, lower_to_hir, pascal_to_snake_case, HirModule};
use crate::lir::{
    lower_body_to_stmts, lower_hir_to_lir, lower_tokens_to_expr, LirBranch, LirExpr,
    LirFieldPatValue, LirInterpPart, LirModule, LirPattern, LirStmt, LirTuplePatElem,
};
use crate::parser::{classify_sentence, parse_code, PipeOp, SemanticKind, Sentence, Token};
use crate::runtime_ext::{create_global_scope, eval_compose_call, make_empty_form, make_some};

// =============================================================================
// 1. ПОТОКОБЕЗОПАСНАЯ СИСТЕМА ЗНАЧЕНИЙ (VALUE) И КОМПАКТНЫЙ ТРЕЙСИНГ
// =============================================================================

#[derive(Clone)]
pub enum Value {
    None,
    Int(i64),
    Float(f64),
    Bool(bool),
    Byte(u8),
    String(String),
    Symbol(String),
    Money(String),
    Percent(f64),
    Date(String),
    Duration(String),
    Range(i64, i64),
    Tuple(Vec<Value>),
    Form(Arc<RwLock<FormInstance>>),
    /// Реактивный поток или блок инструкций, скомпилированный в LIR (+ сырые предложения для Meta.steps_of)
    Stream {
        stmts: Vec<LirStmt>,
        raw: Vec<Sentence>,
    },
    /// Таблица маршрутизации `{ ... }`, скомпилированная в ветки LirBranch
    Router(Vec<LirBranch>),
    Function(Arc<FunctionInstance>),
    Atom(Arc<Mutex<Value>>),
    Channel(Arc<Mutex<VecDeque<Value>>>),
    /// Сигнал раннего выхода из `fold` (`Break { value ... }` или `Done { Value ... }`)
    Done(Box<Value>),
}

impl Value {
    /// Компактное представление внутренней структуры значения для логов и отладки тестов
    pub fn compact_dump(&self) -> String {
        match self {
            Value::None => "None".into(),
            Value::Int(i) => format!("Int({i})"),
            Value::Float(f) => format!("Float({f})"),
            Value::Bool(b) => format!("Bool({b})"),
            Value::Byte(b) => format!("Byte(0x{b:02X})"),
            Value::String(s) => format!("Str({s:?})"),
            Value::Symbol(s) => format!("Sym('{s}')"),
            Value::Money(m) => format!("Money({m})"),
            Value::Percent(p) => format!("Pct({p}%)"),
            Value::Date(d) => format!("Date({d})"),
            Value::Duration(d) => format!("Dur({d})"),
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
                let ancs: Vec<&str> = f.ancestors.keys().map(String::as_str).collect();
                let methods: Vec<&str> = f.methods.keys().map(String::as_str).collect();
                let mut meta = String::new();
                if !ancs.is_empty() {
                    meta.push_str(&format!(" <:[{}]", ancs.join(",")));
                }
                if !methods.is_empty() {
                    meta.push_str(&format!(" fn:[{}]", methods.join(",")));
                }
                format!("Form({}{meta}){{{}}}", f.type_name, flds.join(", "))
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
            (Value::Money(a), Value::Money(b)) => a == b,
            (Value::Percent(a), Value::Percent(b)) => (a - b).abs() < 1e-9,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::Duration(a), Value::Duration(b)) => a == b,
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
    pub value_params: Vec<Sentence>,
    /// Исходные предложения тела формы для пересчета eager-полей `(raw . Decimal.parse_scale4)` при вызове конструктора `Money("100.00")`
    pub raw_body_sentences: Vec<Sentence>,
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
            value_params: Vec::new(),
            raw_body_sentences: Vec::new(),
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
        args: Vec<Sentence>,
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
            Value::Money(m) => write!(f, "{m}"),
            Value::Percent(p) => write!(f, "{p}%"),
            Value::Date(d) => write!(f, "{d}"),
            Value::Duration(d) => write!(f, "{d}"),
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

// =============================================================================
// 2. ОБЛАСТЬ ВИДИМОСТИ (SCOPE) И ПРОЕКЦИЯ ПОЛЕЙ (ЗАКОН 1)
// =============================================================================

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

    /// Проверяет, объявлена ли переменная `name` в текущем локальном скоупе функции/потока
    pub fn has_local(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Проверяет, существует ли уже привязка `name` в текущем скоупе или в родительских скоупах текущей функции
    pub fn has_in_chain(&self, name: &str) -> bool {
        if self.bindings.contains_key(name) {
            return true;
        }
        if let Some(parent_arc) = &self.parent {
            let parent = parent_arc.read().unwrap();
            // Останавливаемся на глобальном скоупе (у которого `parent.is_none()`),
            // чтобы глобальные типы/символы не блокировали распаковку полей внутри функции,
            // но локальные переменные внешних функций/лямбд (`item`, `disc`) не затирались!
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

    /// Единый метод проекции полей формы и `snake_case` имени типа в скоуп (Закон 1a/1b/1c + предки Закона 2).
    /// Не затирает уже существующие локальные переменные функции (`item`, `name` и т.д.)!
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

// =============================================================================
// 3. ВЫПОЛНЕНИЕ LIR-ИНСТРУКЦИЙ И LIR-ВЫРАЖЕНИЙ (ЯДРО LIR VM)
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
            if f.type_name == "Break" || f.type_name == "Done" {
                if let Some(v) = f.get_field("value").or_else(|| f.get_field("Value")) {
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
        return Err(format!(
            "Переменная '{root_name}' не найдена для мутации '{path}'"
        ));
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
                    f.field_order
                        .get(i)
                        .cloned()
                        .ok_or("Индекс поля вне диапазона")?
                } else {
                    seg.to_string()
                }
            };
            if segs.len() == 1 {
                f_arc.write().unwrap().set_field(&target_field, new_val);
            } else {
                let inner = f_arc
                    .read()
                    .unwrap()
                    .get_field(&target_field)
                    .unwrap_or(Value::None);
                let updated_inner = mutate_value_recursive(inner, &segs[1..], new_val)?;
                f_arc
                    .write()
                    .unwrap()
                    .set_field(&target_field, updated_inner);
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
                if segs.len() == 1 {
                    items[i] = new_val;
                } else {
                    items[i] = mutate_value_recursive(items[i].clone(), &segs[1..], new_val)?;
                }
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
                        if !key.contains(|c: char| {
                            c.is_whitespace() || matches!(c, '+' | '-' | '*' | '/' | '%' | '(' | ')')
                        }) {
                            if let Ok(val) = eval_path(key, scope) {
                                out.push_str(&val.to_string());
                                continue;
                            }
                        }
                        if let Ok(sents) = parse_code(key) {
                            if let Some(first) = sents.first() {
                                let dummy_hir = HirModule::default();
                                let expr = lower_tokens_to_expr(first, &dummy_hir);
                                if let Ok(val) = eval_lir_expr(&expr, scope) {
                                    out.push_str(&val.to_string());
                                    continue;
                                }
                            }
                        }
                        out.push_str(&format!("{{{key}}}"));
                    }
                }
            }
            Ok(Value::String(out))
        }
        LirExpr::Ident(path) => eval_path(path, scope),
        LirExpr::Tuple(items) => {
            let vals = items
                .iter()
                .map(|e| eval_lir_expr(e, scope))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::Tuple(vals))
        }
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
                    child.ancestors.insert(type_name.clone(), base);
                    child
                }
                _ => FormInstance::new(type_name),
            };
            form.type_name = type_name.clone();
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
                        body,
                        raw_args,
                        raw_body,
                        ..
                    } => {
                        let func = Arc::new(FunctionInstance {
                            name: k.clone(),
                            body: FunctionBody::LirDefined {
                                args: raw_args.clone(),
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
                        let lifted_val = if k == "departments" && matches!(v, Value::Form(_)) {
                            Value::Tuple(vec![v])
                        } else {
                            v
                        };
                        form.set_field(k, lifted_val);
                    }
                }
            }
            Ok(Value::Form(Arc::new(RwLock::new(form))))
        }
        LirExpr::Unary { op, expr: inner } => {
            let v = eval_lir_expr(inner, scope)?;
            match (op.as_str(), v) {
                ("!", Value::Bool(b)) => Ok(Value::Bool(!b)),
                _ => Err(format!("Унарный оператор '{op}' не применим")),
            }
        }
        LirExpr::Binary { left, op, right } => {
            let lhs = eval_lir_expr(left, scope)?;
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
            if raw_args.is_empty() && (params.is_empty() || params.as_slice() == ["_"]) {
                if params.is_empty() {
                    Ok(Value::Function(Arc::new(FunctionInstance {
                        name: "Method".into(),
                        body: FunctionBody::LirDefined {
                            args: raw_args.clone(),
                            stmts: body.clone(),
                            raw_body: raw_body.clone(),
                            captured_env: Arc::new(RwLock::new(scope.clone())),
                            overrides: HashMap::new(),
                            partial_args: Vec::new(),
                        },
                    })))
                } else {
                    Ok(Value::Stream {
                        stmts: body.clone(),
                        raw: raw_body.clone(),
                    })
                }
            } else {
                Ok(Value::Function(Arc::new(FunctionInstance {
                    name: "Lambda".into(),
                    body: FunctionBody::LirDefined {
                        args: raw_args.clone(),
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
                let b_name = branch_name.clone();
                let b_expr = branch_expr.clone();
                let in_clone = input.clone();
                let parent = Arc::new(RwLock::new(scope.clone()));
                handles.push(thread::spawn(move || -> Result<(String, Value), String> {
                    let mut l_scope = Scope::child_stream(parent, in_clone.clone());
                    if let Value::Form(f) = &in_clone {
                        l_scope.unpack_form(f, true);
                    }
                    let evaluated = eval_lir_expr(&b_expr, &mut l_scope)?;
                    let res = match evaluated {
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

    // Если шаг пайплайна — это встроенное свойство числа (`round_to_int` или `to_float`),
    // или обращение к методу объекта:
    if let LirExpr::Ident(prop) = step {
        match (&input, prop.as_str()) {
            (Value::Float(f), "round_to_int") => return Ok(Value::Int(f.round() as i64)),
            (Value::Int(i), "round_to_int") => return Ok(Value::Int(*i)),
            (Value::Int(i), "to_float") => return Ok(Value::Float(*i as f64)),
            (Value::Float(f), "to_float") => return Ok(Value::Float(*f)),
            _ => {}
        }
    }

    let normalized_input = if matches!(
        step,
        LirExpr::Ident(p) if p == "Meta.StepsOf" || p == "Meta.steps_of"
    ) {
        if let Value::Function(func_arc) = &input {
            if let FunctionBody::LirDefined {
                stmts, raw_body, ..
            } = &func_arc.body
            {
                Value::Stream {
                    stmts: stmts.clone(),
                    raw: raw_body.clone(),
                }
            } else {
                input
            }
        } else {
            input
        }
    } else {
        input
    };

    let target = if matches!(
        step,
        LirExpr::Ident(p) if p == "Meta.StepsOf" || p == "Meta.steps_of"
    ) {
        eval_path("Meta.steps_of", scope)?
    } else {
        let mut t_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
        t_scope.set("_", normalized_input.clone());
        if let Value::Form(f_arc) = &normalized_input {
            t_scope.unpack_form(f_arc, true);
        }
        eval_lir_expr(step, &mut t_scope)?
    };

    if matches!(
        step,
        LirExpr::Ident(p) if p == "Meta.StepsOf" || p == "Meta.steps_of"
    ) {
        if let Value::Function(ref func) = target {
            if let FunctionBody::Native(f) = &func.body {
                return f(vec![normalized_input], scope);
            }
        }
    }

    apply_callable(target, normalized_input, scope)
}

pub fn eval_lir_call(
    fn_name: &str,
    call_args: &[LirExpr],
    piped_first: Option<Value>,
    scope: &mut Scope,
) -> Result<Value, String> {
    if fn_name == "Compose" || fn_name == "Meta.compose" {
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
                            "swap" | "Swap" => {
                                let current_val = snap
                                    .get_field("value")
                                    .or_else(|| snap.get_field("Value"))
                                    .unwrap_or(Value::None);
                                let next_val = apply_callable(arg, current_val, scope)?;
                                let mut fw = f_arc.write().unwrap();
                                if fw.fields.contains_key("value") {
                                    fw.set_field("value", next_val.clone());
                                } else {
                                    fw.set_field("Value", next_val.clone());
                                }
                                return Ok(next_val);
                            }
                            "read" | "Read" => {
                                return Ok(snap
                                    .get_field("value")
                                    .or_else(|| snap.get_field("Value"))
                                    .unwrap_or(Value::None));
                            }
                            _ => {}
                        }
                    }
                }
                Value::Atom(atom_arc) => match method_name {
                    "swap" | "Swap" => {
                        let current_val = atom_arc.lock().unwrap().clone();
                        let next_val = apply_callable(arg, current_val, scope)?;
                        *atom_arc.lock().unwrap() = next_val.clone();
                        return Ok(next_val);
                    }
                    "read" | "Read" | "get" => {
                        return Ok(atom_arc.lock().unwrap().clone());
                    }
                    "set" => {
                        *atom_arc.lock().unwrap() = arg.clone();
                        return Ok(arg);
                    }
                    _ => {}
                },
                Value::Channel(chan_arc) => match method_name {
                    "send" | "Send" => {
                        chan_arc.lock().unwrap().push_back(arg);
                        return Ok(Value::None);
                    }
                    "recv" | "Recv" => {
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
            node.set_field("args", arg.clone());
            node.set_field("Args", arg);
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
    if let Some(m) = form.methods.get(method_name) {
        return Some(m.clone());
    }
    for anc in form.ancestors.values() {
        if let Some(m) = find_method_in_form_or_ancestors(anc, method_name) {
            return Some(m);
        }
    }
    None
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
        if let Some(bound_var) = &branch.bound_type_var {
            b_scope.set(bound_var, arg.clone());
        }

        let matched = match &branch.cond {
            LirPattern::Wildcard => true,
            LirPattern::TypeMatch(type_name) => {
                match_type_or_wildcard(type_name, &arg, &mut b_scope)
            }
            LirPattern::LiteralMatch(lit_expr) => arg == eval_lir_expr(lit_expr, &mut b_scope)?,
            LirPattern::RangeMatch(a, b) => {
                let r = eval_lir_expr(&LirExpr::Range(a.clone(), b.clone()), &mut b_scope)?;
                matches!(
                    apply_callable(r, arg.clone(), &mut b_scope),
                    Ok(Value::Bool(true))
                )
            }
            LirPattern::Predicate(pred_expr) => {
                matches!(eval_lir_expr(pred_expr, &mut b_scope)?, Value::Bool(true))
            }
            LirPattern::TupleMatch(elems) => match_lir_tuple_pattern(elems, &arg, &mut b_scope)?,
            LirPattern::FieldMatch(field_pats) => {
                if let Value::Form(f_arc) = &arg {
                    let f = f_arc.read().unwrap();
                    let mut ok = true;
                    for (key, pat_val) in field_pats {
                        let Some(actual) = f.get_field(key) else {
                            ok = false;
                            break;
                        };
                        match pat_val {
                            LirFieldPatValue::Expr(ex) => {
                                if actual != eval_lir_expr(ex, &mut b_scope)? {
                                    ok = false;
                                    break;
                                }
                            }
                            LirFieldPatValue::Bind(var) => {
                                b_scope.set(var, actual);
                            }
                            LirFieldPatValue::Tuple(t_elems) => {
                                if !match_lir_tuple_pattern(t_elems, &actual, &mut b_scope)? {
                                    ok = false;
                                    break;
                                }
                            }
                        }
                    }
                    ok
                } else {
                    false
                }
            }
        };

        if matched {
            return eval_lir_stmts(&branch.body, Some(arg), &mut b_scope);
        }
    }
    if is_bypass_value(&arg) {
        return Ok(arg);
    }
    Ok(Value::None)
}

fn match_lir_tuple_pattern(
    elems: &[LirTuplePatElem],
    actual: &Value,
    scope: &mut Scope,
) -> Result<bool, String> {
    let items = match actual {
        Value::Tuple(v) => v.clone(),
        other => vec![other.clone()],
    };
    if elems.len() != items.len() {
        return Ok(false);
    }
    for (pat, act_val) in elems.iter().zip(items.iter()) {
        match pat {
            LirTuplePatElem::Wildcard => {}
            LirTuplePatElem::TypeBind {
                type_name,
                var_name,
            } => {
                if !match_type_or_wildcard(type_name, act_val, scope) {
                    return Ok(false);
                }
                scope.set(var_name, act_val.clone());
            }
            LirTuplePatElem::Bind(v) => scope.set(v, act_val.clone()),
            LirTuplePatElem::Exact(ex) => {
                if act_val != &eval_lir_expr(ex, scope)? {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

// =============================================================================
// 4. РАЗРЕШЕНИЕ ПУТЕЙ, МЕТОДОВ И РЕАКТИВНЫХ ПОЛЕЙ (`self` & `Self`)
// =============================================================================

pub fn eval_path(path: &str, scope: &mut Scope) -> Result<Value, String> {
    if path == "_" {
        return scope
            .get("_")
            .ok_or_else(|| "Контекст `_` не определен".into());
    }
    if path.starts_with('_') && path.len() == 2 && path.chars().nth(1).unwrap().is_ascii_digit() {
        return scope
            .get(path)
            .or_else(|| scope.get("_"))
            .ok_or_else(|| format!("Контекст `{path}` не определен"));
    }

    let segments: Vec<&str> = path.split('.').collect();
    let (root_name, root_call) = segments[0]
        .strip_suffix("()")
        .map(|s| (s, true))
        .unwrap_or((segments[0], false));

    let mut current = if let Some(val) = scope.get(root_name).or_else(|| {
        if root_name == "self" {
            scope.get("Self")
        } else if root_name == "Self" {
            scope.get("self")
        } else if root_name.starts_with('_') && root_name.len() == 2 {
            scope.get("_")
        } else {
            None
        }
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
        let (seg, is_call) = seg_raw
            .strip_suffix("()")
            .map(|s| (s, true))
            .unwrap_or((*seg_raw, false));

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
                    } else if seg == "Read" || seg == "read" || seg == "get" {
                        snap.get_field("value")
                            .or_else(|| snap.get_field("Value"))
                            .unwrap_or(Value::None)
                    } else {
                        return Err(format!("Метод '{seg}()' не найден в {}", snap.type_name));
                    }
                } else if let Some(val) = snap.get_field(seg) {
                    if let Value::Stream { stmts, raw } = val {
                        eval_reactive_field(&snap, form_arc.clone(), &stmts, &raw, scope)?
                    } else {
                        val
                    }
                } else if let Some(m) = find_method_in_form_or_ancestors(&snap, seg) {
                    Value::Function(m)
                } else if let Ok(idx) = seg.parse::<i32>() {
                    let i = if idx < 0 {
                        (snap.field_order.len() as i32 + idx) as usize
                    } else {
                        idx as usize
                    };
                    let fname = snap
                        .field_order
                        .get(i)
                        .ok_or("Индекс поля вне диапазона")?;
                    let v = snap.fields.get(fname).unwrap().clone();
                    if let Value::Stream { stmts, raw } = v {
                        eval_reactive_field(&snap, form_arc.clone(), &stmts, &raw, scope)?
                    } else {
                        v
                    }
                } else if let Some(anc) = find_ancestor_recursive(&snap, seg) {
                    Value::Form(Arc::new(RwLock::new(anc)))
                } else {
                    return Err(format!(
                        "Поле или предок '{seg}' не найден в {}",
                        snap.type_name
                    ));
                }
            }
            Value::Atom(a) if matches!(seg, "Read" | "read" | "Value" | "value" | "get") => {
                a.lock().unwrap().clone()
            }
            Value::Channel(q) if (seg == "Recv" || seg == "recv") && is_call => {
                match q.lock().unwrap().pop_front() {
                    Some(v) => make_some(v),
                    None => make_empty_form("None"),
                }
            }
            Value::Tuple(items) => {
                if seg == "Length" || seg == "length" {
                    Value::Int(items.len() as i64)
                } else {
                    let idx: i32 = seg
                        .parse()
                        .map_err(|_| format!("Ожидался индекс кортежа, получено '{seg}'"))?;
                    let i = if idx < 0 {
                        (items.len() as i32 + idx) as usize
                    } else {
                        idx as usize
                    };
                    items
                        .get(i)
                        .cloned()
                        .ok_or("Индекс вне границ кортежа")?
                }
            }
            Value::String(s) if seg == "Length" || seg == "length" => {
                Value::Int(s.chars().count() as i64)
            }
            Value::Int(i) if seg == "to_float" => Value::Float(*i as f64),
            Value::Float(f) if seg == "round_to_int" => Value::Int(f.round() as i64),
            _ => return Err(format!("Нельзя получить '{seg}' из {current:?}")),
        };
    }

    Ok(current)
}

fn find_ancestor_recursive(form: &FormInstance, target: &str) -> Option<FormInstance> {
    form.ancestors.get(target).cloned().or_else(|| {
        form.ancestors
            .values()
            .find_map(|a| find_ancestor_recursive(a, target))
    })
}

fn eval_reactive_field(
    snap: &FormInstance,
    form_arc: Arc<RwLock<FormInstance>>,
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
            if let Value::Stream {
                stmts: dep_stmts,
                raw: dep_raw,
            } = fval
            {
                if dep_raw == raw_sents {
                    break;
                }
                if !stream_uses_free_underscore(dep_raw) {
                    r_scope.set(
                        "self",
                        Value::Form(Arc::new(RwLock::new(resolved_self.clone()))),
                    );
                    r_scope.set(
                        "Self",
                        Value::Form(Arc::new(RwLock::new(resolved_self.clone()))),
                    );
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
    let _ = form_arc;
    Ok(unwrap_done(eval_lir_stmts(stmts, None, &mut r_scope)?))
}

pub fn stream_uses_free_underscore(sents: &[Sentence]) -> bool {
    sents.iter().any(|s| {
        s.iter()
            .any(|tok| matches!(tok, Token::Path(p) if p == "_" || p.starts_with("_.")))
    })
}

pub fn invoke_form_method(
    snap: &FormInstance,
    method: &Arc<FunctionInstance>,
    arg: Value,
    scope: &mut Scope,
) -> Result<Value, String> {
    match &method.body {
        FunctionBody::Native(f) => {
            let args_vec = match arg {
                Value::None => vec![],
                other => vec![other],
            };
            f(args_vec, scope)
        }
        FunctionBody::LirDefined {
            args,
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
            call_scope.stream_depth = 0;

            let self_arc = Arc::new(RwLock::new(snap.clone()));
            call_scope.set("self", Value::Form(self_arc.clone()));
            call_scope.set("Self", Value::Form(self_arc.clone()));
            call_scope.unpack_form(&self_arc, true);
            for (k, v) in overrides {
                call_scope.set(k, v.clone());
            }
            let runtime_args = filter_runtime_arg_defs(args, &call_scope);
            bind_arguments(&runtime_args, &arg, &mut call_scope)?;
            Ok(unwrap_done(eval_lir_stmts(
                stmts,
                Some(arg),
                &mut call_scope,
            )?))
        }
        FunctionBody::Composed(f1, f2) => {
            let mid = apply_callable(f1.clone(), arg, scope)?;
            apply_callable(f2.clone(), mid, scope)
        }
    }
}

// =============================================================================
// 5. АРИФМЕТИКА, ВАЛЮТА `Money` И АЛГЕБРА ТИПОВ (&, |, /)
// =============================================================================

fn parse_money_str(raw: &str) -> Option<(String, f64)> {
    if let Some(idx) = raw.find('$') {
        return Some((raw[..=idx].to_string(), raw[idx + 1..].parse().ok()?));
    }
    Some(("$".to_string(), raw.parse().ok()?))
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

    // 1. Вызов перегруженного оператора внутри формы (`Vec2.+`, `Money.*`)
    if let Value::Form(ref f_arc) = left {
        let snap = f_arc.read().unwrap().clone();
        if let Some(m) = find_method_in_form_or_ancestors(&snap, op) {
            return invoke_form_method(&snap, &m, right, scope);
        }
        if snap.type_name == "Money" && matches!(op, "+" | "-" | "*" | "/") {
            if let Some(Value::Int(s1)) = snap.get_field("scaled") {
                match (op, &right) {
                    ("+" | "-", Value::Form(r_arc)) => {
                        if let Some(Value::Int(s2)) = r_arc.read().unwrap().get_field("scaled") {
                            let res = if op == "+" { s1 + s2 } else { s1 - s2 };
                            let mut m = snap.clone();
                            m.set_field("scaled", Value::Int(res));
                            return Ok(Value::Form(Arc::new(RwLock::new(m))));
                        }
                    }
                    ("*", Value::Float(f)) => {
                        let res = ((s1 as f64) * f).round() as i64;
                        let mut m = snap.clone();
                        m.set_field("scaled", Value::Int(res));
                        return Ok(Value::Form(Arc::new(RwLock::new(m))));
                    }
                    ("*", Value::Int(i)) => {
                        let res = s1 * i;
                        let mut m = snap.clone();
                        m.set_field("scaled", Value::Int(res));
                        return Ok(Value::Form(Arc::new(RwLock::new(m))));
                    }
                    ("/", Value::Float(f)) => {
                        let res = ((s1 as f64) / f).round() as i64;
                        let mut m = snap.clone();
                        m.set_field("scaled", Value::Int(res));
                        return Ok(Value::Form(Arc::new(RwLock::new(m))));
                    }
                    _ => {}
                }
            }
        }
    }

    // 2. Раздел 3 / 5 / 9: `x == Pattern` — инфиксный сахар для `x . matches(Pattern)`
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
            "/" => (b != 0)
                .then(|| Value::Int(a / b))
                .ok_or_else(|| "Деление на ноль".into()),
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

        (Value::Money(m), "*", Value::Int(q)) | (Value::Int(q), "*", Value::Money(m)) => {
            let (p, v) = parse_money_str(&m).ok_or("Некорректный Money")?;
            Ok(Value::Money(format!("{p}{:.2}", v * q as f64)))
        }
        (Value::Money(m), "*", Value::Float(q)) | (Value::Float(q), "*", Value::Money(m)) => {
            let (p, v) = parse_money_str(&m).ok_or("Некорректный Money")?;
            Ok(Value::Money(format!("{p}{:.2}", v * q)))
        }
        (Value::Money(m), "*", Value::Percent(pct))
        | (Value::Percent(pct), "*", Value::Money(m)) => {
            let (p, v) = parse_money_str(&m).ok_or("Некорректный Money")?;
            Ok(Value::Money(format!("{p}{:.2}", v * (pct / 100.0))))
        }
        (Value::Money(m1), op @ ("+" | "-"), Value::Money(m2)) => {
            let (p, v1) = parse_money_str(&m1).ok_or("Некорректный Money")?;
            let (_, v2) = parse_money_str(&m2).ok_or("Некорректный Money")?;
            let res = if op == "+" { v1 + v2 } else { v1 - v2 };
            Ok(Value::Money(format!("{p}{res:.2}")))
        }

        (Value::Bool(a), "&&", Value::Bool(b)) => Ok(Value::Bool(a && b)),
        (Value::Bool(a), "||", Value::Bool(b)) => Ok(Value::Bool(a || b)),
        (Value::String(a), "+", Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),

        (Value::Form(f1), "&", Value::Form(f2)) => {
            let (r1, r2) = (f1.read().unwrap(), f2.read().unwrap());
            if r1.type_name == "Compile" {
                return Ok(Value::Form(f2.clone()));
            }
            if r2.type_name == "Public" || r2.type_name == "Private" || r2.type_name == "Compile" {
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
            merged.value_params = r1.value_params.clone();
            merged.raw_body_sentences = r1.raw_body_sentences.clone();
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
                let orig = r1
                    .field_origins
                    .get(k)
                    .cloned()
                    .unwrap_or_else(|| r1.type_name.clone());
                merged.set_field_with_origin(k, r1.fields[k].clone(), &orig);
            }
            for k in &r2.field_order {
                let rhs_orig = r2
                    .field_origins
                    .get(k)
                    .cloned()
                    .unwrap_or_else(|| r2.type_name.clone());
                let rhs_val = r2.fields[k].clone();
                if is_override {
                    merged.set_field_with_origin(k, rhs_val, &r1.type_name);
                } else if let Some(lhs_orig) = merged.field_origins.get(k).cloned() {
                    if lhs_orig != rhs_orig {
                        merged
                            .ambiguous_fields
                            .entry(k.clone())
                            .or_insert_with(|| vec![lhs_orig])
                            .push(rhs_orig);
                    }
                    merged.fields.insert(k.clone(), rhs_val);
                } else {
                    merged.set_field_with_origin(k, rhs_val, &rhs_orig);
                }
            }
            Ok(Value::Form(Arc::new(RwLock::new(merged))))
        }

        // `Compile & [ ... ]` — выполняет поток во время компиляции/загрузки модуля!
        (Value::Form(f1), "&", Value::Stream { stmts, raw })
        if f1.read().unwrap().type_name == "Compile" =>
            {
                if let Some([Token::Group('{', body), Token::Op(op), Token::Path(macro_name)]) =
                    raw.first().map(Vec::as_slice)
                {
                    if op == "." {
                        let mut form = FormInstance::new("AnonymousForm");
                        populate_form_members(&mut form, body, scope)?;
                        let form_val = Value::Form(Arc::new(RwLock::new(form)));
                        let macro_fn = eval_path(macro_name, scope)?;
                        return apply_callable(macro_fn, form_val, scope);
                    }
                }
                let res = eval_lir_stmts(&stmts, None, scope)?;
                return Ok(unwrap_done(res));
            }

        (Value::Form(f1), "&", pred @ (Value::Function(_) | Value::Stream { .. })) => {
            let mut refined = f1.read().unwrap().clone();
            refined
                .ancestors
                .insert(refined.type_name.clone(), f1.read().unwrap().clone());
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
                args,
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
                        args: args.clone(),
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

        (
            f1 @ (Value::Function(_) | Value::Stream { .. }),
            "&",
            f2 @ (Value::Function(_) | Value::Stream { .. }),
        ) => Ok(Value::Function(Arc::new(FunctionInstance {
            name: "ComposedStream".into(),
            body: FunctionBody::Composed(f1, f2),
        }))),

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
            Ok(Value::Form(Arc::new(RwLock::new(
                subtract_ancestor_or_path(&f.read().unwrap(), &rhs_name),
            ))))
        }

        (a, op, b) => Err(format!(
            "Оператор '{op}' не поддерживается для {a:?} и {b:?}"
        )),
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
                form.field_origins
                    .insert(field_name.to_string(), winner.clone());
                if let Some(val) = form
                    .ancestors
                    .get(&winner)
                    .and_then(|a| a.get_field(field_name))
                {
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
// 6. APPLY CALLABLE & LAW 3 (OUTCOME BYPASS)
// =============================================================================

fn callable_accepts_input(callable: &Value, input: &Value, scope: &mut Scope) -> bool {
    let Value::Function(func_arc) = callable else {
        return true;
    };
    let FunctionBody::LirDefined {
        args, partial_args, ..
    } = &func_arc.body
    else {
        return true;
    };
    let runtime_args = filter_runtime_arg_defs(args, scope);
    let Some(first_param) = runtime_args.get(partial_args.len()) else {
        return true;
    };

    let type_tokens: &[Token] = match first_param.as_slice() {
        [Token::Path(single)] => {
            if matches!(scope.get(single), Some(Value::Form(_))) {
                first_param.as_slice()
            } else {
                &[]
            }
        }
        [Token::Path(_), Token::Op(op), ..] if matches!(op.as_str(), "&" | "|" | "/") => {
            first_param.as_slice()
        }
        [Token::Path(_), rest @ ..] => {
            if matches!(rest.first(), Some(Token::Lit(_) | Token::Group(_, _))) {
                &[]
            } else {
                rest
            }
        }
        _ => &[],
    };

    type_tokens.is_empty() || value_matches_type_tokens(input, type_tokens, scope)
}

fn value_matches_type_tokens(val: &Value, tokens: &[Token], scope: &mut Scope) -> bool {
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Path(t_name)
            if !matches!(
                    t_name.as_str(),
                    "String"
                        | "Symbol"
                        | "Int"
                        | "Float"
                        | "Bool"
                        | "Byte"
                        | "Any"
                        | "Tuple"
                        | "File"
                        | "Path"
                        | "Url"
                        | "List"
                ) =>
                {
                    if let Value::Form(val_arc) = val {
                        let vf = val_arc.read().unwrap();
                        if !vf.has_lineage(t_name) {
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
                    }
                }
            Token::Op(op) if op == "/" => {
                if let (Some(Token::Path(forb)), Value::Form(val_arc)) = (tokens.get(i + 1), val) {
                    if val_arc.read().unwrap().has_lineage(forb) {
                        return false;
                    }
                }
                i += 2;
                continue;
            }
            _ => {}
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
            FunctionBody::Native(f) => {
                let args_vec = match arg {
                    Value::None => vec![],
                    other => vec![other],
                };
                f(args_vec, scope)
            }
            FunctionBody::LirDefined {
                args,
                stmts,
                raw_body,
                captured_env,
                overrides,
                partial_args,
            } => {
                let runtime_args = filter_runtime_arg_defs(args, scope);
                let mut combined = partial_args.clone();
                match &arg {
                    Value::Tuple(items) if runtime_args.len() > 1 => {
                        combined.extend(items.clone())
                    }
                    Value::None => {}
                    other => combined.push(other.clone()),
                }

                // Partial Application (`partial_conn connect("db.internal")`, `add_ten add_two(10)`)
                if !combined.is_empty()
                    && combined.len() < runtime_args.len()
                    && !matches!(arg, Value::None)
                {
                    let has_missing_required =
                        runtime_args.iter().skip(combined.len()).any(|s| {
                            matches!(s.as_slice(), [Token::Path(_)])
                                || (s.len() == 2 && matches!(s[1], Token::Path(_)))
                        });
                    if has_missing_required {
                        return Ok(Value::Function(Arc::new(FunctionInstance {
                            name: format!("{}_partial", func.name),
                            body: FunctionBody::LirDefined {
                                args: args.clone(),
                                stmts: stmts.clone(),
                                raw_body: raw_body.clone(),
                                captured_env: captured_env.clone(),
                                overrides: overrides.clone(),
                                partial_args: combined,
                            },
                        })));
                    }
                }

                // Объединяем глобальное окружение с захваченным, чтобы рекурсивные функции и типы были доступны
                let mut merged_env = scope.clone();
                for (k, v) in &captured_env.read().unwrap().bindings {
                    merged_env.set(k, v.clone());
                }
                let mut call_scope = Scope::child(Arc::new(RwLock::new(merged_env)));
                call_scope.stream_depth = if func.name == "Lambda" {
                    scope.stream_depth + 1
                } else {
                    0
                };

                let packed_arg = if combined.is_empty() {
                    Value::None
                } else if combined.len() == 1 {
                    combined.remove(0)
                } else {
                    Value::Tuple(combined)
                };
                if call_scope.stream_depth == 0 {
                    call_scope.set("_0", packed_arg.clone());
                }
                bind_arguments(&runtime_args, &packed_arg, &mut call_scope)?;
                for (k, v) in overrides {
                    call_scope.set(k, v.clone());
                }
                let res = eval_lir_stmts(stmts, Some(packed_arg), &mut call_scope)?;
                if func.name == "Lambda" {
                    Ok(res)
                } else {
                    Ok(unwrap_done(res))
                }
            }
            FunctionBody::Composed(f1, f2) => {
                let mid = apply_callable(f1.clone(), arg, scope)?;
                apply_callable(f2.clone(), mid, scope)
            }
        },
        Value::Stream { stmts, .. } => {
            let mut s_scope =
                Scope::child_stream(Arc::new(RwLock::new(scope.clone())), arg.clone());
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
            if !router.value_params.is_empty() || is_pascal_case(&router.type_name) {
                if router.union_variants.is_empty() {
                    return instantiate_form_constructor(&router, arg, scope);
                }
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
                    let mut b_scope =
                        Scope::child_stream(Arc::new(RwLock::new(scope.clone())), arg.clone());
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

/// Вызов конструктора формы `Money("100.00")`, `MathOps(10, 2)`, `GreetingCard("Bob")`:
/// привязывает параметры конструктора к `self` и пересчитывает eager-поля `(raw . Decimal.parse_scale4)`!
fn instantiate_form_constructor(
    proto: &FormInstance,
    arg: Value,
    scope: &mut Scope,
) -> Result<Value, String> {
    let mut inst = proto.clone();
    inst.ancestors.insert(proto.type_name.clone(), proto.clone());

    let mut init_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
    let runtime_params = filter_runtime_arg_defs(&proto.value_params, scope);
    bind_arguments(&runtime_params, &arg, &mut init_scope)?;

    for param_sent in &runtime_params {
        if let Some(Token::Path(p)) = param_sent.first() {
            let var_name = if is_pascal_case(p) {
                pascal_to_snake_case(p)
            } else {
                p.clone()
            };
            if let Some(v) = init_scope.get(&var_name) {
                inst.set_field(&var_name, v);
            }
        }
    }

    // Пересчитываем eager-поля тела формы с новыми значениями параметров (`scaled (raw . Decimal.parse_scale4)`)
    if !inst.raw_body_sentences.is_empty() {
        let body_sents = inst.raw_body_sentences.clone();
        populate_form_members(&mut inst, &body_sents, &mut init_scope)?;
    }

    let inst_arc = Arc::new(RwLock::new(inst));
    init_scope.set("self", Value::Form(inst_arc.clone()));
    init_scope.set("Self", Value::Form(inst_arc.clone()));
    Ok(Value::Form(inst_arc))
}

fn filter_runtime_arg_defs(args: &[Sentence], scope: &Scope) -> Vec<Sentence> {
    let mut out = Vec::new();
    for (idx, sent) in args.iter().enumerate() {
        match sent.as_slice() {
            [Token::Path(tp), Token::Group(')', _), ..] if is_pascal_case(tp) => continue,
            [Token::Path(tp), Token::Path(bound)]
            if is_pascal_case(tp) && is_pascal_case(bound) =>
                {
                    continue;
                }
            [Token::Path(single)] if is_pascal_case(single) => {
                // Если `single` — это `A`, `B` в `list_map(A, B, l, action)` или `fmap(A, B, fa Option(A), ...)`,
                // то есть одиночная заглавная буква (или не существующий в скоупе тип, за которым идут еще параметры),
                // это чистый compile-time параметр типа!
                let is_known_form = matches!(scope.get(single), Some(Value::Form(_)));
                let has_more_params = idx + 1 < args.len();
                let used_later = args.iter().skip(idx + 1).any(|later| {
                    later
                        .iter()
                        .skip(1)
                        .any(|t| token_contains_ident(t, single))
                });
                if used_later || (!is_known_form && has_more_params) {
                    continue;
                }
                out.push(sent.clone());
            }
            _ => out.push(sent.clone()),
        }
    }
    out
}

fn token_contains_ident(tok: &Token, ident: &str) -> bool {
    match tok {
        Token::Path(p) => p == ident,
        Token::Group(_, sents) => sents
            .iter()
            .any(|s| s.iter().any(|t| token_contains_ident(t, ident))),
        _ => false,
    }
}

fn match_type_or_wildcard(pattern_name: &str, val: &Value, scope: &mut Scope) -> bool {
    if pattern_name == "_" {
        scope.set("_", val.clone());
        return true;
    }
    if pattern_name == "None" {
        if matches!(val, Value::None) {
            return true;
        }
        if let Value::Form(f_arc) = val {
            if f_arc.read().unwrap().type_name == "None" {
                return true;
            }
        }
        return false;
    }
    if pattern_name == "Form" && matches!(val, Value::Form(_)) {
        return true;
    }
    if pattern_name == "Tuple" && matches!(val, Value::Tuple(_)) {
        return true;
    }
    if pattern_name == "Stream" && matches!(val, Value::Stream { .. } | Value::Function(_)) {
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
                        if !matches!(
                            apply_callable(pred.clone(), val.clone(), &mut p_scope),
                            Ok(Value::Bool(true))
                        ) {
                            return false;
                        }
                    }
                }
            }

            if matched {
                scope.set("_", val.clone());
                scope.set(&pascal_to_snake_case(pattern_name), val.clone());
                scope.unpack_form(form_arc, true);
                return true;
            }
            false
        }
        Value::Tuple(items) => {
            scope.set("_", Value::Tuple(items.clone()));
            true
        }
        _ => false,
    }
}

fn bind_arguments(
    arg_defs: &[Sentence],
    incoming: &Value,
    scope: &mut Scope,
) -> Result<(), String> {
    scope.set("_", incoming.clone());
    if arg_defs.is_empty() {
        if let Value::Form(f_arc) = incoming {
            scope.unpack_form(f_arc, true);
        }
        return Ok(());
    }

    let items: Vec<Value> = match incoming {
        Value::Tuple(v) if arg_defs.len() > 1 => v.clone(),
        Value::None => vec![],
        other => vec![other.clone()],
    };

    let dummy_hir = HirModule::default();
    for (idx, param_sent) in arg_defs.iter().enumerate() {
        let provided = items.get(idx).cloned();
        match param_sent.as_slice() {
            // Закон 1d: Деструктуризация полей `{ name } WithName`
            [Token::Group('{', field_sents), ..] => {
                if let Some(Value::Form(f_arc)) = &provided {
                    let f = f_arc.read().unwrap();
                    for fs in field_sents {
                        if let [Token::Path(fname)] = fs.as_slice() {
                            if let Some(fv) = f.get_field(fname) {
                                scope.set(fname, fv);
                            }
                        }
                    }
                }
            }
            // Закон 1d: Деструктуризация кортежа в параметре `(idx Int, (item Item, disc Int))`
            [Token::Group('(', tuple_sents), ..] => {
                if let Some(sub_val) = &provided {
                    bind_arguments(tuple_sents, sub_val, scope)?;
                }
            }
            // Закон 1c: Одиночный идентификатор `fn(Age)` -> связывает `age` (и `Age`), а если это форма — распаковывает поля
            [Token::Path(single)] => {
                let val = provided.unwrap_or(Value::None);
                let is_upper = is_pascal_case(single);
                scope.set(single, val.clone());
                if is_upper {
                    scope.set(&pascal_to_snake_case(single), val.clone());
                    if let Value::Form(f_arc) = &val {
                        scope.unpack_form(f_arc, true);
                    }
                }
            }
            // Анонимный пересеченный слот `FinalizeBill(User & Cart)`
            [Token::Path(first_type), Token::Op(op), ..]
            if is_pascal_case(first_type) && matches!(op.as_str(), "&" | "|" | "/") =>
                {
                    if let Some(val) = &provided {
                        scope.set(&pascal_to_snake_case(first_type), val.clone());
                        if let Value::Form(f_arc) = val {
                            scope.unpack_form(f_arc, true);
                        }
                    }
                }
            // Именованный параметр `fn(u User)`, `connect(host String "localhost")`, `connect(Host "localhost")`
            [Token::Path(param_name), rest @ ..] => {
                if let Some(val) = provided {
                    scope.set(param_name, val.clone());
                    if let Value::Form(f_arc) = &val {
                        scope.unpack_form(f_arc, true);
                    }
                } else {
                    let def_slice = extract_default_slice(rest);
                    if !def_slice.is_empty() {
                        let lir_def = lower_tokens_to_expr(def_slice, &dummy_hir);
                        let def_val = eval_lir_expr(&lir_def, scope)?;
                        scope.set(param_name, def_val);
                    } else {
                        scope.set(param_name, Value::None);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn extract_default_slice(rest: &[Token]) -> &[Token] {
    match rest {
        [Token::Path(ty), def @ ..]
        if is_pascal_case(ty)
            && !def.is_empty()
            && !matches!(
                    def.first(),
                    Some(Token::Op(_) | Token::Group(')', _) | Token::Group('[', _))
                ) =>
            {
                def
            }
        [Token::Lit(_)] | [Token::Group(_, _)] => rest,
        _ => &[],
    }
}

// =============================================================================
// 7. ОБРАБОТКА ПРЕДЛОЖЕНИЙ (ЧЕРЕЗ ПОНИЖЕНИЕ В LIR) И ЗАПУСК LIR-МОДУЛЯ
// =============================================================================

pub fn populate_form_members(
    form: &mut FormInstance,
    sentences: &[Sentence],
    scope: &mut Scope,
) -> Result<(), String> {
    let dummy_hir = HirModule::default();
    for s in sentences {
        match classify_sentence(s) {
            SemanticKind::FormDef { name, body, tail } => {
                let mut sub = FormInstance::new(&name);
                sub.raw_body_sentences = body.clone();
                populate_form_members(&mut sub, &body, scope)?;
                let mut val = Value::Form(Arc::new(RwLock::new(sub)));
                if !tail.is_empty() {
                    val = eval_operator_tail(val, &tail, scope)?;
                }
                form.set_field(&name, val);
            }
            SemanticKind::Binding { key, value } => {
                let (clean_val, _) = strip_runtime_tags(&value);
                let val_slice = match clean_val.as_slice() {
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
                    [Token::Path(ty), Token::Group(')', _), def @ ..]
                    if is_pascal_case(ty)
                        && !def.is_empty()
                        && !matches!(def.first(), Some(Token::Op(_))) =>
                        {
                            def
                        }
                    other => other,
                };
                let v = match val_slice {
                    [Token::Path(p)]
                    if matches!(
                            p.as_str(),
                            "String"
                                | "Symbol"
                                | "Int"
                                | "Float"
                                | "Bool"
                                | "Byte"
                                | "Money"
                                | "File"
                                | "Path"
                                | "Any"
                        ) || (is_pascal_case(p)
                        && matches!(scope.get(p), Some(Value::Symbol(_)))) =>
                        {
                            Value::Symbol(p.clone())
                        }
                    _ => {
                        let lir_e = lower_tokens_to_expr(val_slice, &dummy_hir);
                        eval_lir_expr(&lir_e, scope).unwrap_or(Value::Symbol(key.clone()))
                    }
                };
                scope.set(&key, v.clone());
                form.set_field(&key, v);
            }
            SemanticKind::TupleBinding { tuple, value } => {
                if let Some(first_sent) = tuple.first() {
                    let k = eval_sentence(first_sent, scope)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|_| "()".into());
                    let lir_e = lower_tokens_to_expr(&value, &dummy_hir);
                    let v = eval_lir_expr(&lir_e, scope).unwrap_or(Value::None);
                    form.set_field(&k, v);
                }
            }
            SemanticKind::StreamOrBranch { name, stream } => {
                form.set_field(
                    &name,
                    Value::Stream {
                        stmts: lower_body_to_stmts(&stream, &dummy_hir),
                        raw: stream,
                    },
                );
            }
            SemanticKind::FunctionDef {
                name, args, body, ..
            } => {
                let env_arc = Arc::new(RwLock::new(scope.clone()));
                let func = Arc::new(FunctionInstance {
                    name: name.clone(),
                    body: FunctionBody::LirDefined {
                        args,
                        stmts: lower_body_to_stmts(&body, &dummy_hir),
                        raw_body: body,
                        captured_env: env_arc,
                        overrides: HashMap::new(),
                        partial_args: Vec::new(),
                    },
                });
                form.methods.insert(name, func);
            }
            SemanticKind::OperatorMethod { op, args, rest } => {
                let body = match rest.first() {
                    Some(Token::Group('[', b)) => b.clone(),
                    _ => vec![rest],
                };
                let func = Arc::new(FunctionInstance {
                    name: op.clone(),
                    body: FunctionBody::LirDefined {
                        args,
                        stmts: lower_body_to_stmts(&body, &dummy_hir),
                        raw_body: body,
                        captured_env: Arc::new(RwLock::new(scope.clone())),
                        overrides: HashMap::new(),
                        partial_args: Vec::new(),
                    },
                });
                form.methods.insert(op, func);
            }
            _ => {}
        }
    }
    Ok(())
}

fn strip_runtime_tags(tokens: &[Token]) -> (Vec<Token>, bool) {
    let mut cleaned = Vec::new();
    let mut is_private = false;
    let mut i = 0;
    while i < tokens.len() {
        if i + 1 < tokens.len() {
            if let (Token::Op(op), Token::Path(tag)) = (&tokens[i], &tokens[i + 1]) {
                if op == "&" && matches!(tag.as_str(), "Private" | "Public" | "Compile") {
                    if tag == "Private" {
                        is_private = true;
                    }
                    i += 2;
                    continue;
                }
            }
        }
        cleaned.push(tokens[i].clone());
        i += 1;
    }
    (cleaned, is_private)
}

pub fn eval_operator_tail(
    mut current: Value,
    tail: &[Token],
    scope: &mut Scope,
) -> Result<Value, String> {
    let dummy_hir = HirModule::default();
    let mut i = 0;
    while i < tail.len() {
        if let Token::Group('{', override_sents) = &tail[i] {
            let mut anon = FormInstance::new("AnonymousForm");
            populate_form_members(&mut anon, override_sents, scope)?;
            current = eval_binary_op_in_scope(
                current,
                "&",
                Value::Form(Arc::new(RwLock::new(anon))),
                scope,
            )?;
            i += 1;
            continue;
        }
        if let Token::Op(op) = &tail[i] {
            let next_op_offset = tail[i + 1..]
                .iter()
                .position(|t| matches!(t, Token::Op(_) | Token::Group('{', _)))
                .map(|pos| i + 1 + pos)
                .unwrap_or(tail.len());
            let rhs_slice = &tail[i + 1..next_op_offset];

            if op == "/" {
                if let (Value::Form(f_arc), [Token::Path(path_str)]) = (&current, rhs_slice) {
                    let new_val = subtract_ancestor_or_path(&f_arc.read().unwrap(), path_str);
                    current = Value::Form(Arc::new(RwLock::new(new_val)));
                    i = next_op_offset;
                    continue;
                }
            }
            if op == "&" {
                if let [Token::Group('{', override_sents)] = rhs_slice {
                    let mut anon = FormInstance::new("AnonymousForm");
                    populate_form_members(&mut anon, override_sents, scope)?;
                    current = eval_binary_op_in_scope(
                        current,
                        "&",
                        Value::Form(Arc::new(RwLock::new(anon))),
                        scope,
                    )?;
                    i = next_op_offset;
                    continue;
                }
            }
            let lir_rhs = lower_tokens_to_expr(rhs_slice, &dummy_hir);
            let rhs = eval_lir_expr(&lir_rhs, scope)?;
            current = eval_binary_op_in_scope(current, op, rhs, scope)?;
            i = next_op_offset;
        } else {
            i += 1;
        }
    }
    Ok(current)
}

pub fn eval_sentence(sentence: &Sentence, scope: &mut Scope) -> Result<Value, String> {
    let dummy_hir = HirModule::default();
    match classify_sentence(sentence) {
        SemanticKind::LambdaOrTupleBranch { args, body } => {
            Ok(Value::Function(Arc::new(FunctionInstance {
                name: "Lambda".into(),
                body: FunctionBody::LirDefined {
                    args,
                    stmts: lower_body_to_stmts(&body, &dummy_hir),
                    raw_body: body,
                    captured_env: Arc::new(RwLock::new(scope.clone())),
                    overrides: HashMap::new(),
                    partial_args: Vec::new(),
                },
            })))
        }
        SemanticKind::StreamOrBranch { name, stream } => {
            // В Разделе 20: `step_1 [ _ + 10 ]` и `sample_stream [ _ + 1, _ * 2, _ - 3 ]` — это именованные потоки `Value::Stream`!
            let val = Value::Stream {
                stmts: lower_body_to_stmts(&stream, &dummy_hir),
                raw: stream,
            };
            scope.set(&name, val.clone());
            Ok(val)
        }
        SemanticKind::Binding { key, value } => {
            let val = match value.as_slice() {
                [Token::Path(p)]
                if matches!(
                        p.as_str(),
                        "String"
                            | "Symbol"
                            | "Int"
                            | "Float"
                            | "Bool"
                            | "Byte"
                            | "Money"
                            | "File"
                            | "Any"
                    ) =>
                    {
                        Value::Symbol(key.clone())
                    }
                [Token::Group('(', inner)] if inner.len() == 1 => {
                    // Обработка `Product (Compile & [ { id Int, title String } . derive_json ])`
                    let inner_sent = &inner[0];
                    if let [Token::Path(c), Token::Op(amp), Token::Group('[', stream_sents)] =
                        inner_sent.as_slice()
                    {
                        if c == "Compile" && amp == "&" {
                            if let Some([Token::Group('{', body), Token::Op(dot), Token::Path(macro_name)]) =
                                stream_sents.first().map(Vec::as_slice)
                            {
                                if dot == "." {
                                    let mut form = FormInstance::new(&key);
                                    populate_form_members(&mut form, body, scope)?;
                                    let form_val = Value::Form(Arc::new(RwLock::new(form)));
                                    let macro_fn = eval_path(macro_name, scope)?;
                                    let transformed = apply_callable(macro_fn, form_val, scope)?;
                                    if let Value::Form(f_arc) = &transformed {
                                        f_arc.write().unwrap().type_name = key.clone();
                                    }
                                    scope.set(&key, transformed.clone());
                                    return Ok(transformed);
                                }
                            }
                        }
                    }
                    let lir_e = lower_tokens_to_expr(&value, &dummy_hir);
                    let v = eval_lir_expr(&lir_e, scope)?;
                    if let Value::Form(f_arc) = &v {
                        if is_pascal_case(&key) {
                            f_arc.write().unwrap().type_name = key.clone();
                        }
                    }
                    v
                }
                _ => {
                    let lir_e = lower_tokens_to_expr(&value, &dummy_hir);
                    let v = eval_lir_expr(&lir_e, scope)?;
                    if let Value::Form(f_arc) = &v {
                        if is_pascal_case(&key) {
                            f_arc.write().unwrap().type_name = key.clone();
                        }
                    }
                    v
                }
            };
            scope.set(&key, val.clone());
            Ok(val)
        }
        SemanticKind::TupleBinding { tuple, value } => {
            if matches!(value.first(), Some(Token::Op(_))) {
                let lir_e = lower_tokens_to_expr(sentence, &dummy_hir);
                return eval_lir_expr(&lir_e, scope);
            }
            let lir_e = lower_tokens_to_expr(&value, &dummy_hir);
            let val = eval_lir_expr(&lir_e, scope)?;
            if let Value::Tuple(items) = &val {
                for (pat_s, act_val) in tuple.iter().zip(items.iter()) {
                    if let [Token::Path(v)] = pat_s.as_slice() {
                        if v != "_" {
                            scope.set(v, act_val.clone());
                        }
                    }
                }
            }
            Ok(val)
        }
        SemanticKind::FormDef { name, body, tail } => {
            let mut form = match scope.get(&name) {
                Some(Value::Form(prev_arc)) => {
                    let prev = prev_arc.read().unwrap().clone();
                    let mut next = prev.clone();
                    next.ancestors.insert(name.clone(), prev);
                    next
                }
                _ => FormInstance::new(&name),
            };
            form.raw_body_sentences = body.clone();
            populate_form_members(&mut form, &body, scope)?;
            let mut val = Value::Form(Arc::new(RwLock::new(form)));
            if !tail.is_empty() {
                val = eval_operator_tail(val, &tail, scope)?;
                if let Value::Form(f_arc) = &val {
                    f_arc.write().unwrap().type_name = name.clone();
                }
            }
            scope.set(&name, val.clone());
            Ok(val)
        }
        SemanticKind::SubtypeFormDef {
            name,
            base,
            body,
            tail,
        } => {
            let mut form = match scope.get(&base) {
                Some(Value::Form(base_arc)) => {
                    let b_inst = base_arc.read().unwrap().clone();
                    let mut child = b_inst.clone();
                    child.type_name = name.clone();
                    child.ancestors.insert(base.clone(), b_inst);
                    child
                }
                _ => FormInstance::new(&name),
            };
            form.raw_body_sentences = body.clone();
            populate_form_members(&mut form, &body, scope)?;
            let mut val = Value::Form(Arc::new(RwLock::new(form)));
            if !tail.is_empty() {
                val = eval_operator_tail(val, &tail, scope)?;
                if let Value::Form(f_arc) = &val {
                    let mut fw = f_arc.write().unwrap();
                    let snap = fw.clone();
                    if snap.type_name == name {
                        fw.ancestors.insert(name.clone(), snap);
                    }
                    fw.type_name = name.clone();
                }
            }
            scope.set(&name, val.clone());
            Ok(val)
        }
        SemanticKind::GenericDef {
            name, params, body, ..
        } => {
            if name == "Map" && scope.get("Map").is_some() {
                return Ok(scope.get("Map").unwrap());
            }
            let mut form = FormInstance::new(&name);
            form.value_params = params.clone();

            for p_sent in &params {
                if let [Token::Path(pname), rest @ ..] = p_sent.as_slice() {
                    if !is_pascal_case(pname) {
                        let def_slice = extract_default_slice(rest);
                        let def_val = if !def_slice.is_empty() {
                            let lir_e = lower_tokens_to_expr(def_slice, &dummy_hir);
                            eval_lir_expr(&lir_e, scope).unwrap_or(Value::None)
                        } else {
                            Value::None
                        };
                        form.set_field(pname, def_val);
                    }
                }
            }

            let mut init_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
            for k in &form.field_order {
                if let Some(v) = form.fields.get(k) {
                    init_scope.set(k, v.clone());
                }
            }

            if let [Token::Group('{', f_body), ..] = body.as_slice() {
                form.raw_body_sentences = f_body.clone();
                populate_form_members(&mut form, f_body, &mut init_scope)?;
            }
            let val = Value::Form(Arc::new(RwLock::new(form)));
            scope.set(&name, val.clone());
            Ok(val)
        }
        SemanticKind::AlgebraicDef { name, expr } => {
            if let Some(Token::Path(first_name)) = expr.first() {
                if expr.iter().any(|t| matches!(t, Token::Group('{', _))) {
                    let start_val = scope
                        .get(first_name)
                        .unwrap_or_else(|| make_empty_form(first_name));
                    let start_idx = if matches!(expr.get(1), Some(Token::Group(')', _))) {
                        2
                    } else {
                        1
                    };
                    let val = eval_operator_tail(start_val, &expr[start_idx..], scope)?;
                    if let Value::Form(f_arc) = &val {
                        let mut fw = f_arc.write().unwrap();
                        let snap = fw.clone();
                        if is_pascal_case(&name) && snap.type_name != name {
                            fw.ancestors.insert(snap.type_name.clone(), snap);
                            fw.type_name = name.clone();
                        }
                    }
                    scope.set(&name, val.clone());
                    return Ok(val);
                }
            }

            if let [Token::Group('(', inner)] = expr.as_slice() {
                if inner.len() == 1 {
                    let inner_sent = &inner[0];
                    if let [Token::Path(c), Token::Op(amp), Token::Group('[', stream_sents)] =
                        inner_sent.as_slice()
                    {
                        if c == "Compile" && amp == "&" {
                            if let Some([Token::Group('{', body), Token::Op(dot), Token::Path(macro_name)]) =
                                stream_sents.first().map(Vec::as_slice)
                            {
                                if dot == "." {
                                    let mut form = FormInstance::new(&name);
                                    populate_form_members(&mut form, body, scope)?;
                                    let form_val = Value::Form(Arc::new(RwLock::new(form)));
                                    let macro_fn = eval_path(macro_name, scope)?;
                                    let transformed = apply_callable(macro_fn, form_val, scope)?;
                                    if let Value::Form(f_arc) = &transformed {
                                        f_arc.write().unwrap().type_name = name.clone();
                                    }
                                    scope.set(&name, transformed.clone());
                                    return Ok(transformed);
                                }
                            }
                        }
                    }
                }
            }

            let lir_e = lower_tokens_to_expr(&expr, &dummy_hir);
            let val = eval_lir_expr(&lir_e, scope)?;
            if let Value::Form(f_arc) = &val {
                let mut fw = f_arc.write().unwrap();
                let snap = fw.clone();
                if is_pascal_case(&name) && snap.type_name != "UnionType" && snap.type_name != name
                {
                    fw.ancestors.insert(snap.type_name.clone(), snap);
                    fw.type_name = name.clone();
                }
            }
            scope.set(&name, val.clone());
            Ok(val)
        }
        SemanticKind::FunctionDef {
            name, args, body, ..
        } => {
            let env_arc = Arc::new(RwLock::new(scope.clone()));
            let val = Value::Function(Arc::new(FunctionInstance {
                name: name.clone(),
                body: FunctionBody::LirDefined {
                    args,
                    stmts: lower_body_to_stmts(&body, &dummy_hir),
                    raw_body: body,
                    captured_env: env_arc.clone(),
                    overrides: HashMap::new(),
                    partial_args: Vec::new(),
                },
            }));
            env_arc.write().unwrap().set(&name, val.clone());
            scope.set(&name, val.clone());
            Ok(val)
        }
        SemanticKind::CallExpr { .. } | SemanticKind::Expression(_) => {
            let lir_e = lower_tokens_to_expr(sentence, &dummy_hir);
            eval_lir_expr(&lir_e, scope)
        }
        SemanticKind::Pipeline(tokens) => {
            if let [Token::Path(form_name), Token::Group('{', body), Token::Op(op), Token::Path(macro_name)] =
                tokens.as_slice()
            {
                if op == "." {
                    let mut form = FormInstance::new(form_name);
                    populate_form_members(&mut form, body, scope)?;
                    let form_val = Value::Form(Arc::new(RwLock::new(form)));
                    let macro_fn = eval_path(macro_name, scope)?;
                    let transformed = apply_callable(macro_fn, form_val, scope)?;
                    if let Value::Form(f_arc) = &transformed {
                        f_arc.write().unwrap().type_name = form_name.clone();
                    }
                    scope.set(form_name, transformed.clone());
                    return Ok(transformed);
                }
            }
            if let [Token::Group('{', body), Token::Op(op), Token::Path(macro_name)] =
                tokens.as_slice()
            {
                if op == "." {
                    let mut form = FormInstance::new("AnonymousForm");
                    populate_form_members(&mut form, body, scope)?;
                    let form_val = Value::Form(Arc::new(RwLock::new(form)));
                    let macro_fn = eval_path(macro_name, scope)?;
                    return apply_callable(macro_fn, form_val, scope);
                }
            }
            let lir_e = lower_tokens_to_expr(&tokens, &dummy_hir);
            eval_lir_expr(&lir_e, scope)
        }
        _ => Ok(Value::None),
    }
}

pub fn run_lir_module(lir: &LirModule) -> Result<Value, String> {
    let mut global_scope = create_global_scope();

    let mut last_val = Value::None;
    for sentence in &lir.raw_sentences {
        last_val = eval_sentence(sentence, &mut global_scope)?;
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