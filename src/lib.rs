pub mod builders;
mod emitter;
mod node;
mod synthesizer_core_impl;
mod types;
pub mod typescape;

#[cfg(feature = "iac-bridge")]
pub mod iac_bridge;

pub use emitter::emit_file;
pub use node::{BinOperator, Binding, FlakeInput, FnArg, ModuleOption, NixNode, StringPart};
pub use types::{NixType, SubmoduleOption};
