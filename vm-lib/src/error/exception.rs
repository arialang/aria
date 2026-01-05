// SPDX-License-Identifier: Apache-2.0

use aria_parser::ast::SourcePointer;
use haxby_opcodes::BuiltinTypeId;

use crate::{
    builtins::VmGlobals,
    error::{
        backtrace::Backtrace,
        vm_error::{VmError, VmErrorReason},
    },
    runtime_value::{RuntimeValue, list::List, object::Object},
    symbol::INTERNED_ATTR_BACKTRACE,
    vm::VirtualMachine,
};

pub struct VmException {
    pub value: RuntimeValue,
    pub backtrace: Backtrace,
}

impl VmException {
    pub fn from_value(value: RuntimeValue) -> Self {
        Self {
            value,
            backtrace: Default::default(),
        }
    }

    pub fn from_value_and_loc(value: RuntimeValue, loc: Option<SourcePointer>) -> Self {
        let mut this = VmException::from_value(value);
        if let Some(loc) = loc {
            this = this.thrown_at(loc);
        }

        this
    }

    pub fn thrown_at(self, loc: SourcePointer) -> Self {
        if self.backtrace.len() == 1 && self.backtrace.first_entry().unwrap() == loc {
            self
        } else {
            let mut new_bt = self.backtrace.clone();
            new_bt.push(loc);
            Self {
                value: self.value.clone(),
                backtrace: new_bt,
            }
        }
    }

    pub fn is_builtin_unimplemented(&self, vm: &mut VirtualMachine) -> bool {
        self.value.is_builtin_unimplemented(vm)
    }
}

impl VmException {
    pub(crate) fn fill_in_backtrace(&self, builtins: &VmGlobals) {
        let bt_list = List::from(&[]);
        for bt_entry in self.backtrace.entries_iter() {
            let buf_name = bt_entry.buffer.name.clone();
            let buf_line = bt_entry
                .buffer
                .line_index_for_position(bt_entry.location.start);
            let buf_name = RuntimeValue::String(buf_name.into());
            let buf_line = RuntimeValue::Integer((buf_line as i64).into());
            bt_list.append(RuntimeValue::List(List::from(&[buf_name, buf_line])));
        }
        let _ = self.value.write_attribute(
            INTERNED_ATTR_BACKTRACE,
            RuntimeValue::List(bt_list),
            builtins,
        );
    }
}

impl VmException {
    pub fn from_vmerror(err: VmError, builtins: &VmGlobals) -> Result<VmException, VmError> {
        macro_rules! some_or_err {
            ($opt:expr, $err:expr) => {
                match $opt {
                    Some(val) => val,
                    None => return Err($err),
                }
            };
        }

        let rt_err_type = builtins.get_builtin_type_by_id(BuiltinTypeId::RuntimeError);

        let rt_err = some_or_err!(rt_err_type.as_enum(), err);

        struct ExceptionData {
            case: usize,
            payload: Option<RuntimeValue>,
        }

        let e_data = match &err.reason {
            VmErrorReason::DivisionByZero => ExceptionData {
                case: some_or_err!(rt_err.get_idx_of_case("DivisionByZero"), err),
                payload: None,
            },
            VmErrorReason::EnumWithoutPayload => ExceptionData {
                case: some_or_err!(rt_err.get_idx_of_case("EnumWithoutPayload"), err),
                payload: None,
            },
            VmErrorReason::IndexOutOfBounds(idx) => ExceptionData {
                case: some_or_err!(rt_err.get_idx_of_case("IndexOutOfBounds"), err),
                payload: Some(RuntimeValue::Integer((*idx as i64).into())),
            },
            VmErrorReason::MismatchedArgumentCount(expected, actual) => {
                let argc_mismatch_sym = builtins
                    .intern_symbol("ArgcMismatch")
                    .map_err(|_| err.clone())?;
                let argc_mismatch = some_or_err!(rt_err.load_named_value(argc_mismatch_sym), err);
                let argc_mismatch = some_or_err!(argc_mismatch.as_struct(), err);
                let argc_mismatch_obj = Object::new(argc_mismatch);
                let expected_sym = builtins
                    .intern_symbol("expected")
                    .map_err(|_| err.clone())?;
                let actual_sym = builtins.intern_symbol("actual").map_err(|_| err.clone())?;
                argc_mismatch_obj.write(
                    expected_sym,
                    RuntimeValue::Integer((*expected as i64).into()),
                );
                argc_mismatch_obj.write(actual_sym, RuntimeValue::Integer((*actual as i64).into()));
                ExceptionData {
                    case: some_or_err!(rt_err.get_idx_of_case("MismatchedArgumentCount"), err),
                    payload: Some(RuntimeValue::Object(argc_mismatch_obj)),
                }
            }
            VmErrorReason::NoSuchCase(s) => ExceptionData {
                case: some_or_err!(rt_err.get_idx_of_case("NoSuchCase"), err),
                payload: Some(RuntimeValue::String(s.clone().into())),
            },
            VmErrorReason::NoSuchIdentifier(s) => ExceptionData {
                case: some_or_err!(rt_err.get_idx_of_case("NoSuchIdentifier"), err),
                payload: Some(RuntimeValue::String(s.clone().into())),
            },
            VmErrorReason::NoSuchSymbol(sym) => {
                if let Some(sym_str) = builtins.resolve_symbol(*sym) {
                    ExceptionData {
                        case: some_or_err!(rt_err.get_idx_of_case("NoSuchIdentifier"), err),
                        payload: Some(RuntimeValue::String(sym_str.into())),
                    }
                } else {
                    return Err(err);
                }
            }
            VmErrorReason::OperationFailed(s) => ExceptionData {
                case: some_or_err!(rt_err.get_idx_of_case("OperationFailed"), err),
                payload: Some(RuntimeValue::String(s.clone().into())),
            },
            VmErrorReason::UnexpectedType => ExceptionData {
                case: some_or_err!(rt_err.get_idx_of_case("UnexpectedType"), err),
                payload: None,
            },
            _ => {
                return Err(err);
            }
        };

        let exception_value = RuntimeValue::EnumValue(some_or_err!(
            rt_err.make_value(e_data.case, e_data.payload),
            err
        ));
        Ok(VmException::from_value_and_loc(exception_value, err.loc))
    }
}
