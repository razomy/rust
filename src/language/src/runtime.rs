// src/runtime.rs
// =============================================================================
// RAZOMY V4.7 — THREAD-SAFE INTERPRETER & DATAFLOW RUNTIME ENGINE
// =============================================================================

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use crate::hir::{lower_to_hir, HirModule};
use crate::{
    classify_sentence, has_pipe_op, parse_code, split_pipeline, Literal, PipeOp, PipeStep,
    SemanticKind, Sentence, Token,
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
    Router(Vec<Sentence>), // Блок маршрутизации и паттерн-матчинга `{ ... }`
    Function(Arc<FunctionInstance>),
    Atom(Arc<Mutex<Value>>),
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
                let r1 = f1.read().unwrap();
                let r2 = f2.read().unwrap();
                r1.type_name == r2.type_name && r1.fields == r2.fields
            }
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
    pub ancestors: HashMap<String, FormInstance>, // Закон 2: Полная история Lineage
    pub ambiguous_fields: HashMap<String, Vec<String>>,
    pub predicates: Vec<Value>,     // Предикаты уточнения (`User & IsAdult`)
    pub union_variants: Vec<Value>, // Варианты объединения (`'admin' | 'manager'`, `Circle | Rect`)
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
        self.field_origins
            .insert(key.to_string(), origin.to_string());
        self.ambiguous_fields.remove(key);
    }

    pub fn get_field(&self, key: &str) -> Option<Value> {
        self.fields.get(key).cloned()
    }

    pub fn has_lineage(&self, target_type: &str) -> bool {
        if self.type_name == target_type || self.ancestors.contains_key(target_type) {
            return true;
        }
        for anc in self.ancestors.values() {
            if anc.has_lineage(target_type) {
                return true;
            }
        }
        false
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
                write!(f, "(")?;
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{it:?}")?;
                }
                write!(f, ")")
            }
            Value::Form(form) => {
                let form = form.read().unwrap();
                write!(f, "{} {{ ", form.type_name)?;
                for key in &form.field_order {
                    if let Some(val) = form.fields.get(key) {
                        write!(f, "{key}: {val:?}, ")?;
                    }
                }
                write!(f, "}}")
            }
            Value::Stream(_) => write!(f, "[Stream]"),
            Value::Router(_) => write!(f, "{{Router}}"),
            Value::Function(func) => write!(f, "Fn({})", func.name),
            Value::Atom(a) => write!(f, "Atom({:?})", a.lock().unwrap()),
            Value::Done(v) => write!(f, "Done({:?})", v),
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
// 2. ОБЛАСТЬ ВИДИМОСТИ (SCOPE)
// =============================================================================

#[derive(Clone, Default)]
pub struct Scope {
    pub parent: Option<Arc<RwLock<Scope>>>,
    pub bindings: HashMap<String, Value>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            parent: None,
            bindings: HashMap::new(),
        }
    }

    pub fn child(parent: Arc<RwLock<Scope>>) -> Self {
        Self {
            parent: Some(parent),
            bindings: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(val) = self.bindings.get(name) {
            return Some(val.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.read().unwrap().get(name);
        }
        None
    }

    pub fn set(&mut self, name: &str, val: Value) {
        self.bindings.insert(name.to_string(), val);
    }
}

// =============================================================================
// 3. ВЫЧИСЛИТЕЛЬ ВЫРАЖЕНИЙ, ИНТЕРПОЛЯЦИИ И ПУТЕЙ
// =============================================================================

pub fn eval_literal(lit: &Literal, scope: &mut Scope) -> Value {
    match lit {
        Literal::Int(i) => Value::Int(*i),
        Literal::Float(f) => Value::Float(*f),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Byte(b) => Value::Byte(*b),
        Literal::String(s) => Value::String(interpolate_string(s, scope)),
        Literal::Url(u) => Value::String(interpolate_string(u, scope)),
        Literal::File(f) => Value::String(interpolate_string(f, scope)),
        Literal::Symbol(s) => Value::Symbol(s.clone()),
        Literal::Money(m) => Value::Money(m.clone()),
        Literal::Percent(p) => Value::Percent(*p),
        Literal::Date(d) => Value::Date(d.clone()),
        Literal::Duration(d) => Value::Duration(d.clone()),
        Literal::Coord(c) => Value::String(c.clone()),
        Literal::Color(c) => Value::String(c.clone()),
        Literal::IpAddr(ip) => Value::String(ip.clone()),
        Literal::Version(v) => Value::String(v.clone()),
        Literal::Email(e) => Value::String(e.clone()),
        Literal::Time(t) => Value::String(t.clone()),
    }
}

/// Интерполяция строк с поддержкой путей `{Name}`, `{U.Name}`, `{State.Read().Count}`
fn interpolate_string(raw: &str, scope: &mut Scope) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next_c) = chars.next() {
                out.push(next_c);
            }
        } else if c == '{' {
            let mut key = String::new();
            for sub in chars.by_ref() {
                if sub == '}' {
                    break;
                }
                key.push(sub);
            }
            let trimmed = key.trim();
            if let Ok(val) = eval_path(trimmed, scope) {
                out.push_str(&val.to_string());
            } else if let Some(val) = scope.get(trimmed) {
                out.push_str(&val.to_string());
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
            let s_val = eval_token(start_tok, scope)?;
            let e_val = eval_token(end_tok, scope)?;
            match (s_val, e_val) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Range(a, b)),
                _ => Ok(Value::Range(0, 0)),
            }
        }
        Token::Group('(', sentences) | Token::Group(')', sentences) => {
            if sentences.is_empty() {
                Ok(Value::Tuple(vec![]))
            } else if sentences.len() == 1 {
                eval_sentence(&sentences[0], scope)
            } else {
                let mut items = Vec::new();
                for s in sentences {
                    items.push(eval_sentence(s, scope)?);
                }
                Ok(Value::Tuple(items))
            }
        }
        Token::Group('[', sentences) => Ok(Value::Stream(sentences.clone())),
        Token::Group('{', sentences) => {
            let is_router = sentences.iter().any(|s| {
                matches!(
                    classify_sentence(s),
                    SemanticKind::PatternBranch { .. } | SemanticKind::LambdaOrTupleBranch { .. }
                ) || matches!(
                    classify_sentence(s),
                    SemanticKind::StreamOrBranch { ref name, .. }
                        if name == "_"
                            || matches!(
                                scope.get(name),
                                Some(Value::Form(_) | Value::Function(_) | Value::Stream(_))
                            )
                )
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
            SemanticKind::Binding { key, value } => {
                let v = eval_expression(&value, scope)?;
                form.set_field(&key, v);
            }
            SemanticKind::StreamOrBranch {
                name: field_name,
                stream,
            } => {
                form.set_field(&field_name, Value::Stream(stream));
            }
            SemanticKind::FunctionDef {
                name: method_name,
                args,
                body,
                ..
            } => {
                let func = Arc::new(FunctionInstance {
                    name: method_name.clone(),
                    body: FunctionBody::UserDefined {
                        args,
                        body,
                        captured_env: Arc::new(RwLock::new(scope.clone())),
                        overrides: HashMap::new(),
                    },
                });
                form.methods.insert(method_name.clone(), func.clone());
                form.set_field(&method_name, Value::Function(func));
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
            .ok_or_else(|| "Контекст `_` не определен в текущем потоке".into());
    }

    let segments: Vec<&str> = path.split('.').collect();
    let root_raw = segments[0];
    let (root_name, root_is_call) = root_raw
        .strip_suffix("()")
        .map(|s| (s, true))
        .unwrap_or((root_raw, false));

    let mut current = if let Some(val) = scope.get(root_name) {
        if root_is_call {
            apply_callable(val, Value::None, scope)?
        } else {
            val
        }
    } else if segments.len() == 1 {
        // Имя примитивного типа или тега в описании спецификации (`String`, `Int`)
        return Ok(Value::Symbol(root_name.to_string()));
    } else {
        return Err(format!("Неопределенный идентификатор: '{root_name}'"));
    };

    for seg_raw in &segments[1..] {
        let (seg, is_zero_arg_call) = seg_raw
            .strip_suffix("()")
            .map(|s| (s, true))
            .unwrap_or((*seg_raw, false));

        let next_val = match &current {
            Value::Form(form_arc) => {
                let form_snapshot = form_arc.read().unwrap().clone();

                // Проверка Закона 2: Запрет прямого чтения коллизионного поля без пути предка
                if let Some(origins) = form_snapshot.ambiguous_fields.get(seg) {
                    return Err(format!(
                        "Нарушение Закона 2 (Lineage): Поле '{seg}' в '{}' неоднозначно (коллизия предков: {}). Используйте явный путь предка!",
                        form_snapshot.type_name,
                        origins.join(" & ")
                    ));
                }

                if is_zero_arg_call {
                    if let Some(method) = form_snapshot.methods.get(seg) {
                        invoke_form_method(&form_snapshot, method, Value::None, scope)?
                    } else if seg == "Read" {
                        form_snapshot.get_field("Value").unwrap_or(Value::None)
                    } else {
                        return Err(format!(
                            "Метод '{seg}()' не найден в форме {}",
                            form_snapshot.type_name
                        ));
                    }
                } else if let Some(val) = form_snapshot.get_field(seg) {
                    // Закон 1a: Реактивное поле пересчитывается на лету при каждом чтении!
                    if let Value::Stream(stream_sents) = val {
                        eval_reactive_field(&form_snapshot, form_arc.clone(), &stream_sents, scope)?
                    } else {
                        val
                    }
                } else if let Some(method) = form_snapshot.methods.get(seg) {
                    Value::Function(method.clone())
                } else if let Ok(idx) = seg.parse::<i32>() {
                    let i = if idx < 0 {
                        (form_snapshot.field_order.len() as i32 + idx) as usize
                    } else {
                        idx as usize
                    };
                    if let Some(field_name) = form_snapshot.field_order.get(i) {
                        let v = form_snapshot.fields.get(field_name).unwrap().clone();
                        if let Value::Stream(stream_sents) = v {
                            eval_reactive_field(
                                &form_snapshot,
                                form_arc.clone(),
                                &stream_sents,
                                scope,
                            )?
                        } else {
                            v
                        }
                    } else {
                        return Err(format!("Индекс {idx} вне диапазона полей формы"));
                    }
                } else if let Some(ancestor) = find_ancestor_recursive(&form_snapshot, seg) {
                    Value::Form(Arc::new(RwLock::new(ancestor)))
                } else {
                    return Err(format!(
                        "Поле или предок '{seg}' не найдено в форме {}",
                        form_snapshot.type_name
                    ));
                }
            }
            Value::Atom(atom_arc) => {
                if seg == "Read" && is_zero_arg_call {
                    atom_arc.lock().unwrap().clone()
                } else if seg == "Value" {
                    atom_arc.lock().unwrap().clone()
                } else {
                    return Err(format!("Неизвестный метод атома: {seg}"));
                }
            }
            Value::Tuple(items) => {
                if let Ok(idx) = seg.parse::<i32>() {
                    let i = if idx < 0 {
                        (items.len() as i32 + idx) as usize
                    } else {
                        idx as usize
                    };
                    items
                        .get(i)
                        .cloned()
                        .ok_or_else(|| format!("Индекс {idx} вне границ кортежа"))?
                } else {
                    return Err(format!(
                        "Нельзя обратиться по строковому ключу '{seg}' к кортежу"
                    ));
                }
            }
            Value::String(s) if seg == "Length" => Value::Int(s.chars().count() as i64),
            _ => {
                return Err(format!(
                    "Нельзя получить поле '{seg}' из не-структурного значения {current:?}"
                ))
            }
        };

        current = next_val;
    }

    Ok(current)
}

fn find_ancestor_recursive(form: &FormInstance, target: &str) -> Option<FormInstance> {
    if let Some(anc) = form.ancestors.get(target) {
        return Some(anc.clone());
    }
    for anc in form.ancestors.values() {
        if let Some(deep) = find_ancestor_recursive(anc, target) {
            return Some(deep);
        }
    }
    None
}

fn eval_reactive_field(
    form_snapshot: &FormInstance,
    form_arc: Arc<RwLock<FormInstance>>,
    stream_sents: &[Sentence],
    scope: &mut Scope,
) -> Result<Value, String> {
    let mut reactive_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
    reactive_scope.set("Self", Value::Form(form_arc.clone()));
    // Закон 1a: Проецируем все поля формы (включая каскадные реактивные поля!)
    for fname in &form_snapshot.field_order {
        if let Some(fval) = form_snapshot.fields.get(fname) {
            if let Value::Stream(dep_sents) = fval {
                if dep_sents != stream_sents {
                    if let Ok(computed) =
                        eval_reactive_field(form_snapshot, form_arc.clone(), dep_sents, scope)
                    {
                        reactive_scope.set(fname, computed);
                    }
                }
            } else {
                reactive_scope.set(fname, fval.clone());
            }
        }
    }
    eval_stream_body(
        stream_sents,
        Some(Value::Form(form_arc)),
        &mut reactive_scope,
    )
}

fn invoke_form_method(
    form_snapshot: &FormInstance,
    method: &Arc<FunctionInstance>,
    arg: Value,
    scope: &mut Scope,
) -> Result<Value, String> {
    match &method.body {
        FunctionBody::Native(native_fn) => native_fn(vec![arg], scope),
        FunctionBody::UserDefined {
            args,
            body,
            captured_env,
            overrides,
        } => {
            let mut call_scope = Scope::child(captured_env.clone());
            let self_val = Value::Form(Arc::new(RwLock::new(form_snapshot.clone())));
            call_scope.set("Self", self_val.clone());
            // Закон 1a: Поля формы доступны напрямую в ее методах и операторах!
            for (k, v) in &form_snapshot.fields {
                call_scope.set(k, v.clone());
            }
            for (k, v) in overrides {
                call_scope.set(k, v.clone());
            }
            bind_arguments(args, &arg, &mut call_scope)?;
            eval_stream_body(body, Some(arg), &mut call_scope)
        }
        FunctionBody::Composed(f1, f2) => {
            let mid = apply_callable(f1.clone(), arg, scope)?;
            apply_callable(f2.clone(), mid, scope)
        }
    }
}

// --- Вспомогательный парсер валюты (`$100.00`, `USD$500.00`) ---
fn parse_money(raw: &str) -> Option<(String, f64)> {
    let idx = raw.find('$')?;
    let prefix = &raw[..=idx];
    let num: f64 = raw[idx + 1..].parse().ok()?;
    Some((prefix.to_string(), num))
}

fn format_money(prefix: &str, amount: f64) -> Value {
    Value::Money(format!("{prefix}{amount:.2}"))
}

pub fn eval_binary_op_in_scope(
    left: Value,
    op: &str,
    right: Value,
    scope: &mut Scope,
) -> Result<Value, String> {
    // 1. Проверяем перегрузку операторов внутри Формы (`Vec2 { +(Other Vec2) [...] }`)
    if let Value::Form(ref f_arc) = left {
        let f_snap = f_arc.read().unwrap().clone();
        if let Some(method) = f_snap.methods.get(op) {
            return invoke_form_method(&f_snap, method, right, scope);
        }
    }

    match (left, op, right) {
        // Целые числа
        (Value::Int(a), "+", Value::Int(b)) => Ok(Value::Int(a + b)),
        (Value::Int(a), "-", Value::Int(b)) => Ok(Value::Int(a - b)),
        (Value::Int(a), "*", Value::Int(b)) => Ok(Value::Int(a * b)),
        (Value::Int(a), "/", Value::Int(b)) => {
            if b == 0 {
                Err("Деление на ноль".into())
            } else {
                Ok(Value::Int(a / b))
            }
        }
        (Value::Int(a), "%", Value::Int(b)) => Ok(Value::Int(a % b)),
        (Value::Int(a), "**", Value::Int(b)) => Ok(Value::Int(a.pow(b.max(0) as u32))),
        (Value::Int(a), "<<", Value::Int(b)) => Ok(Value::Int(a << b)),
        (Value::Int(a), ">>", Value::Int(b)) => Ok(Value::Int(a >> b)),

        // Вещественные числа и смешанная арифметика Int/Float
        (Value::Float(a), "+", Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Float(a), "-", Value::Float(b)) => Ok(Value::Float(a - b)),
        (Value::Float(a), "*", Value::Float(b)) => Ok(Value::Float(a * b)),
        (Value::Float(a), "/", Value::Float(b)) => Ok(Value::Float(a / b)),
        (Value::Float(a), "**", Value::Float(b)) => Ok(Value::Float(a.powf(b))),

        (Value::Int(a), "+", Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
        (Value::Float(a), "+", Value::Int(b)) => Ok(Value::Float(a + b as f64)),
        (Value::Int(a), "-", Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
        (Value::Float(a), "-", Value::Int(b)) => Ok(Value::Float(a - b as f64)),
        (Value::Int(a), "*", Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
        (Value::Float(a), "*", Value::Int(b)) => Ok(Value::Float(a * b as f64)),
        (Value::Int(a), "/", Value::Float(b)) => Ok(Value::Float(a as f64 / b)),
        (Value::Float(a), "/", Value::Int(b)) => Ok(Value::Float(a / b as f64)),

        // Валюта (Money) и Проценты (Percent): Раздел 1 и Раздел 9 (Reactive Invoice)
        (Value::Money(m), "*", Value::Int(q)) | (Value::Int(q), "*", Value::Money(m)) => {
            let (pref, val) = parse_money(&m).ok_or("Некорректный формат Money")?;
            Ok(format_money(&pref, val * (q as f64)))
        }
        (Value::Money(m), "*", Value::Float(q)) | (Value::Float(q), "*", Value::Money(m)) => {
            let (pref, val) = parse_money(&m).ok_or("Некорректный формат Money")?;
            Ok(format_money(&pref, val * q))
        }
        (Value::Money(m), "*", Value::Percent(p)) | (Value::Percent(p), "*", Value::Money(m)) => {
            let (pref, val) = parse_money(&m).ok_or("Некорректный формат Money")?;
            Ok(format_money(&pref, val * (p / 100.0)))
        }
        (Value::Money(m1), "+", Value::Money(m2)) => {
            let (pref, v1) = parse_money(&m1).ok_or("Некорректный формат Money")?;
            let (_, v2) = parse_money(&m2).ok_or("Некорректный формат Money")?;
            Ok(format_money(&pref, v1 + v2))
        }
        (Value::Money(m1), "-", Value::Money(m2)) => {
            let (pref, v1) = parse_money(&m1).ok_or("Некорректный формат Money")?;
            let (_, v2) = parse_money(&m2).ok_or("Некорректный формат Money")?;
            Ok(format_money(&pref, v1 - v2))
        }

        // Сравнения
        (Value::Int(a), ">=", Value::Int(b)) => Ok(Value::Bool(a >= b)),
        (Value::Int(a), "<=", Value::Int(b)) => Ok(Value::Bool(a <= b)),
        (Value::Int(a), ">", Value::Int(b)) => Ok(Value::Bool(a > b)),
        (Value::Int(a), "<", Value::Int(b)) => Ok(Value::Bool(a < b)),

        (Value::Float(a), ">=", Value::Float(b)) => Ok(Value::Bool(a >= b)),
        (Value::Float(a), "<=", Value::Float(b)) => Ok(Value::Bool(a <= b)),
        (Value::Float(a), ">", Value::Float(b)) => Ok(Value::Bool(a > b)),
        (Value::Float(a), "<", Value::Float(b)) => Ok(Value::Bool(a < b)),

        (a, "==", b) => Ok(Value::Bool(a == b)),
        (a, "!=", b) => Ok(Value::Bool(a != b)),

        (Value::Bool(a), "&&", Value::Bool(b)) => Ok(Value::Bool(a && b)),
        (Value::Bool(a), "||", Value::Bool(b)) => Ok(Value::Bool(a || b)),

        (Value::String(a), "+", Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),

        // 1. Слияние форм `&` (Закон 2: Полное сохранение Lineage и фиксация коллизий)
        (Value::Form(f1), "&", Value::Form(f2)) => {
            let f1_read = f1.read().unwrap();
            let f2_read = f2.read().unwrap();

            let is_override = f2_read.type_name == "AnonymousForm";
            let new_type_name = if is_override {
                f1_read.type_name.clone()
            } else {
                format!("{}+{}", f1_read.type_name, f2_read.type_name)
            };

            let mut merged = FormInstance::new(&new_type_name);
            merged.ancestors = f1_read.ancestors.clone();
            for (k, v) in &f2_read.ancestors {
                merged.ancestors.insert(k.clone(), v.clone());
            }
            merged
                .ancestors
                .insert(f1_read.type_name.clone(), f1_read.clone());
            if !is_override {
                merged
                    .ancestors
                    .insert(f2_read.type_name.clone(), f2_read.clone());
            }

            merged.methods = f1_read.methods.clone();
            for (m_name, m_fn) in &f2_read.methods {
                merged.methods.insert(m_name.clone(), m_fn.clone());
            }
            merged.predicates = f1_read.predicates.clone();
            merged.predicates.extend(f2_read.predicates.clone());
            merged.ambiguous_fields = f1_read.ambiguous_fields.clone();

            for k in &f1_read.field_order {
                let orig = f1_read
                    .field_origins
                    .get(k)
                    .cloned()
                    .unwrap_or_else(|| f1_read.type_name.clone());
                merged.set_field_with_origin(k, f1_read.fields.get(k).unwrap().clone(), &orig);
            }

            for k in &f2_read.field_order {
                let rhs_orig = f2_read
                    .field_origins
                    .get(k)
                    .cloned()
                    .unwrap_or_else(|| f2_read.type_name.clone());
                let rhs_val = f2_read.fields.get(k).unwrap().clone();

                if is_override {
                    // Анонимный блок `& { Quantity 5 }` переопределяет значение поля без коллизии
                    merged.set_field_with_origin(k, rhs_val, &f1_read.type_name);
                } else if let Some(lhs_orig) = merged.field_origins.get(k).cloned() {
                    if lhs_orig != rhs_orig {
                        merged
                            .ambiguous_fields
                            .entry(k.clone())
                            .or_insert_with(|| vec![lhs_orig])
                            .push(rhs_orig.clone());
                    }
                    merged.fields.insert(k.clone(), rhs_val);
                } else {
                    merged.set_field_with_origin(k, rhs_val, &rhs_orig);
                }
            }

            Ok(Value::Form(Arc::new(RwLock::new(merged))))
        }

        // 2. Уточняющий тип (Refinement Type): `Adult User & IsAdult` или `EmailStr String & [ ... ]`
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

        // 3. Перепривязка именованных аргументов функции: `Connect & { Port 9090, Tls true }`
        (Value::Function(func_arc), "&", Value::Form(override_arc)) => {
            let override_form = override_arc.read().unwrap();
            if let FunctionBody::UserDefined {
                args,
                body,
                captured_env,
                overrides,
            } = &func_arc.body
            {
                let mut new_overrides = overrides.clone();
                for (k, v) in &override_form.fields {
                    new_overrides.insert(k.clone(), v.clone());
                }
                Ok(Value::Function(Arc::new(FunctionInstance {
                    name: func_arc.name.clone(),
                    body: FunctionBody::UserDefined {
                        args: args.clone(),
                        body: body.clone(),
                        captured_env: captured_env.clone(),
                        overrides: new_overrides,
                    },
                })))
            } else {
                Ok(Value::Function(func_arc))
            }
        }

        // 4. Композиция потоков и функций: `Step1 & Step2`, `Trim & ToUpper`
        (
            f1 @ (Value::Function(_) | Value::Stream(_)),
            "&",
            f2 @ (Value::Function(_) | Value::Stream(_)),
        ) => Ok(Value::Function(Arc::new(FunctionInstance {
            name: "ComposedStream".into(),
            body: FunctionBody::Composed(f1, f2),
        }))),

        // 5. Метаданные видимости `Val & Private` / `Val & Public` -> возвращает само значение
        (val, "&", Value::Form(_)) => Ok(val),

        // 6. Объединение типов и тегов `A | B` (`'admin' | 'manager'`, `Circle | Rect`)
        (left_val, "|", right_val) => {
            let mut union_form = FormInstance::new("UnionType");
            for v in [left_val, right_val] {
                if let Value::Form(f_arc) = &v {
                    let f_read = f_arc.read().unwrap();
                    if !f_read.union_variants.is_empty() {
                        union_form
                            .union_variants
                            .extend(f_read.union_variants.clone());
                        continue;
                    }
                }
                union_form.union_variants.push(v);
            }
            Ok(Value::Form(Arc::new(RwLock::new(union_form))))
        }

        // 7. Алгебраическое вычитание полей и предков `/` (Раздел 2 и 8)
        (Value::Form(f), "/", Value::Symbol(sym) | Value::String(sym)) => {
            let mut form = f.read().unwrap().clone();
            form.fields.remove(&sym);
            form.field_order.retain(|k| k != &sym);
            form.field_origins.remove(&sym);
            form.ambiguous_fields.remove(&sym);
            for anc in form.ancestors.values_mut() {
                anc.fields.remove(&sym);
                anc.field_order.retain(|k| k != &sym);
            }
            Ok(Value::Form(Arc::new(RwLock::new(form))))
        }
        (Value::Form(f), "/", Value::Form(rhs_arc)) => {
            let rhs_name = rhs_arc.read().unwrap().type_name.clone();
            let form = subtract_ancestor_or_path(&f.read().unwrap(), &rhs_name);
            Ok(Value::Form(Arc::new(RwLock::new(form))))
        }

        (a, op, b) => Err(format!(
            "Оператор '{op}' не поддерживается для {a:?} и {b:?}"
        )),
    }
}

/// Вычитание конкретного поля предка (`UserAndCloud / User.Name`) или целого предка (`UserAndCloud / Cloud`)
fn subtract_ancestor_or_path(source: &FormInstance, target_path: &str) -> FormInstance {
    let mut form = source.clone();
    if let Some((anc_name, field_name)) = target_path.split_once('.') {
        // Случай C: `UserAndCloud / User.Name` -> снимает коллизию и активирует значение оставшегося предка!
        if let Some(anc) = form.ancestors.get_mut(anc_name) {
            anc.fields.remove(field_name);
            anc.field_order.retain(|k| k != field_name);
        }
        if let Some(origins) = form.ambiguous_fields.get_mut(field_name) {
            origins.retain(|o| o != anc_name);
            if origins.len() == 1 {
                let remaining_anc = origins[0].clone();
                form.ambiguous_fields.remove(field_name);
                form.field_origins
                    .insert(field_name.to_string(), remaining_anc.clone());
                if let Some(rem_inst) = form.ancestors.get(&remaining_anc) {
                    if let Some(rem_val) = rem_inst.get_field(field_name) {
                        form.fields.insert(field_name.to_string(), rem_val);
                    }
                }
            } else if origins.is_empty() {
                form.ambiguous_fields.remove(field_name);
                form.fields.remove(field_name);
                form.field_order.retain(|k| k != field_name);
            }
        } else {
            form.fields.remove(field_name);
            form.field_order.retain(|k| k != field_name);
            form.field_origins.remove(field_name);
        }
    } else {
        // Случай E: `UserAndCloud / Cloud` -> удаляет весь срез предка Cloud и его поля
        form.ancestors.remove(target_path);
        let keys_to_check: Vec<String> = form.field_order.clone();
        for k in keys_to_check {
            if let Some(origins) = form.ambiguous_fields.get_mut(&k) {
                origins.retain(|o| o != target_path);
                if origins.len() == 1 {
                    let winner = origins[0].clone();
                    form.ambiguous_fields.remove(&k);
                    form.field_origins.insert(k.clone(), winner.clone());
                    if let Some(w_inst) = form.ancestors.get(&winner) {
                        if let Some(w_val) = w_inst.get_field(&k) {
                            form.fields.insert(k.clone(), w_val);
                        }
                    }
                }
            } else if form.field_origins.get(&k).map(|s| s.as_str()) == Some(target_path) {
                form.fields.remove(&k);
                form.field_order.retain(|f| f != &k);
                form.field_origins.remove(&k);
            }
        }
    }
    form
}

// =============================================================================
// 4. ПАЙПЛАЙНЫ, МАРШРУТИЗАТОР И ТРИ ЖЕЛЕЗНЫХ ЗАКОНА
// =============================================================================

pub fn eval_expression(tokens: &[Token], scope: &mut Scope) -> Result<Value, String> {
    if tokens.is_empty() {
        return Ok(Value::None);
    }

    // 1. Пайплайны данных (`.`, `.>`, `.&`, `.?`, `.|`)
    if has_pipe_op(tokens) {
        let (head, steps) = split_pipeline(tokens);
        let mut current = eval_expression(&head, scope)?;
        for step in steps {
            current = eval_pipe_step(current, step, scope)?;
        }
        return Ok(current);
    }

    // 2. Унарный оператор `!X`
    if let [Token::Op(op), rest @ ..] = tokens {
        if op == "!" && !rest.is_empty() {
            let val = eval_expression(rest, scope)?;
            return match val {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                _ => Err(format!("Унарный оператор '!' не применим к {val:?}")),
            };
        }
    }

    // 3. Вызов функции `Connect("db.internal")`, `RunBuild()`, `Heading("...")`
    if let [Token::Path(fn_name), Token::Group(')', args)] = tokens {
        return eval_call_expr(fn_name, args, None, scope);
    }

    // 4. Инстанцирование именованной формы (с возможным хвостом `&` или `/`):
    // `User { Name "Alice", Age 22 }` или `JonnName { ParentName JonnDoy } & { ParentName "JonnDoyBoy" }`
    if tokens.len() >= 2 {
        if let (Token::Path(type_name), Token::Group('{', sentences)) = (&tokens[0], &tokens[1]) {
            let mut form = if let Some(Value::Form(proto_arc)) = scope.get(type_name) {
                let base_inst = proto_arc.read().unwrap().clone();
                let mut child = base_inst.clone();
                child.ancestors.insert(type_name.clone(), base_inst);
                child
            } else {
                FormInstance::new(type_name)
            };
            form.type_name = type_name.clone();
            populate_form_members(&mut form, sentences, scope)?;

            let mut current = Value::Form(Arc::new(RwLock::new(form)));
            if tokens.len() > 2 {
                current = eval_operator_tail(current, &tokens[2..], scope)?;
            }
            return Ok(current);
        }
    }

    // 5. Одиночный токен
    if tokens.len() == 1 {
        return eval_token(&tokens[0], scope);
    }

    // 6. Выражение с бинарными операторами и приоритетами (`3.14 * R * R`, `A + B`, `User & Cart`, `User / User.PasswordHash`)
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
    // Находим оператор с наименьшим приоритетом (вычисляем слева направо)
    let mut lowest_idx: Option<usize> = None;
    let mut lowest_prec: u8 = u8::MAX;

    for (i, tok) in tokens.iter().enumerate() {
        if i == 0 {
            continue;
        }
        if let Token::Op(op) = tok {
            // Если `/` применяется к числам — у него приоритет мультипликативный (7), если к формам — алгебраический (2)
            let prec = if op == "/" {
                if matches!(tokens.first(), Some(Token::Path(p)) if scope.get(p).map(|v| matches!(v, Value::Form(_))).unwrap_or(false))
                {
                    2
                } else {
                    7
                }
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

            // Специальная обработка структурного вычитания по пути: `User / User.PasswordHash` или `UserAndCloud / Cloud`
            if op == "/" {
                if let (Value::Form(f_arc), [Token::Path(path_str)]) = (&left, rhs_tokens) {
                    let updated = subtract_ancestor_or_path(&f_arc.read().unwrap(), path_str);
                    return Ok(Value::Form(Arc::new(RwLock::new(updated))));
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
            if let Some(rhs_tok) = tail.get(i + 1) {
                if op == "/" {
                    if let (Value::Form(f_arc), Token::Path(path_str)) = (&current, rhs_tok) {
                        let updated = subtract_ancestor_or_path(&f_arc.read().unwrap(), path_str);
                        current = Value::Form(Arc::new(RwLock::new(updated)));
                        i += 2;
                        continue;
                    }
                }
                let rhs = eval_token(rhs_tok, scope)?;
                current = eval_binary_op_in_scope(current, op, rhs, scope)?;
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    Ok(current)
}

fn eval_pipe_step(input: Value, step: PipeStep, scope: &mut Scope) -> Result<Value, String> {
    // Закон 3 (Railway-free): Автоматический пропуск None
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
            // Закон 3: Если целевая функция обогащения (`.& Db.FindCart`) не принимает текущий тип,
            // пропускаем шаг без вызова!
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
            let res = apply_callable(pred, input.clone(), scope)?;
            match res {
                Value::Bool(true) => Ok(input),
                _ => Ok(Value::None),
            }
        }
        PipeOp::Parallel => {
            if let Some(Token::Group('{', branches)) = step.expr.first() {
                let mut handles: Vec<std::thread::JoinHandle<Result<(String, Value), String>>> = Vec::new();
                for branch in branches {
                    let branch_clone = branch.clone();
                    let input_clone = input.clone();
                    let parent_scope = Arc::new(RwLock::new(scope.clone()));

                    let handle = thread::spawn(move || {
                        let mut local_scope = Scope::child(parent_scope);
                        local_scope.set("_", input_clone.clone());
                        if let Value::Form(f) = &input_clone {
                            for (k, v) in &f.read().unwrap().fields {
                                local_scope.set(k, v.clone());
                            }
                        }
                        match classify_sentence(&branch_clone) {
                            SemanticKind::StreamOrBranch { name, stream } => {
                                let v = eval_stream_body(
                                    &stream,
                                    Some(input_clone),
                                    &mut local_scope,
                                )?;
                                Ok((name, v))
                            }
                            _ => {
                                let v = eval_sentence(&branch_clone, &mut local_scope)?;
                                Ok(("Result".to_string(), v))
                            }
                        }
                    });
                    handles.push(handle);
                }

                let mut merged = FormInstance::new("ParallelResult");
                for h in handles {
                    if let Ok(Ok((field_name, val))) = h.join() {
                        merged.set_field(&field_name, val);
                    }
                }
                Ok(Value::Form(Arc::new(RwLock::new(merged))))
            } else {
                Err("Оператор .| ожидает блок {}".into())
            }
        }
    }
}

/// Вычисляет шаг пайплайна с поддержкой UFCS `A . Fn(B, C)` и предикатов диапазонов `Val . (1..100)`
fn eval_pipe_target(input: Value, expr: &[Token], scope: &mut Scope) -> Result<Value, String> {
    // UFCS: `Items . Fold(0, (Acc, N) [ ... ])` или `Line . Split(" ")`
    if let [Token::Path(fn_name), Token::Group(')', call_args)] = expr {
        return eval_call_expr(fn_name, call_args, Some(input), scope);
    }

    let mut target_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
    target_scope.set("_", input.clone());
    if let Value::Form(f_arc) = &input {
        for (k, v) in &f_arc.read().unwrap().fields {
            target_scope.set(k, v.clone());
        }
    }

    let target = eval_expression(expr, &mut target_scope)?;
    apply_callable(target, input, scope)
}

/// Проверка Закона 3 (Railway-free): Соответствует ли входное значение контракту первого параметра функции?
fn callable_accepts_input(callable: &Value, input: &Value, scope: &mut Scope) -> bool {
    if let Value::Function(func_arc) = callable {
        if let FunctionBody::UserDefined { args, .. } = &func_arc.body {
            if let Some(first_param) = args.first() {
                let type_tokens: &[Token] = match first_param.as_slice() {
                    [Token::Path(single)] => {
                        let is_type = scope
                            .get(single)
                            .map(|v| matches!(v, Value::Form(_)))
                            .unwrap_or(false);
                        if is_type {
                            first_param.as_slice()
                        } else {
                            &[]
                        }
                    }
                    [Token::Path(_), Token::Op(op), ..]
                    if op == "&" || op == "|" || op == "/" =>
                        {
                            first_param.as_slice()
                        }
                    [Token::Path(_), rest @ ..] => {
                        // Если второй токен — это дефолтный литерал `Host "localhost"`, то это не ограничение типа Формы
                        if matches!(rest.first(), Some(Token::Lit(_) | Token::Group(_, _))) {
                            &[]
                        } else {
                            rest
                        }
                    }
                    _ => &[],
                };

                if !type_tokens.is_empty() {
                    return value_matches_type_tokens(input, type_tokens, scope);
                }
            }
        }
    }
    true
}

fn value_matches_type_tokens(val: &Value, tokens: &[Token], scope: &mut Scope) -> bool {
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Path(type_name) => {
                // Игнорируем базовые примитивные аннотации вроде `String`, `Int`, `Any`
                if matches!(
                    type_name.as_str(),
                    "String" | "Int" | "Float" | "Bool" | "Any" | "Tuple" | "File"
                ) {
                    return true;
                }
                if let Value::Form(val_arc) = val {
                    let val_form = val_arc.read().unwrap();
                    if !val_form.has_lineage(type_name) {
                        // Проверим, не является ли `type_name` Union-типом (`Shape Circle | Rect`)
                        if let Some(Value::Form(target_arc)) = scope.get(type_name) {
                            let target_form = target_arc.read().unwrap();
                            if !target_form.union_variants.is_empty() {
                                let matches_union = target_form.union_variants.iter().any(|v| {
                                    if let Value::Form(vf) = v {
                                        val_form.has_lineage(&vf.read().unwrap().type_name)
                                    } else {
                                        false
                                    }
                                });
                                if !matches_union {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                }
            }
            Token::Op(op) if op == "/" => {
                // `Order / Cancelled` -> запрещает вход формы, имеющей предка `Cancelled`
                if let Some(Token::Path(forbidden)) = tokens.get(i + 1) {
                    if let Value::Form(val_arc) = val {
                        if val_arc.read().unwrap().has_lineage(forbidden) {
                            return false;
                        }
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
    // Закон 3 (Outcome Bypass): Если входное значение является альтернативной веткой Union (например, `OutOfStock`
    // вместо ожидаемого `Order & Reserved`), функция автоматически пропускается!
    if !callable_accepts_input(&callable, &arg, scope) {
        return Ok(arg);
    }

    match callable {
        Value::Function(func) => match &func.body {
            FunctionBody::Native(native_fn) => {
                let args_vec = match arg {
                    Value::Tuple(items) => items,
                    Value::None => vec![],
                    other => vec![other],
                };
                native_fn(args_vec, scope)
            }
            FunctionBody::UserDefined {
                args,
                body,
                captured_env,
                overrides,
            } => {
                let mut call_scope = Scope::child(captured_env.clone());
                bind_arguments(args, &arg, &mut call_scope)?;
                for (k, v) in overrides {
                    call_scope.set(k, v.clone());
                }
                eval_stream_body(body, Some(arg), &mut call_scope)
            }
            FunctionBody::Composed(f1, f2) => {
                let mid = apply_callable(f1.clone(), arg, scope)?;
                apply_callable(f2.clone(), mid, scope)
            }
        },
        Value::Stream(sentences) => {
            let mut stream_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
            stream_scope.set("_", arg.clone());
            // Закон 1c: В анонимный поток `[ ... ]` поля входной формы проецируются автоматически!
            if let Value::Form(form_arc) = &arg {
                let form = form_arc.read().unwrap();
                for (k, v) in &form.fields {
                    if let Value::Stream(dep_sents) = v {
                        if let Ok(computed) =
                            eval_reactive_field(&form, form_arc.clone(), dep_sents, scope)
                        {
                            stream_scope.set(k, computed);
                        }
                    } else {
                        stream_scope.set(k, v.clone());
                    }
                }
            }
            eval_stream_body(&sentences, Some(arg), &mut stream_scope)
        }
        Value::Router(branches) => eval_router(&branches, arg, scope),
        // Проверка вхождения в диапазон: `Val . (1..100)`
        Value::Range(start, end) => match arg {
            Value::Int(v) => Ok(Value::Bool(v >= start && v <= end)),
            _ => Ok(Value::Bool(false)),
        },
        Value::Form(router_arc) => {
            let router = router_arc.read().unwrap().clone();
            // Проверка вхождения в Union: `_.Role . ('admin' | 'manager')`
            if !router.union_variants.is_empty() {
                return Ok(Value::Bool(router.union_variants.contains(&arg)));
            }
            for key in &router.field_order {
                if let Some(Value::Stream(body)) = router.fields.get(key) {
                    let mut branch_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                    if match_type_or_wildcard(key, &arg, &mut branch_scope) {
                        return eval_stream_body(body, Some(arg), &mut branch_scope);
                    }
                }
            }
            Ok(Value::None)
        }
        _ => Err(format!("Значение {callable:?} не может быть вызвано")),
    }
}

fn eval_call_expr(
    fn_name: &str,
    call_args: &[Sentence],
    piped_first_arg: Option<Value>,
    scope: &mut Scope,
) -> Result<Value, String> {
    let mut evaluated_args = Vec::new();
    if let Some(first) = piped_first_arg {
        evaluated_args.push(first);
    }
    for a in call_args {
        evaluated_args.push(eval_sentence(a, scope)?);
    }

    // Специальная поддержка `Atom.Swap(Transition)`
    if let Some((obj_name, method_name)) = fn_name.split_once('.') {
        if method_name == "Swap" {
            if let Some(Value::Atom(atom_arc)) = scope.get(obj_name) {
                let transition = evaluated_args.into_iter().next().unwrap_or(Value::None);
                let current_val = atom_arc.lock().unwrap().clone();
                let next_val = apply_callable(transition, current_val, scope)?;
                *atom_arc.lock().unwrap() = next_val.clone();
                return Ok(next_val);
            } else if let Some(Value::Form(f_arc)) = scope.get(obj_name) {
                let transition = evaluated_args.into_iter().next().unwrap_or(Value::None);
                let current_val = f_arc
                    .read()
                    .unwrap()
                    .get_field("Value")
                    .unwrap_or(Value::None);
                let next_val = apply_callable(transition, current_val, scope)?;
                f_arc.write().unwrap().set_field("Value", next_val.clone());
                return Ok(next_val);
            }
        }
    }

    let func = eval_path(fn_name, scope)?;
    let arg = if evaluated_args.is_empty() {
        Value::None
    } else if evaluated_args.len() == 1 {
        evaluated_args.remove(0)
    } else {
        Value::Tuple(evaluated_args)
    };
    apply_callable(func, arg, scope)
}

/// Вычисление паттерн-матчинга `{ [ _ >= 18 ] [ ... ], (0, 0) [ ... ], User [ ... ], _ [ ... ] }`
fn eval_router(branches: &[Sentence], arg: Value, scope: &mut Scope) -> Result<Value, String> {
    for branch in branches {
        match classify_sentence(branch) {
            // 1. Ветка с предикатом `[ _ >= 18 ] [ ... ]`, структурой `{ Method 'GET', Path ("users", Id) }` или литералом `true [ ... ]`
            SemanticKind::PatternBranch { pattern, body } => {
                let mut branch_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                branch_scope.set("_", arg.clone());
                if let Value::Form(f) = &arg {
                    for (k, v) in &f.read().unwrap().fields {
                        branch_scope.set(k, v.clone());
                    }
                }

                let matched = match pattern {
                    Token::Lit(lit) => {
                        let expected = eval_literal(&lit, &mut branch_scope);
                        arg == expected
                    }
                    Token::Group('[', pred_sents) => {
                        let res =
                            eval_stream_body(&pred_sents, Some(arg.clone()), &mut branch_scope)?;
                        matches!(res, Value::Bool(true))
                    }
                    Token::Group('{', field_pats) => {
                        if let Value::Form(f_arc) = &arg {
                            let f = f_arc.read().unwrap();
                            let mut all_ok = true;
                            for fp in field_pats {
                                if let SemanticKind::Binding { key, value } = classify_sentence(&fp)
                                {
                                    let Some(actual_field) = f.get_field(&key) else {
                                        all_ok = false;
                                        break;
                                    };
                                    // Поддержка вложенной деструктуризации кортежа `Path ("users", Id)`
                                    if let [Token::Group('(' | ')', tuple_pats)] = value.as_slice()
                                    {
                                        if !match_tuple_pattern(
                                            tuple_pats,
                                            &actual_field,
                                            &mut branch_scope,
                                        )? {
                                            all_ok = false;
                                            break;
                                        }
                                    } else {
                                        let expected = eval_expression(&value, &mut branch_scope)?;
                                        if actual_field != expected {
                                            all_ok = false;
                                            break;
                                        }
                                    }
                                }
                            }
                            all_ok
                        } else {
                            false
                        }
                    }
                    _ => false,
                };

                if matched {
                    return eval_stream_body(&body, Some(arg), &mut branch_scope);
                }
            }

            // 2. Ветка сопоставления кортежа `(0, 0) [ "FizzBuzz" ]`, `(0, _) [ "Fizz" ]`
            SemanticKind::LambdaOrTupleBranch { args, body } => {
                let mut branch_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                branch_scope.set("_", arg.clone());
                if match_tuple_pattern(&args, &arg, &mut branch_scope)? {
                    return eval_stream_body(&body, Some(arg), &mut branch_scope);
                }
            }

            // 3. Ветка по имени типа `User [ ... ]`, `Adult [ ... ]` или дефолтная ветка `_ [ ... ]`
            SemanticKind::StreamOrBranch { name, stream } => {
                let mut branch_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                if match_type_or_wildcard(&name, &arg, &mut branch_scope) {
                    return eval_stream_body(&stream, Some(arg), &mut branch_scope);
                }
            }

            _ => {}
        }
    }
    Ok(Value::None)
}

fn match_tuple_pattern(
    pat_sents: &[Sentence],
    actual: &Value,
    scope: &mut Scope,
) -> Result<bool, String> {
    let actual_items = match actual {
        Value::Tuple(items) => items.clone(),
        other => vec![other.clone()],
    };
    if pat_sents.len() != actual_items.len() {
        return Ok(false);
    }
    for (pat_s, act_val) in pat_sents.iter().zip(actual_items.iter()) {
        match pat_s.as_slice() {
            [Token::Path(var_name)] if var_name == "_" => {}
            [Token::Path(var_name)] => {
                // Закон 1b: Извлечение переменной из кортежа `("users", Id)` в скоуп ветки!
                scope.set(var_name, act_val.clone());
            }
            _ => {
                let expected = eval_sentence(pat_s, scope)?;
                if act_val != &expected {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

fn match_type_or_wildcard(pattern_name: &str, val: &Value, scope: &mut Scope) -> bool {
    if pattern_name == "_" {
        scope.set("_", val.clone());
        return true;
    }
    match val {
        Value::Form(form_arc) => {
            let form = form_arc.read().unwrap().clone();
            let mut matched = form.has_lineage(pattern_name);

            // Если в скоупе определен уточняющий тип `Adult User & IsAdult`, проверяем его предков и предикаты!
            if let Some(Value::Form(target_type_arc)) = scope.get(pattern_name) {
                let target_type = target_type_arc.read().unwrap().clone();
                if !matched {
                    matched = target_type
                        .ancestors
                        .keys()
                        .all(|anc| form.has_lineage(anc));
                }
                if matched && !target_type.predicates.is_empty() {
                    for pred in &target_type.predicates {
                        let mut pred_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                        if !matches!(
                            apply_callable(pred.clone(), val.clone(), &mut pred_scope),
                            Ok(Value::Bool(true))
                        ) {
                            return false;
                        }
                    }
                }
            }

            if matched {
                // Закон 1b: Проекция всех не-коллизионных полей в скоуп при матчинге по типу
                for (k, v) in &form.fields {
                    if !form.ambiguous_fields.contains_key(k) {
                        scope.set(k, v.clone());
                    }
                }
                // Также делаем доступными срезы предков для разрешения коллизий Закона 2 (`User.Id`, `Order.Id`)
                for (anc_name, anc_inst) in &form.ancestors {
                    scope.set(
                        anc_name,
                        Value::Form(Arc::new(RwLock::new(anc_inst.clone()))),
                    );
                }
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

/// Привязка аргументов с соблюдением Закона 1, Закона 1c и дефолтных значений
fn bind_arguments(
    arg_defs: &[Sentence],
    incoming: &Value,
    scope: &mut Scope,
) -> Result<(), String> {
    scope.set("_", incoming.clone());

    if arg_defs.is_empty() {
        // Без аргументов (поток) -> авто-распаковка формы
        if let Value::Form(form_arc) = incoming {
            let form = form_arc.read().unwrap();
            for (k, v) in &form.fields {
                if !form.ambiguous_fields.contains_key(k) {
                    scope.set(k, v.clone());
                }
            }
        }
        return Ok(());
    }

    // Если функция ожидает несколько параметров `(A, B)`, а пришел `Tuple(items)`
    let incoming_items: Vec<Value> = match incoming {
        Value::Tuple(items) if arg_defs.len() > 1 => items.clone(),
        Value::None => vec![],
        other => vec![other.clone()],
    };

    for (idx, param_sent) in arg_defs.iter().enumerate() {
        let provided_val = incoming_items.get(idx).cloned();

        match param_sent.as_slice() {
            // Закон 1c: Одиночный идентификатор `Fn(User)` или обычный слот `Fn(Msg)`
            [Token::Path(single)] => {
                let val = provided_val.unwrap_or(Value::None);
                let is_type = single
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                    && matches!(val, Value::Form(_));
                scope.set(single, val.clone());
                if is_type {
                    if let Value::Form(form_arc) = &val {
                        let form = form_arc.read().unwrap();
                        for (k, v) in &form.fields {
                            if !form.ambiguous_fields.contains_key(k) {
                                scope.set(k, v.clone());
                            }
                        }
                    }
                }
            }
            // Закон 1c: Анонимный пересеченный слот `FinalizeBill(User & Cart)` -> распаковывает все поля!
            [Token::Path(_), Token::Op(op), ..] if op == "&" || op == "|" => {
                let val = provided_val.unwrap_or(Value::None);
                if let Value::Form(form_arc) = &val {
                    let form = form_arc.read().unwrap();
                    for (k, v) in &form.fields {
                        if !form.ambiguous_fields.contains_key(k) {
                            scope.set(k, v.clone());
                        }
                    }
                }
            }
            // Закон 1: Именованный параметр `Fn(U User)` или дефолтный `Connect(Host "localhost")`
            [Token::Path(param_name), rest @ ..] => {
                if let Some(val) = provided_val {
                    scope.set(param_name, val);
                } else if !rest.is_empty() {
                    let default_val = eval_expression(rest, scope)?;
                    scope.set(param_name, default_val);
                } else {
                    scope.set(param_name, Value::None);
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn eval_stream_body(
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
        // Функциональный ранний выход `Done { Value _ }`
        if let Value::Done(val) = last {
            return Ok(*val);
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
// 5. ОБРАБОТКА ПРЕДЛОЖЕНИЙ (SENTENCES)
// =============================================================================

pub fn eval_sentence(sentence: &Sentence, scope: &mut Scope) -> Result<Value, String> {
    let kind = classify_sentence(sentence);

    match kind {
        SemanticKind::LambdaOrTupleBranch { args, body } => {
            let func = FunctionInstance {
                name: "Lambda".into(),
                body: FunctionBody::UserDefined {
                    args,
                    body,
                    captured_env: Arc::new(RwLock::new(scope.clone())),
                    overrides: HashMap::new(),
                },
            };
            Ok(Value::Function(Arc::new(func)))
        }

        SemanticKind::StreamOrBranch { name, stream } => {
            let func = FunctionInstance {
                name: name.clone(),
                body: FunctionBody::UserDefined {
                    args: Vec::new(),
                    body: stream,
                    captured_env: Arc::new(RwLock::new(scope.clone())),
                    overrides: HashMap::new(),
                },
            };
            let val = Value::Function(Arc::new(func));
            scope.set(&name, val.clone());
            Ok(val)
        }

        SemanticKind::Binding { key, value } => {
            let val = eval_expression(&value, scope)?;
            scope.set(&key, val.clone());
            Ok(val)
        }

        SemanticKind::TupleBinding { tuple, value } => {
            let val = eval_expression(&value, scope)?;
            let _ = match_tuple_pattern(&tuple, &val, scope)?;
            Ok(val)
        }

        SemanticKind::FormDef { name, body, tail } => {
            let mut form = FormInstance::new(&name);
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
            let mut form = if let Some(Value::Form(base_arc)) = scope.get(&base) {
                let base_inst = base_arc.read().unwrap().clone();
                let mut child = base_inst.clone();
                child.type_name = name.clone();
                child.ancestors.insert(base.clone(), base_inst);
                child
            } else {
                FormInstance::new(&name)
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

        SemanticKind::AlgebraicDef { name, expr } => {
            let mut val = eval_expression(&expr, scope)?;
            if let Value::Form(f_arc) = &val {
                let mut f_write = f_arc.write().unwrap();
                let prev_clone = f_write.clone();
                if prev_clone.type_name != "UnionType" && prev_clone.type_name != name {
                    f_write
                        .ancestors
                        .insert(prev_clone.type_name.clone(), prev_clone);
                }
                f_write.type_name = name.clone();
            }
            scope.set(&name, val.clone());
            Ok(val)
        }

        SemanticKind::FunctionDef {
            name, args, body, ..
        } => {
            let func = FunctionInstance {
                name: name.clone(),
                body: FunctionBody::UserDefined {
                    args,
                    body,
                    captured_env: Arc::new(RwLock::new(scope.clone())),
                    overrides: HashMap::new(),
                },
            };
            let val = Value::Function(Arc::new(func));
            scope.set(&name, val.clone());
            Ok(val)
        }

        SemanticKind::CallExpr { name, args } => eval_call_expr(&name, &args, None, scope),

        SemanticKind::Pipeline(tokens) | SemanticKind::Expression(tokens) => {
            eval_expression(&tokens, scope)
        }

        _ => Ok(Value::None),
    }
}

// =============================================================================
// 6. СТАНДАРТНАЯ БИБЛИОТЕКА И ЗАПУСК ПРОГРАММЫ
// =============================================================================

fn register_native_fn<F>(scope: &mut Scope, name: &str, f: F)
where
    F: Fn(Vec<Value>, &mut Scope) -> Result<Value, String> + Send + Sync + 'static,
{
    scope.set(
        name,
        Value::Function(Arc::new(FunctionInstance {
            name: name.into(),
            body: FunctionBody::Native(Arc::new(f)),
        })),
    );
}

fn expand_iterable(val: Value) -> Vec<Value> {
    match val {
        Value::Tuple(items) => items,
        Value::Range(start, end) => (start..=end).map(Value::Int).collect(),
        Value::None => vec![],
        other => vec![other],
    }
}

pub fn create_global_scope() -> Scope {
    let mut scope = Scope::new();

    // 1. Shell.Log и Shell.Exec
    let mut shell = FormInstance::new("Shell");
    let log_native: NativeFn = Arc::new(|args, _| {
        if let Some(msg) = args.first() {
            println!("{msg}");
        }
        Ok(Value::None)
    });
    let log_fn = Arc::new(FunctionInstance {
        name: "Log".into(),
        body: FunctionBody::Native(log_native),
    });
    shell.methods.insert("Log".into(), log_fn.clone());
    shell.set_field("Log", Value::Function(log_fn));

    let exec_native: NativeFn = Arc::new(|args, _| {
        if let Some(cmd) = args.first() {
            println!("[Exec]: {cmd}");
        }
        Ok(Value::String("success".into()))
    });
    let exec_fn = Arc::new(FunctionInstance {
        name: "Exec".into(),
        body: FunctionBody::Native(exec_native),
    });
    shell.methods.insert("Exec".into(), exec_fn.clone());
    shell.set_field("Exec", Value::Function(exec_fn));

    scope.set("Shell", Value::Form(Arc::new(RwLock::new(shell))));

    // 2. Atom(T) и Done
    register_native_fn(&mut scope, "Atom", |args, _| {
        let initial = args.first().cloned().unwrap_or(Value::None);
        Ok(Value::Atom(Arc::new(Mutex::new(initial))))
    });
    scope.set(
        "Done",
        Value::Form(Arc::new(RwLock::new(FormInstance::new("Done")))),
    );

    // 3. Встроенные функции работы со строками (`Trim`, `ToUpper`, `Split`, `Join`, `Length`)
    register_native_fn(&mut scope, "Trim", |args, _| match args.first() {
        Some(Value::String(s)) => Ok(Value::String(s.trim().to_string())),
        other => Ok(other.cloned().unwrap_or(Value::None)),
    });
    register_native_fn(&mut scope, "ToUpper", |args, _| match args.first() {
        Some(Value::String(s)) => Ok(Value::String(s.to_uppercase())),
        other => Ok(other.cloned().unwrap_or(Value::None)),
    });
    register_native_fn(&mut scope, "Split", |args, _| {
        if let (Some(Value::String(s)), Some(Value::String(delim))) = (args.first(), args.get(1)) {
            let parts = s
                .split(delim.as_str())
                .map(|p| Value::String(p.to_string()))
                .collect();
            Ok(Value::Tuple(parts))
        } else {
            Ok(Value::Tuple(vec![]))
        }
    });
    register_native_fn(&mut scope, "Join", |args, _| {
        if let (Some(col), Some(Value::String(sep))) = (args.first(), args.get(1)) {
            let items = expand_iterable(col.clone());
            let joined = items
                .into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(sep);
            Ok(Value::String(joined))
        } else {
            Ok(Value::String(String::new()))
        }
    });
    register_native_fn(&mut scope, "Length", |args, _| match args.first() {
        Some(Value::String(s)) => Ok(Value::Int(s.chars().count() as i64)),
        Some(Value::Tuple(items)) => Ok(Value::Int(items.len() as i64)),
        _ => Ok(Value::Int(0)),
    });

    // 4. Функции коллекций и потоков (`Filter`, `Map`, `Fold` с поддержкой `Done`, `ForEach`, `Find`, `TakeWhile`)
    register_native_fn(&mut scope, "Filter", |args, scope| {
        if let (Some(col), Some(pred)) = (args.first(), args.get(1)) {
            let mut out = Vec::new();
            for item in expand_iterable(col.clone()) {
                if matches!(
                    apply_callable(pred.clone(), item.clone(), scope)?,
                    Value::Bool(true)
                ) {
                    out.push(item);
                }
            }
            Ok(Value::Tuple(out))
        } else {
            Ok(Value::Tuple(vec![]))
        }
    });

    register_native_fn(&mut scope, "Map", |args, scope| {
        if let (Some(col), Some(mapper)) = (args.first(), args.get(1)) {
            let mut out = Vec::new();
            for item in expand_iterable(col.clone()) {
                out.push(apply_callable(mapper.clone(), item, scope)?);
            }
            Ok(Value::Tuple(out))
        } else {
            Ok(Value::Tuple(vec![]))
        }
    });

    register_native_fn(&mut scope, "Fold", |args, scope| {
        if let (Some(col), Some(init), Some(folder)) = (args.first(), args.get(1), args.get(2)) {
            let mut acc = init.clone();
            for item in expand_iterable(col.clone()) {
                let step_res = apply_callable(
                    folder.clone(),
                    Value::Tuple(vec![acc, item]),
                    scope,
                )?;
                match step_res {
                    Value::Done(early_val) => return Ok(*early_val),
                    Value::Form(f_arc) if f_arc.read().unwrap().type_name == "Done" => {
                        return Ok(f_arc
                            .read()
                            .unwrap()
                            .get_field("Value")
                            .unwrap_or(Value::None));
                    }
                    normal => acc = normal,
                }
            }
            Ok(acc)
        } else {
            Ok(Value::None)
        }
    });

    register_native_fn(&mut scope, "ForEach", |args, scope| {
        if let (Some(col), Some(action)) = (args.first(), args.get(1)) {
            for item in expand_iterable(col.clone()) {
                let _ = apply_callable(action.clone(), item, scope)?;
            }
        }
        Ok(Value::None)
    });

    register_native_fn(&mut scope, "Find", |args, scope| {
        if let (Some(col), Some(pred)) = (args.first(), args.get(1)) {
            for item in expand_iterable(col.clone()) {
                if matches!(
                    apply_callable(pred.clone(), item.clone(), scope)?,
                    Value::Bool(true)
                ) {
                    return Ok(item);
                }
            }
        }
        Ok(Value::None)
    });

    register_native_fn(&mut scope, "TakeWhile", |args, scope| {
        if let (Some(col), Some(pred)) = (args.first(), args.get(1)) {
            let mut out = Vec::new();
            for item in expand_iterable(col.clone()) {
                if matches!(
                    apply_callable(pred.clone(), item.clone(), scope)?,
                    Value::Bool(true)
                ) {
                    out.push(item);
                } else {
                    break;
                }
            }
            Ok(Value::Tuple(out))
        } else {
            Ok(Value::Tuple(vec![]))
        }
    });

    scope
}

pub fn run_hir_module(hir: &HirModule) -> Result<Value, String> {
    let mut global_scope = create_global_scope();

    // 1. Предварительно создаем все формы из статически разрешенного HIR
    for form_name in &hir.form_order {
        if form_name == "Shell" {
            continue;
        }
        if let Some(hir_form) = hir.forms.get(form_name) {
            let mut inst = FormInstance::new(form_name);
            for fname in &hir_form.field_order {
                if let Some(hf) = hir_form.fields.get(fname) {
                    if !hf.default_expr.is_empty() {
                        if let Ok(val) = eval_expression(&hf.default_expr, &mut global_scope) {
                            inst.set_field_with_origin(fname, val, &hf.origin_form);
                        }
                    }
                    if hf.is_ambiguous {
                        inst.ambiguous_fields
                            .insert(fname.clone(), hf.conflicting_origins.clone());
                    }
                }
            }
            global_scope.set(form_name, Value::Form(Arc::new(RwLock::new(inst))));
        }
    }

    // 2. Выполняем предложения верхнего уровня
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
