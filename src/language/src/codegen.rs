// src/codegen.rs
// =============================================================================
// RAZOMY V5.0 — MULTI-TARGET CODE GENERATORS (TYPESCRIPT & C#)
// Трейт TargetEmitter: добавить новый язык = реализовать этот трейт!
// =============================================================================

use crate::codegen_csharp::CSharpEmitter;
use crate::codegen_typescript::TypeScriptEmitter;
use crate::hir::{lower_to_hir, HirModule};
use crate::lir::{
    lower_hir_to_lir, LirExpr, LirInterpPart, LirModule, LirPattern, LirStmt, Target,
};
use crate::parser::{parse_code, PipeOp};

pub trait TargetEmitter {
    fn emit_module(&self, lir: &LirModule) -> String;
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
