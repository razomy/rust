// src/runtime_ext.rs
// =============================================================================
// RAZOMY V5.0 — STDLIB, EXTENSIONS, DIALECTS & METAPROGRAMMING ENGINE (LIR)
// Включает: Коллекции, Map, Meta (Reflect/Compose/StepsOf/Rewrite), Parse, UI, Port, Http.
// =============================================================================

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

use crate::hir::HirModule;
use crate::lir::lower_body_to_stmts;
use crate::parser::{Literal, Sentence, Token};
use crate::runtime::{
    apply_callable, eval_sentence, FormInstance, FunctionBody, FunctionInstance, Scope, Value,
};

// =============================================================================
// 1. ХЕЛПЕРЫ РЕГИСТРАЦИИ И КОНСТРУКТОРЫ БАЗОВЫХ ФОРМ
// =============================================================================

pub fn reg_fn<F>(scope: &mut Scope, name: &str, f: F)
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

pub fn reg_mod_fn<F>(module: &mut FormInstance, name: &str, f: F)
where
    F: Fn(Vec<Value>, &mut Scope) -> Result<Value, String> + Send + Sync + 'static,
{
    let fn_arc = Arc::new(FunctionInstance {
        name: name.into(),
        body: FunctionBody::Native(Arc::new(f)),
    });
    module.methods.insert(name.into(), fn_arc.clone());
    module.set_field(name, Value::Function(fn_arc));
}

pub fn make_empty_form(name: &str) -> Value {
    Value::Form(Arc::new(RwLock::new(FormInstance::new(name))))
}

pub fn make_some(val: Value) -> Value {
    let mut s = FormInstance::new("Some");
    s.set_field("Value", val);
    Value::Form(Arc::new(RwLock::new(s)))
}

pub fn expand_iterable(val: Value) -> Vec<Value> {
    match val {
        Value::Tuple(items) => items,
        Value::Range(start, end) => (start..=end).map(Value::Int).collect(),
        Value::Form(f_arc) => {
            let f = f_arc.read().unwrap();
            if f.type_name == "Cons" {
                let mut out = Vec::new();
                if let Some(h) = f.get_field("Head") {
                    out.push(h);
                }
                if let Some(t) = f.get_field("Tail") {
                    out.extend(expand_iterable(t));
                }
                out
            } else {
                vec![Value::Form(f_arc.clone())]
            }
        }
        Value::None => vec![],
        other => vec![other],
    }
}

// =============================================================================
// 2. ЧАСТИЧНОЕ ВЫЧИСЛЕНИЕ `Compose([ ... ])` (РАЗДЕЛ 17.2)
// =============================================================================

pub fn eval_compose_call(call_args: &[Sentence], scope: &mut Scope) -> Result<Value, String> {
    if let Some([Token::Group('[', stream_sents)]) = call_args.first().map(Vec::as_slice) {
        let partially_evaluated = stream_sents
            .iter()
            .map(|s| partial_eval_tokens(s, scope))
            .collect::<Result<Vec<_>, _>>()?;
        let dummy_hir = HirModule::default();
        return Ok(Value::Stream {
            stmts: lower_body_to_stmts(&partially_evaluated, &dummy_hir),
            raw: partially_evaluated,
        });
    }
    Ok(Value::None)
}

fn partial_eval_tokens(tokens: &[Token], scope: &mut Scope) -> Result<Sentence, String> {
    let mut out = Vec::new();
    for tok in tokens {
        match tok {
            Token::Group('(', inner) if inner.len() == 1 => {
                let val = eval_sentence(&inner[0], scope)?;
                out.push(value_to_token(val));
            }
            _ => out.push(tok.clone()),
        }
    }
    Ok(out)
}

fn value_to_token(val: Value) -> Token {
    match val {
        Value::Int(i) => Token::Lit(Literal::Int(i)),
        Value::Float(f) => Token::Lit(Literal::Float(f)),
        Value::Bool(b) => Token::Lit(Literal::Bool(b)),
        Value::String(s) => Token::Lit(Literal::String(s)),
        Value::Symbol(s) => Token::Lit(Literal::Symbol(s)),
        _ => Token::Path(val.to_string()),
    }
}

// =============================================================================
// 3. ИНИЦИАЛИЗАЦИЯ ГЛОБАЛЬНОГО ОКРУЖЕНИЯ, STDLIB И ДИАЛЕКТОВ
// =============================================================================

pub fn create_global_scope() -> Scope {
    let mut scope = Scope::new();

    for tag in ["None", "Done", "Public", "Private", "Zero", "Leaf", "Reset"] {
        scope.set(tag, make_empty_form(tag));
    }

    register_shell_and_primitives(&mut scope);
    register_string_and_collection_combinators(&mut scope);
    register_meta_and_dialects(&mut scope);

    scope
}

fn register_shell_and_primitives(scope: &mut Scope) {
    let mut shell = FormInstance::new("Shell");
    reg_mod_fn(&mut shell, "Log", |args, _| {
        if let Some(m) = args.first() {
            println!("{m}");
        }
        Ok(Value::None)
    });
    reg_mod_fn(&mut shell, "Exec", |args, _| {
        if let Some(c) = args.first() {
            println!("[Exec]: {c}");
        }
        let mut ok = FormInstance::new("BuildSuccess");
        ok.set_field("Status", Value::String("ok".into()));
        ok.set_field("Artifact", Value::String("target/release/razomy".into()));
        Ok(Value::Form(Arc::new(RwLock::new(ok))))
    });
    scope.set("Shell", Value::Form(Arc::new(RwLock::new(shell))));

    reg_fn(scope, "bit_and", |a, _| match (a.first(), a.get(1)) {
        (Some(Value::Int(x)), Some(Value::Int(y))) => Ok(Value::Int(x & y)),
        _ => Ok(Value::Int(0)),
    });
    reg_fn(scope, "bit_or", |a, _| match (a.first(), a.get(1)) {
        (Some(Value::Int(x)), Some(Value::Int(y))) => Ok(Value::Int(x | y)),
        _ => Ok(Value::Int(0)),
    });
    reg_fn(scope, "bit_xor", |a, _| match (a.first(), a.get(1)) {
        (Some(Value::Int(x)), Some(Value::Int(y))) => Ok(Value::Int(x ^ y)),
        _ => Ok(Value::Int(0)),
    });
    reg_fn(scope, "native_print", |a, _| {
        if let Some(m) = a.first() {
            println!("{m}");
        }
        Ok(Value::None)
    });
    reg_fn(scope, "native_exec", |a, _| {
        Ok(Value::String(
            a.first().map(ToString::to_string).unwrap_or_default(),
        ))
    });
    reg_fn(scope, "native_sha256", |a, _| {
        Ok(Value::String(format!(
            "sha256:{}",
            a.first().map(ToString::to_string).unwrap_or_default()
        )))
    });
    reg_fn(scope, "Timeout", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    reg_fn(scope, "Atom", |a, _| {
        Ok(Value::Atom(Arc::new(Mutex::new(
            a.into_iter().next().unwrap_or(Value::None),
        ))))
    });
    reg_fn(scope, "Channel", |_, _| {
        Ok(Value::Channel(Arc::new(Mutex::new(VecDeque::new()))))
    });
}

fn register_string_and_collection_combinators(scope: &mut Scope) {
    reg_fn(scope, "Trim", |a, _| match a.first() {
        Some(Value::String(s)) => Ok(Value::String(s.trim().to_string())),
        o => Ok(o.cloned().unwrap_or(Value::None)),
    });
    reg_fn(scope, "ToUpper", |a, _| match a.first() {
        Some(Value::String(s)) => Ok(Value::String(s.to_uppercase())),
        o => Ok(o.cloned().unwrap_or(Value::None)),
    });
    reg_fn(scope, "Split", |a, _| match (a.first(), a.get(1)) {
        (Some(Value::String(s)), Some(Value::String(d))) => Ok(Value::Tuple(
            s.split(d.as_str())
                .map(|p| Value::String(p.to_string()))
                .collect(),
        )),
        _ => Ok(Value::Tuple(vec![])),
    });
    reg_fn(scope, "Join", |a, _| match (a.first(), a.get(1)) {
        (Some(col), Some(Value::String(sep))) => Ok(Value::String(
            expand_iterable(col.clone())
                .into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(sep),
        )),
        _ => Ok(Value::String(String::new())),
    });
    reg_fn(scope, "Length", |a, _| match a.first() {
        Some(Value::String(s)) => Ok(Value::Int(s.chars().count() as i64)),
        Some(Value::Tuple(v)) => Ok(Value::Int(v.len() as i64)),
        _ => Ok(Value::Int(0)),
    });

    let mut map_mod = FormInstance::new("Map");
    reg_mod_fn(&mut map_mod, "Get", |a, _| match (a.first(), a.get(1)) {
        (Some(Value::Form(f_arc)), Some(key)) => {
            let k_str = match key {
                Value::String(s) | Value::Symbol(s) => s.clone(),
                other => other.to_string(),
            };
            match f_arc.read().unwrap().get_field(&k_str) {
                Some(v) => Ok(make_some(v)),
                None => Ok(make_empty_form("None")),
            }
        }
        _ => Ok(make_empty_form("None")),
    });
    reg_mod_fn(&mut map_mod, "__call__", |a, sc| {
        let (Some(col), Some(mapper)) = (a.first(), a.get(1)) else {
            return Ok(Value::Tuple(vec![]));
        };
        let mut out = Vec::new();
        for item in expand_iterable(col.clone()) {
            out.push(apply_callable(mapper.clone(), item, sc)?);
        }
        Ok(Value::Tuple(out))
    });
    scope.set("Map", Value::Form(Arc::new(RwLock::new(map_mod))));

    reg_fn(scope, "Filter", |a, sc| {
        let (Some(col), Some(pred)) = (a.first(), a.get(1)) else {
            return Ok(Value::Tuple(vec![]));
        };
        let mut out = Vec::new();
        for item in expand_iterable(col.clone()) {
            if matches!(
                apply_callable(pred.clone(), item.clone(), sc)?,
                Value::Bool(true)
            ) {
                out.push(item);
            }
        }
        Ok(Value::Tuple(out))
    });
    reg_fn(scope, "FlatMap", |a, sc| {
        let (Some(col), Some(mapper)) = (a.first(), a.get(1)) else {
            return Ok(Value::Tuple(vec![]));
        };
        let mut out = Vec::new();
        for item in expand_iterable(col.clone()) {
            let mapped = apply_callable(mapper.clone(), item, sc)?;
            out.extend(expand_iterable(mapped));
        }
        Ok(Value::Tuple(out))
    });
    reg_fn(scope, "Fold", |a, sc| {
        let (Some(col), Some(init), Some(folder)) = (a.first(), a.get(1), a.get(2)) else {
            return Ok(Value::None);
        };
        let mut acc = init.clone();
        for item in expand_iterable(col.clone()) {
            match apply_callable(folder.clone(), Value::Tuple(vec![acc, item]), sc)? {
                Value::Done(early) => return Ok(*early),
                Value::Form(f_arc) if f_arc.read().unwrap().type_name == "Done" => {
                    return Ok(f_arc.read().unwrap().get_field("Value").unwrap_or(Value::None));
                }
                normal => acc = normal,
            }
        }
        Ok(acc)
    });
    reg_fn(scope, "ForEach", |a, sc| {
        if let (Some(col), Some(act)) = (a.first(), a.get(1)) {
            for item in expand_iterable(col.clone()) {
                let _ = apply_callable(act.clone(), item, sc)?;
            }
        }
        Ok(Value::None)
    });
    reg_fn(scope, "Find", |a, sc| {
        if let (Some(col), Some(pred)) = (a.first(), a.get(1)) {
            for item in expand_iterable(col.clone()) {
                if matches!(
                    apply_callable(pred.clone(), item.clone(), sc)?,
                    Value::Bool(true)
                ) {
                    return Ok(item);
                }
            }
        }
        Ok(Value::None)
    });
    reg_fn(scope, "TakeWhile", |a, sc| {
        let (Some(col), Some(pred)) = (a.first(), a.get(1)) else {
            return Ok(Value::Tuple(vec![]));
        };
        let mut out = Vec::new();
        for item in expand_iterable(col.clone()) {
            if matches!(
                apply_callable(pred.clone(), item.clone(), sc)?,
                Value::Bool(true)
            ) {
                out.push(item);
            } else {
                break;
            }
        }
        Ok(Value::Tuple(out))
    });
}

fn register_meta_and_dialects(scope: &mut Scope) {
    let mut meta = FormInstance::new("Meta");
    reg_mod_fn(&mut meta, "TypeOf", |a, _| match a.first() {
        Some(Value::Form(f)) => Ok(Value::String(f.read().unwrap().type_name.clone())),
        Some(v) => Ok(Value::String(format!("{v:?}"))),
        None => Ok(Value::String("None".into())),
    });
    reg_mod_fn(&mut meta, "FieldsOf", |a, _| {
        let Some(Value::Form(f_arc)) = a.first() else {
            return Ok(Value::Tuple(vec![]));
        };
        let f = f_arc.read().unwrap();
        let list = f
            .field_order
            .iter()
            .filter_map(|k| {
                let v = f.fields.get(k)?;
                let mut info = FormInstance::new("FieldMeta");
                info.set_field("Name", Value::String(k.clone()));
                info.set_field("Type", Value::String(format!("{v:?}")));
                info.set_field("Value", v.clone());
                Some(Value::Form(Arc::new(RwLock::new(info))))
            })
            .collect();
        Ok(Value::Tuple(list))
    });
    reg_mod_fn(&mut meta, "StepsOf", |a, _| match a.first() {
        Some(Value::Stream { raw, .. }) => {
            let dummy_hir = HirModule::default();
            Ok(Value::Tuple(
                raw.iter()
                    .map(|s| Value::Stream {
                        stmts: lower_body_to_stmts(std::slice::from_ref(s), &dummy_hir),
                        raw: vec![s.clone()],
                    })
                    .collect(),
            ))
        }
        _ => Ok(Value::Tuple(vec![])),
    });
    reg_mod_fn(&mut meta, "Rewrite", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    scope.set("Meta", Value::Form(Arc::new(RwLock::new(meta))));

    let mut json_mod = FormInstance::new("Json");
    reg_mod_fn(&mut json_mod, "FormatValue", |a, _| match a.first() {
        Some(Value::String(s)) => Ok(Value::String(format!("\"{s}\""))),
        Some(v) => Ok(Value::String(v.to_string())),
        None => Ok(Value::String("null".into())),
    });
    reg_mod_fn(&mut json_mod, "Encode", |a, _| {
        Ok(Value::String(format!("{:?}", a.first())))
    });
    reg_mod_fn(&mut json_mod, "Parse", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    scope.set("Json", Value::Form(Arc::new(RwLock::new(json_mod))));

    let mut http_mod = FormInstance::new("Http");
    reg_mod_fn(&mut http_mod, "Get", |a, _| {
        Ok(Value::String(format!(
            "HTTP_200({})",
            a.first().map(ToString::to_string).unwrap_or_default()
        )))
    });
    scope.set("Http", Value::Form(Arc::new(RwLock::new(http_mod))));

    let mut port_mod = FormInstance::new("Port");
    reg_mod_fn(&mut port_mod, "Read", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    reg_mod_fn(&mut port_mod, "Write", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    scope.set("Port", Value::Form(Arc::new(RwLock::new(port_mod))));

    let mut ui_mod = FormInstance::new("UI");
    reg_mod_fn(&mut ui_mod, "View", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    reg_mod_fn(&mut ui_mod, "Draw", |a, _| Ok(Value::Tuple(a)));
    scope.set("UI", Value::Form(Arc::new(RwLock::new(ui_mod))));

    let mut regex_mod = FormInstance::new("Regex");
    reg_mod_fn(&mut regex_mod, "Matches", |a, _| match a.first() {
        Some(Value::String(s)) => Ok(Value::Bool(s.contains('@') && s.contains('.'))),
        _ => Ok(Value::Bool(false)),
    });
    scope.set("Regex", Value::Form(Arc::new(RwLock::new(regex_mod))));

    reg_fn(scope, "Parse", |a, _| {
        if let Some(Value::String(line)) = a.first() {
            let mut entry = FormInstance::new("LogEntry");
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            entry.set_field(
                "Date",
                Value::String(parts.first().copied().unwrap_or("").into()),
            );
            entry.set_field(
                "Level",
                Value::String(
                    parts
                        .get(1)
                        .copied()
                        .unwrap_or("")
                        .trim_matches(&['[', ']'][..])
                        .into(),
                ),
            );
            entry.set_field(
                "Msg",
                Value::String(parts.get(2).copied().unwrap_or("").into()),
            );
            return Ok(Value::Form(Arc::new(RwLock::new(entry))));
        }
        Ok(Value::Bool(true))
    });
}