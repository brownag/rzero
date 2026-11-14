//! r0-wasm: an R-subset interpreter core intended for WebAssembly builds.

pub mod value;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod vm;
pub mod isa;
