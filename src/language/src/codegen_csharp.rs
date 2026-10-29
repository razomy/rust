// src/codegen_csharp.rs
// =============================================================================
// 2. C# EMITTER (.NET 8+ RECORDS & PATTERN MATCHING — RAZOMY V5.7)
// =============================================================================

use crate::codegen::{normalize_ts_index_path, CodegenDialect, TargetEmitter};
use crate::lir::{LirExpr, LirInterpPart, LirModule};

pub struct CSharpEmitter;

impl CodegenDialect for CSharpEmitter {
    fn is_csharp(&self) -> bool {
        true
    }

    fn map_type(&self, ty: &str) -> String {
        if ty.contains(" | ") {
            return "dynamic".into();
        }
        match ty {
            "Int" => "long".into(),
            "Float" => "double".into(),
            "Bool" => "bool".into(),
            "Byte" => "byte".into(),
            "String" | "Symbol" | "Money" | "Usd" | "Date" | "Time" | "Duration" | "Url"
            | "Email" | "File" | "Path" | "Pair" | "Hex" | "Ip" | "Version" => "string".into(),
            "None" => "object?".into(),
            "Any" | "Form" | "Stream" | "ReactiveStream" | "_" => "dynamic".into(),
            "Tuple" | "Range" => "object[]".into(),
            other if other.starts_with('\'') => "string".into(),
            other if other.starts_with("Tuple(") => "object[]".into(),
            other if other.contains('(') || other.contains('[') || other.contains('/') => {
                "dynamic".into()
            }
            other => other.to_string(),
        }
    }

    fn emit_expr(&self, expr: &LirExpr) -> String {
        match expr {
            LirExpr::None => "null".into(),
            LirExpr::Int(i) => format!("{i}L"),
            LirExpr::Float(f) => format!("{f}d"),
            LirExpr::Bool(b) => b.to_string(),
            LirExpr::Byte(b) => format!("(byte){b}"),
            LirExpr::Str(s) | LirExpr::Symbol(s) => format!("{s:?}"),
            LirExpr::InterpStr(parts) => {
                let mut s = String::from("$\"");
                for p in parts {
                    match p {
                        LirInterpPart::Lit(l) => s.push_str(&l.replace('"', "\\\"")),
                        LirInterpPart::Expr(e) => s.push_str(&format!("{{{e}}}")),
                    }
                }
                s.push('"');
                s
            }
            LirExpr::Ident(id) => {
                if Self::is_console_log(id) {
                    "Console.WriteLine".into()
                } else {
                    normalize_ts_index_path(id)
                }
            }
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
                    "is" => format!("{l} is {r}"),
                    "tuple_len_eq" => format!("({l}.Length == {r})"),
                    "&" => format!("RazomyRuntime.Merge({l}, {r})"),
                    _ => format!("({l} {op} {r})"),
                }
            }
            LirExpr::Unary { op, expr } => format!("{op}({})", self.emit_expr(expr)),
            LirExpr::Call { callee, args } => {
                let mapped = self.map_callee(callee);
                let arg_str: Vec<String> = args.iter().map(|a| self.emit_expr(a)).collect();
                format!("{mapped}({})", arg_str.join(", "))
            }
            LirExpr::Pipe { input, op, step } => self.emit_pipe(input, op, step),
            LirExpr::Router { input, branches } => self.emit_router(input, branches),
            LirExpr::Lambda { params, body, .. } => {
                let p_str = params
                    .iter()
                    .map(|p| format!("dynamic {p}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut stmts = vec!["  var _1 = _; var _2 = _;".to_string()];
                stmts.extend(body.iter().map(|s| self.emit_stmt(s, "  ")));
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
            if st.is_prelude {
                continue;
            }
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
                out.push_str("            var self = this; var Self = this;\n");
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
                out.push_str("        var self = this; var Self = this;\n");
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
                if let Some(self_var) = fname.strip_prefix("@self_bind:") {
                    out.push_str(&format!("        var {self_var} = _;\n"));
                } else {
                    out.push_str(&format!("        var {fname} = _.{fname};\n"));
                }
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
