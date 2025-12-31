// SPDX-License-Identifier: Apache-2.0

use aria_compiler::bc_reader::BytecodeReader;

use crate::{
    frame::Frame,
    runtime_module::RuntimeModule,
    vm::{ExecutionResult, RunloopExit},
};

#[allow(unused)]
pub struct RunloopFrame {
    pub(crate) reader: BytecodeReader,
    pub(crate) module: RuntimeModule,
    pub(crate) frame: Frame,
}

pub enum CallInvocationScheme {
    Runloop(RunloopFrame),
    RustNative(ExecutionResult<RunloopExit<Frame>>),
}
