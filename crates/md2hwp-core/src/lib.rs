#![forbid(unsafe_code)]
//! Backend-neutral core for `md2hwp`.

pub mod ast2ir_rules;
pub mod ir;
pub mod ir_io;
pub mod normalize;
pub mod pandoc_input;
pub mod validate;

pub use ast2ir_rules::{Ast2IrRules, RulesError, load_builtin_rules};
pub use ir::Document;
pub use ir_io::{IrReadError, IrReadErrorCode, IrWriteError, read_ir, write_ir};
pub use normalize::{NormalizeError, normalize_pandoc};
pub use pandoc_input::{PandocDocument, PandocInputError, read_pandoc_json};
pub use validate::{SemanticError, ValidatedDocument, ValidationLimits, validate};
