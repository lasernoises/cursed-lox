mod compiler;
mod locals;
mod statements;

#[cfg(test)]
mod tests;

use crate::bytecode::*;
use compiler::{Compiler, ContextType};
use lox_bytecode::opcode;
use lox_syntax::ast::*;
use lox_syntax::position::Diagnostic;
use statements::compile_ast;

pub fn compile(ast: &Ast) -> Result<Module, Vec<Diagnostic>> {
    let mut compiler = Compiler::new();

    let _ = compiler.with_context(ContextType::TopLevel, |compiler| {
        compile_ast(compiler, ast);
        compiler.add_u8(opcode::NIL);
        compiler.add_u8(opcode::RETURN);
    });

    if compiler.has_errors() {
        Err(compiler.into_errors())
    } else {
        Ok(compiler.into_module())
    }
}
