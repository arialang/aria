// SPDX-License-Identifier: Apache-2.0
use aria_compiler::module::CompiledModule;
use vm::{ExecutionResult, RunloopExit, VirtualMachine, VmOptions};

pub mod arity;
pub mod builtins;
pub mod console;
pub mod error;
pub mod frame;
pub mod mixin_includer;
pub mod opcodes;
pub mod runloop;
pub mod runtime_module;
pub mod runtime_value;
pub mod stack;
pub mod vm;

#[cfg(test)]
mod test;

pub struct HaxbyEvalResult {
    pub exit: RunloopExit,
    pub vm: VirtualMachine,
}

pub fn haxby_eval(module: CompiledModule, options: VmOptions) -> ExecutionResult<HaxbyEvalResult> {
    let mut vm = VirtualMachine::with_options(options);

    let rle = vm.load_module("eval", module)?;
    let rm = match rle {
        RunloopExit::Ok(m) => m.module,
        RunloopExit::Exception(e) => {
            return Ok(HaxbyEvalResult {
                exit: RunloopExit::Exception(e),
                vm,
            });
        }
    };
    let rle = vm.execute_module(&rm)?;

    Ok(HaxbyEvalResult { exit: rle, vm })
}
