// src/codegen.rs
// =============================================================================
// RAZOMY V5.7 — MULTI-TARGET CODE GENERATORS (TYPESCRIPT & C#)
// Трейт TargetEmitter: добавить новый язык = реализовать этот трейт!
// =============================================================================

use crate::codegen_csharp::CSharpEmitter;
use crate::codegen_typescript::TypeScriptEmitter;
use crate::hir::{lower_to_hir, HirModule};
use crate::lir::{lower_hir_to_lir, LirBranch, LirExpr, LirModule, LirStmt, Target};
use crate::parser::{parse_code, PipeOp};

pub trait TargetEmitter {
    fn emit_module(&self, lir: &LirModule) -> String;
}

/// Единый параметризованный диалект кодогенерации для выражений, инструкций и паттерн-роутеров
pub trait CodegenDialect {
    fn is_csharp(&self) -> bool;
    fn map_type(&self, ty: &str) -> String;
    fn emit_expr(&self, expr: &LirExpr) -> String;

    fn is_console_log(callee: &str) -> bool {
        matches!(callee, "Shell.log" | "Shell.Log")
    }

    fn map_callee<'a>(&self, callee: &'a str) -> &'a str {
        if Self::is_console_log(callee) {
            if self.is_csharp() {
                "Console.WriteLine"
            } else {
                "console.log"
            }
        } else {
            callee
        }
    }

    fn emit_stmt(&self, stmt: &LirStmt, indent: &str) -> String {
        match stmt {
            LirStmt::Let { name, value } => {
                let kw = if self.is_csharp() { "var" } else { "let" };
                format!("{indent}{kw} {name} = {};", self.emit_expr(value))
            }
            LirStmt::AssignPath { path, value } => {
                let lhs = if self.is_csharp() {
                    path.clone()
                } else {
                    normalize_ts_index_path(path)
                };
                format!("{indent}{lhs} = {};", self.emit_expr(value))
            }
            LirStmt::DestructTuple { names, value } => {
                if self.is_csharp() {
                    format!(
                        "{indent}var ({}) = {};",
                        names.join(", "),
                        self.emit_expr(value)
                    )
                } else {
                    format!(
                        "{indent}const [{}] = {};",
                        names.join(", "),
                        self.emit_expr(value)
                    )
                }
            }
            LirStmt::Expr(e) => format!("{indent}{};", self.emit_expr(e)),
            LirStmt::Return(e) => format!("{indent}return {};", self.emit_expr(e)),
        }
    }

    fn emit_pipe(&self, input: &LirExpr, op: &PipeOp, step: &LirExpr) -> String {
        let inp = self.emit_expr(input);
        let st = self.emit_expr(step);
        match op {
            PipeOp::Dot => match step {
                LirExpr::Call { callee, args } => {
                    let mapped = self.map_callee(callee);
                    let mut all_args = vec![inp];
                    all_args.extend(args.iter().map(|a| self.emit_expr(a)));
                    format!("{mapped}({})", all_args.join(", "))
                }
                _ => format!("{st}({inp})"),
            },
            PipeOp::Tap => {
                if self.is_csharp() {
                    format!("RazomyRuntime.Tap({inp}, {st})")
                } else {
                    format!("((_tap) => {{ {st}(_tap); return _tap; }})({inp})")
                }
            }
            PipeOp::Enrich => {
                if self.is_csharp() {
                    format!("RazomyRuntime.Merge({inp}, {st}({inp}))")
                } else {
                    format!("((_in) => ({{ ..._in, ...{st}(_in) }}))({inp})")
                }
            }
            PipeOp::Filter => {
                if self.is_csharp() {
                    format!("({st}({inp}) ? {inp} : null)")
                } else {
                    format!("((_in) => ({st}(_in) ? _in : null))({inp})")
                }
            }
            PipeOp::Parallel => {
                if self.is_csharp() {
                    format!("{st}({inp})")
                } else {
                    format!("({st})({inp})")
                }
            }
        }
    }

    /// Единый компилятор роутеров (`LirBranch`) для TypeScript и C#
    fn emit_router(&self, input: &LirExpr, branches: &[LirBranch]) -> String {
        let inp = self.emit_expr(input);
        let cs = self.is_csharp();
        let bind_kw = if cs { "var" } else { "const" };

        let mut code = if cs {
            String::from("new Func<dynamic, dynamic>((_) => {\n  var _1 = _; var _2 = _;\n")
        } else {
            String::from("((_: any) => {\n  const _1 = _; const _2 = _;\n")
        };

        for b in branches {
            code.push_str(&format!("  if ({}) {{\n", self.emit_expr(&b.condition)));
            if cs {
                for fname in &b.unpacked_fields {
                    code.push_str(&format!("    var {fname} = _.{fname};\n"));
                }
            } else if !b.unpacked_fields.is_empty() {
                code.push_str(&format!(
                    "    const {{ {} }} = _ as any;\n",
                    b.unpacked_fields.join(", ")
                ));
            }
            for (var, val_expr) in &b.bindings {
                code.push_str(&format!(
                    "    {bind_kw} {var} = {};\n",
                    self.emit_expr(val_expr)
                ));
            }
            for st in &b.body {
                code.push_str(&format!("{}\n", self.emit_stmt(st, "    ")));
            }
            code.push_str("  }\n");
        }

        let null_ret = if cs { "null!" } else { "null" };
        code.push_str(&format!("  return {null_ret};\n}})({inp})"));
        code
    }
}

pub fn normalize_ts_index_path(path: &str) -> String {
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

pub fn transpile(source: &str, target: Target) -> Result<String, String> {
    let sentences = parse_code(source).map_err(|e| format!("Ошибка парсинга:\n{e}"))?;
    let hir = lower_to_hir(&sentences);
    Ok(transpile_hir(&hir, target))
}

pub fn transpile_hir(hir: &HirModule, target: Target) -> String {
    let lir = lower_hir_to_lir(hir);
    match target {
        Target::TypeScript => TypeScriptEmitter.emit_module(&lir),
        Target::CSharp => CSharpEmitter.emit_module(&lir),
    }
}