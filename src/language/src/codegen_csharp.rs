// =============================================================================
// 2. C# EMITTER (.NET 8+ RECORDS & PATTERN MATCHING)
// =============================================================================

use crate::codegen::TargetEmitter;
use crate::lir::{
    LirExpr, LirFieldPatValue, LirInterpPart, LirModule, LirPattern, LirStmt, LirTuplePatElem,
};
use crate::parser::PipeOp;

pub struct CSharpEmitter;

impl CSharpEmitter {
    fn map_type(&self, ty: &str) -> String {
        if ty.contains(" | ") {
            return "dynamic".into();
        }
        match ty {
            "Int" => "long".into(),
            "Float" | "Percent" => "double".into(),
            "Bool" => "bool".into(),
            "Byte" => "byte".into(),
            "String" | "Money" | "Date" | "Time" | "Duration" | "Url" | "Email" | "File"
            | "Coord" | "Color" | "IpAddr" | "Version" => "string".into(),
            "None" => "object?".into(),
            "Any" | "Form" | "Stream" | "ReactiveStream" => "dynamic".into(),
            "Tuple" | "Range" => "object[]".into(),
            other if other.starts_with('\'') => "string".into(),
            other if other.contains('(') => {
                let base = other.split('(').next().unwrap_or("dynamic");
                format!("{base}<dynamic>")
            }
            other => other.to_string(),
        }
    }

    fn emit_stmt(&self, stmt: &LirStmt, indent: &str) -> String {
        match stmt {
            LirStmt::Let { name, value } => {
                format!("{indent}var {name} = {};", self.emit_expr(value))
            }
            LirStmt::DestructTuple { names, value } => {
                format!(
                    "{indent}var ({}) = {};",
                    names.join(", "),
                    self.emit_expr(value)
                )
            }
            LirStmt::Expr(e) => format!("{indent}{};", self.emit_expr(e)),
            LirStmt::Return(e) => format!("{indent}return {};", self.emit_expr(e)),
        }
    }

    fn emit_expr(&self, expr: &LirExpr) -> String {
        match expr {
            LirExpr::None => "null".into(),
            LirExpr::Int(i) => format!("{i}L"),
            LirExpr::Float(f) | LirExpr::Percent(f) => format!("{f}d"),
            LirExpr::Bool(b) => b.to_string(),
            LirExpr::Byte(b) => format!("(byte){b}"),
            LirExpr::Str(s)
            | LirExpr::Symbol(s)
            | LirExpr::Money(s)
            | LirExpr::Date(s)
            | LirExpr::Duration(s) => format!("\"{s}\""),
            LirExpr::InterpStr(parts) => {
                let mut s = String::from("$\"");
                for p in parts {
                    match p {
                        LirInterpPart::Lit(l) => s.push_str(l),
                        LirInterpPart::Expr(e) => s.push_str(&format!("{{{e}}}")),
                    }
                }
                s.push('"');
                s
            }
            LirExpr::Ident(id) => match id.as_str() {
                "Shell.Log" => "Console.WriteLine".into(),
                _ => id.clone(),
            },
            LirExpr::Tuple(items) => {
                let list: Vec<String> = items.iter().map(|e| self.emit_expr(e)).collect();
                format!("new object[] {{ {} }}", list.join(", "))
            }
            LirExpr::Range(a, b) => format!(
                "RazomyRuntime.Range({}, {})",
                self.emit_expr(a),
                self.emit_expr(b)
            ),
            LirExpr::StructInit { type_name, fields } => {
                let pairs: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{k} = {}", self.emit_expr(v)))
                    .collect();
                if type_name == "AnonymousForm" {
                    format!("new {{ {} }}", pairs.join(", "))
                } else {
                    format!("new {type_name} {{ {} }}", pairs.join(", "))
                }
            }
            LirExpr::Binary { left, op, right } => {
                let l = self.emit_expr(left);
                let r = self.emit_expr(right);
                match op.as_str() {
                    "&" => format!("RazomyRuntime.Merge({l}, {r})"),
                    _ => format!("({l} {op} {r})"),
                }
            }
            LirExpr::Unary { op, expr } => format!("{op}({})", self.emit_expr(expr)),
            LirExpr::Call { callee, args } => {
                let mapped = if callee == "Shell.Log" {
                    "Console.WriteLine"
                } else {
                    callee.as_str()
                };
                let arg_str: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
                format!("{mapped}({})", arg_str.join(", "))
            }
            LirExpr::Pipe { input, op, step } => {
                let inp = self.emit_expr(input);
                let st = self.emit_expr(step);
                match op {
                    PipeOp::Dot => match step.as_ref() {
                        LirExpr::Call { callee, args } => {
                            let mapped = if callee == "Shell.Log" {
                                "Console.WriteLine"
                            } else {
                                callee.as_str()
                            };
                            let mut all_args = vec![inp];
                            all_args.extend(args.iter().map(|a| self.emit_expr(a)));
                            format!("{mapped}({})", all_args.join(", "))
                        }
                        _ => format!("{st}({inp})"),
                    },
                    PipeOp::Tap => format!("RazomyRuntime.Tap({inp}, {st})"),
                    PipeOp::Enrich => format!("RazomyRuntime.Merge({inp}, {st}({inp}))"),
                    PipeOp::Filter => format!("({st}({inp}) ? {inp} : null)"),
                    PipeOp::Parallel => format!("{st}({inp})"),
                }
            }
            LirExpr::Router { input, branches } => {
                let inp = self.emit_expr(input);
                let mut code = String::from("new Func<dynamic, dynamic>((_) => {\n");
                for b in branches {
                    let mut extra_binds = Vec::new();
                    let cond = match &b.cond {
                        LirPattern::Wildcard => "true".into(),
                        LirPattern::TypeMatch(t) => format!("_ is {t}"),
                        LirPattern::LiteralMatch(lit) => {
                            format!("Equals(_, {})", self.emit_expr(lit))
                        }
                        LirPattern::RangeMatch(a, b) => {
                            format!("(_ >= {} && _ <= {})", self.emit_expr(a), self.emit_expr(b))
                        }
                        LirPattern::Predicate(p) => self.emit_expr(p),
                        LirPattern::TupleMatch(elems) => {
                            let mut conds = vec![format!("_.Length == {}", elems.len())];
                            for (i, el) in elems.iter().enumerate() {
                                match el {
                                    LirTuplePatElem::Wildcard => {}
                                    LirTuplePatElem::Bind(v) => {
                                        extra_binds.push(format!("    var {v} = _[{i}];\n"));
                                    }
                                    LirTuplePatElem::Exact(ex) => {
                                        conds.push(format!(
                                            "Equals(_[{i}], {})",
                                            self.emit_expr(ex)
                                        ));
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
                                        conds.push(format!(
                                            "Equals(_.{k}, {})",
                                            self.emit_expr(ex)
                                        ));
                                    }
                                    LirFieldPatValue::Tuple(elems) => {
                                        conds.push(format!("_.{k}.Length == {}", elems.len()));
                                        for (i, el) in elems.iter().enumerate() {
                                            match el {
                                                LirTuplePatElem::Wildcard => {}
                                                LirTuplePatElem::Bind(var) => {
                                                    extra_binds
                                                        .push(format!("    var {var} = _.{k}[{i}];\n"));
                                                }
                                                LirTuplePatElem::Exact(ex) => {
                                                    conds.push(format!(
                                                        "Equals(_.{k}[{i}], {})",
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
                    for fname in &b.unpacked_fields {
                        code.push_str(&format!("    var {fname} = _.{fname};\n"));
                    }
                    for bind_line in extra_binds {
                        code.push_str(&bind_line);
                    }
                    for st in &b.body {
                        code.push_str(&format!("{}\n", self.emit_stmt(st, "    ")));
                    }
                    code.push_str("  }\n");
                }
                code.push_str(&format!("  return null!;\n}})({inp})"));
                code
            }
            LirExpr::Lambda { params, body, .. } => {
                let p_str = params
                    .iter()
                    .map(|p| format!("dynamic {p}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let stmts: Vec<String> = body.iter().map(|s| self.emit_stmt(s, "  ")).collect();
                format!("(({p_str}) => {{\n{}\n}})", stmts.join("\n"))
            }
        }
    }
}

impl TargetEmitter for CSharpEmitter {
    fn emit_module(&self, lir: &LirModule) -> String {
        let mut out = String::from(
            "// Generated by Razomy Multi-Target Transpiler (Target: C# .NET 8+)\n\
             using System;\n\
             using System.Linq;\n\
             using System.Collections.Generic;\n\n\
             namespace RazomyGenerated;\n\n\
             public static class RazomyRuntime\n\
             {\n\
                 public static long[] Range(long a, long b) => Enumerable.Range((int)a, (int)(b - a + 1)).Select(x => (long)x).ToArray();\n\
                 public static T Tap<T>(T val, Action<T> action) { action(val); return val; }\n\
                 public static dynamic Merge(dynamic a, dynamic b) => b;\n\
             }\n\n",
        );

        for st in &lir.structs {
            if !st.union_variants.is_empty() {
                out.push_str(&format!(
                    "// Union Type {}: {}\n",
                    st.name,
                    st.union_variants.join(" | ")
                ));
                continue;
            }

            let tparams = if st.type_params.is_empty() {
                String::new()
            } else {
                format!("<{}>", st.type_params.join(", "))
            };

            out.push_str(&format!("public record {}{tparams}\n{{\n", st.name));
            for f in &st.fields {
                let def = f
                    .default_val
                    .as_ref()
                    .map(|d| format!(" = {};", self.emit_expr(d)))
                    .unwrap_or_else(|| " = default!;".into());
                out.push_str(&format!(
                    "    public {} {} {{ get; init; }}{def} // from {}\n",
                    self.map_type(&f.ty),
                    f.name,
                    f.origin
                ));
            }
            for g in &st.reactive_getters {
                out.push_str(&format!(
                    "    public dynamic {}\n    {{\n        get\n        {{\n",
                    g.name
                ));
                out.push_str("            var Self = this;\n");
                for stmt in &g.body {
                    out.push_str(&format!("{}\n", self.emit_stmt(stmt, "            ")));
                }
                out.push_str("        }\n    }\n");
            }
            for m in &st.methods {
                let m_params: Vec<String> = m
                    .params
                    .iter()
                    .filter(|(n, _, _)| n != "_")
                    .map(|(n, ty, def)| {
                        let cs_ty = self.map_type(ty);
                        match def {
                            Some(d) => format!("{cs_ty} {n} = {}", self.emit_expr(d)),
                            None => format!("{cs_ty} {n}"),
                        }
                    })
                    .collect();
                out.push_str(&format!(
                    "    public {} {}({})\n    {{\n",
                    self.map_type(&m.return_ty),
                    m.name,
                    m_params.join(", ")
                ));
                out.push_str("        var Self = this;\n");
                for stmt in &m.body {
                    out.push_str(&format!("{}\n", self.emit_stmt(stmt, "        ")));
                }
                out.push_str("    }\n");
            }
            out.push_str("}\n\n");
        }

        out.push_str("public static class RazomyModule\n{\n");

        for g in &lir.globals {
            out.push_str(&format!(
                "    public static readonly {} {} = {};\n",
                self.map_type(&g.ty),
                g.name,
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
                    let cs_ty = self.map_type(ty);
                    match def {
                        Some(d) => format!("{cs_ty} {n} = {}", self.emit_expr(d)),
                        None if n == "_" => format!("{cs_ty} {n}"),
                        None => format!("{cs_ty} {n}"),
                    }
                })
                .collect();

            out.push_str(&format!(
                "    public static {} {}({})\n    {{\n",
                self.map_type(&func.return_ty),
                func.name,
                params_str.join(", ")
            ));

            for fname in &func.auto_unpacked_fields {
                out.push_str(&format!("        var {fname} = _.{fname};\n"));
            }

            for stmt in &func.body {
                out.push_str(&format!("{}\n", self.emit_stmt(stmt, "        ")));
            }
            out.push_str("    }\n\n");
        }

        if !lir.top_exprs.is_empty() {
            out.push_str("    public static void RunTopLevel()\n    {\n");
            for expr in &lir.top_exprs {
                out.push_str(&format!("        {};\n", self.emit_expr(expr)));
            }
            out.push_str("    }\n");
        }

        out.push_str("}\n");
        out
    }
}