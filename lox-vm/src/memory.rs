mod bound_method;
mod class;
mod closure;
mod import;
mod instance;
mod list;
mod native_function;
mod upvalue;

pub use bound_method::*;
pub use class::*;
pub use closure::*;
pub use import::*;
pub use instance::*;
pub use list::*;
pub use native_function::*;
pub use upvalue::*;

use crate::string::LoxString;
use lox_gc::Gc;
use std::fmt;

pub fn print(value: Gc<()>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if value.is::<String>() {
        write!(f, "{}", value.cast::<String>().as_str())
    } else if value.is::<LoxString>() {
        write!(f, "{}", value.cast::<LoxString>().as_str())
    } else if value.is::<Closure>() {
        write!(f, "<fn {}>", value.cast::<Closure>().function.name)
    } else if value.is::<BoundMethod>() {
        write!(f, "<bound {}>", value.cast::<BoundMethod>().method)
    } else if value.is::<NativeFunction>() {
        write!(f, "<native fn>")
    } else if value.is::<Class>() {
        write!(f, "{}", value.cast::<Class>().name)
    } else if value.is::<Instance>() {
        write!(f, "{} instance", value.cast::<Instance>().class.name)
    } else if value.is::<Import>() {
        write!(f, "<import {}>", value.cast::<Import>().name)
    } else if value.is::<List>() {
        write!(f, "{}", value.cast::<List>())
    } else {
        write!(f, "<unknown>")
    }
}
