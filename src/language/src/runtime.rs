// src/runtime.rs
// =============================================================================
// RAZOMY V4.7 — CORE RUNTIME ENGINE (SYSTEM KERNEL)
// Потокобезопасное ядро: Value, Scope, Законы 1–3, Lineage, Пайплайны и Роутеры.
// =============================================================================

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use crate::hir::{lower_to_hir, HirModule};
use crate::parser::{
    classify_sentence, has_pipe_op, parse_code, split_pipeline, Literal, PipeOp, PipeStep,
    SemanticKind, Sentence, Token,
};
use crate::runtime_ext::{
    create_global_scope, eval_compose_call, make_empty_form, make_some,
};

// =============================================================================
// 1. ПОТОКОБЕЗОПАСНАЯ СИСТЕМА ЗНАЧЕНИЙ (VALUE)
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
    Stream(Vec<Sentence>),
    Router(Vec<Sentence>),
    Function(Arc<FunctionInstance>),
    Atom(Arc<Mutex<Value>>),
    Channel(Arc<Mutex<VecDeque<Value>>>),
    Done(Box<Value>),
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
    UserDefined {
        args: Vec<Sentence>,
        body: Vec<Sentence>,
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
            Value::Stream(_) => write!(f, "[Stream]"),
            Value::Router(_) => write!(f, "{{Router}}"),
            Value::Function(func) => write!(f, "Fn({})", func.name),
            Value::Atom(a) => write!(f, "Atom({:?})", a.lock().unwrap()),
            Value::Channel(_) => write!(f, "Channel"),
            Value::Done(v) => write!(f, "Done({v:?})"),
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
}

impl Scope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child(parent: Arc<RwLock<Scope>>) -> Self {
        Self {
            parent: Some(parent),
            bindings: HashMap::new(),
        }
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

    /// Единый метод проекции полей формы в скоуп (Закон 1a/1b/1c + предки Закона 2)
    pub fn unpack_form(&mut self, form_arc: &Arc<RwLock<FormInstance>>, eval_reactive: bool) {
        let form = form_arc.read().unwrap().clone();
        for fname in &form.field_order {
            if form.ambiguous_fields.contains_key(fname) {
                continue;
            }
            if let Some(val) = form.fields.get(fname) {
                if eval_reactive {
                    if let Value::Stream(sents) = val {
                        if !stream_uses_free_underscore(sents) {
                            if let Ok(comp) = eval_stream_body(sents, None, self) {
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
            self.set(
                anc_name,
                Value::Form(Arc::new(RwLock::new(anc_inst.clone()))),
            );
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
// 3. ВЫЧИСЛЕНИЕ ТОКЕНОВ, ИНТЕРПОЛЯЦИИ И СЛИТНЫХ ПУТЕЙ
// =============================================================================

pub fn eval_literal(lit: &Literal, scope: &mut Scope) -> Value {
    match lit {
        Literal::Int(i) => Value::Int(*i),
        Literal::Float(f) => Value::Float(*f),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Byte(b) => Value::Byte(*b),
        Literal::String(s) | Literal::Url(s) | Literal::File(s) => {
            Value::String(interpolate_string(s, scope))
        }
        Literal::Symbol(s) => Value::Symbol(s.clone()),
        Literal::Money(m) => Value::Money(m.clone()),
        Literal::Percent(p) => Value::Percent(*p),
        Literal::Date(d) => Value::Date(d.clone()),
        Literal::Duration(d) => Value::Duration(d.clone()),
        Literal::Coord(c)
        | Literal::Color(c)
        | Literal::IpAddr(c)
        | Literal::Version(c)
        | Literal::Email(c)
        | Literal::Time(c) => Value::String(c.clone()),
    }
}

fn interpolate_string(raw: &str, scope: &mut Scope) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next_c) = chars.next() {
                out.push(next_c);
            }
        } else if c == '{' {
            let key: String = chars.by_ref().take_while(|&ch| ch != '}').collect();
            let trimmed = key.trim();
            if let Ok(val) = eval_path(trimmed, scope) {
                out.push_str(&val.to_string());
            } else if let Ok(sents) = parse_code(trimmed) {
                if let Some(first) = sents.first() {
                    if let Ok(val) = eval_sentence(first, scope) {
                        out.push_str(&val.to_string());
                        continue;
                    }
                }
                out.push_str(&format!("{{{key}}}"));
            } else {
                out.push_str(&format!("{{{key}}}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn eval_token(token: &Token, scope: &mut Scope) -> Result<Value, String> {
    match token {
        Token::Lit(lit) => Ok(eval_literal(lit, scope)),
        Token::Path(path) => eval_path(path, scope),
        Token::Range(start_tok, end_tok) => {
            match (eval_token(start_tok, scope)?, eval_token(end_tok, scope)?) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Range(a, b)),
                (Value::String(a), Value::String(b)) => Ok(Value::Range(
                    a.chars().next().unwrap_or('a') as i64,
                    b.chars().next().unwrap_or('z') as i64,
                )),
                _ => Ok(Value::Range(0, 0)),
            }
        }
        Token::Group('(' | ')', sentences) => match sentences.as_slice() {
            [] => Ok(Value::Tuple(vec![])),
            [single] => eval_sentence(single, scope),
            many => Ok(Value::Tuple(
                many.iter()
                    .map(|s| eval_sentence(s, scope))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
        },
        Token::Group('[', sentences) => Ok(Value::Stream(sentences.clone())),
        Token::Group('{', sentences) => {
            let is_router = sentences.iter().any(|s| match classify_sentence(s) {
                SemanticKind::PatternBranch { .. } | SemanticKind::LambdaOrTupleBranch { .. } => {
                    true
                }
                SemanticKind::StreamOrBranch { name, .. } => {
                    name == "_"
                        || matches!(
                            scope.get(&name),
                            Some(Value::Form(_) | Value::Function(_) | Value::Stream(_))
                        )
                }
                _ => false,
            });
            if is_router {
                Ok(Value::Router(sentences.clone()))
            } else {
                let mut form = FormInstance::new("AnonymousForm");
                populate_form_members(&mut form, sentences, scope)?;
                Ok(Value::Form(Arc::new(RwLock::new(form))))
            }
        }
        _ => Err(format!("Невозможно вычислить токен: {token:?}")),
    }
}

pub fn populate_form_members(
    form: &mut FormInstance,
    sentences: &[Sentence],
    scope: &mut Scope,
) -> Result<(), String> {
    for s in sentences {
        match classify_sentence(s) {
            SemanticKind::FormDef { name, body, tail } => {
                let mut sub = FormInstance::new(&name);
                populate_form_members(&mut sub, &body, scope)?;
                let mut val = Value::Form(Arc::new(RwLock::new(sub)));
                if !tail.is_empty() {
                    val = eval_operator_tail(val, &tail, scope)?;
                }
                form.set_field(&name, val);
            }
            SemanticKind::Binding { key, value } => {
                let v = match value.as_slice() {
                    [Token::Path(p)]
                    if matches!(
                            p.as_str(),
                            "String" | "Int" | "Float" | "Bool" | "Byte" | "Money" | "File" | "Any"
                        ) =>
                        {
                            Value::Symbol(p.clone())
                        }
                    _ => eval_expression(&value, scope)?,
                };
                form.set_field(&key, v);
            }
            SemanticKind::TupleBinding { tuple, value } => {
                if let Some(first_sent) = tuple.first() {
                    let k = eval_sentence(first_sent, scope)?.to_string();
                    let v = eval_expression(&value, scope)?;
                    form.set_field(&k, v);
                }
            }
            SemanticKind::StreamOrBranch { name, stream } => {
                form.set_field(&name, Value::Stream(stream));
            }
            SemanticKind::FunctionDef {
                name, args, body, ..
            } => {
                let env_arc = Arc::new(RwLock::new(scope.clone()));
                let func = Arc::new(FunctionInstance {
                    name: name.clone(),
                    body: FunctionBody::UserDefined {
                        args,
                        body,
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
                    body: FunctionBody::UserDefined {
                        args,
                        body,
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

pub fn eval_path(path: &str, scope: &mut Scope) -> Result<Value, String> {
    if path == "_" {
        return scope
            .get("_")
            .ok_or_else(|| "Контекст `_` не определен".into());
    }

    let segments: Vec<&str> = path.split('.').collect();
    let (root_name, root_call) = segments[0]
        .strip_suffix("()")
        .map(|s| (s, true))
        .unwrap_or((segments[0], false));

    let mut current = if let Some(val) = scope.get(root_name) {
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
                    if let Some(m) = snap.methods.get(seg) {
                        invoke_form_method(&snap, m, Value::None, scope)?
                    } else if seg == "Read" {
                        snap.get_field("Value").unwrap_or(Value::None)
                    } else {
                        return Err(format!("Метод '{seg}()' не найден в {}", snap.type_name));
                    }
                } else if let Some(val) = snap.get_field(seg) {
                    if let Value::Stream(sents) = val {
                        eval_reactive_field(&snap, form_arc.clone(), &sents, scope)?
                    } else {
                        val
                    }
                } else if let Some(m) = snap.methods.get(seg) {
                    Value::Function(m.clone())
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
                    if let Value::Stream(sents) = v {
                        eval_reactive_field(&snap, form_arc.clone(), &sents, scope)?
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
            Value::Atom(a) if seg == "Read" || seg == "Value" => a.lock().unwrap().clone(),
            Value::Channel(q) if seg == "Recv" && is_call => match q.lock().unwrap().pop_front() {
                Some(v) => make_some(v),
                None => make_empty_form("None"),
            },
            Value::Tuple(items) => {
                if seg == "Length" {
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
            Value::String(s) if seg == "Length" => Value::Int(s.chars().count() as i64),
            _ => return Err(format!("Нельзя получить '{seg}' из {current:?}")),
        };
    }

    Ok(current)
}

fn find_ancestor_recursive(form: &FormInstance, target: &str) -> Option<FormInstance> {
    form.ancestors
        .get(target)
        .cloned()
        .or_else(|| {
            form.ancestors
                .values()
                .find_map(|a| find_ancestor_recursive(a, target))
        })
}

fn eval_reactive_field(
    snap: &FormInstance,
    form_arc: Arc<RwLock<FormInstance>>,
    stream_sents: &[Sentence],
    scope: &mut Scope,
) -> Result<Value, String> {
    if stream_uses_free_underscore(stream_sents) {
        return Ok(Value::Stream(stream_sents.to_vec()));
    }
    let mut r_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
    r_scope.set("Self", Value::Form(form_arc));

    for fname in &snap.field_order {
        if let Some(fval) = snap.fields.get(fname) {
            if let Value::Stream(dep_sents) = fval {
                if dep_sents == stream_sents {
                    break;
                }
                if !stream_uses_free_underscore(dep_sents) {
                    if let Ok(comp) = eval_stream_body(dep_sents, None, &mut r_scope) {
                        r_scope.set(fname, unwrap_done(comp));
                    }
                }
            } else {
                r_scope.set(fname, fval.clone());
            }
        }
    }
    Ok(unwrap_done(eval_stream_body(stream_sents, None, &mut r_scope)?))
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
                Value::Tuple(items) => items,
                Value::None => vec![],
                other => vec![other],
            };
            f(args_vec, scope)
        }
        FunctionBody::UserDefined {
            args,
            body,
            captured_env,
            overrides,
            ..
        } => {
            let mut call_scope = Scope::child(captured_env.clone());
            let self_arc = Arc::new(RwLock::new(snap.clone()));
            call_scope.set("Self", Value::Form(self_arc.clone()));
            call_scope.unpack_form(&self_arc, true);
            for (k, v) in overrides {
                call_scope.set(k, v.clone());
            }
            bind_arguments(args, &arg, &mut call_scope)?;
            Ok(unwrap_done(eval_stream_body(body, Some(arg), &mut call_scope)?))
        }
        FunctionBody::Composed(f1, f2) => {
            let mid = apply_callable(f1.clone(), arg, scope)?;
            apply_callable(f2.clone(), mid, scope)
        }
    }
}

// =============================================================================
// 4. АРИФМЕТИКА, ВАЛЮТА И АЛГЕБРА ТИПОВ (&, |, /)
// =============================================================================

fn parse_money(raw: &str) -> Option<(String, f64)> {
    let idx = raw.find('$')?;
    Some((raw[..=idx].to_string(), raw[idx + 1..].parse().ok()?))
}

pub fn eval_binary_op_in_scope(
    left: Value,
    op: &str,
    right: Value,
    scope: &mut Scope,
) -> Result<Value, String> {
    if let Value::Form(ref f_arc) = left {
        let snap = f_arc.read().unwrap().clone();
        if let Some(m) = snap.methods.get(op) {
            return invoke_form_method(&snap, m, right, scope);
        }
    }

    if op == "==" {
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
            let (p, v) = parse_money(&m).ok_or("Некорректный Money")?;
            Ok(Value::Money(format!("{p}{:.2}", v * q as f64)))
        }
        (Value::Money(m), "*", Value::Float(q)) | (Value::Float(q), "*", Value::Money(m)) => {
            let (p, v) = parse_money(&m).ok_or("Некорректный Money")?;
            Ok(Value::Money(format!("{p}{:.2}", v * q)))
        }
        (Value::Money(m), "*", Value::Percent(pct))
        | (Value::Percent(pct), "*", Value::Money(m)) => {
            let (p, v) = parse_money(&m).ok_or("Некорректный Money")?;
            Ok(Value::Money(format!("{p}{:.2}", v * (pct / 100.0))))
        }
        (Value::Money(m1), op @ ("+" | "-"), Value::Money(m2)) => {
            let (p, v1) = parse_money(&m1).ok_or("Некорректный Money")?;
            let (_, v2) = parse_money(&m2).ok_or("Некорректный Money")?;
            let res = if op == "+" { v1 + v2 } else { v1 - v2 };
            Ok(Value::Money(format!("{p}{res:.2}")))
        }

        (Value::Bool(a), "&&", Value::Bool(b)) => Ok(Value::Bool(a && b)),
        (Value::Bool(a), "||", Value::Bool(b)) => Ok(Value::Bool(a || b)),
        (Value::String(a), "+", Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),

        // 1. Слияние форм `&` (Закон 2: Lineage + фиксация коллизий)
        (Value::Form(f1), "&", Value::Form(f2)) => {
            let (r1, r2) = (f1.read().unwrap(), f2.read().unwrap());
            if r2.type_name == "Public" || r2.type_name == "Private" {
                return Ok(Value::Form(f1.clone()));
            }
            let is_override = r2.type_name == "AnonymousForm";
            let new_name = if is_override {
                r1.type_name.clone()
            } else {
                format!("{}+{}", r1.type_name, r2.type_name)
            };

            let mut merged = FormInstance::new(&new_name);
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

        // 2. Уточнение типа `User & IsAdult`
        (Value::Form(f1), "&", pred @ (Value::Function(_) | Value::Stream(_))) => {
            let mut refined = f1.read().unwrap().clone();
            refined
                .ancestors
                .insert(refined.type_name.clone(), f1.read().unwrap().clone());
            refined.predicates.push(pred);
            Ok(Value::Form(Arc::new(RwLock::new(refined))))
        }
        (Value::Symbol(base_sym), "&", pred @ (Value::Function(_) | Value::Stream(_))) => {
            let mut refined = FormInstance::new(&base_sym);
            refined.predicates.push(pred);
            Ok(Value::Form(Arc::new(RwLock::new(refined))))
        }

        // 3. Перепривязка именованных слотов функции `Connect & { Port 9090 }`
        (Value::Function(func_arc), "&", Value::Form(ov_arc)) => {
            let ov = ov_arc.read().unwrap();
            if ov.type_name == "Public" || ov.type_name == "Private" {
                return Ok(Value::Function(func_arc));
            }
            if let FunctionBody::UserDefined {
                args,
                body,
                captured_env,
                overrides,
                partial_args,
            } = &func_arc.body
            {
                let mut new_ov = overrides.clone();
                new_ov.extend(ov.fields.clone());
                Ok(Value::Function(Arc::new(FunctionInstance {
                    name: func_arc.name.clone(),
                    body: FunctionBody::UserDefined {
                        args: args.clone(),
                        body: body.clone(),
                        captured_env: captured_env.clone(),
                        overrides: new_ov,
                        partial_args: partial_args.clone(),
                    },
                })))
            } else {
                Ok(Value::Function(func_arc))
            }
        }

        // 4. Композиция потоков `Step1 & Step2`
        (
            f1 @ (Value::Function(_) | Value::Stream(_)),
            "&",
            f2 @ (Value::Function(_) | Value::Stream(_)),
        ) => Ok(Value::Function(Arc::new(FunctionInstance {
            name: "ComposedStream".into(),
            body: FunctionBody::Composed(f1, f2),
        }))),

        // 5. Метаданные видимости `Val & Private`
        (val, "&", Value::Form(_)) => Ok(val),

        // 6. Объединение `A | B`
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

        // 7. Вычитание `/` для Форм и Карт (`ConfigMap / "host"`, `Combined / 'Name'`)
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
// 5. ВЫРАЖЕНИЯ, ПАЙПЛАЙНЫ И ТРИ ЖЕЛЕЗНЫХ ЗАКОНА
// =============================================================================

pub fn eval_expression(tokens: &[Token], scope: &mut Scope) -> Result<Value, String> {
    if tokens.is_empty() {
        return Ok(Value::None);
    }

    if has_pipe_op(tokens) {
        let (head, steps) = split_pipeline(tokens);
        let mut current = eval_expression(&head, scope)?;
        for step in steps {
            current = eval_pipe_step(current, step, scope)?;
        }
        return Ok(current);
    }

    if let [Token::Op(op), rest @ ..] = tokens {
        if op == "!" && !rest.is_empty() {
            return match eval_expression(rest, scope)? {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                v => Err(format!("Оператор '!' не применим к {v:?}")),
            };
        }
    }

    if let [Token::Path(fn_name), Token::Group(')', args)] = tokens {
        return eval_call_expr(fn_name, args, None, scope);
    }

    // Ровно 2 токена `TypeName { ... }` — инстанцирование формы.
    // Если токенов больше (например, `User { ... } & Cloud { ... }`),
    // они будут корректно разбиты по бинарному оператору в `eval_infix_tokens`!
    if let [Token::Path(type_name), Token::Group('{', sentences)] = tokens {
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
        populate_form_members(&mut form, sentences, scope)?;
        return Ok(Value::Form(Arc::new(RwLock::new(form))));
    }

    if tokens.len() == 1 {
        return eval_token(&tokens[0], scope);
    }

    eval_infix_tokens(tokens, scope)
}

fn op_precedence(op: &str) -> u8 {
    match op {
        "|" => 1,
        "&" | "/" => 2,
        "||" => 3,
        "&&" => 4,
        "==" | "!=" | ">" | "<" | ">=" | "<=" => 5,
        "+" | "-" | "<<" | ">>" => 6,
        "*" | "%" => 7,
        "**" => 8,
        _ => 0,
    }
}

fn eval_infix_tokens(tokens: &[Token], scope: &mut Scope) -> Result<Value, String> {
    let mut lowest_idx = None;
    let mut lowest_prec = u8::MAX;

    for (i, tok) in tokens.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if let Token::Op(op) = tok {
            let prec = if op == "/" {
                let is_form = matches!(
                    tokens.first(),
                    Some(Token::Path(p)) if matches!(scope.get(p), Some(Value::Form(_)))
                );
                if is_form { 2 } else { 7 }
            } else {
                op_precedence(op)
            };
            if prec > 0 && prec <= lowest_prec {
                lowest_prec = prec;
                lowest_idx = Some(i);
            }
        }
    }

    if let Some(idx) = lowest_idx {
        if let Token::Op(op) = &tokens[idx] {
            let left = eval_expression(&tokens[..idx], scope)?;
            let rhs_tokens = &tokens[idx + 1..];
            if op == "/" {
                if let (Value::Form(f_arc), [Token::Path(path_str)]) = (&left, rhs_tokens) {
                    let sub = subtract_ancestor_or_path(&f_arc.read().unwrap(), path_str);
                    return Ok(Value::Form(Arc::new(RwLock::new(sub))));
                }
            }
            let right = eval_expression(rhs_tokens, scope)?;
            return eval_binary_op_in_scope(left, op, right, scope);
        }
    }
    eval_token(&tokens[0], scope)
}

pub fn eval_operator_tail(
    mut current: Value,
    tail: &[Token],
    scope: &mut Scope,
) -> Result<Value, String> {
    let mut i = 0;
    while i < tail.len() {
        if let Token::Op(op) = &tail[i] {
            let next_op_offset = tail[i + 1..]
                .iter()
                .position(|t| matches!(t, Token::Op(_)))
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
            let rhs = eval_expression(rhs_slice, scope)?;
            current = eval_binary_op_in_scope(current, op, rhs, scope)?;
            i = next_op_offset;
        } else {
            i += 1;
        }
    }
    Ok(current)
}

fn eval_pipe_step(input: Value, step: PipeStep, scope: &mut Scope) -> Result<Value, String> {
    if matches!(input, Value::None) && step.op != PipeOp::Filter {
        return Ok(Value::None);
    }

    match step.op {
        PipeOp::Dot => eval_pipe_target(input, &step.expr, scope),
        PipeOp::Tap => {
            let _ = eval_pipe_target(input.clone(), &step.expr, scope)?;
            Ok(input)
        }
        PipeOp::Enrich => {
            if let Ok(target) = eval_expression(&step.expr, scope) {
                if !callable_accepts_input(&target, &input, scope) {
                    return Ok(input);
                }
            }
            let enriched = eval_pipe_target(input.clone(), &step.expr, scope)?;
            if matches!(enriched, Value::None) {
                Ok(Value::None)
            } else {
                eval_binary_op_in_scope(input, "&", enriched, scope)
            }
        }
        PipeOp::Filter => {
            if matches!(input, Value::None) {
                return Ok(Value::None);
            }
            let pred = eval_expression(&step.expr, scope)?;
            match apply_callable(pred, input.clone(), scope)? {
                Value::Bool(true) => Ok(input),
                _ => Ok(Value::None),
            }
        }
        PipeOp::Parallel => {
            let Some(Token::Group('{', branches)) = step.expr.first() else {
                return Err("Оператор .| ожидает блок {}".into());
            };
            let mut handles = Vec::new();
            for branch in branches {
                let b_clone = branch.clone();
                let in_clone = input.clone();
                let parent = Arc::new(RwLock::new(scope.clone()));
                handles.push(thread::spawn(move || -> Result<(String, Value), String> {
                    let mut l_scope = Scope::child(parent);
                    l_scope.set("_", in_clone.clone());
                    if let Value::Form(f) = &in_clone {
                        l_scope.unpack_form(f, true);
                    }
                    match classify_sentence(&b_clone) {
                        SemanticKind::StreamOrBranch { name, stream } => Ok((
                            name,
                            unwrap_done(eval_stream_body(&stream, Some(in_clone), &mut l_scope)?),
                        )),
                        _ => Ok(("Result".into(), eval_sentence(&b_clone, &mut l_scope)?)),
                    }
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

fn eval_pipe_target(input: Value, expr: &[Token], scope: &mut Scope) -> Result<Value, String> {
    if let [Token::Path(fn_name), Token::Group(')', call_args)] = expr {
        return eval_call_expr(fn_name, call_args, Some(input), scope);
    }
    // Если вызывается `SampleStream . Meta.StepsOf`, а `SampleStream` хранится как `Value::Function`,
    // конвертируем его тело обратно в `Value::Stream`, чтобы `Meta.StepsOf` видел все шаги потока!
    let normalized_input = if matches!(expr, [Token::Path(p)] if p == "Meta.StepsOf") {
        if let Value::Function(func_arc) = &input {
            if let FunctionBody::UserDefined { body, .. } = &func_arc.body {
                Value::Stream(body.clone())
            } else {
                input
            }
        } else {
            input
        }
    } else {
        input
    };

    let mut t_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
    t_scope.set("_", normalized_input.clone());
    if let Value::Form(f_arc) = &normalized_input {
        t_scope.unpack_form(f_arc, true);
    }
    let target = eval_expression(expr, &mut t_scope)?;
    apply_callable(target, normalized_input, scope)
}

fn callable_accepts_input(callable: &Value, input: &Value, scope: &mut Scope) -> bool {
    let Value::Function(func_arc) = callable else {
        return true;
    };
    let FunctionBody::UserDefined {
        args, partial_args, ..
    } = &func_arc.body
    else {
        return true;
    };
    let Some(first_param) = args.get(partial_args.len()) else {
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
                    "String" | "Int" | "Float" | "Bool" | "Any" | "Tuple" | "File" | "List"
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
                    Value::Tuple(items) => items,
                    Value::None => vec![],
                    other => vec![other],
                };
                f(args_vec, scope)
            }
            FunctionBody::UserDefined {
                args,
                body,
                captured_env,
                overrides,
                partial_args,
            } => {
                let mut combined = partial_args.clone();
                match &arg {
                    Value::Tuple(items) if args.len() > 1 => combined.extend(items.clone()),
                    Value::None => {}
                    other => combined.push(other.clone()),
                }

                // Поддержка Partial Application (Раздел 4.3: `AddTen AddTwo(10)`)
                if !combined.is_empty() && combined.len() < args.len() && !matches!(arg, Value::None)
                {
                    let has_missing_required = args.iter().skip(combined.len()).any(|s| {
                        matches!(s.as_slice(), [Token::Path(_)])
                            || (s.len() == 2 && matches!(s[1], Token::Path(_)))
                    });
                    if has_missing_required {
                        return Ok(Value::Function(Arc::new(FunctionInstance {
                            name: format!("{}_partial", func.name),
                            body: FunctionBody::UserDefined {
                                args: args.clone(),
                                body: body.clone(),
                                captured_env: captured_env.clone(),
                                overrides: overrides.clone(),
                                partial_args: combined,
                            },
                        })));
                    }
                }

                let mut call_scope = Scope::child(captured_env.clone());
                let packed_arg = if combined.is_empty() {
                    Value::None
                } else if combined.len() == 1 {
                    combined.remove(0)
                } else {
                    Value::Tuple(combined)
                };
                bind_arguments(args, &packed_arg, &mut call_scope)?;
                for (k, v) in overrides {
                    call_scope.set(k, v.clone());
                }
                let res = eval_stream_body(body, Some(packed_arg), &mut call_scope)?;
                // Для лямбд сохраняем `Value::Done` (чтобы `Fold` получил сигнал остановки),
                // а для обычных именованных функций снимаем обертку `Done`.
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
        Value::Stream(sentences) => {
            let mut s_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
            s_scope.set("_", arg.clone());
            if let Value::Form(f_arc) = &arg {
                s_scope.unpack_form(f_arc, true);
            }
            eval_stream_body(&sentences, Some(arg), &mut s_scope)
        }
        Value::Router(branches) => eval_router(&branches, arg, scope),
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
                if let Some(Value::Stream(body)) = router.fields.get(key) {
                    let mut b_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                    if match_type_or_wildcard(key, &arg, &mut b_scope) {
                        return eval_stream_body(body, Some(arg), &mut b_scope);
                    }
                }
            }
            Ok(Value::None)
        }
        _ => Err(format!("Значение {callable:?} не может быть вызвано")),
    }
}

pub fn eval_call_expr(
    fn_name: &str,
    call_args: &[Sentence],
    piped_first: Option<Value>,
    scope: &mut Scope,
) -> Result<Value, String> {
    if fn_name == "Compose" {
        return eval_compose_call(call_args, scope);
    }

    let mut evaluated = Vec::new();
    if let Some(first) = piped_first {
        evaluated.push(first);
    }
    for a in call_args {
        evaluated.push(eval_sentence(a, scope)?);
    }

    let arg = match evaluated.len() {
        0 => Value::None,
        1 => evaluated.remove(0),
        _ => Value::Tuple(evaluated),
    };

    if let Some((obj_path, method_name)) = fn_name.rsplit_once('.') {
        if let Ok(obj_val) = eval_path(obj_path, scope) {
            match obj_val {
                Value::Form(f_arc) => {
                    let snap = f_arc.read().unwrap().clone();
                    if let Some(method) = snap.methods.get(method_name) {
                        return invoke_form_method(&snap, method, arg, scope);
                    }
                }
                Value::Atom(atom_arc) => match method_name {
                    "Swap" => {
                        let current_val = atom_arc.lock().unwrap().clone();
                        let next_val = apply_callable(arg, current_val, scope)?;
                        *atom_arc.lock().unwrap() = next_val.clone();
                        return Ok(next_val);
                    }
                    "Read" => {
                        return Ok(atom_arc.lock().unwrap().clone());
                    }
                    _ => {}
                },
                Value::Channel(chan_arc) => match method_name {
                    "Send" => {
                        chan_arc.lock().unwrap().push_back(arg);
                        return Ok(Value::None);
                    }
                    "Recv" => {
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
            node.set_field("Args", arg);
            return Ok(Value::Form(Arc::new(RwLock::new(node))));
        }
        apply_callable(func, arg, scope)
    } else {
        Err(format!("Функция '{fn_name}' не найдена"))
    }
}

fn eval_router(branches: &[Sentence], arg: Value, scope: &mut Scope) -> Result<Value, String> {
    for branch in branches {
        match classify_sentence(branch) {
            SemanticKind::PatternBranch { pattern, body } => {
                let mut b_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                b_scope.set("_", arg.clone());
                if let Value::Form(f) = &arg {
                    b_scope.unpack_form(f, true);
                }
                let matched = match pattern {
                    Token::Lit(lit) => arg == eval_literal(&lit, &mut b_scope),
                    Token::Range(a, b) => {
                        let r = eval_token(&Token::Range(a, b), &mut b_scope)?;
                        matches!(
                            apply_callable(r, arg.clone(), &mut b_scope),
                            Ok(Value::Bool(true))
                        )
                    }
                    Token::Group('[', pred_sents) => {
                        matches!(
                            eval_stream_body(&pred_sents, Some(arg.clone()), &mut b_scope)?,
                            Value::Bool(true)
                        )
                    }
                    Token::Group('{', field_pats) => {
                        if let Value::Form(f_arc) = &arg {
                            let f = f_arc.read().unwrap();
                            let mut ok = true;
                            for fp in field_pats {
                                if let SemanticKind::Binding { key, value } = classify_sentence(&fp)
                                {
                                    let Some(actual) = f.get_field(&key) else {
                                        ok = false;
                                        break;
                                    };
                                    if let [Token::Group('(' | ')', t_pats)] = value.as_slice() {
                                        if !match_tuple_pattern(t_pats, &actual, &mut b_scope)? {
                                            ok = false;
                                            break;
                                        }
                                    } else if actual != eval_expression(&value, &mut b_scope)? {
                                        ok = false;
                                        break;
                                    }
                                }
                            }
                            ok
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                if matched {
                    return eval_stream_body(&body, Some(arg), &mut b_scope);
                }
            }
            SemanticKind::LambdaOrTupleBranch { args, body } => {
                let mut b_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                b_scope.set("_", arg.clone());
                if match_tuple_pattern(&args, &arg, &mut b_scope)? {
                    return eval_stream_body(&body, Some(arg), &mut b_scope);
                }
            }
            SemanticKind::StreamOrBranch { name, stream } => {
                let mut b_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                if match_type_or_wildcard(&name, &arg, &mut b_scope) {
                    return eval_stream_body(&stream, Some(arg), &mut b_scope);
                }
            }
            _ => {}
        }
    }
    Ok(Value::None)
}

pub fn match_tuple_pattern(
    pat_sents: &[Sentence],
    actual: &Value,
    scope: &mut Scope,
) -> Result<bool, String> {
    let items = match actual {
        Value::Tuple(v) => v.clone(),
        other => vec![other.clone()],
    };
    if pat_sents.len() != items.len() {
        return Ok(false);
    }
    for (pat_s, act_val) in pat_sents.iter().zip(items.iter()) {
        match pat_s.as_slice() {
            [Token::Path(v)] if v == "_" => {}
            [Token::Path(v)] => scope.set(v, act_val.clone()),
            _ if act_val != &eval_sentence(pat_s, scope)? => return Ok(false),
            _ => {}
        }
    }
    Ok(true)
}

fn match_type_or_wildcard(pattern_name: &str, val: &Value, scope: &mut Scope) -> bool {
    if pattern_name == "_" {
        scope.set("_", val.clone());
        return true;
    }
    if pattern_name == "None" && matches!(val, Value::None) {
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

    for (idx, param_sent) in arg_defs.iter().enumerate() {
        let provided = items.get(idx).cloned();
        match param_sent.as_slice() {
            [Token::Path(single)] => {
                let val = provided.unwrap_or(Value::None);
                let is_type = single.chars().next().map(char::is_uppercase).unwrap_or(false)
                    && matches!(val, Value::Form(_));
                scope.set(single, val.clone());
                if is_type {
                    if let Value::Form(f_arc) = &val {
                        scope.unpack_form(f_arc, true);
                    }
                }
            }
            [Token::Path(_), Token::Op(op), ..] if matches!(op.as_str(), "&" | "|" | "/") => {
                if let Some(Value::Form(f_arc)) = &provided {
                    scope.unpack_form(f_arc, true);
                }
            }
            [Token::Path(param_name), rest @ ..] => {
                if let Some(val) = provided {
                    scope.set(param_name, val);
                } else if !rest.is_empty() {
                    let def_val = eval_expression(rest, scope)?;
                    scope.set(param_name, def_val);
                } else {
                    scope.set(param_name, Value::None);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn eval_stream_body(
    body: &[Sentence],
    input: Option<Value>,
    scope: &mut Scope,
) -> Result<Value, String> {
    if let Some(inp) = input {
        scope.set("_", inp);
    }
    let mut last = Value::None;
    for s in body {
        last = eval_sentence(s, scope)?;
        // ВАЖНО: пробрасываем `Value::Done(val)` вверх без снятия обертки,
        // чтобы `Done` из вложенного роутера доходил до `Fold`!
        if let Value::Done(_) = last {
            return Ok(last);
        }
        if let Value::Form(f_arc) = &last {
            let f = f_arc.read().unwrap();
            if f.type_name == "Done" {
                if let Some(v) = f.get_field("Value") {
                    return Ok(Value::Done(Box::new(v)));
                }
            }
        }
    }
    Ok(last)
}

// =============================================================================
// 6. ОБРАБОТКА ПРЕДЛОЖЕНИЙ (SENTENCES) И ЗАПУСК МОДУЛЯ
// =============================================================================

pub fn eval_sentence(sentence: &Sentence, scope: &mut Scope) -> Result<Value, String> {
    match classify_sentence(sentence) {
        SemanticKind::LambdaOrTupleBranch { args, body } => {
            Ok(Value::Function(Arc::new(FunctionInstance {
                name: "Lambda".into(),
                body: FunctionBody::UserDefined {
                    args,
                    body,
                    captured_env: Arc::new(RwLock::new(scope.clone())),
                    overrides: HashMap::new(),
                    partial_args: Vec::new(),
                },
            })))
        }
        SemanticKind::StreamOrBranch { name, stream } => {
            let env_arc = Arc::new(RwLock::new(scope.clone()));
            let val = Value::Function(Arc::new(FunctionInstance {
                name: name.clone(),
                body: FunctionBody::UserDefined {
                    args: Vec::new(),
                    body: stream,
                    captured_env: env_arc.clone(),
                    overrides: HashMap::new(),
                    partial_args: Vec::new(),
                },
            }));
            env_arc.write().unwrap().set(&name, val.clone());
            scope.set(&name, val.clone());
            Ok(val)
        }
        SemanticKind::Binding { key, value } => {
            let val = match value.as_slice() {
                [Token::Path(p)]
                if matches!(
                        p.as_str(),
                        "String" | "Int" | "Float" | "Bool" | "Byte" | "Money" | "File" | "Any"
                    ) =>
                    {
                        Value::Symbol(key.clone())
                    }
                _ => eval_expression(&value, scope)?,
            };
            scope.set(&key, val.clone());
            Ok(val)
        }
        SemanticKind::TupleBinding { tuple, value } => {
            // Если после первой скобочной группы `(V1 + V2)` идет оператор `* 2.0`,
            // то это не деструктуризация кортежа, а инфиксное выражение `(V1 + V2) * 2.0`!
            if matches!(value.first(), Some(Token::Op(_))) {
                return eval_expression(sentence, scope);
            }
            let val = eval_expression(&value, scope)?;
            let _ = match_tuple_pattern(&tuple, &val, scope)?;
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
        SemanticKind::GenericDef { name, body, .. } => {
            if name == "Map" && scope.get("Map").is_some() {
                return Ok(scope.get("Map").unwrap());
            }
            let mut form = FormInstance::new(&name);
            if let [Token::Group('{', f_body), ..] = body.as_slice() {
                populate_form_members(&mut form, f_body, scope)?;
            }
            let val = Value::Form(Arc::new(RwLock::new(form)));
            scope.set(&name, val.clone());
            Ok(val)
        }
        SemanticKind::AlgebraicDef { name, expr } => {
            let val = eval_expression(&expr, scope)?;
            if let Value::Form(f_arc) = &val {
                let mut fw = f_arc.write().unwrap();
                let snap = fw.clone();
                if snap.type_name != "UnionType" && snap.type_name != name {
                    fw.ancestors.insert(snap.type_name.clone(), snap);
                }
                fw.type_name = name.clone();
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
                body: FunctionBody::UserDefined {
                    args,
                    body,
                    captured_env: env_arc.clone(),
                    overrides: HashMap::new(),
                    partial_args: Vec::new(),
                },
            }));
            env_arc.write().unwrap().set(&name, val.clone());
            scope.set(&name, val.clone());
            Ok(val)
        }
        SemanticKind::CallExpr { name, args } => eval_call_expr(&name, &args, None, scope),
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
            eval_expression(&tokens, scope)
        }
        SemanticKind::Expression(tokens) => eval_expression(&tokens, scope),
        _ => Ok(Value::None),
    }
}

pub fn run_hir_module(hir: &HirModule) -> Result<Value, String> {
    let mut global_scope = create_global_scope();

    let mut last_val = Value::None;
    for sentence in &hir.top_level_sentences {
        last_val = eval_sentence(sentence, &mut global_scope)?;
    }

    if let Some(main_fn) = global_scope.get("Main") {
        return apply_callable(main_fn, Value::None, &mut global_scope);
    }

    Ok(last_val)
}

pub fn run_program(source: &str) -> Result<Value, String> {
    let sentences = parse_code(source).map_err(|e| format!("Ошибка парсинга:\n{e}"))?;
    let hir = lower_to_hir(&sentences);
    run_hir_module(&hir)
}