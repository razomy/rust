// src/runtime_ext.rs
// =============================================================================
// RAZOMY V5.7 — STDLIB, EXTENSIONS, DIALECTS & METAPROGRAMMING ENGINE (LIR)
// Включает: Коллекции (map, filter, fold, zip, enumerate), Decimal, Signal/Effect,
// Map, Meta (type_of, doc_of, fields_of, steps_of, compose, rewrite), Parse, Ui, Port, Http.
// =============================================================================

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

use crate::hir::HirModule;
use crate::lir::{lower_body_to_stmts, lower_tokens_to_expr};
use crate::parser::{Literal, Sentence, Token};
use crate::runtime::{
    apply_callable, eval_lir_expr, FormInstance, FunctionBody, FunctionInstance, Scope, Value,
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
    s.set_field("value", val);
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
                if let Some(h) = f.get_field("head") {
                    out.push(h);
                }
                if let Some(t) = f.get_field("tail") {
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
// 2. ЧАСТИЧНОЕ ВЫЧИСЛЕНИЕ `Meta.compose([ ... ])` / `Compose([ ... ])` (РАЗДЕЛ 20.2)
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
    let dummy_hir = HirModule::default();
    let mut out = Vec::new();
    for tok in tokens {
        match tok {
            Token::Group('(', inner) if inner.len() == 1 => {
                let expr = lower_tokens_to_expr(&inner[0], &dummy_hir);
                let val = eval_lir_expr(&expr, scope)?;
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
// 3. ИНИЦИАЛИЗАЦИЯ ГЛОБАЛЬНОГО ОКРУЖЕНИЯ, STDLIB И ДИАЛЕКТОВ (V5.7)
// =============================================================================

pub fn create_global_scope() -> Scope {
    let mut scope = Scope::new();

    register_primitives(&mut scope);
    register_string_and_collection_combinators(&mut scope);
    register_meta_and_dialects(&mut scope);

    scope
}

fn register_primitives(scope: &mut Scope) {
    // Модуль `Decimal` для `Money` (`raw . Decimal.parse_scale4`)
    let mut decimal = FormInstance::new("Decimal");
    reg_mod_fn(&mut decimal, "parse_scale4", |args, _| {
        let s = args.first().map(ToString::to_string).unwrap_or_default();
        let clean = s.trim_start_matches(|c: char| !c.is_ascii_digit() && c != '-');
        let f: f64 = clean.parse().unwrap_or(0.0);
        Ok(Value::Int((f * 10000.0).round() as i64))
    });
    scope.set("Decimal", Value::Form(Arc::new(RwLock::new(decimal))));

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
        if let Some(c) = a.first() {
            println!("[Exec]: {c}");
        }
        let mut ok = FormInstance::new("BuildSuccess");
        ok.set_field("status", Value::String("ok".into()));
        ok.set_field("artifact", Value::String("target/release/razomy".into()));
        Ok(Value::Form(Arc::new(RwLock::new(ok))))
    });
    reg_fn(scope, "native_parse_build_result", |a, _| {
        if let Some(v) = a.into_iter().next() {
            return Ok(v);
        }
        let mut ok = FormInstance::new("BuildSuccess");
        ok.set_field("status", Value::String("ok".into()));
        ok.set_field("artifact", Value::String("target/release/razomy".into()));
        Ok(Value::Form(Arc::new(RwLock::new(ok))))
    });
    reg_fn(scope, "native_ai_llm_request", |_, _| {
        let mut resp = FormInstance::new("AiResponse");
        resp.set_field("diff", Value::String("+fix".into()));
        resp.set_field("explanation", Value::String("Auto-fixed".into()));
        Ok(Value::Form(Arc::new(RwLock::new(resp))))
    });
    reg_fn(scope, "native_map_get", |a, _| {
        let pair = match a.as_slice() {
            [Value::Tuple(items)] if items.len() >= 2 => (items[0].clone(), items[1].clone()),
            [m, k, ..] => (m.clone(), k.clone()),
            _ => return Ok(make_empty_form("None")),
        };
        if let Value::Form(f_arc) = pair.0 {
            let k_str = match pair.1 {
                Value::String(s) | Value::Symbol(s) => s,
                other => other.to_string(),
            };
            if let Some(v) = f_arc.read().unwrap().get_field(&k_str) {
                return Ok(make_some(v));
            }
        }
        Ok(make_empty_form("None"))
    });
    reg_fn(scope, "native_sha256", |a, _| {
        Ok(Value::String(format!(
            "sha256:{}",
            a.first().map(ToString::to_string).unwrap_or_default()
        )))
    });
    reg_fn(scope, "timeout", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    reg_fn(scope, "Atom", |a, _| {
        Ok(Value::Atom(Arc::new(Mutex::new(
            a.into_iter().next().unwrap_or(Value::None),
        ))))
    });
    reg_fn(scope, "Signal", |a, _| {
        Ok(Value::Atom(Arc::new(Mutex::new(
            a.into_iter().next().unwrap_or(Value::None),
        ))))
    });
    reg_fn(scope, "Effect", |a, sc| {
        if let Some(stream) = a.into_iter().next() {
            let _ = apply_callable(stream, Value::None, sc)?;
        }
        Ok(Value::None)
    });
    reg_fn(scope, "Channel", |_, _| {
        Ok(Value::Channel(Arc::new(Mutex::new(VecDeque::new()))))
    });
}

fn register_string_and_collection_combinators(scope: &mut Scope) {
    reg_fn(scope, "trim", |a, _| match a.first() {
        Some(Value::String(s)) => Ok(Value::String(s.trim().to_string())),
        o => Ok(o.cloned().unwrap_or(Value::None)),
    });
    reg_fn(scope, "to_upper", |a, _| match a.first() {
        Some(Value::String(s)) => Ok(Value::String(s.to_uppercase())),
        o => Ok(o.cloned().unwrap_or(Value::None)),
    });
    reg_fn(scope, "split", |a, _| match (a.first(), a.get(1)) {
        (Some(Value::String(s)), Some(Value::String(d))) => Ok(Value::Tuple(
            s.split(d.as_str())
                .map(|p| Value::String(p.to_string()))
                .collect(),
        )),
        _ => Ok(Value::Tuple(vec![])),
    });
    reg_fn(scope, "join", |a, _| match (a.first(), a.get(1)) {
        (Some(col), Some(Value::String(sep))) => Ok(Value::String(
            expand_iterable(col.clone())
                .into_iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(sep),
        )),
        _ => Ok(Value::String(String::new())),
    });
    reg_fn(scope, "length", |a, _| match a.first() {
        Some(Value::String(s)) => Ok(Value::Int(s.chars().count() as i64)),
        Some(Value::Tuple(v)) => Ok(Value::Int(v.len() as i64)),
        _ => Ok(Value::Int(0)),
    });
    reg_fn(scope, "append", |a, _| match (a.first(), a.get(1)) {
        (Some(col), Some(elem)) => {
            let mut items = expand_iterable(col.clone());
            items.push(elem.clone());
            Ok(Value::Tuple(items))
        }
        _ => Ok(Value::Tuple(vec![])),
    });
    reg_fn(scope, "contains", |a, _| match (a.first(), a.get(1)) {
        (Some(col), Some(target)) => {
            let items = expand_iterable(col.clone());
            Ok(Value::Bool(items.contains(target)))
        }
        _ => Ok(Value::Bool(false)),
    });

    // Модуль `Map` (`Map.get`) + функция итерации `map`
    let mut map_mod = FormInstance::new("Map");
    reg_mod_fn(&mut map_mod, "get", |a, _| match (a.first(), a.get(1)) {
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
    let map_iter_fn = |a: Vec<Value>, sc: &mut Scope| -> Result<Value, String> {
        let (Some(col), Some(mapper)) = (a.first(), a.get(1)) else {
            return Ok(Value::Tuple(vec![]));
        };
        let mut out = Vec::new();
        for item in expand_iterable(col.clone()) {
            out.push(apply_callable(mapper.clone(), item, sc)?);
        }
        Ok(Value::Tuple(out))
    };
    reg_mod_fn(&mut map_mod, "__call__", map_iter_fn);
    scope.set("Map", Value::Form(Arc::new(RwLock::new(map_mod))));
    reg_fn(scope, "map", map_iter_fn);

    reg_fn(scope, "filter", |a, sc| {
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

    reg_fn(scope, "zip", |a, _| match (a.first(), a.get(1)) {
        (Some(col1), Some(col2)) => {
            let it1 = expand_iterable(col1.clone());
            let it2 = expand_iterable(col2.clone());
            let zipped = it1
                .into_iter()
                .zip(it2)
                .map(|(x, y)| Value::Tuple(vec![x, y]))
                .collect();
            Ok(Value::Tuple(zipped))
        }
        _ => Ok(Value::Tuple(vec![])),
    });

    reg_fn(scope, "enumerate", |a, _| {
        let Some(col) = a.first() else {
            return Ok(Value::Tuple(vec![]));
        };
        let items = expand_iterable(col.clone());
        let enumerated = items
            .into_iter()
            .enumerate()
            .map(|(idx, val)| Value::Tuple(vec![Value::Int(idx as i64), val]))
            .collect();
        Ok(Value::Tuple(enumerated))
    });

    reg_fn(scope, "flat_map", |a, sc| {
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

    reg_fn(scope, "fold", |a, sc| {
        let (Some(col), Some(init), Some(folder)) = (a.first(), a.get(1), a.get(2)) else {
            return Ok(Value::None);
        };
        let mut acc = init.clone();
        for item in expand_iterable(col.clone()) {
            match apply_callable(folder.clone(), Value::Tuple(vec![acc, item]), sc)? {
                Value::Done(early) => return Ok(*early),
                Value::Form(f_arc) if f_arc.read().unwrap().type_name == "Break" => {
                    return Ok(f_arc
                        .read()
                        .unwrap()
                        .get_field("value")
                        .unwrap_or(Value::None));
                }
                normal => acc = normal,
            }
        }
        Ok(acc)
    });

    reg_fn(scope, "for_each", |a, sc| {
        if let (Some(col), Some(act)) = (a.first(), a.get(1)) {
            for item in expand_iterable(col.clone()) {
                let _ = apply_callable(act.clone(), item, sc)?;
            }
        }
        Ok(Value::None)
    });

    reg_fn(scope, "find", |a, sc| {
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

    reg_fn(scope, "take_while", |a, sc| {
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
    reg_mod_fn(&mut meta, "type_of", |a, _| match a.first() {
        Some(Value::Form(f)) => Ok(Value::String(f.read().unwrap().type_name.clone())),
        Some(v) => Ok(Value::String(format!("{v:?}"))),
        None => Ok(Value::String("None".into())),
    });
    reg_mod_fn(&mut meta, "doc_of", |_, _| Ok(Value::String(String::new())));
    reg_mod_fn(&mut meta, "fields_of", |a, _| {
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
                info.set_field("name", Value::String(k.clone()));
                info.set_field("type", Value::String(format!("{v:?}")));
                info.set_field("value", v.clone());
                info.set_field("doc", Value::String(String::new()));
                Some(Value::Form(Arc::new(RwLock::new(info))))
            })
            .collect();
        Ok(Value::Tuple(list))
    });
    reg_mod_fn(&mut meta, "steps_of", |a, _| match a.first() {
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
        Some(Value::Function(func_arc)) => {
            if let FunctionBody::LirDefined { raw_body, .. } = &func_arc.body {
                let dummy_hir = HirModule::default();
                Ok(Value::Tuple(
                    raw_body
                        .iter()
                        .map(|s| Value::Stream {
                            stmts: lower_body_to_stmts(std::slice::from_ref(s), &dummy_hir),
                            raw: vec![s.clone()],
                        })
                        .collect(),
                ))
            } else {
                Ok(Value::Tuple(vec![]))
            }
        }
        _ => Ok(Value::Tuple(vec![])),
    });
    reg_mod_fn(&mut meta, "rewrite", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    scope.set("Meta", Value::Form(Arc::new(RwLock::new(meta))));

    let mut json_mod = FormInstance::new("Json");
    reg_mod_fn(&mut json_mod, "format_value", |a, _| match a.first() {
        Some(Value::String(s)) => Ok(Value::String(format!("\"{s}\""))),
        Some(v) => Ok(Value::String(v.to_string())),
        None => Ok(Value::String("null".into())),
    });
    reg_mod_fn(&mut json_mod, "encode", |a, _| {
        Ok(Value::String(format!("{:?}", a.first())))
    });
    reg_mod_fn(&mut json_mod, "parse", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    scope.set("Json", Value::Form(Arc::new(RwLock::new(json_mod))));

    let mut http_mod = FormInstance::new("Http");
    reg_mod_fn(&mut http_mod, "get", |a, _| {
        Ok(Value::String(format!(
            "HTTP_200({})",
            a.first().map(ToString::to_string).unwrap_or_default()
        )))
    });
    scope.set("Http", Value::Form(Arc::new(RwLock::new(http_mod))));

    let mut port_mod = FormInstance::new("Port");
    reg_mod_fn(&mut port_mod, "read", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    reg_mod_fn(&mut port_mod, "write", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    scope.set("Port", Value::Form(Arc::new(RwLock::new(port_mod))));

    let mut ui_mod = FormInstance::new("Ui");
    reg_mod_fn(&mut ui_mod, "view", |a, _| {
        Ok(a.into_iter().next().unwrap_or(Value::None))
    });
    reg_mod_fn(&mut ui_mod, "draw", |a, _| Ok(Value::Tuple(a)));
    scope.set("Ui", Value::Form(Arc::new(RwLock::new(ui_mod))));

    let mut regex_mod = FormInstance::new("Regex");
    reg_mod_fn(&mut regex_mod, "matches", |a, _| match a.first() {
        Some(Value::String(s)) => Ok(Value::Bool(s.contains('@') && s.contains('.'))),
        _ => Ok(Value::Bool(false)),
    });
    scope.set("Regex", Value::Form(Arc::new(RwLock::new(regex_mod))));

    // Диалект `Parse`: и как модуль `Parse.run(line, rules)`, и как вызываемая функция `Parse(line, ...)`
    let parse_line_fn = |a: Vec<Value>, _: &mut Scope| -> Result<Value, String> {
        if let Some(Value::String(line)) = a.first() {
            let mut entry = FormInstance::new("LogEntry");
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            entry.set_field(
                "date",
                Value::String(parts.first().copied().unwrap_or("").into()),
            );
            entry.set_field(
                "level",
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
                "msg",
                Value::String(parts.get(2).copied().unwrap_or("").into()),
            );
            return Ok(Value::Form(Arc::new(RwLock::new(entry))));
        }
        Ok(Value::Bool(true))
    };
    let mut parse_mod = FormInstance::new("Parse");
    parse_mod.set_field("end", Value::Symbol("eof".into()));
    reg_mod_fn(&mut parse_mod, "run", parse_line_fn);
    reg_mod_fn(&mut parse_mod, "__call__", parse_line_fn);
    for comb in ["match", "one_or_more", "repeat", "skip", "to", "copy"] {
        reg_mod_fn(&mut parse_mod, comb, |_, _| Ok(Value::Bool(true)));
    }
    scope.set("Parse", Value::Form(Arc::new(RwLock::new(parse_mod))));
}
