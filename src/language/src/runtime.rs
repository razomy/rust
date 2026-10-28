// src/runtime.rs
// =============================================================================
// RAZOMY V4.7 — THREAD-SAFE INTERPRETER & DATAFLOW RUNTIME ENGINE
// =============================================================================

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

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
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Byte(a), Value::Byte(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Money(a), Value::Money(b)) => a == b,
            (Value::Percent(a), Value::Percent(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::Duration(a), Value::Duration(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct FormInstance {
    pub type_name: String,
    pub fields: HashMap<String, Value>,
    pub field_order: Vec<String>,
    pub ancestors: HashMap<String, FormInstance>, // Закон 2: Lineage
}

impl FormInstance {
    pub fn new(type_name: &str) -> Self {
        Self {
            type_name: type_name.to_string(),
            fields: HashMap::new(),
            field_order: Vec::new(),
            ancestors: HashMap::new(),
        }
    }

    pub fn set_field(&mut self, key: &str, val: Value) {
        if !self.fields.contains_key(key) {
            self.field_order.push(key.to_string());
        }
        self.fields.insert(key.to_string(), val);
    }

    pub fn get_field(&self, key: &str) -> Option<Value> {
        self.fields.get(key).cloned()
    }
}

pub type NativeFn = Arc<dyn Fn(Vec<Value>, &mut Scope) -> Result<Value, String> + Send + Sync>;

#[derive(Clone)]
pub enum FunctionBody {
    UserDefined {
        args: Vec<Sentence>,
        body: Vec<Sentence>,
        captured_env: Arc<RwLock<Scope>>,
    },
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
        Literal::Symbol(s) => Value::Symbol(s.clone()),
        Literal::Money(m) => Value::Money(m.clone()),
        Literal::Percent(p) => Value::Percent(*p),
        Literal::Date(d) => Value::Date(d.clone()),
        Literal::Duration(d) => Value::Duration(d.clone()),
        _ => Value::String(format!("{lit:?}")),
    }
}

/// Интерполяция строк с поддержкой путей `{Name}`, `{U.Name}`, `{User.Id}`
fn interpolate_string(raw: &str, scope: &mut Scope) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
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
        Token::Group('(', sentences) => {
            if sentences.len() == 1 {
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
            // Если внутри `{ ... }` есть паттерн-ветки `[ _ >= 18 ] [...]` или `Type [...]`,
            // то это Router для пайплайна, иначе — анонимная форма
            let is_router = sentences.iter().any(|s| {
                matches!(
                    classify_sentence(s),
                    SemanticKind::PatternBranch { .. }
                        | SemanticKind::StreamOrBranch { .. }
                        | SemanticKind::LambdaOrTupleBranch { .. }
                )
            });

            if is_router {
                Ok(Value::Router(sentences.clone()))
            } else {
                let mut form = FormInstance::new("AnonymousForm");
                for s in sentences {
                    if let SemanticKind::Binding { key, value } = classify_sentence(s) {
                        let v = eval_expression(&value, scope)?;
                        form.set_field(&key, v);
                    }
                }
                Ok(Value::Form(Arc::new(RwLock::new(form))))
            }
        }
        _ => Err(format!("Невозможно вычислить токен: {token:?}")),
    }
}

pub fn eval_path(path: &str, scope: &mut Scope) -> Result<Value, String> {
    if path == "_" {
        return scope
            .get("_")
            .ok_or_else(|| "Контекст `_` не определен в текущем потоке".into());
    }

    let segments: Vec<&str> = path.split('.').collect();
    let root_name = segments[0];

    let mut current = scope
        .get(root_name)
        .ok_or_else(|| format!("Неопределенный идентификатор: '{root_name}'"))?;

    for seg in &segments[1..] {
        let next_val = match &current {
            Value::Form(form_arc) => {
                let form_snapshot = form_arc.read().unwrap().clone();
                if let Some(val) = form_snapshot.get_field(seg) {
                    // Закон 1a: Если поле формы является реактивным потоком `Subtotal [ Price * Quantity ]`,
                    // вычисляем его на лету с полями этой формы в скоупе!
                    if let Value::Stream(stream_sents) = val {
                        let mut reactive_scope =
                            Scope::child(Arc::new(RwLock::new(scope.clone())));
                        for (k, v) in &form_snapshot.fields {
                            if !matches!(v, Value::Stream(_)) {
                                reactive_scope.set(k, v.clone());
                            }
                        }
                        eval_stream_body(
                            &stream_sents,
                            Some(Value::Form(form_arc.clone())),
                            &mut reactive_scope,
                        )?
                    } else {
                        val
                    }
                } else if let Ok(idx) = seg.parse::<i32>() {
                    let i = if idx < 0 {
                        (form_snapshot.field_order.len() as i32 + idx) as usize
                    } else {
                        idx as usize
                    };
                    if let Some(field_name) = form_snapshot.field_order.get(i) {
                        form_snapshot.fields.get(field_name).unwrap().clone()
                    } else {
                        return Err(format!("Индекс {idx} вне диапазона полей формы"));
                    }
                } else if let Some(ancestor) = form_snapshot.ancestors.get(*seg) {
                    Value::Form(Arc::new(RwLock::new(ancestor.clone())))
                } else {
                    return Err(format!(
                        "Поле '{seg}' не найдено в форме {}",
                        form_snapshot.type_name
                    ));
                }
            }
            Value::Tuple(items) => {
                if let Ok(idx) = seg.parse::<usize>() {
                    items
                        .get(idx)
                        .cloned()
                        .ok_or_else(|| format!("Индекс {idx} вне границ кортежа"))?
                } else {
                    return Err(format!(
                        "Нельзя обратиться по строковому ключу '{seg}' к кортежу"
                    ));
                }
            }
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

pub fn eval_binary_op(left: Value, op: &str, right: Value) -> Result<Value, String> {
    match (left, op, right) {
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

        (Value::Float(a), "+", Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Float(a), "-", Value::Float(b)) => Ok(Value::Float(a - b)),
        (Value::Float(a), "*", Value::Float(b)) => Ok(Value::Float(a * b)),
        (Value::Float(a), "/", Value::Float(b)) => Ok(Value::Float(a / b)),

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

        // Слияние форм & (Закон 2: Lineage сохраняется)
        (Value::Form(f1), "&", Value::Form(f2)) => {
            let f1_read = f1.read().unwrap();
            let f2_read = f2.read().unwrap();

            let mut merged =
                FormInstance::new(&format!("{}+{}", f1_read.type_name, f2_read.type_name));
            merged
                .ancestors
                .insert(f1_read.type_name.clone(), f1_read.clone());
            merged
                .ancestors
                .insert(f2_read.type_name.clone(), f2_read.clone());

            for k in &f1_read.field_order {
                merged.set_field(k, f1_read.fields.get(k).unwrap().clone());
            }
            for k in &f2_read.field_order {
                merged.set_field(k, f2_read.fields.get(k).unwrap().clone());
            }
            Ok(Value::Form(Arc::new(RwLock::new(merged))))
        }

        // Вычитание полей /
        (Value::Form(f), "/", Value::Symbol(sym)) => {
            let mut form = f.read().unwrap().clone();
            form.fields.remove(&sym);
            form.field_order.retain(|k| k != &sym);
            Ok(Value::Form(Arc::new(RwLock::new(form))))
        }

        (a, op, b) => Err(format!("Оператор '{op}' не поддерживается для {a:?} и {b:?}")),
    }
}

// =============================================================================
// 4. ПАЙПЛАЙНЫ, МАРШРУТИЗАТОР И ТРИ ЖЕЛЕЗНЫХ ЗАКОНА
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

    if tokens.len() == 3 {
        if let Token::Op(op) = &tokens[1] {
            let left = eval_token(&tokens[0], scope)?;
            let right = eval_token(&tokens[2], scope)?;
            return eval_binary_op(left, op, right);
        }
    }

    if tokens.len() == 1 {
        return eval_token(&tokens[0], scope);
    }

    // Инстанцирование именованной формы с наследованием дефолтных полей прототипа:
    // `CurrentUser User { Name "Alice", Age 22 }`
    if tokens.len() >= 2 {
        if let (Token::Path(type_name), Token::Group('{', sentences)) = (&tokens[0], &tokens[1]) {
            let mut form = if let Some(Value::Form(proto_arc)) = scope.get(type_name) {
                proto_arc.read().unwrap().clone()
            } else {
                FormInstance::new(type_name)
            };
            form.type_name = type_name.clone();

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
                    _ => {}
                }
            }
            return Ok(Value::Form(Arc::new(RwLock::new(form))));
        }
    }

    eval_token(&tokens[0], scope)
}

fn eval_pipe_step(input: Value, step: PipeStep, scope: &mut Scope) -> Result<Value, String> {
    // Закон 3 (Railway-free): Автоматический пропуск None
    if matches!(input, Value::None) && step.op != PipeOp::Filter {
        return Ok(Value::None);
    }

    match step.op {
        PipeOp::Dot => {
            let target = eval_expression(&step.expr, scope)?;
            apply_callable(target, input, scope)
        }
        PipeOp::Tap => {
            let target = eval_expression(&step.expr, scope)?;
            let _ = apply_callable(target, input.clone(), scope)?;
            Ok(input)
        }
        PipeOp::Enrich => {
            let target = eval_expression(&step.expr, scope)?;
            let enriched = apply_callable(target, input.clone(), scope)?;
            eval_binary_op(input, "&", enriched)
        }
        PipeOp::Filter => {
            let pred = eval_expression(&step.expr, scope)?;
            let res = apply_callable(pred, input.clone(), scope)?;
            match res {
                Value::Bool(true) => Ok(input),
                _ => Ok(Value::None),
            }
        }
        PipeOp::Parallel => {
            if let Some(Token::Group('{', branches)) = step.expr.first() {
                let mut handles = Vec::new();
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
                        eval_sentence(&branch_clone, &mut local_scope)
                    });
                    handles.push(handle);
                }

                let mut merged = FormInstance::new("ParallelResult");
                for h in handles {
                    if let Ok(Ok(val)) = h.join() {
                        if let Value::Form(f) = val {
                            let f_read = f.read().unwrap();
                            for k in &f_read.field_order {
                                merged.set_field(k, f_read.fields.get(k).unwrap().clone());
                            }
                        }
                    }
                }
                Ok(Value::Form(Arc::new(RwLock::new(merged))))
            } else {
                Err("Оператор .| ожидает блок {}".into())
            }
        }
    }
}

pub fn apply_callable(callable: Value, arg: Value, scope: &mut Scope) -> Result<Value, String> {
    match callable {
        Value::Function(func) => match &func.body {
            FunctionBody::Native(native_fn) => native_fn(vec![arg], scope),
            FunctionBody::UserDefined {
                args,
                body,
                captured_env,
            } => {
                let mut call_scope = Scope::child(captured_env.clone());
                bind_arguments(args, &arg, &mut call_scope)?;
                eval_stream_body(body, Some(arg), &mut call_scope)
            }
        },
        Value::Stream(sentences) => {
            let mut stream_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
            stream_scope.set("_", arg.clone());
            // Закон 1c: В анонимный поток `[ ... ]` поля входной формы проецируются автоматически!
            if let Value::Form(form_arc) = &arg {
                let form = form_arc.read().unwrap();
                for (k, v) in &form.fields {
                    stream_scope.set(k, v.clone());
                }
            }
            eval_stream_body(&sentences, Some(arg), &mut stream_scope)
        }
        Value::Router(branches) => eval_router(&branches, arg, scope),
        Value::Form(router_arc) => {
            let router = router_arc.read().unwrap();
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

/// Вычисление паттерн-матчинга `{ [ _ >= 18 ] [ ... ], User [ ... ], _ [ ... ] }`
fn eval_router(branches: &[Sentence], arg: Value, scope: &mut Scope) -> Result<Value, String> {
    for branch in branches {
        match classify_sentence(branch) {
            // 1. Ветка с предикатом `[ _ >= 18 ] [ ... ]` или структурным паттерном `{ Role 'admin' } [ ... ]`
            SemanticKind::PatternBranch { pattern, body } => {
                let mut branch_scope = Scope::child(Arc::new(RwLock::new(scope.clone())));
                branch_scope.set("_", arg.clone());
                if let Value::Form(f) = &arg {
                    for (k, v) in &f.read().unwrap().fields {
                        branch_scope.set(k, v.clone());
                    }
                }

                let matched = match pattern {
                    Token::Group('[', pred_sents) => {
                        let res = eval_stream_body(&pred_sents, Some(arg.clone()), &mut branch_scope)?;
                        matches!(res, Value::Bool(true))
                    }
                    Token::Group('{', field_pats) => {
                        if let Value::Form(f_arc) = &arg {
                            let f = f_arc.read().unwrap();
                            let mut all_ok = true;
                            for fp in field_pats {
                                if let SemanticKind::Binding { key, value } = classify_sentence(&fp)
                                {
                                    let expected = eval_expression(&value, &mut branch_scope)?;
                                    if f.get_field(&key) != Some(expected) {
                                        all_ok = false;
                                        break;
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

            // 2. Ветка по имени типа `User [ ... ]` или дефолтная ветка `_ [ ... ]`
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

fn match_type_or_wildcard(pattern_name: &str, val: &Value, scope: &mut Scope) -> bool {
    if pattern_name == "_" {
        scope.set("_", val.clone());
        return true;
    }
    match val {
        Value::Form(form_arc) => {
            let form = form_arc.read().unwrap();
            if form.type_name == pattern_name || form.ancestors.contains_key(pattern_name) {
                // Закон 1b: Проекция всех полей в скоуп при матчинге по типу
                for (k, v) in &form.fields {
                    scope.set(k, v.clone());
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

/// Привязка аргументов с соблюдением Закона 1 и Закона 1c
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
                scope.set(k, v.clone());
            }
        }
        return Ok(());
    }

    if let Some(first_sent) = arg_defs.first() {
        match first_sent.as_slice() {
            // Закон 1c: Одиночный идентификатор типа `Fn(User)` -> распаковывает поля в скоуп!
            [Token::Path(single)] => {
                let is_type = single
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false);
                scope.set(single, incoming.clone());
                if is_type {
                    if let Value::Form(form_arc) = incoming {
                        let form = form_arc.read().unwrap();
                        for (k, v) in &form.fields {
                            scope.set(k, v.clone());
                        }
                    }
                }
            }
            // Закон 1: Именованный параметр `Fn(U User)` -> привязывает ТОЛЬКО `U`, не засоряя скоуп!
            [Token::Path(param_name), ..] => {
                scope.set(param_name, incoming.clone());
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
        if let Value::Done(val) = last {
            return Ok(*val);
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
        SemanticKind::StreamOrBranch { name, stream } => {
            let func = FunctionInstance {
                name: name.clone(),
                body: FunctionBody::UserDefined {
                    args: Vec::new(),
                    body: stream,
                    captured_env: Arc::new(RwLock::new(scope.clone())),
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

        SemanticKind::FormDef { name, body, .. } => {
            let mut form = FormInstance::new(&name);
            for s in body {
                match classify_sentence(&s) {
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
                    _ => {}
                }
            }
            let val = Value::Form(Arc::new(RwLock::new(form)));
            scope.set(&name, val.clone());
            Ok(val)
        }

        SemanticKind::SubtypeFormDef {
            name, base, body, ..
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

            for s in body {
                match classify_sentence(&s) {
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
                    _ => {}
                }
            }
            let val = Value::Form(Arc::new(RwLock::new(form)));
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
                },
            };
            let val = Value::Function(Arc::new(func));
            scope.set(&name, val.clone());
            Ok(val)
        }

        SemanticKind::CallExpr { name, args } => {
            let func = scope
                .get(&name)
                .ok_or_else(|| format!("Функция '{name}' не определена"))?;
            let mut evaluated_args = Vec::new();
            for a in args {
                evaluated_args.push(eval_sentence(&a, scope)?);
            }
            let arg = if evaluated_args.len() == 1 {
                evaluated_args.remove(0)
            } else {
                Value::Tuple(evaluated_args)
            };
            apply_callable(func, arg, scope)
        }

        SemanticKind::Pipeline(tokens) | SemanticKind::Expression(tokens) => {
            eval_expression(&tokens, scope)
        }

        _ => Ok(Value::None),
    }
}

// =============================================================================
// 6. СТАНДАРТНАЯ БИБЛИОТЕКА И ЗАПУСК ПРОГРАММЫ
// =============================================================================

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
    shell.set_field(
        "Log",
        Value::Function(Arc::new(FunctionInstance {
            name: "Log".into(),
            body: FunctionBody::Native(log_native),
        })),
    );

    let exec_native: NativeFn = Arc::new(|args, _| {
        if let Some(cmd) = args.first() {
            println!("[Exec]: {cmd}");
        }
        Ok(Value::String("success".into()))
    });
    shell.set_field(
        "Exec",
        Value::Function(Arc::new(FunctionInstance {
            name: "Exec".into(),
            body: FunctionBody::Native(exec_native),
        })),
    );

    scope.set("Shell", Value::Form(Arc::new(RwLock::new(shell))));

    // 2. Atom(T)
    let atom_native: NativeFn = Arc::new(|args, _| {
        let initial = args.first().cloned().unwrap_or(Value::None);
        Ok(Value::Atom(Arc::new(Mutex::new(initial))))
    });
    scope.set(
        "Atom",
        Value::Function(Arc::new(FunctionInstance {
            name: "Atom".into(),
            body: FunctionBody::Native(atom_native),
        })),
    );

    scope
}

// Добавь этот импорт в начало src/runtime.rs:
use crate::hir::{lower_to_hir, HirModule};

// И замени функцию run_program в конце src/runtime.rs на эти две:
pub fn run_hir_module(hir: &HirModule) -> Result<Value, String> {
    let mut global_scope = create_global_scope();

    // 1. Предварительно создаем все формы из статически разрешенного HIR (включая слияния & и вычитания /)
    for form_name in &hir.form_order {
        if form_name == "Shell" {
            continue;
        }
        if let Some(hir_form) = hir.forms.get(form_name) {
            let mut inst = FormInstance::new(form_name);
            for fname in &hir_form.field_order {
                if let Some(hf) = hir_form.fields.get(fname) {
                    if !hf.default_expr.is_empty() {
                        let val = eval_expression(&hf.default_expr, &mut global_scope)?;
                        inst.set_field(fname, val);
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