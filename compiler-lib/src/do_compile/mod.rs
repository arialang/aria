// SPDX-License-Identifier: Apache-2.0
use std::{collections::HashSet, fmt::Display, path::PathBuf, rc::Rc};

use aria_parser::ast::{
    ArgumentDecl, ArgumentList, AssertStatement, CodeBlock, DeclarationId, ElsePiece, EnumCaseDecl,
    EnumDecl, EnumDeclEntry, Expression, FunctionBody, Identifier, MatchPattern,
    MatchPatternEnumCase, MatchRule, MatchStatement, MethodAccess, MethodDecl, MixinIncludeDecl,
    OperatorDecl, ParsedModule, ReturnStatement, SourceBuffer, SourcePointer, Statement,
    StringLiteral, StructDecl, StructEntry, ValDeclStatement, prettyprint::PrettyPrintable,
    source_to_ast,
};
use haxby_opcodes::{builtin_type_ids::BUILTIN_TYPE_ANY, function_attribs::*};
use thiserror::Error;

use crate::{
    CompilationOptions,
    builder::{block::BasicBlock, compiler_opcodes::CompilerOpcode, func::FunctionBuilder},
    constant_value::{ConstantValue, ConstantValuesError},
    module::CompiledModule,
    scope::{CompilationScope, ScopeError, ScopeErrorReason},
};

#[derive(Debug, Error)]
pub enum CompilationErrorReason {
    #[error("identifier is reserved: '{0}'")]
    ReservedIdentifier(String),
    #[error("no such identifier: '{0}'")]
    NoSuchIdentifier(String),
    #[error("function body is larger than allowed")]
    FunctionBodyTooLarge,
    #[error("list length is out of bounds")]
    ListTooLarge,
    #[error("attempt to modify a read-only value")]
    ReadOnlyValue,
    #[error("{0} is not a valid literal")]
    InvalidLiteral(String),
    #[error("{0} is not a valid operator")]
    InvalidOperator(String),
    #[error("{0} cannot be reversed")]
    IrreversibleOperator(String),
    #[error("operator {0} accepts {1} arguments, but {2} were declared")]
    OperatorArityMismatch(String, OperatorArity, usize),
    #[error("attempt to read a write-only value")]
    WriteOnlyValue,
    #[error("parser error: {0}")]
    ParserError(String),
    #[error("module contains too many constant values")]
    TooManyConstants,
    #[error("function accepts too many arguments")]
    TooManyArguments,
    #[error("argument without a default value follows argument with default value")]
    DefaultArgsMustTrail,
    #[error("flow control statement not permitted in current context")]
    FlowControlNotAllowed,
    #[error("argument name '{0}' is already defined for this function")]
    DuplicateArgumentName(String),
    #[error("struct members do not support type hints")]
    NoTypeHintOnStructMember,
    #[error("nested closures are not supported")]
    NestedClosureDisallowed,
    #[error("attempted to write to {0} values, but {1} were provided")]
    AssignmentArityMismatch(usize, usize),
}

impl From<&ScopeErrorReason> for CompilationErrorReason {
    fn from(value: &ScopeErrorReason) -> Self {
        match value {
            ScopeErrorReason::TooManyConstants => CompilationErrorReason::TooManyConstants,
            ScopeErrorReason::NoSuchIdentifier(s) => {
                CompilationErrorReason::NoSuchIdentifier(s.clone())
            }
            ScopeErrorReason::OverlyDeepClosure => CompilationErrorReason::NestedClosureDisallowed,
        }
    }
}

impl From<ScopeError> for CompilationError {
    fn from(value: ScopeError) -> Self {
        CompilationError {
            loc: value.loc,
            reason: CompilationErrorReason::from(&value.reason),
        }
    }
}

impl From<&ConstantValuesError> for CompilationErrorReason {
    fn from(value: &ConstantValuesError) -> Self {
        match value {
            ConstantValuesError::OutOfSpace => CompilationErrorReason::TooManyConstants,
        }
    }
}

pub struct CompilationError {
    pub loc: SourcePointer,
    pub reason: CompilationErrorReason,
}

impl std::fmt::Display for CompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at {}, error occurred: {}", self.loc, self.reason)
    }
}

impl std::fmt::Debug for CompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at {}, error occurred: {}", self.loc, self.reason)
    }
}

pub type CompilationResult<T = (), E = CompilationError> = Result<T, E>;

#[derive(Default)]
struct ControlFlowTargets {
    break_dest: Option<Rc<BasicBlock>>,
    continue_dest: Option<Rc<BasicBlock>>,
}

struct CompileParams<'a> {
    module: &'a mut CompiledModule,
    scope: &'a CompilationScope,
    writer: &'a mut FunctionBuilder,
    cflow: &'a ControlFlowTargets,
    options: &'a CompilationOptions,
}

trait CompileNode<'a, T = (), E = CompilationError> {
    fn do_compile(&self, params: &'a mut CompileParams) -> CompilationResult<T, E>;

    fn insert_const_or_fail(
        &self,
        params: &mut CompileParams,
        ct: ConstantValue,
        loc: &SourcePointer,
    ) -> CompilationResult<u16> {
        match params.module.constants.insert(ct) {
            Ok(idx) => Ok(idx),
            Err(_) => Err(CompilationError {
                loc: loc.clone(),
                reason: CompilationErrorReason::TooManyConstants,
            }),
        }
    }

    fn return_unit_value(
        &self,
        params: &mut CompileParams,
        loc: &SourcePointer,
    ) -> CompilationResult {
        let unit_const_idx =
            self.insert_const_or_fail(params, ConstantValue::String("unit".to_owned()), loc)?;

        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(
                CompilerOpcode::PushBuiltinTy(haxby_opcodes::builtin_type_ids::BUILTIN_TYPE_UNIT),
                loc.clone(),
            )
            .write_opcode_and_source_info(
                CompilerOpcode::NewEnumVal(false, unit_const_idx),
                loc.clone(),
            )
            .write_opcode_and_source_info(CompilerOpcode::Return, loc.clone());

        Ok(())
    }
}

mod nodes;
mod postfix;

fn ensure_arg_list_is_correct(args: &ArgumentList) -> CompilationResult {
    ensure_unique_arg_names(args)?;
    ensure_default_args_trailing(args)?;
    Ok(())
}

fn ensure_unique_arg_names(args: &ArgumentList) -> CompilationResult {
    let mut arg_set = HashSet::new();
    for arg in &args.names {
        if arg_set.contains(arg.name()) {
            return Err(CompilationError {
                loc: arg.loc.clone(),
                reason: CompilationErrorReason::DuplicateArgumentName(arg.name().to_owned()),
            });
        } else {
            arg_set.insert(arg.name().to_owned());
        }
    }

    Ok(())
}

fn ensure_default_args_trailing(args: &ArgumentList) -> CompilationResult {
    let mut found_default = false;
    for arg in &args.names {
        if arg.deft.is_some() {
            found_default = true;
        } else if found_default {
            return Err(CompilationError {
                loc: arg.loc.clone(),
                reason: CompilationErrorReason::DefaultArgsMustTrail,
            });
        }
    }

    Ok(())
}

fn emit_arg_at_target(
    arg: &ArgumentDecl,
    idx: u8,
    params: &mut CompileParams,
) -> CompilationResult {
    if let Some(deft_expr) = arg.deft.as_ref() {
        let block = params
            .writer
            .append_block_at_end(&format!("supplied_arg_{idx}"));
        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(
                CompilerOpcode::JumpIfArgSupplied(idx, block.clone()),
                arg.loc.clone(),
            );
        deft_expr.do_compile(params)?;
        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(CompilerOpcode::Jump(block.clone()), arg.loc.clone());
        params.writer.set_current_block(block);
    }
    if let Some(ty) = arg.type_info() {
        ty.do_compile(params)?;
    } else {
        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(
                CompilerOpcode::PushBuiltinTy(BUILTIN_TYPE_ANY),
                arg.loc.clone(),
            );
    }
    params.scope.emit_typed_define(
        arg.name(),
        &mut params.module.constants,
        params.writer.get_current_block(),
        arg.loc.clone(),
    )?;
    params.scope.emit_write(
        arg.name(),
        &mut params.module.constants,
        params.writer.get_current_block(),
        arg.loc.clone(),
    )?;

    Ok(())
}

#[allow(dead_code)]
struct ArgumentCountInfo {
    user_args: u8,
    required_args: u8,
    default_args: u8,
    varargs: bool,
}

fn emit_args_at_target(
    prefix_args: &[ArgumentDecl],
    args: &ArgumentList,
    suffix_args: &[ArgumentDecl],
    params: &mut CompileParams,
) -> CompilationResult<ArgumentCountInfo> {
    ensure_arg_list_is_correct(args)?;

    let total_args = prefix_args.len() + args.names.len() + suffix_args.len();
    if total_args > u8::MAX.into() {
        return Err(CompilationError {
            loc: args.loc.clone(),
            reason: CompilationErrorReason::TooManyArguments,
        });
    }

    let mut argc_info = ArgumentCountInfo {
        user_args: args.len() as u8,
        required_args: 0,
        default_args: 0,
        varargs: args.vararg,
    };

    let mut arg_idx: u8 = 0;

    for arg in prefix_args {
        emit_arg_at_target(arg, arg_idx, params)?;
        argc_info.required_args += 1;
        arg_idx += 1;
    }

    for arg in &args.names {
        emit_arg_at_target(arg, arg_idx, params)?;
        if arg.deft.is_some() {
            argc_info.default_args += 1;
        } else {
            argc_info.required_args += 1;
        }
        arg_idx += 1;
    }

    for arg in suffix_args {
        emit_arg_at_target(arg, arg_idx, params)?;
        argc_info.required_args += 1;
        arg_idx += 1;
    }

    if args.vararg {
        params.scope.emit_untyped_define(
            "varargs",
            &mut params.module.constants,
            params.writer.get_current_block(),
            args.loc.clone(),
        )?;
    }
    Ok(argc_info)
}

// assume your parent struct is on the stack
fn emit_type_mixin_include_decl_compile(
    mi: &MixinIncludeDecl,
    params: &mut CompileParams,
) -> CompilationResult {
    mi.what.do_compile(params)?;
    params
        .writer
        .get_current_block()
        .write_opcode_and_source_info(CompilerOpcode::IncludeMixin, mi.loc.clone());
    Ok(())
}

// assume your parent struct is on the stack
fn emit_method_decl_compile(md: &MethodDecl, params: &mut CompileParams) -> CompilationResult {
    md.do_compile(params)?;

    let name_idx = md.insert_const_or_fail(
        params,
        ConstantValue::String(md.name.value.clone()),
        &md.loc,
    )?;
    params
        .writer
        .get_current_block()
        .write_opcode_and_source_info(
            CompilerOpcode::BindMethod(
                if md.args.vararg {
                    FUNC_ACCEPTS_VARARG
                } else {
                    0
                } | FUNC_IS_METHOD
                    | if md.access == MethodAccess::Type {
                        METHOD_ATTRIBUTE_TYPE
                    } else {
                        0
                    },
                name_idx,
            ),
            md.loc.clone(),
        );

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorArity {
    Exactly(usize),
    AtLeast(usize),
}

impl OperatorArity {
    fn unary() -> Self {
        OperatorArity::Exactly(1)
    }

    fn any() -> Self {
        OperatorArity::AtLeast(0)
    }

    fn is_acceptable(&self, arg_count: usize) -> bool {
        match self {
            OperatorArity::Exactly(n) => *n == arg_count,
            OperatorArity::AtLeast(n) => arg_count >= *n,
        }
    }
}

impl Display for OperatorArity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperatorArity::Exactly(n) => write!(f, "exactly {n}"),
            OperatorArity::AtLeast(n) => write!(f, "at least {n}"),
        }
    }
}

struct OperatorInfo {
    arity: OperatorArity, // number of arguments (minus the receiver this)
    direct_name: &'static str,
    reverse_name: &'static str,
}

lazy_static::lazy_static! {
    static ref OPERATOR_INFO: std::collections::HashMap<&'static str, OperatorInfo> = {
        let mut map = std::collections::HashMap::new();

        map.insert(
            "+",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "add",
                reverse_name: "radd",
            },
        );

        map.insert(
            "-",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "sub",
                reverse_name: "rsub",
            },
        );

        map.insert(
            "*",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "mul",
                reverse_name: "rmul",
            },
        );

        map.insert(
            "/",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "div",
                reverse_name: "rdiv",
            },
        );

        map.insert(
            "%",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "rem",
                reverse_name: "rrem",
            },
        );

        map.insert(
            "<<",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "lshift",
                reverse_name: "rlshift",
            },
        );

        map.insert(
            ">>",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "rshift",
                reverse_name: "rrshift",
            },
        );

        map.insert("==",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "equals",
                reverse_name: "",
            },
        );

        map.insert("<",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "lt",
                reverse_name: "gt",
            },
        );

        map.insert(">",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "gt",
                reverse_name: "lt",
            },
        );

        map.insert("<=",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "lteq",
                reverse_name: "gteq",
            },
        );

        map.insert(">=",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "gteq",
                reverse_name: "lteq",
            },
        );

        map.insert("&",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "bwand",
                reverse_name: "rbwand",
            },
        );

        map.insert("|",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "bwor",
                reverse_name: "rbwor",
            },
        );

        map.insert("^",
            OperatorInfo {
                arity: OperatorArity::unary(),
                direct_name: "xor",
                reverse_name: "rxor",
            },
        );

        map.insert("u-",
            OperatorInfo {
                arity: OperatorArity::Exactly(0),
                direct_name: "neg",
                reverse_name: "",
            },
        );

        map.insert("()",
            OperatorInfo {
                arity: OperatorArity::any(),
                direct_name: "call",
                reverse_name: "",
            },
        );

        map.insert("[]",
            OperatorInfo {
                arity: OperatorArity::any(),
                direct_name: "read_index",
                reverse_name: "",
            },
        );

        map.insert("[]=",
            OperatorInfo {
                arity: OperatorArity::AtLeast(1),
                direct_name: "write_index",
                reverse_name: "",
            },
        );

        map
    };
}

// assume your parent struct is on the stack
fn emit_operator_decl_compile(op: &OperatorDecl, params: &mut CompileParams) -> CompilationResult {
    let op_symbol = op
        .symbol
        .prettyprint(
            aria_parser::ast::prettyprint::printout_accumulator::PrintoutAccumulator::default(),
        )
        .value();

    let op_info = match OPERATOR_INFO.get(op_symbol.as_str()) {
        Some(info) => info,
        None => {
            return Err(CompilationError {
                loc: op.loc.clone(),
                reason: CompilationErrorReason::InvalidOperator(op_symbol),
            });
        }
    };

    if !op_info.arity.is_acceptable(op.args.len()) {
        return Err(CompilationError {
            loc: op.loc.clone(),
            reason: CompilationErrorReason::OperatorArityMismatch(
                op_symbol,
                op_info.arity,
                op.args.len(),
            ),
        });
    }

    if op.reverse && op_info.reverse_name.is_empty() {
        return Err(CompilationError {
            loc: op.loc.clone(),
            reason: CompilationErrorReason::IrreversibleOperator(op_symbol),
        });
    }

    let op_fn_name = format!(
        "_op_impl_{}",
        if op.reverse {
            op_info.reverse_name
        } else {
            op_info.direct_name
        }
    );

    let md = MethodDecl {
        loc: op.loc.clone(),
        access: MethodAccess::Instance,
        name: Identifier {
            loc: op.loc.clone(),
            value: op_fn_name,
        },
        args: op.args.clone(),
        body: op.body.clone(),
    };

    emit_method_decl_compile(&md, params)
}

// assume your parent struct is on the stack
fn emit_type_val_decl_compile(
    vd: &ValDeclStatement,
    params: &mut CompileParams,
) -> CompilationResult {
    for decl in &vd.decls {
        if decl.id.ty.is_some() {
            return Err(CompilationError {
                loc: vd.loc.clone(),
                reason: CompilationErrorReason::NoTypeHintOnStructMember,
            });
        }
        decl.val.do_compile(params)?;
        let name_idx = vd.insert_const_or_fail(
            params,
            ConstantValue::String(decl.id.name.value.clone()),
            &vd.loc,
        )?;
        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(CompilerOpcode::WriteAttribute(name_idx), vd.loc.clone());
    }
    Ok(())
}

// assume your parent struct is on the stack
fn emit_type_members_compile(
    entries: &[StructEntry],
    params: &mut CompileParams,
    drop_at_end: bool,
) -> CompilationResult {
    for se in entries {
        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(CompilerOpcode::Dup, se.loc().clone());

        match se {
            aria_parser::ast::StructEntry::Method(md) => emit_method_decl_compile(md, params)?,
            aria_parser::ast::StructEntry::Operator(od) => emit_operator_decl_compile(od, params)?,
            aria_parser::ast::StructEntry::Variable(vd) => emit_type_val_decl_compile(vd, params)?,
            aria_parser::ast::StructEntry::Struct(sd) => {
                do_struct_compile(sd, params)?;

                let name_idx = sd.insert_const_or_fail(
                    params,
                    ConstantValue::String(sd.name.value.clone()),
                    &sd.loc,
                )?;
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::WriteAttribute(name_idx),
                        sd.loc.clone(),
                    );
            }
            aria_parser::ast::StructEntry::Enum(ed) => {
                do_enum_compile(ed, params, |name, params| {
                    params
                        .writer
                        .get_current_block()
                        .write_opcode_and_source_info(CompilerOpcode::Swap, ed.loc.clone());
                    params
                        .writer
                        .get_current_block()
                        .write_opcode_and_source_info(CompilerOpcode::Copy(1), ed.loc.clone());

                    let name_idx = ed.insert_const_or_fail(
                        params,
                        ConstantValue::String(name.to_owned()),
                        &ed.loc,
                    )?;

                    params
                        .writer
                        .get_current_block()
                        .write_opcode_and_source_info(
                            CompilerOpcode::WriteAttribute(name_idx),
                            ed.loc.clone(),
                        );
                    Ok(())
                })?;
            }
            aria_parser::ast::StructEntry::MixinInclude(mi) => {
                emit_type_mixin_include_decl_compile(mi, params)?
            }
        }
    }

    if drop_at_end {
        // remove the last leftover struct
        #[allow(deprecated)] // no entry to ascribe this write to
        params
            .writer
            .get_current_block()
            .write_opcode(CompilerOpcode::Pop);
    }

    Ok(())
}

fn do_struct_compile(sd: &StructDecl, params: &mut CompileParams) -> CompilationResult {
    let self_name = StringLiteral {
        loc: sd.loc.clone(),
        value: sd.name.value.clone(),
    };
    self_name.do_compile(params)?;
    params
        .writer
        .get_current_block()
        .write_opcode_and_source_info(CompilerOpcode::BuildStruct, sd.loc.clone());

    emit_type_members_compile(&sd.body, params, false)
}

fn do_enum_compile<T>(
    ed: &EnumDecl,
    params: &mut CompileParams,
    name_writer: T,
) -> CompilationResult
where
    T: FnOnce(&str, &mut CompileParams) -> CompilationResult,
{
    let self_name = StringLiteral {
        loc: ed.loc.clone(),
        value: ed.name.value.clone(),
    };
    self_name.do_compile(params)?;

    params
        .writer
        .get_current_block()
        .write_opcode_and_source_info(CompilerOpcode::BuildEnum, ed.loc.clone());
    name_writer(&ed.name.value, params)?;

    let mut cases: Vec<EnumCaseDecl> = vec![];
    let mut entries: Vec<StructEntry> = vec![];

    for ede in &ed.body {
        match ede {
            EnumDeclEntry::EnumCaseDecl(case) => cases.push(case.clone()),
            EnumDeclEntry::StructEntry(entry) => entries.push(entry.clone()),
        }
    }

    let enum_helper_methods = generate_case_helpers_extension_for_enum(&cases);
    emit_type_members_compile(&enum_helper_methods, params, false)?;

    emit_type_members_compile(&entries, params, false)?;

    emit_enum_cases(&cases, params)?;

    params
        .writer
        .get_current_block()
        .write_opcode_and_source_info(CompilerOpcode::Pop, ed.loc.clone());

    Ok(())
}

fn generate_is_case_helper_for_enum(case: &EnumCaseDecl) -> MethodDecl {
    let return_true_stmt = ReturnStatement::from(&Expression::from(&Identifier {
        loc: case.loc.clone(),
        value: "true".to_owned(),
    }));

    let return_false_stmt = ReturnStatement::from(&Expression::from(&Identifier {
        loc: case.loc.clone(),
        value: "false".to_owned(),
    }));

    let match_case_pattern = MatchPatternEnumCase {
        loc: case.loc.clone(),
        case: case.name.clone(),
        payload: None,
    };

    let match_rule = MatchRule {
        loc: case.loc.clone(),
        patterns: vec![MatchPattern::MatchPatternEnumCase(match_case_pattern)],
        then: CodeBlock::from(&Statement::ReturnStatement(return_true_stmt)),
    };

    let match_statement = MatchStatement {
        loc: case.loc.clone(),
        expr: Expression::from(&Identifier {
            loc: case.loc.clone(),
            value: "this".to_owned(),
        }),
        rules: vec![match_rule],
        els: Some(ElsePiece {
            loc: case.loc.clone(),
            then: CodeBlock::from(&Statement::ReturnStatement(return_false_stmt)),
        }),
    };

    let method_body = FunctionBody {
        code: CodeBlock::from(&Statement::MatchStatement(match_statement)),
    };
    MethodDecl {
        loc: case.loc.clone(),
        access: MethodAccess::Instance,
        name: Identifier {
            loc: case.loc.clone(),
            value: format!("is_{}", case.name.value),
        },
        args: ArgumentList::empty(case.loc.clone()),
        body: method_body,
    }
}

fn generate_unwap_case_helper_for_enum(case: &EnumCaseDecl) -> MethodDecl {
    assert!(case.payload.is_some());

    let return_true_stmt = ReturnStatement::from(&Expression::from(&Identifier {
        loc: case.loc.clone(),
        value: "__case_payload".to_owned(),
    }));

    let assert_false_stmt = AssertStatement {
        loc: case.loc.clone(),
        val: Expression::from(&Identifier {
            loc: case.loc.clone(),
            value: "false".to_owned(),
        }),
    };

    let match_case_pattern = MatchPatternEnumCase {
        loc: case.loc.clone(),
        case: case.name.clone(),
        payload: Some(DeclarationId {
            loc: case.loc.clone(),
            name: Identifier {
                loc: case.loc.clone(),
                value: "__case_payload".to_owned(),
            },
            ty: None,
        }),
    };

    let match_rule = MatchRule {
        loc: case.loc.clone(),
        patterns: vec![MatchPattern::MatchPatternEnumCase(match_case_pattern)],
        then: CodeBlock::from(&Statement::ReturnStatement(return_true_stmt)),
    };

    let match_statement = MatchStatement {
        loc: case.loc.clone(),
        expr: Expression::from(&Identifier {
            loc: case.loc.clone(),
            value: "this".to_owned(),
        }),
        rules: vec![match_rule],
        els: Some(ElsePiece {
            loc: case.loc.clone(),
            then: CodeBlock::from(&Statement::AssertStatement(assert_false_stmt)),
        }),
    };

    let method_body = FunctionBody {
        code: CodeBlock::from(&Statement::MatchStatement(match_statement)),
    };
    MethodDecl {
        loc: case.loc.clone(),
        access: MethodAccess::Instance,
        name: Identifier {
            loc: case.loc.clone(),
            value: format!("unwrap_{}", case.name.value),
        },
        args: ArgumentList::empty(case.loc.clone()),
        body: method_body,
    }
}

fn generate_case_helpers_extension_for_enum(cases: &[EnumCaseDecl]) -> Vec<StructEntry> {
    let mut entries: Vec<StructEntry> = vec![];

    for case in cases {
        entries.push(StructEntry::Method(Box::new(
            generate_is_case_helper_for_enum(case),
        )));
        if case.payload.is_some() {
            entries.push(StructEntry::Method(Box::new(
                generate_unwap_case_helper_for_enum(case),
            )));
        }
    }

    entries
}

// assume your parent enum is on the stack
fn emit_enum_cases(cases: &[EnumCaseDecl], params: &mut CompileParams) -> CompilationResult {
    for case in cases {
        case.do_compile(params)?;
    }

    Ok(())
}

pub(crate) fn compile_from_ast(
    ast: &ParsedModule,
    options: &CompilationOptions,
) -> CompilationResult<CompiledModule, Vec<CompilationError>> {
    let mut dest = CompiledModule::default();
    let scope = CompilationScope::module();
    let mut mod_init_bytecode = FunctionBuilder::default();
    let cflow = ControlFlowTargets::default();

    let mut c_params = CompileParams {
        module: &mut dest,
        scope: &scope,
        writer: &mut mod_init_bytecode,
        cflow: &cflow,
        options,
    };

    ast.do_compile(&mut c_params)?;

    Ok(dest)
}

pub(crate) fn compile_from_source(
    src: &SourceBuffer,
    options: &CompilationOptions,
) -> CompilationResult<CompiledModule, Vec<CompilationError>> {
    let ast = match source_to_ast(src) {
        Ok(ast) => ast,
        Err(err) => {
            return Err(vec![CompilationError {
                loc: err.loc,
                reason: CompilationErrorReason::ParserError(err.msg),
            }]);
        }
    };

    compile_from_ast(&ast, options).map(|mut module| {
        // The source buffer name is the cannonalized path to the source file if it was created from a file.
        // Once we check it is an existing file, we can use it to find the widget path.
        let src_path = PathBuf::from(&src.name);

        module.widget_root_path = if src_path.exists() {
            // Ensure the source buffer name is a file path
            debug_assert!(src_path.is_file());

            let mut ancestors = src_path.ancestors();
            loop {
                if let Some(widget_path) = ancestors.next() {
                    let widget_json = widget_path.join("widget.json");
                    if !widget_json.exists() {
                        continue;
                    }

                    break Some(widget_path.to_path_buf());
                }

                break None;
            }
        } else {
            None
        };

        module
    })
}
