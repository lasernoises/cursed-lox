use lox_bytecode::bytecode::Module;
use super::memory::*;
use super::interner::{Symbol, Interner};
use super::gc::{Gc, Trace, Heap};
use crate::fiber::Fiber;
use std::collections::HashMap;
use std::cell::UnsafeCell;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum Signal {
    Done,
    More,
    RuntimeError,
    ContextSwitch,
}

//TODO thiserror
//TODO RuntimeError
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VmError {
    Unknown,
    StackEmpty,
    FrameEmpty,
    StringConstantExpected,
    GlobalNotDefined,
    InvalidCallee,
    IncorrectArity,
    UnexpectedConstant,
    ClosureConstantExpected,
    UnexpectedValue,
    UndefinedProperty,
    Unimplemented,
    UnknownImport,
}

pub struct Runtime {
    pub fiber: Gc<UnsafeCell<Fiber>>,
    next_fiber: Option<Gc<UnsafeCell<Fiber>>>,
    init_symbol: Symbol,
    pub interner: Interner,
    imports: HashMap<String, Gc<Import>>,
    pub heap: Heap,

    // Env
    pub print: for<'r> fn(&'r str),
    pub import: for<'r> fn(&'r str) -> Option<Module>,

    ip: *const u8,
}

impl Trace for Runtime {
    #[inline]
    fn trace(&self) {
        self.fiber.trace();
        self.next_fiber.trace();
        self.imports.trace();
    }
}

pub fn default_print(value: &str) {
    println!("{}", value);
}

pub fn default_import(_value: &str) -> Option<Module> {
    None
}

impl Runtime {
    pub fn new() -> Self {
        let mut interner = Interner::new();
        let heap = Heap::new();
        let fiber = heap.manage(UnsafeCell::new(Fiber::new(None)));
        let mut imports = HashMap::new();

        let globals = heap.manage(Import::new());
        imports.insert("_globals".to_string(), globals);

        Self {
            fiber,
            next_fiber: None,
            init_symbol: interner.intern("init"),
            interner,
            heap,
            imports,
            print: default_print,
            import: default_import,

            ip: std::ptr::null(),
        }
    }

    pub fn context_switch(&mut self) {
        if let Some(next_fiber) = self.next_fiber {
            self.store_ip();
            self.fiber = next_fiber;
            self.load_ip();
        }

        self.next_fiber = None;
    }

    pub fn switch_to(&mut self, fiber: Gc<UnsafeCell<Fiber>>) -> Signal {
        self.next_fiber = Some(fiber);
        Signal::ContextSwitch
    }

    #[inline]
    pub fn fiber(&self) -> &Fiber {
        unsafe { &*self.fiber.get() }
    }

    #[inline]
    pub fn fiber_mut(&mut self) -> &mut Fiber {
        unsafe { &mut *self.fiber.get() }
    }

    pub fn print(&self, value: &str) {
        (self.print)(value);
    }

    pub fn with_module(&mut self, module: Module) {
        let closure = self.prepare_interpret(module);
        self.fiber_mut().stack.push(Value::Closure(closure));
        self.fiber_mut().begin_frame(closure);
        self.load_ip();
    }

    pub fn adjust_size<T: 'static + Trace>(&mut self, object: Gc<T>) {
        self.heap.adjust_size(object);
    }

    pub fn manage<T: Trace>(&mut self, data: T) -> Gc<T> {
        self.heap.collect(&[self]);
        self.heap.manage(data)
    }

    //TODO Use name given instead of _root
    fn prepare_interpret(&mut self, module: Module) -> Gc<Closure> {
        let import = Import::with_module(module, &mut self.interner);
        let import = self.manage(import);
        self.imports.insert("_root".into(), import);
        self.globals_import().copy_to(&import);

        let function = Function {
            arity: 0,
            chunk_index: 0,
            name: "top".into(),
            import,
        };

        self.manage(Closure {
            upvalues: vec![],
            function,
        })
    }

    pub fn import(&mut self, path: &str) -> Option<Gc<Import>> {
        if let Some(import) = self.imports.get(path) {
            Some(*import)
        } else {
            None
        }
    }

    pub fn load_import(&mut self, path: &str) -> Result<Gc<Import>, VmError> {
        let module = (self.import)(path);
        if let Some(module) = module {
            let import = Import::with_module(module, &mut self.interner);
            let import = self.manage(import);
            self.imports.insert(path.into(), import);
            self.globals_import().copy_to(&import);

            Ok(import)
        } else {
            Err(VmError::UnknownImport)
        }
    }

    pub fn current_import(&self) -> Gc<Import> {
        self.fiber().current_frame().closure.function.import
    }

    pub fn globals_import(&self) -> Gc<Import> {
        *self.imports.get("_globals").expect("Could not find globals import")
    }

    //TODO Reduce duplicate code paths
    #[inline(never)]
    pub fn call(&mut self, arity: usize, callee: Value) -> Signal {
        self.store_ip();

        match callee {
            Value::Closure(callee) => {
                if callee.function.arity != arity {
                    return self.fiber_mut().runtime_error(VmError::IncorrectArity);
                }
                self.fiber_mut().begin_frame(callee);
            }
            Value::NativeFunction(callee) => {
                let args = self.fiber_mut().stack.pop_n(arity);
                let _this = self.fiber_mut().stack.pop(); // discard callee
                let result = (callee.code)(&args);
                self.fiber_mut().stack.push(result);
            }
            Value::Class(class) => {
                let instance = self.manage(Instance::new(class));
                self.fiber_mut().stack.rset(arity, Value::Instance(instance));

                if let Some(initializer) = class.method(self.init_symbol) {
                    if let Value::Closure(initializer) = initializer {
                        if initializer.function.arity != arity {
                            return self.fiber_mut().runtime_error(VmError::IncorrectArity);
                        }
                        self.fiber_mut().begin_frame(initializer);
                    } else {
                        return self.fiber_mut().runtime_error(VmError::UnexpectedValue);
                    }
                } else if arity != 0 {
                    // Arity must be 0 without initializer
                    return self.fiber_mut().runtime_error(VmError::IncorrectArity);
                }
            }
            Value::BoundMethod(bind) => {
                self.fiber_mut().stack.rset(arity, Value::Instance(bind.receiver));
                return self.call(arity, bind.method);

            },
            _ => return self.fiber_mut().runtime_error(VmError::InvalidCallee),
        }

        self.load_ip();

        Signal::More
    }

    pub fn push_string(&mut self, string: impl Into<String>) {
        let root = self.manage(string.into());
        self.fiber_mut().stack.push(Value::String(root));
    }


    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        unsafe {
            let slice = &*std::ptr::slice_from_raw_parts(self.ip, 4);
            let value = u32::from_le_bytes(slice.try_into().unwrap());
            self.ip = self.ip.add(4);
            value
        }
    }

    #[inline]
    pub fn next_i16(&mut self) -> i16 {
        unsafe {
            let slice = &*std::ptr::slice_from_raw_parts(self.ip, 2);
            let value = i16::from_le_bytes(slice.try_into().unwrap());
            self.ip = self.ip.add(2);
            value
        }
    }

    #[inline]
    pub fn next_u8(&mut self) -> u8 {
        unsafe {
            let value = std::ptr::read(self.ip);
            self.ip = self.ip.add(1);
            value
        }
    }

    #[inline]
    pub fn store_ip(&mut self) {
        let ip = self.ip;
        self.fiber_mut().current_frame_mut().store_ip(ip);
    }

    #[inline]
    pub fn load_ip(&mut self) {
        self.ip = self.fiber().current_frame().load_ip();
    }

    #[inline]
    pub fn set_ip(&mut self, to: i16) {
        unsafe {
            self.ip = self.ip.offset(to as isize);
        }
    }
}
