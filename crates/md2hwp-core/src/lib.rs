#![forbid(unsafe_code)]
//! Backend-neutral core for `md2hwp`.

pub mod ir;
pub mod ir_io;
pub mod validate;

pub use ir::Document;
pub use ir_io::{IrReadError, IrReadErrorCode, IrWriteError, read_ir, write_ir};
pub use validate::{SemanticError, ValidatedDocument, ValidationLimits, validate};
