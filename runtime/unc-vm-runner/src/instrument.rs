//! Instrumentation of wasm code for gas metering and stack limiting.
//!
//! The code in this module was originally vendored from MIT/Apache wasm-utils
//! crate from the parity ecosystem:
//!
//! <https://github.com/utnet-org/wasm-utils/commit/2bf8068571869197a6974916be208017f2aafb62>
//!
//!
//! As every little detail of instrumentation matters for the semantics of our
//! protocol, we want to maintain the implementation ourselves.
//!
//! At the moment, the implementation is a direct copy, but we don't intend  to
//! keep the code aligned with the upstream, feel free to refactor if you find
//! something odd!.
pub(crate) mod gas;
pub(crate) mod rules;
pub(crate) mod stack_height;
