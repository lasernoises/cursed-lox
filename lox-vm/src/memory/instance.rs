use crate::interner::Symbol;
use crate::memory::Class;
use crate::table::Table;
use crate::value::Value;
use lox_gc::{Gc, Trace, Tracer};
use std::cell::UnsafeCell;

pub struct Instance {
    pub class: Gc<Class>,
    fields: UnsafeCell<Table>,
}

impl Instance {
    pub fn new(klass: Gc<Class>) -> Self {
        Self {
            class: klass,
            fields: Default::default(),
        }
    }

    #[inline]
    pub fn field(&self, symbol: Symbol) -> Option<Value> {
        self.fields().get(symbol)
    }

    pub fn set_field(&self, symbol: Symbol, value: Value) {
        let fields = unsafe { &mut *self.fields.get() };
        fields.set(symbol, value);
    }

    fn fields(&self) -> &Table {
        unsafe { &*self.fields.get() }
    }
}

unsafe impl Trace for Instance {
    #[inline]
    fn trace(&self, tracer: &mut Tracer) {
        self.class.trace(tracer);
        self.fields.trace(tracer);
    }
}
