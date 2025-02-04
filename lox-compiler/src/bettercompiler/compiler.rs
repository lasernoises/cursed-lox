use std::collections::HashMap;

use lox_syntax::position::Diagnostic;
use lox_syntax::position::Span;

use super::locals::*;
use crate::bytecode::*;
use lox_bytecode::opcode;

#[derive(Copy, Clone, PartialEq)]
pub enum ContextType {
    Function,
    Initializer,
    Method,
    TopLevel,
}

struct CompilerContext {
    context_type: ContextType,
    chunk_index: ChunkIndex,
    locals: Locals,
    upvalues: Vec<Upvalue>,
}

pub struct Compiler {
    module: Module,
    contexts: Vec<CompilerContext>,
    errors: Vec<Diagnostic>,
    identifiers: HashMap<String, IdentifierIndex>,
    numbers: HashMap<u64, ConstantIndex>,
    strings: HashMap<String, ConstantIndex>,
}

impl CompilerContext {
    fn new(context_type: ContextType, chunk_index: ChunkIndex) -> CompilerContext {
        CompilerContext {
            context_type,
            chunk_index,
            locals: Locals::new(),
            upvalues: vec![],
        }
    }

    fn add_upvalue(&mut self, upvalue: Upvalue) -> StackIndex {
        for i in 0..self.upvalues.len() {
            let existing_upvalue = &self.upvalues[i];
            if upvalue == *existing_upvalue {
                return i;
            }
        }

        self.upvalues.push(upvalue);

        self.upvalues.len() - 1
    }

    fn resolve_local(&self, name: &str) -> Result<Option<StackIndex>, ()> {
        if let Some(local) = self.locals.get(name) {
            if !local.initialized() {
                Err(())
            } else {
                Ok(Some(local.slot()))
            }
        } else {
            Ok(None)
        }
    }
}

impl Compiler {
    fn current_context(&self) -> &CompilerContext {
        self.contexts.last().expect("no context")
    }

    fn current_context_mut(&mut self) -> &mut CompilerContext {
        self.contexts.last_mut().expect("no context")
    }

    fn current_chunk_mut(&mut self) -> &mut Chunk {
        self.module.chunk_mut(self.current_context().chunk_index)
    }

    fn current_chunk(&self) -> &Chunk {
        self.module.chunk(self.current_context().chunk_index)
    }

    fn begin_context(&mut self, context_type: ContextType) {
        let chunk = self.module.add_chunk();
        self.contexts
            .push(CompilerContext::new(context_type, chunk));
    }

    fn end_context(&mut self) -> (ChunkIndex, Vec<Upvalue>) {
        let context = self.contexts.pop().expect("no context");
        (context.chunk_index, context.upvalues)
    }

    fn begin_scope(&mut self) {
        self.current_context_mut().locals.begin_scope();
    }

    fn end_scope(&mut self) {
        for local in self.current_context_mut().locals.end_scope().iter().rev() {
            if local.captured() {
                self.add_u8(opcode::CLOSE_UPVALUE);
            } else {
                self.add_u8(opcode::POP);
            }
        }
    }

    pub fn new() -> Compiler {
        Compiler {
            module: Module::new(),
            contexts: vec![],
            errors: vec![],
            identifiers: HashMap::new(),
            numbers: HashMap::new(),
            strings: HashMap::new(),
        }
    }

    pub fn into_module(self) -> Module {
        self.module
    }

    pub fn into_errors(self) -> Vec<Diagnostic> {
        self.errors
    }

    pub fn context_type(&self) -> ContextType {
        self.current_context().context_type
    }

    pub fn add_error(&mut self, message: &str, span: Span) {
        self.errors.push(Diagnostic {
            message: message.to_string(),
            span,
        });
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn in_method_or_initializer_nested(&self) -> bool {
        for context in self.contexts.iter().rev() {
            match context.context_type {
                ContextType::Method => return true,
                ContextType::Initializer => return true,
                _ => (),
            }
        }
        false
    }

    pub fn with_scope<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.begin_scope();
        f(self);
        self.end_scope();
    }

    pub fn is_scoped(&mut self) -> bool {
        let c = self.current_context();
        c.locals.scope_depth() > 0
    }

    pub fn with_context<F>(&mut self, context_type: ContextType, f: F) -> (ChunkIndex, Vec<Upvalue>)
    where
        F: FnOnce(&mut Self),
    {
        self.begin_context(context_type);

        //TODO Move to begin_context

        if context_type != ContextType::Function {
            self.add_local("this");
        } else {
            self.add_local("");
        }
        self.mark_local_initialized();

        f(self);
        self.end_context()
    }

    pub fn with_scoped_context<F>(
        &mut self,
        context_type: ContextType,
        f: F,
    ) -> (ChunkIndex, Vec<Upvalue>)
    where
        F: FnOnce(&mut Self),
    {
        self.with_context(context_type, |compiler| {
            compiler.begin_scope();
            f(compiler)
        })
    }

    pub fn add_u8(&mut self, instruction: u8) -> InstructionIndex {
        self.current_chunk_mut().add_u8(instruction)
    }

    pub fn add_u32(&mut self, instruction: u32) -> InstructionIndex {
        self.current_chunk_mut().add_u32(instruction)
    }

    pub fn add_i16(&mut self, instruction: i16) -> InstructionIndex {
        self.current_chunk_mut().add_i16(instruction)
    }

    pub fn add_u16(&mut self, instruction: u16) -> InstructionIndex {
        self.current_chunk_mut().add_u16(instruction)
    }

    pub fn patch_instruction(&mut self, index: InstructionIndex) {
        self.current_chunk_mut().patch_instruction(index)
    }

    pub fn patch_instruction_to(&mut self, index: InstructionIndex, to: InstructionIndex) {
        self.current_chunk_mut().patch_instruction_to(index, to)
    }

    pub fn instruction_index(&self) -> InstructionIndex {
        self.current_chunk().instruction_index()
    }

    pub fn add_local(&mut self, name: &str) {
        self.current_context_mut().locals.insert(name);
    }

    pub fn has_local_in_current_scope(&self, name: &str) -> bool {
        self.current_context()
            .locals
            .get_at_current_depth(name)
            .is_some()
    }

    pub fn mark_local_initialized(&mut self) {
        //TODO refactor
        //TODO Return early if not scoped
        self.current_context_mut().locals.mark_initialized()
    }

    pub fn resolve_local(&mut self, name: &str) -> Option<StackIndex> {
        match self.current_context().resolve_local(name) {
            Err(_) => {
                self.add_error("Local not initialized", Span::empty()); //TODO
                None
            }
            Ok(local) => local,
        }
    }

    pub fn add_number(&mut self, value: f64) -> ConstantIndex {
        if let Some(index) = self.numbers.get(&value.to_bits()) {
            *index
        } else {
            let index = self.module.add_number(value);
            self.numbers.insert(value.to_bits(), index);
            index
        }
    }

    pub fn add_string(&mut self, value: &str) -> ConstantIndex {
        if let Some(index) = self.strings.get(value) {
            *index
        } else {
            let index = self.module.add_string(value);
            self.strings.insert(value.to_string(), index);
            index
        }
    }

    pub fn add_closure(&mut self, closure: Closure) -> ClosureIndex {
        self.module.add_closure(closure)
    }

    pub fn add_class(&mut self, class: Class) -> ClassIndex {
        self.module.add_class(class)
    }

    pub fn add_identifier(&mut self, identifier: &str) -> IdentifierIndex {
        if let Some(index) = self.identifiers.get(identifier) {
            *index
        } else {
            let index = self.module.add_identifier(identifier);
            self.identifiers.insert(identifier.to_string(), index);
            index
        }
    }

    pub fn resolve_upvalue(&mut self, name: &str) -> Option<StackIndex> {
        for i in (0..(self.contexts.len() - 1)).rev() {
            // Skip the current context
            match self.contexts[i].resolve_local(name) {
                Err(_) => {
                    self.add_error("Local not initialized", Span::empty()); //TODO
                    return None;
                }
                Ok(Some(local)) => {
                    //TODO expect() this instead?, locals should *never* be uninitialized when resolving upvalues
                    self.contexts[i].locals.mark_captured(local);
                    let mut upvalue = self.contexts[i + 1].add_upvalue(Upvalue::Local(local));
                    for j in (i + 2)..self.contexts.len() {
                        upvalue = self.contexts[j].add_upvalue(Upvalue::Upvalue(upvalue));
                    }
                    return Some(upvalue);
                }
                Ok(None) => (),
            }
        }

        None
    }
}
