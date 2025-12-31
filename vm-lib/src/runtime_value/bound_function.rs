// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use crate::{
    frame::Frame,
    runloop::CallInvocationScheme,
    runtime_value::function::PartialFunctionApplication,
    vm::{ExecutionResult, VirtualMachine},
};

use super::{CallResult, RuntimeValue, function::Function};

struct BoundFunctionImpl {
    this: RuntimeValue,
    func: Function,
}

#[derive(Clone)]
pub struct BoundFunction {
    imp: Rc<BoundFunctionImpl>,
}

impl BoundFunction {
    pub(super) fn bind(this: RuntimeValue, func: Function) -> Self {
        Self {
            imp: Rc::new(BoundFunctionImpl { this, func }),
        }
    }

    pub fn this(&self) -> &RuntimeValue {
        &self.imp.this
    }

    pub fn func(&self) -> &Function {
        &self.imp.func
    }

    #[inline(always)]
    pub fn prepare_invocation(
        &self,
        argc: u8,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
    ) -> ExecutionResult<CallInvocationScheme> {
        let partial_application =
            PartialFunctionApplication::default().with_suffix_arg(self.this().clone());
        self.func()
            .prepare_invocation(argc, cur_frame, vm, &partial_application)
    }

    pub fn eval(
        &self,
        argc: u8,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
        discard_result: bool,
    ) -> ExecutionResult<CallResult> {
        let partial_application =
            PartialFunctionApplication::default().with_suffix_arg(self.this().clone());
        self.func()
            .eval(argc, cur_frame, vm, &partial_application, discard_result)
    }
}

impl PartialEq for BoundFunction {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.imp, &other.imp)
    }
}
impl Eq for BoundFunction {}
