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
    symbol::{INTERNED_ATTR_ACTUAL, INTERNED_ATTR_BACKTRACE, INTERNED_ATTR_EXPECTED, Symbol},
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
    pub(crate) fn fill_in_backtrace(&self, builtins: &mut VmGlobals) {
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
    pub fn from_vmerror(err: VmError, builtins: &mut VmGlobals) -> Result<VmException, VmError> {
        macro_rules! some_or_err {
            ($opt:expr, $err:expr) => {
                match $opt {
                    Some(val) => val,
                    None => return Err($err),
                }
            };
        }

        use crate::builtins::runtime_error::{
            RUNTIME_ERR_CASE_DIVISION_BY_ZERO_IDX, RUNTIME_ERR_CASE_ENUM_WITHOUT_PAYLOAD_IDX,
            RUNTIME_ERR_CASE_INDEX_OUT_OF_BOUNDS_IDX, RUNTIME_ERR_CASE_MISMATCHED_ARGC_IDX,
            RUNTIME_ERR_CASE_NO_SUCH_CASE_IDX, RUNTIME_ERR_CASE_NO_SUCH_IDENTIFIER_IDX,
            RUNTIME_ERR_CASE_OPERATION_FAILED_IDX, RUNTIME_ERR_CASE_UNEXPECTED_TYPE_IDX,
        };

        let rt_err_type = builtins.get_builtin_type_by_id(BuiltinTypeId::RuntimeError);

        let rt_err = some_or_err!(rt_err_type.as_enum(), err);

        struct ExceptionData {
            case: usize,
            payload: Option<RuntimeValue>,
        }

        let e_data = match &err.reason {
            VmErrorReason::DivisionByZero => ExceptionData {
                case: RUNTIME_ERR_CASE_DIVISION_BY_ZERO_IDX,
                payload: None,
            },
            VmErrorReason::EnumWithoutPayload => ExceptionData {
                case: RUNTIME_ERR_CASE_ENUM_WITHOUT_PAYLOAD_IDX,
                payload: None,
            },
            VmErrorReason::IndexOutOfBounds(idx) => ExceptionData {
                case: RUNTIME_ERR_CASE_INDEX_OUT_OF_BOUNDS_IDX,
                payload: Some(RuntimeValue::Integer((*idx as i64).into())),
            },
            VmErrorReason::MismatchedArgumentCount(expected, actual) => {
                let argc_mismatch_sym = builtins
                    .intern_symbol("ArgcMismatch")
                    .expect("too many symbols interned");
                let argc_mismatch =
                    some_or_err!(rt_err.load_named_value(builtins, argc_mismatch_sym), err);
                let argc_mismatch = some_or_err!(argc_mismatch.as_struct(), err);
                let argc_mismatch_obj = RuntimeValue::Object(Object::new(argc_mismatch));
                let _ = argc_mismatch_obj.write_attribute(
                    INTERNED_ATTR_EXPECTED,
                    RuntimeValue::Integer((*expected as i64).into()),
                    builtins,
                );
                let _ = argc_mismatch_obj.write_attribute(
                    INTERNED_ATTR_ACTUAL,
                    RuntimeValue::Integer((*actual as i64).into()),
                    builtins,
                );
                ExceptionData {
                    case: RUNTIME_ERR_CASE_MISMATCHED_ARGC_IDX,
                    payload: Some(argc_mismatch_obj),
                }
            }
            VmErrorReason::NoSuchCase(s) => ExceptionData {
                case: RUNTIME_ERR_CASE_NO_SUCH_CASE_IDX,
                payload: Some(RuntimeValue::String(s.clone().into())),
            },
            VmErrorReason::NoSuchIdentifier(s) => ExceptionData {
                case: RUNTIME_ERR_CASE_NO_SUCH_IDENTIFIER_IDX,
                payload: Some(RuntimeValue::String(s.clone().into())),
            },
            VmErrorReason::NoSuchSymbol(n, kind) => {
                if let Some(name_for_sym) = builtins.resolve_symbol(Symbol(*n)) {
                    ExceptionData {
                        case: match kind {
                            crate::error::vm_error::SymbolKind::Identifier => {
                                RUNTIME_ERR_CASE_NO_SUCH_IDENTIFIER_IDX
                            }
                            crate::error::vm_error::SymbolKind::Case => {
                                RUNTIME_ERR_CASE_NO_SUCH_CASE_IDX
                            }
                        },
                        payload: Some(RuntimeValue::String(name_for_sym.to_owned().into())),
                    }
                } else {
                    return Err(err);
                }
            }
            VmErrorReason::OperationFailed(s) => ExceptionData {
                case: RUNTIME_ERR_CASE_OPERATION_FAILED_IDX,
                payload: Some(RuntimeValue::String(s.clone().into())),
            },
            VmErrorReason::UnexpectedType => ExceptionData {
                case: RUNTIME_ERR_CASE_UNEXPECTED_TYPE_IDX,
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
