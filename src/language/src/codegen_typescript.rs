// src/codegen_typescript.rs
// =============================================================================
// 1. TYPESCRIPT EMITTER (RAZOMY V5.7)
// =============================================================================

use crate::codegen::TargetEmitter;
use crate::lir::{
    LirExpr, LirFieldPatValue, LirInterpPart, LirModule, LirPattern, LirStmt, LirTuplePatElem,
};
use crate::parser::PipeOp;

pub struct TypeScriptEmitter;

impl TypeScriptEmitter {
    fn map_type(&self, ty: &str) -> String {
        if ty.contains(" | ") {
            return ty
                .split(" | ")
                .map(|part| self.map_type(part.trim()))
                .collect::<Vec<_>>()
                .join(" | ");
        }
        match ty {
            "Int" | "Float" | "Byte" | "Percent" => "number".into(),
            "Bool" => "boolean".into(),
            "String" | "Symbol" | "Money" | "Usd" | "Date" | "Time" | "Duration" | "Url"
            | "Email" | "File" | "Path" | "Pair" | "Hex" | "Ip" | "Version" => "string".into(),
            "None" => "null".into(),
            "Any" | "Form" | "Stream" | "ReactiveStream" | "_" => "any".into(),
            "Tuple" | "Range" => "any[]".into(),
            other if other.starts_with('\'') => format!("\"{}\"", other.trim_matches('\'')),
            other if other.starts_with("Tuple(") => "any[]".into(),
            other if other.contains('(') || other.contains('[') || other.contains('/') => {
                "any".into()
            }
            other => other.to_string(),
        }
    }

    fn emit_stmt(&self, stmt: &LirStmt, indent: &str) -> String {
        match stmt {
            LirStmt::Let { name, value } => {
                format!("{indent}let {name} = {};", self.emit_expr(value))
            }
            LirStmt::AssignPath { path, value } => {
                let normalized = Self::normalize_index_path(path);
                format!("{indent}{normalized} = {};", self.emit_expr(value))
            }
            LirStmt::DestructTuple { names, value } => {
                format!(
                    "{indent}const [{}] = {};",
                    names.join(", "),
                    self.emit_expr(value)
                )
            }
            LirStmt::Expr(e) => format!("{indent}{};", self.emit_expr(e)),
            LirStmt::Return(e) => format!("{indent}return {};", self.emit_expr(e)),
        }
    }

    fn normalize_index_path(path: &str) -> String {
        let mut out = String::new();
        for (i, seg) in path.split('.').enumerate() {
            if i == 0 {
                out.push_str(seg);
            } else if seg.parse::<i32>().is_ok() {
                out.push_str(&format!("[{seg}]"));
            } else {
                out.push('.');
                out.push_str(seg);
            }
        }
        out
    }

    fn is_console_log(callee: &str) -> bool {
        matches!(callee, "Shell.log" | "Shell.Log")
    }

    fn emit_expr(&self, expr: &LirExpr) -> String {
        match expr {
            LirExpr::None => "null".into(),
            LirExpr::Int(i) => i.to_string(),
            LirExpr::Float(f) => f.to_string(),
            LirExpr::Bool(b) => b.to_string(),
            LirExpr::Byte(b) => b.to_string(),
            LirExpr::Str(s) | LirExpr::Symbol(s) => format!("{s:?}"),
            LirExpr::InterpStr(parts) => {
                let mut s = String::from("`");
                for p in parts {
                    match p {
                        LirInterpPart::Lit(l) => s.push_str(&l.replace('`', "\\`")),
                        LirInterpPart::Expr(e) => s.push_str(&format!("${{{e}}}")),
                    }
                }
                s.push('`');
                s
            }
            LirExpr::Ident(id) => {
                if Self::is_console_log(id) {
                    "console.log".into()
                } else {
                    Self::normalize_index_path(id)
                }
            }
            LirExpr::Tuple(items) => {
                let list: Vec<String> = items.iter().map(|e| self.emit_expr(e)).collect();
                format!("[{}]", list.join(", "))
            }
            LirExpr::Range(a, b) => {
                format!("Razomy.range({}, {})", self.emit_expr(a), self.emit_expr(b))
            }
            LirExpr::StructInit { type_name, fields } => {
                let mut pairs = Vec::new();
                for (k, v) in fields {
                    pairs.push(format!("{k}: {}", self.emit_expr(v)));
                }
                if type_name == "AnonymousForm" {
                    format!("({{ {} }})", pairs.join(", "))
                } else {
                    format!("create{type_name}({{ {} }})", pairs.join(", "))
                }
            }
            LirExpr::Binary { left, op, right } => {
                let l = self.emit_expr(left);
                let r = self.emit_expr(right);
                match op.as_str() {
                    "&" => format!("({{ ...{l}, ...{r} }})"),
                    "==" => format!("({l} === {r})"),
                    "!=" => format!("({l} !== {r})"),
                    _ => format!("({l} {op} {r})"),
                }
            }
            LirExpr::Unary { op, expr } => format!("{op}({})", self.emit_expr(expr)),
            LirExpr::Call { callee, args } => {
                let mapped_callee = if Self::is_console_log(callee) {
                    "console.log"
                } else {
                    callee.as_str()
                };
                let arg_str: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
                format!("{mapped_callee}({})", arg_str.join(", "))
            }
            LirExpr::Pipe { input, op, step } => {
                let inp = self.emit_expr(input);
                let st = self.emit_expr(step);
                match op {
                    PipeOp::Dot => match step.as_ref() {
                        LirExpr::Call { callee, args } => {
                            let mapped = if Self::is_console_log(callee) {
                                "console.log"
                            } else {
                                callee.as_str()
                            };
                            let mut all_args = vec![inp];
                            all_args.extend(args.iter().map(|a| self.emit_expr(a)));
                            format!("{mapped}({})", all_args.join(", "))
                        }
                        _ => format!("{st}({inp})"),
                    },
                    PipeOp::Tap => format!("((_tap) => {{ {st}(_tap); return _tap; }})({inp})"),
                    PipeOp::Enrich => format!("((_in) => ({{ ..._in, ...{st}(_in) }}))({inp})"),
                    PipeOp::Filter => format!("((_in) => ({st}(_in) ? _in : null))({inp})"),
                    PipeOp::Parallel => format!("({st})({inp})"),
                }
            }
            LirExpr::Router { input, branches } => {
                let inp = self.emit_expr(input);
                let mut code = String::from("((_: any) => {\n  const _1 = _; const _2 = _;\n");
                for b in branches {
                    let mut extra_binds = Vec::new();
                    if let Some(bound_var) = &b.bound_type_var {
                        extra_binds.push(format!("    const {bound_var} = _;\n"));
                    }
                    let cond = match &b.cond {
                        LirPattern::Wildcard => "true".into(),
                        LirPattern::TypeMatch(t) => format!("_?.__type === \"{t}\""),
                        LirPattern::LiteralMatch(lit) => format!("_ === {}", self.emit_expr(lit)),
                        LirPattern::RangeMatch(a, b) => {
                            format!("(_ >= {} && _ <= {})", self.emit_expr(a), self.emit_expr(b))
                        }
                        LirPattern::Predicate(p) => self.emit_expr(p),
                        LirPattern::TupleMatch(elems) => {
                            let mut conds =
                                vec![format!("Array.isArray(_) && _.length === {}", elems.len())];
                            for (i, el) in elems.iter().enumerate() {
                                match el {
                                    LirTuplePatElem::Wildcard => {}
                                    LirTuplePatElem::TypeBind {
                                        type_name,
                                        var_name,
                                    } => {
                                        conds.push(format!("_[{i}]?.__type === \"{type_name}\""));
                                        extra_binds
                                            .push(format!("    const {var_name} = _[{i}];\n"));
                                    }
                                    LirTuplePatElem::Bind(v) => {
                                        extra_binds.push(format!("    const {v} = _[{i}];\n"));
                                    }
                                    LirTuplePatElem::Exact(ex) => {
                                        conds.push(format!("_[{i}] === {}", self.emit_expr(ex)));
                                    }
                                }
                            }
                            conds.join(" && ")
                        }
                        LirPattern::FieldMatch(flds) => {
                            let mut conds = Vec::new();
                            for (k, v) in flds {
                                match v {
                                    LirFieldPatValue::Expr(ex) => {
                                        conds.push(format!("_?.{k} === {}", self.emit_expr(ex)));
                                    }
                                    LirFieldPatValue::Bind(var) => {
                                        conds.push(format!("_?.{k} !== undefined"));
                                        extra_binds.push(format!("    const {var} = _.{k};\n"));
                                    }
                                    LirFieldPatValue::Tuple(elems) => {
                                        conds.push(format!(
                                            "Array.isArray(_?.{k}) && _?.{k}.length === {}",
                                            elems.len()
                                        ));
                                        for (i, el) in elems.iter().enumerate() {
                                            match el {
                                                LirTuplePatElem::Wildcard => {}
                                                LirTuplePatElem::TypeBind {
                                                    type_name,
                                                    var_name,
                                                } => {
                                                    conds.push(format!(
                                                        "_.{k}[{i}]?.__type === \"{type_name}\""
                                                    ));
                                                    extra_binds.push(format!(
                                                        "    const {var_name} = _.{k}[{i}];\n"
                                                    ));
                                                }
                                                LirTuplePatElem::Bind(var) => {
                                                    extra_binds.push(format!(
                                                        "    const {var} = _.{k}[{i}];\n"
                                                    ));
                                                }
                                                LirTuplePatElem::Exact(ex) => {
                                                    conds.push(format!(
                                                        "_.{k}[{i}] === {}",
                                                        self.emit_expr(ex)
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if conds.is_empty() {
                                "true".into()
                            } else {
                                conds.join(" && ")
                            }
                        }
                    };
                    code.push_str(&format!("  if ({cond}) {{\n"));
                    if !b.unpacked_fields.is_empty() {
                        code.push_str(&format!(
                            "    const {{ {} }} = _ as any;\n",
                            b.unpacked_fields.join(", ")
                        ));
                    }
                    for bind_line in extra_binds {
                        code.push_str(&bind_line);
                    }
                    for st in &b.body {
                        code.push_str(&format!("{}\n", self.emit_stmt(st, "    ")));
                    }
                    code.push_str("  }\n");
                }
                code.push_str(&format!("  return null;\n}})({inp})"));
                code
            }
            LirExpr::Lambda { params, body, .. } => {
                let p_str = params
                    .iter()
                    .map(|p| format!("{p}: any"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut stmts = vec!["  const _1 = _; const _2 = _;".to_string()];
                stmts.extend(body.iter().map(|s| self.emit_stmt(s, "  ")));
                format!("(({p_str}) => {{\n{}\n}})", stmts.join("\n"))
            }
        }
    }
}

impl TargetEmitter for TypeScriptEmitter {
    fn emit_module(&self, lir: &LirModule) -> String {
        let mut out = String::from(
            "// Generated by Razomy Multi-Target Transpiler (Target: TypeScript)\n\
             export const Razomy = {\n  \
               range: (a: number, b: number) => Array.from({ length: b - a + 1 }, (_, i) => a + i),\n\
             };\n\n",
        );

        for st in &lir.structs {
            if !st.union_variants.is_empty() {
                let variants: Vec<String> =
                    st.union_variants.iter().map(|v| self.map_type(v)).collect();
                out.push_str(&format!(
                    "export type {} = {};\n\n",
                    st.name,
                    variants.join(" | ")
                ));
                continue;
            }

            let tparams = if st.type_params.is_empty() {
                String::new()
            } else {
                format!("<{}>", st.type_params.join(", "))
            };

            out.push_str(&format!("export interface {}{tparams} {{\n", st.name));
            out.push_str(&format!("  readonly __type?: \"{}\";\n", st.name));
            for f in &st.fields {
                out.push_str(&format!(
                    "  {}: {}; // from {}\n",
                    f.name,
                    self.map_type(&f.ty),
                    f.origin
                ));
            }
            for g in &st.reactive_getters {
                out.push_str(&format!("  readonly {}: any;\n", g.name));
            }
            for m in &st.methods {
                let m_params: Vec<String> = m
                    .params
                    .iter()
                    .filter(|(n, _, _)| n != "_")
                    .map(|(n, ty, _)| format!("{n}: {}", self.map_type(ty)))
                    .collect();
                out.push_str(&format!(
                    "  {}({}): {};\n",
                    m.name,
                    m_params.join(", "),
                    self.map_type(&m.return_ty)
                ));
            }
            out.push_str("}\n\n");

            out.push_str(&format!(
                "export function create{}{tparams}(init: Partial<{}{tparams}> = {}): {}{tparams} {{\n  return {{\n    __type: \"{}\",\n",
                st.name, st.name, "{}", st.name, st.name
            ));
            for f in &st.fields {
                let def = f
                    .default_val
                    .as_ref()
                    .map(|d| self.emit_expr(d))
                    .unwrap_or_else(|| "undefined as any".into());
                out.push_str(&format!("    {0}: init.{0} ?? {def},\n", f.name));
            }
            for g in &st.reactive_getters {
                out.push_str(&format!("    get {}() {{\n", g.name));
                out.push_str("      const self = this; const Self = this;\n");
                for f in &st.fields {
                    out.push_str(&format!("      const {0} = this.{0};\n", f.name));
                }
                for prev_g in &st.reactive_getters {
                    if prev_g.name == g.name {
                        break;
                    }
                    out.push_str(&format!("      const {0} = this.{0};\n", prev_g.name));
                }
                for stmt in &g.body {
                    out.push_str(&format!("{}\n", self.emit_stmt(stmt, "      ")));
                }
                out.push_str("    },\n");
            }
            for m in &st.methods {
                let m_params: Vec<String> = m
                    .params
                    .iter()
                    .filter(|(n, _, _)| n != "_")
                    .map(|(n, ty, def)| {
                        let ts_ty = self.map_type(ty);
                        match def {
                            Some(d) => format!("{n}: {ts_ty} = {}", self.emit_expr(d)),
                            None => format!("{n}: {ts_ty}"),
                        }
                    })
                    .collect();
                out.push_str(&format!("    {}({}) {{\n", m.name, m_params.join(", ")));
                out.push_str("      const self = this; const Self = this;\n");
                for f in &st.fields {
                    out.push_str(&format!("      const {0} = this.{0};\n", f.name));
                }
                for stmt in &m.body {
                    out.push_str(&format!("{}\n", self.emit_stmt(stmt, "      ")));
                }
                out.push_str("    },\n");
            }
            out.push_str("  };\n}\n\n");
        }

        for g in &lir.globals {
            out.push_str(&format!(
                "export const {}: {} = {};\n",
                g.name,
                self.map_type(&g.ty),
                self.emit_expr(&g.value)
            ));
        }
        if !lir.globals.is_empty() {
            out.push('\n');
        }

        for func in &lir.functions {
            let params_str: Vec<String> = func
                .params
                .iter()
                .map(|(n, ty, def)| {
                    let ts_ty = self.map_type(ty);
                    match def {
                        Some(d) => format!("{n}: {ts_ty} = {}", self.emit_expr(d)),
                        None => format!("{n}: {ts_ty}"),
                    }
                })
                .collect();

            out.push_str(&format!(
                "export function {}({}): {} {{\n",
                func.name,
                params_str.join(", "),
                self.map_type(&func.return_ty)
            ));

            let mut plain_fields = Vec::new();
            for item in &func.auto_unpacked_fields {
                if let Some(self_var) = item.strip_prefix("@self_bind:") {
                    out.push_str(&format!("  const {self_var} = _;\n"));
                } else {
                    plain_fields.push(item.clone());
                }
            }
            if !plain_fields.is_empty() {
                out.push_str(&format!(
                    "  const {{ {} }} = _ as any;\n",
                    plain_fields.join(", ")
                ));
            }

            for stmt in &func.body {
                out.push_str(&format!("{}\n", self.emit_stmt(stmt, "  ")));
            }
            out.push_str("}\n\n");
        }

        for expr in &lir.top_exprs {
            out.push_str(&format!("{};\n", self.emit_expr(expr)));
        }

        out
    }
}
