//! r0-wasm: an R-subset interpreter core intended for WebAssembly builds.

pub mod value;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod vm;
pub mod isa;

pub use vm::R0Vm;
pub use value::R0Val;

pub const DEFAULT_FUEL: i64 = 100_000;

/// Evaluate a script in a fresh VM with `DEFAULT_FUEL`.
pub fn eval_script(src: &str) -> Result<R0Val, String> {
    R0Vm::new(DEFAULT_FUEL).eval_script(src)
}
