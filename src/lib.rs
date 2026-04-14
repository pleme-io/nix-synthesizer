mod node;
mod types;
mod emitter;
pub mod builders;

#[cfg(feature = "iac-bridge")]
pub mod iac_bridge;

pub use node::{Binding, BinOperator, FnArg, FlakeInput, ModuleOption, NixNode, StringPart};
pub use types::{NixType, SubmoduleOption};
pub use emitter::emit_file;
