// SPDX-License-Identifier: Apache-2.0
use aria_parser::ast::{Expression, ExpressionList, Identifier, SourcePointer};
use haxby_opcodes::BuiltinTypeId;

use crate::{builder::compiler_opcodes::CompilerOpcode, constant_value::ConstantValue};

use super::{
    CompilationError, CompilationErrorReason, CompilationResult, CompileNode, CompileParams,
};

#[derive(Debug)]
pub(super) struct FieldWrite {
    pub(super) field: Identifier,
    pub(super) value: Expression,
}

#[derive(Debug)]
pub(super) struct IndexWrite {
    pub(super) index: ExpressionList,
    pub(super) value: Expression,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub(super) enum ObjWrite {
    Field(FieldWrite),
    Index(IndexWrite),
}

impl ObjWrite {
    fn loc(&self) -> &SourcePointer {
        match self {
            ObjWrite::Field(f) => &f.field.loc,
            ObjWrite::Index(i) => &i.index.loc,
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub(super) enum PostfixValue {
    Primary(Box<aria_parser::ast::Primary>),
    Attribute(Box<PostfixValue>, Box<Identifier>),
    Call(Box<PostfixValue>, Box<ExpressionList>, SourcePointer),
    Case(Box<PostfixValue>, Box<Identifier>, Option<Expression>),
    Index(Box<PostfixValue>, Box<aria_parser::ast::ExpressionList>),
    ObjWrite(Box<PostfixValue>, Vec<ObjWrite>),
    TryProtocol(
        Box<PostfixValue>,
        Box<aria_parser::ast::PostfixTermTryProtocol>,
    ),
}

impl<'a> PostfixValue {
    pub(super) fn emit_read(&self, params: &'a mut CompileParams) -> CompilationResult {
        match self {
            PostfixValue::Primary(primary) => primary.do_compile(params),
            PostfixValue::Call(base, args, loc) => {
                for expr in args.expressions.iter().rev() {
                    expr.do_compile(params)?;
                }
                let argc = args.expressions.len();
                base.emit_read(params)?;
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(CompilerOpcode::Call(argc as u8), loc.clone());
                Ok(())
            }
            PostfixValue::Case(base, case, payload) => {
                if let Some(p) = payload {
                    p.do_compile(params)?;
                }
                base.emit_read(params)?;
                let identifier_idx = match params
                    .module
                    .constants
                    .insert(ConstantValue::String(case.value.clone()))
                {
                    Ok(c) => c,
                    Err(_) => {
                        return Err(CompilationError {
                            loc: case.loc.clone(),
                            reason: CompilationErrorReason::TooManyConstants,
                        });
                    }
                };
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::NewEnumVal(payload.is_some(), identifier_idx),
                        case.loc.clone(),
                    );
                Ok(())
            }
            PostfixValue::Index(base, index) => {
                base.emit_read(params)?;
                index.do_compile(params)?;
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::ReadIndex(index.expressions.len() as u8),
                        index.loc.clone(),
                    );
                Ok(())
            }
            PostfixValue::Attribute(base, identifier) => {
                let identifier_idx = match params
                    .module
                    .constants
                    .insert(ConstantValue::String(identifier.value.clone()))
                {
                    Ok(c) => c,
                    Err(_) => {
                        return Err(CompilationError {
                            loc: identifier.loc.clone(),
                            reason: CompilationErrorReason::TooManyConstants,
                        });
                    }
                };
                base.emit_read(params)?;
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::ReadAttribute(identifier_idx),
                        identifier.loc.clone(),
                    );
                Ok(())
            }
            PostfixValue::ObjWrite(base, terms) => {
                base.emit_read(params)?;
                for term in terms {
                    params
                        .writer
                        .get_current_block()
                        .write_opcode_and_source_info(CompilerOpcode::Dup, term.loc().clone());
                    match term {
                        ObjWrite::Field(field_write) => {
                            let identifier_idx = params
                                .module
                                .constants
                                .insert(ConstantValue::String(field_write.field.value.clone()))
                                .map_err(|_| CompilationError {
                                    loc: field_write.field.loc.clone(),
                                    reason: CompilationErrorReason::TooManyConstants,
                                })?;

                            field_write.value.do_compile(params)?;
                            params
                                .writer
                                .get_current_block()
                                .write_opcode_and_source_info(
                                    CompilerOpcode::WriteAttribute(identifier_idx),
                                    term.loc().clone(),
                                );
                        }
                        ObjWrite::Index(index_write) => {
                            index_write.index.do_compile(params)?;
                            index_write.value.do_compile(params)?;
                            params
                                .writer
                                .get_current_block()
                                .write_opcode_and_source_info(
                                    CompilerOpcode::WriteIndex(1_u8),
                                    term.loc().clone(),
                                );
                        }
                    }
                }
                Ok(())
            }
            PostfixValue::TryProtocol(base, tp) => {
                let mode = match tp.mode {
                    aria_parser::ast::TryProtocolMode::Return => {
                        haxby_opcodes::try_unwrap_protocol_mode::PROPAGATE_ERROR
                    }
                    aria_parser::ast::TryProtocolMode::Assert => {
                        haxby_opcodes::try_unwrap_protocol_mode::ASSERT_ERROR
                    }
                };

                base.emit_read(params)?;

                let try_unwrap_protocol_idx = params
                    .module
                    .constants
                    .insert(ConstantValue::String("try_unwrap_protocol".to_string()))
                    .map_err(|_| CompilationError {
                        loc: tp.loc.clone(),
                        reason: CompilationErrorReason::TooManyConstants,
                    })?;
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::PushBuiltinTy(BuiltinTypeId::Result),
                        tp.loc.clone(),
                    )
                    .write_opcode_and_source_info(
                        CompilerOpcode::ReadAttribute(try_unwrap_protocol_idx),
                        tp.loc.clone(),
                    )
                    .write_opcode_and_source_info(CompilerOpcode::Call(1), tp.loc.clone())
                    .write_opcode_and_source_info(
                        CompilerOpcode::TryUnwrapProtocol(mode),
                        tp.loc.clone(),
                    );
                Ok(())
            }
        }
    }

    pub(super) fn emit_write(
        &self,
        val: &aria_parser::ast::Expression,
        params: &'a mut CompileParams,
    ) -> CompilationResult {
        match self {
            PostfixValue::Primary(primary) => match primary.as_ref() {
                aria_parser::ast::Primary::Identifier(id) => {
                    val.do_compile(params)?;
                    params.scope.emit_write(
                        &id.value,
                        &mut params.module.constants,
                        params.writer.get_current_block(),
                        primary.loc().clone(),
                    )?;
                    Ok(())
                }
                _ => Err(CompilationError {
                    loc: primary.loc().clone(),
                    reason: CompilationErrorReason::ReadOnlyValue,
                }),
            },
            PostfixValue::Call(.., loc) => Err(CompilationError {
                loc: loc.clone(),
                reason: CompilationErrorReason::ReadOnlyValue,
            }),
            PostfixValue::Case(_, case, _) => Err(CompilationError {
                loc: case.loc.clone(),
                reason: CompilationErrorReason::ReadOnlyValue,
            }),
            PostfixValue::Index(base, index) => {
                base.emit_read(params)?;
                index.do_compile(params)?;
                val.do_compile(params)?;
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::WriteIndex(index.expressions.len() as u8),
                        index.loc.clone(),
                    );
                Ok(())
            }
            PostfixValue::Attribute(base, identifier) => {
                let identifier_idx = match params
                    .module
                    .constants
                    .insert(ConstantValue::String(identifier.value.clone()))
                {
                    Ok(c) => c,
                    Err(_) => {
                        return Err(CompilationError {
                            loc: identifier.loc.clone(),
                            reason: CompilationErrorReason::TooManyConstants,
                        });
                    }
                };
                base.emit_read(params)?;
                val.do_compile(params)?;
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::WriteAttribute(identifier_idx),
                        identifier.loc.clone(),
                    );
                Ok(())
            }
            PostfixValue::ObjWrite(_, terms) => {
                let loc = terms.first().map(|x| x.loc()).unwrap_or(val.loc()).clone();
                Err(CompilationError {
                    loc,
                    reason: CompilationErrorReason::WriteOnlyValue,
                })
            }
            PostfixValue::TryProtocol(_, tp) => Err(CompilationError {
                loc: tp.loc.clone(),
                reason: CompilationErrorReason::ReadOnlyValue,
            }),
        }
    }
}

impl From<&aria_parser::ast::PostfixExpression> for PostfixValue {
    fn from(value: &aria_parser::ast::PostfixExpression) -> Self {
        let mut current = PostfixValue::Primary(Box::new(value.base.clone()));
        for term in &value.terms {
            match term {
                aria_parser::ast::PostfixTerm::PostfixTermAttribute(attr) => {
                    current = PostfixValue::Attribute(Box::new(current), Box::new(attr.id.clone()))
                }
                aria_parser::ast::PostfixTerm::PostfixTermIndex(index) => {
                    current = PostfixValue::Index(Box::new(current), Box::new(index.index.clone()))
                }
                aria_parser::ast::PostfixTerm::PostfixTermCall(call) => {
                    current = PostfixValue::Call(
                        Box::new(current),
                        Box::new(call.args.clone()),
                        call.loc.clone(),
                    )
                }
                aria_parser::ast::PostfixTerm::PostfixTermEnumCase(case) => {
                    current = PostfixValue::Case(
                        Box::new(current),
                        Box::new(case.id.clone()),
                        case.payload.clone(),
                    )
                }
                aria_parser::ast::PostfixTerm::PostfixTermObjectWrite(wrt) => {
                    use aria_parser::ast::PostfixTermWrite::{
                        PostfixTermFieldWrite, PostfixTermIndexWrite,
                    };

                    let mut terms = vec![];
                    for term in &wrt.terms.terms {
                        match term {
                            PostfixTermFieldWrite(term) => {
                                let expr = if let Some(expr) = &term.val {
                                    expr.clone()
                                } else {
                                    Expression::from(&term.id)
                                };
                                terms.push(ObjWrite::Field(FieldWrite {
                                    field: term.id.clone(),
                                    value: expr,
                                }));
                            }
                            PostfixTermIndexWrite(term) => {
                                terms.push(ObjWrite::Index(IndexWrite {
                                    index: term.idx.clone(),
                                    value: term.val.clone(),
                                }));
                            }
                        }
                    }
                    current = PostfixValue::ObjWrite(Box::new(current), terms)
                }
                aria_parser::ast::PostfixTerm::PostfixTermTryProtocol(tp) => {
                    current = PostfixValue::TryProtocol(Box::new(current), Box::new(tp.clone()))
                }
            }
        }

        current
    }
}
