use crate::array::Array;
use crate::interner::{Interner, Symbol};
use crate::string::LoxString;
use crate::table::Table;
use crate::value::Value;
use lox_bytecode::bytecode;
use lox_bytecode::bytecode::{Chunk, ClassIndex, ClosureIndex, ConstantIndex, Module};
use lox_gc::{Gc, Trace, Tracer};
use std::cell::UnsafeCell;

//TODO Drop module
pub struct Import {
    pub name: LoxString,
    module: Module,
    globals: UnsafeCell<Table>,
    symbols: Array<Symbol>,
    strings: Array<Gc<LoxString>>,
}

unsafe impl Trace for Import {
    #[inline]
    fn trace(&self, tracer: &mut Tracer) {
        self.name.trace(tracer);
        self.globals.trace(tracer);
        self.symbols.mark(tracer);
        self.strings.trace(tracer);
    }
}

impl Import {
    pub fn new(name: impl Into<LoxString>) -> Self {
        Self {
            name: name.into(),
            module: Module::new(),
            globals: Default::default(),
            symbols: Default::default(),
            strings: Default::default(),
        }
    }

    pub(crate) fn with_module(
        name: impl Into<LoxString>,
        module: Module,
        interner: &mut Interner,
    ) -> Self {
        let symbols = module
            .identifiers()
            .iter()
            .map(|identifier| interner.intern(identifier))
            .collect();

        let strings: Array<Gc<LoxString>> = module
            .strings
            .iter()
            .map(|value| lox_gc::manage(value.into()))
            .collect();

        Self {
            name: name.into(),
            module,
            globals: Default::default(),
            symbols,
            strings,
        }
    }

    pub fn copy_to(&self, other: &Import) {
        let dst = unsafe { &mut *other.globals.get() };
        self.globals().copy_to(dst);
    }

    fn globals(&self) -> &Table {
        unsafe { &*self.globals.get() }
    }

    #[inline]
    pub(crate) fn symbol(&self, index: ConstantIndex) -> Symbol {
        unsafe { *self.symbols.get_unchecked(index) }
    }

    pub(crate) fn chunk(&self, index: usize) -> &Chunk {
        self.module.chunk(index)
    }

    #[inline]
    pub(crate) fn number(&self, index: ConstantIndex) -> f64 {
        self.module.number(index)
    }

    #[inline]
    pub(crate) fn string(&self, index: ConstantIndex) -> Gc<LoxString> {
        unsafe { *self.strings.get_unchecked(index) }
    }

    //TODO rename to make it clear this is not an alive closure.
    pub(crate) fn class(&self, index: ClassIndex) -> &bytecode::Class {
        self.module.class(index)
    }

    //TODO rename to make it clear this is not an alive closure.
    pub(crate) fn closure(&self, index: ClosureIndex) -> &bytecode::Closure {
        self.module.closure(index)
    }

    pub fn set_global(&self, key: Symbol, value: Value) {
        let globals = unsafe { &mut *self.globals.get() };
        globals.set(key, value);
    }

    pub fn has_global(&self, key: Symbol) -> bool {
        self.globals().has(key)
    }

    #[inline]
    pub fn global(&self, key: Symbol) -> Option<Value> {
        self.globals().get(key)
    }
}
