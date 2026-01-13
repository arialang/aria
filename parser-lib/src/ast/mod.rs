// SPDX-License-Identifier: Apache-2.0
use std::{fmt::Display, path::Path, rc::Rc};

use pest::{Parser, error::InputLocation};

use crate::grammar::{HaxbyParser, Rule};

mod derive;
mod nodes;
pub mod prettyprint;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Location {
    pub start: usize,
    pub stop: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBuffer {
    pub content: Rc<String>,
    pub name: String,
}

impl AsRef<str> for SourceBuffer {
    fn as_ref(&self) -> &str {
        self.content.as_ref()
    }
}

impl SourceBuffer {
    pub fn stdin(input: &str) -> Self {
        Self {
            content: Rc::new(input.to_owned()),
            name: String::from("<stdin>"),
        }
    }

    pub fn stdin_with_name(input: &str, name: &str) -> Self {
        Self {
            content: Rc::new(input.to_owned()),
            name: String::from(name),
        }
    }

    pub fn from_path(path: &Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let path = match std::fs::canonicalize(path) {
            Ok(cp) => cp.to_str().unwrap_or("<unknown>").to_owned(),
            Err(_) => "<unknown>".to_owned(),
        };
        Ok(Self {
            content: Rc::new(content),
            name: path,
        })
    }

    pub fn file(path: &str) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let path = match std::fs::canonicalize(path) {
            Ok(cp) => match cp.to_str() {
                Some(cps) => cps,
                None => path,
            }
            .to_owned(),
            Err(_) => path.to_owned(),
        };
        Ok(Self {
            content: Rc::new(content),
            name: path,
        })
    }

    pub fn as_str(&self) -> String {
        self.content.as_ref().to_owned()
    }

    pub fn pointer(&self, loc: Location) -> SourcePointer {
        SourcePointer {
            location: loc,
            buffer: self.clone(),
        }
    }
}

impl SourceBuffer {
    pub fn lines(&self) -> Vec<String> {
        self.content.lines().map(|x| x.to_owned()).collect()
    }

    pub fn indices_for_position(&self, pos: usize) -> (usize, usize) {
        let start = self.content[..pos].rfind('\n').map_or(0, |idx| idx + 1);
        let end = self.content[pos..]
            .find('\n')
            .map_or(self.content.len(), |idx| pos + idx);
        (start, end)
    }

    pub fn line_for_position(&self, pos: usize) -> String {
        let (start, end) = self.indices_for_position(pos);
        self.content[start..end].to_owned()
    }

    pub fn line_index_for_position(&self, pos: usize) -> usize {
        self.content[..pos].chars().filter(|&c| c == '\n').count()
    }

    pub fn pointer_to_whole_buffer(&self) -> SourcePointer {
        let start = 0;
        let stop = self.content.len();
        let loc = Location { start, stop };
        self.pointer(loc)
    }

    pub fn pointer_to_last_line(&self) -> SourcePointer {
        let (start, stop) = self.indices_for_position(self.content.len() - 1);
        let loc = Location { start, stop };
        self.pointer(loc)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePointer {
    pub location: Location,
    pub buffer: SourceBuffer,
}

impl Display for SourcePointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}",
            self.buffer.name,
            1 + self.buffer.line_index_for_position(self.location.start)
        )
    }
}

impl<'i> From<&pest::Span<'i>> for Location {
    fn from(value: &pest::Span<'i>) -> Self {
        Self {
            start: value.start(),
            stop: value.end(),
        }
    }
}

impl From<&InputLocation> for Location {
    fn from(value: &InputLocation) -> Self {
        match value {
            InputLocation::Pos(pos) => Location {
                start: *pos,
                stop: *pos,
            },
            InputLocation::Span(span) => Location {
                start: span.0,
                stop: span.1,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntLiteralBase {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntLiteral {
    pub loc: SourcePointer,
    pub base: IntLiteralBase,
    pub val: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloatLiteral {
    pub loc: SourcePointer,
    pub val: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringLiteral {
    pub loc: SourcePointer,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    pub loc: SourcePointer,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierList {
    pub loc: SourcePointer,
    pub identifiers: Vec<Identifier>,
}

impl IdentifierList {
    pub fn empty(loc: SourcePointer) -> Self {
        Self {
            loc,
            identifiers: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionList {
    pub loc: SourcePointer,
    pub expressions: Vec<Expression>,
}

impl ExpressionList {
    pub fn empty(loc: SourcePointer) -> Self {
        Self {
            loc,
            expressions: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListLiteral {
    pub loc: SourcePointer,
    pub items: ExpressionList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParenExpression {
    pub loc: SourcePointer,
    pub value: Box<Expression>,
}

impl From<&Expression> for ParenExpression {
    fn from(value: &Expression) -> Self {
        Self {
            loc: value.loc().clone(),
            value: Box::new(value.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Primary {
    IntLiteral(IntLiteral),
    FloatLiteral(FloatLiteral),
    Identifier(Identifier),
    ListLiteral(ListLiteral),
    StringLiteral(StringLiteral),
    ParenExpression(ParenExpression),
}

impl Primary {
    pub fn loc(&self) -> &SourcePointer {
        match self {
            Self::IntLiteral(il) => &il.loc,
            Self::FloatLiteral(fp) => &fp.loc,
            Self::Identifier(id) => &id.loc,
            Self::ListLiteral(ll) => &ll.loc,
            Self::StringLiteral(sl) => &sl.loc,
            Self::ParenExpression(pe) => &pe.loc,
        }
    }

    pub fn is_int_literal(&self) -> bool {
        matches!(self, Self::IntLiteral(_))
    }

    pub fn is_float_literal(&self) -> bool {
        matches!(self, Self::FloatLiteral(_))
    }

    pub fn as_int_literal(&self) -> Option<&IntLiteral> {
        if let Self::IntLiteral(il) = self {
            Some(il)
        } else {
            None
        }
    }

    pub fn as_float_literal(&self) -> Option<&FloatLiteral> {
        if let Self::FloatLiteral(fl) = self {
            Some(fl)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixTermAttribute {
    pub loc: SourcePointer,
    pub id: Identifier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixTermIndex {
    pub loc: SourcePointer,
    pub index: ExpressionList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixTermCall {
    pub loc: SourcePointer,
    pub args: ExpressionList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixTermEnumCase {
    pub loc: SourcePointer,
    pub id: Identifier,
    pub payload: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixTermFieldWrite {
    pub loc: SourcePointer,
    pub id: Identifier,
    pub val: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixTermIndexWrite {
    pub loc: SourcePointer,
    pub idx: ExpressionList,
    pub val: Expression,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostfixTermWrite {
    PostfixTermFieldWrite(PostfixTermFieldWrite),
    PostfixTermIndexWrite(PostfixTermIndexWrite),
}

impl PostfixTermWrite {
    pub fn loc(&self) -> &SourcePointer {
        match self {
            Self::PostfixTermFieldWrite(fw) => &fw.loc,
            Self::PostfixTermIndexWrite(iw) => &iw.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixTermWriteList {
    pub loc: SourcePointer,
    pub terms: Vec<PostfixTermWrite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixTermObjectWrite {
    pub loc: SourcePointer,
    pub terms: PostfixTermWriteList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TryProtocolMode {
    Return,
    Assert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixTermTryProtocol {
    pub loc: SourcePointer,
    pub mode: TryProtocolMode,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostfixTerm {
    PostfixTermAttribute(PostfixTermAttribute),
    PostfixTermIndex(PostfixTermIndex),
    PostfixTermCall(PostfixTermCall),
    PostfixTermObjectWrite(PostfixTermObjectWrite),
    PostfixTermEnumCase(PostfixTermEnumCase),
    PostfixTermTryProtocol(PostfixTermTryProtocol),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixExpression {
    pub loc: SourcePointer,
    pub base: Primary,
    pub terms: Vec<PostfixTerm>,
}

impl PostfixExpression {
    pub fn attrib_read(base: &Primary, name: &str) -> PostfixExpression {
        let read_attr = PostfixTerm::PostfixTermAttribute(PostfixTermAttribute {
            loc: base.loc().clone(),
            id: Identifier {
                loc: base.loc().clone(),
                value: name.to_owned(),
            },
        });
        Self {
            loc: base.loc().clone(),
            base: base.clone(),
            terms: vec![read_attr],
        }
    }

    pub fn method_call(base: &Primary, name: &str, args: &[Expression]) -> PostfixExpression {
        let read_attr = PostfixTerm::PostfixTermAttribute(PostfixTermAttribute {
            loc: base.loc().clone(),
            id: Identifier {
                loc: base.loc().clone(),
                value: name.to_owned(),
            },
        });
        let call_attr = PostfixTerm::PostfixTermCall(PostfixTermCall {
            loc: base.loc().clone(),
            args: ExpressionList {
                loc: base.loc().clone(),
                expressions: args.to_vec(),
            },
        });
        Self {
            loc: base.loc().clone(),
            base: base.clone(),
            terms: vec![read_attr, call_attr],
        }
    }
}

impl From<&Primary> for PostfixExpression {
    fn from(value: &Primary) -> Self {
        Self {
            loc: value.loc().clone(),
            base: value.clone(),
            terms: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostfixRvalue {
    pub loc: SourcePointer,
    pub expr: PostfixExpression,
}

impl From<&PostfixExpression> for PostfixRvalue {
    fn from(value: &PostfixExpression) -> Self {
        Self {
            loc: value.loc.clone(),
            expr: value.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum UnarySymbol {
    Exclamation,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnaryOperation {
    pub loc: SourcePointer,
    pub operand: Option<UnarySymbol>,
    pub postfix: PostfixRvalue,
}

impl From<&PostfixRvalue> for UnaryOperation {
    fn from(value: &PostfixRvalue) -> Self {
        Self {
            loc: value.loc.clone(),
            operand: None,
            postfix: value.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MulSymbol {
    Star,
    Slash,
    Percent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddSymbol {
    Plus,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddEqSymbol {
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    ShiftLeftEq,
    ShiftRightEq,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MulOperation {
    pub loc: SourcePointer,
    pub left: UnaryOperation,
    pub right: Vec<(MulSymbol, UnaryOperation)>,
}

impl From<&UnaryOperation> for MulOperation {
    fn from(value: &UnaryOperation) -> Self {
        Self {
            loc: value.loc.clone(),
            left: value.clone(),
            right: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddOperation {
    pub loc: SourcePointer,
    pub left: MulOperation,
    pub right: Vec<(AddSymbol, MulOperation)>,
}

impl From<&MulOperation> for AddOperation {
    fn from(value: &MulOperation) -> Self {
        Self {
            loc: value.loc.clone(),
            left: value.clone(),
            right: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum ShiftSymbol {
    Leftward,
    Rightward,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftOperation {
    pub loc: SourcePointer,
    pub left: AddOperation,
    pub right: Option<(ShiftSymbol, AddOperation)>,
}

impl From<&AddOperation> for ShiftOperation {
    fn from(value: &AddOperation) -> Self {
        Self {
            loc: value.loc.clone(),
            left: value.clone(),
            right: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum RelSymbol {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelOperation {
    pub loc: SourcePointer,
    pub left: ShiftOperation,
    pub right: Option<(RelSymbol, ShiftOperation)>,
}

impl From<&ShiftOperation> for RelOperation {
    fn from(value: &ShiftOperation) -> Self {
        Self {
            loc: value.loc.clone(),
            left: value.clone(),
            right: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompSymbol {
    Equal,
    NotEqual,
    Isa,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompOperation {
    pub loc: SourcePointer,
    pub left: RelOperation,
    pub right: Option<(CompSymbol, RelOperation)>,
}

impl From<&RelOperation> for CompOperation {
    fn from(value: &RelOperation) -> Self {
        Self {
            loc: value.loc.clone(),
            left: value.clone(),
            right: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogSymbol {
    Ampersand,
    DoubleAmpersand,
    DoublePipe,
    Pipe,
    Caret,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogOperation {
    pub loc: SourcePointer,
    pub left: CompOperation,
    pub right: Vec<(LogSymbol, CompOperation)>,
}

impl From<&CompOperation> for LogOperation {
    fn from(value: &CompOperation) -> Self {
        Self {
            loc: value.loc.clone(),
            left: value.clone(),
            right: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum LambdaBody {
    Expression(Expression),
    CodeBlock(CodeBlock),
}

impl From<&LambdaBody> for FunctionBody {
    fn from(value: &LambdaBody) -> Self {
        match value {
            LambdaBody::Expression(e) => {
                let ret_stmt = ReturnStatement {
                    loc: e.loc().clone(),
                    val: Some(e.clone()),
                };
                Self {
                    code: CodeBlock {
                        loc: ret_stmt.loc.clone(),
                        entries: vec![Statement::ReturnStatement(ret_stmt)],
                    },
                }
            }
            LambdaBody::CodeBlock(b) => Self { code: b.clone() },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LambdaFunction {
    pub loc: SourcePointer,
    pub args: ArgumentList,
    pub body: Box<LambdaBody>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionBody {
    pub code: CodeBlock,
}

impl FunctionBody {
    pub fn loc(&self) -> &SourcePointer {
        &self.code.loc
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TernaryExpression {
    pub loc: SourcePointer,
    pub condition: Box<LogOperation>,
    pub true_expression: Box<Expression>,
    pub false_expression: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TryUnwrapExpression {
    pub loc: SourcePointer,
    pub left: Box<LogOperation>,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Expression {
    LambdaFunction(LambdaFunction),
    LogOperation(LogOperation),
    TernaryExpression(TernaryExpression),
    TryUnwrapExpression(TryUnwrapExpression),
}

impl From<&LogOperation> for Expression {
    fn from(value: &LogOperation) -> Self {
        Self::LogOperation(value.clone())
    }
}

impl From<&Identifier> for Expression {
    fn from(value: &Identifier) -> Self {
        let pfe = PostfixExpression::from(&Primary::Identifier(value.clone()));
        Self::from(&pfe)
    }
}

impl From<&Primary> for Expression {
    fn from(value: &Primary) -> Self {
        let pfe = PostfixExpression::from(value);
        Self::from(&pfe)
    }
}

impl From<&UnaryOperation> for Expression {
    fn from(value: &UnaryOperation) -> Self {
        Self::from(&LogOperation::from(&CompOperation::from(
            &RelOperation::from(&ShiftOperation::from(&AddOperation::from(
                &MulOperation::from(value),
            ))),
        )))
    }
}

impl From<&PostfixExpression> for Expression {
    fn from(value: &PostfixExpression) -> Self {
        Self::from(&LogOperation::from(&CompOperation::from(
            &RelOperation::from(&ShiftOperation::from(&AddOperation::from(
                &MulOperation::from(&UnaryOperation::from(&PostfixRvalue::from(value))),
            ))),
        )))
    }
}

impl Expression {
    pub fn loc(&self) -> &SourcePointer {
        match self {
            Expression::LogOperation(c) => &c.loc,
            Expression::LambdaFunction(f) => &f.loc,
            Expression::TernaryExpression(t) => &t.loc,
            Expression::TryUnwrapExpression(t) => &t.loc,
        }
    }
}

impl Expression {
    pub fn call_function_passing_me(&self, func_name: &str) -> Expression {
        let loc = self.loc().clone();

        let func_ident = Identifier {
            loc: loc.clone(),
            value: func_name.to_owned(),
        };

        let base = Primary::Identifier(func_ident);

        let args = ExpressionList {
            loc: loc.clone(),
            expressions: vec![self.clone()],
        };

        let call = PostfixTerm::PostfixTermCall(PostfixTermCall {
            loc: loc.clone(),
            args,
        });

        let pfe = PostfixExpression {
            loc,
            base,
            terms: vec![call],
        };

        Expression::from(&pfe)
    }

    pub fn is_function_call(&self) -> (bool, Option<&str>) {
        fn peel(log: &LogOperation) -> Option<&PostfixExpression> {
            if !log.right.is_empty() {
                return None;
            }
            let comp: &CompOperation = &log.left;
            if comp.right.is_some() {
                return None;
            }
            let rel: &RelOperation = &comp.left;
            if rel.right.is_some() {
                return None;
            }
            let shift: &ShiftOperation = &rel.left;
            if shift.right.is_some() {
                return None;
            }
            let add: &AddOperation = &shift.left;
            if !add.right.is_empty() {
                return None;
            }
            let mul: &MulOperation = &add.left;
            if !mul.right.is_empty() {
                return None;
            }
            let UnaryOperation { postfix, .. } = &mul.left;
            Some(&postfix.expr)
        }

        fn resolve_name(pfe: &PostfixExpression) -> Option<&str> {
            let last = pfe.terms.last()?;
            if !matches!(last, PostfixTerm::PostfixTermCall(_)) {
                return None;
            }
            if pfe.terms.len() == 1
                && let crate::ast::Primary::Identifier(id) = &pfe.base
            {
                return Some(&id.value);
            }
            if pfe.terms.len() >= 2
                && let PostfixTerm::PostfixTermAttribute(attr) = &pfe.terms[pfe.terms.len() - 2]
            {
                return Some(&attr.id.value);
            }
            None
        }

        match self {
            Expression::LogOperation(log) => {
                if let Some(pfe) = peel(log)
                    && matches!(pfe.terms.last(), Some(PostfixTerm::PostfixTermCall(_)))
                {
                    return (true, resolve_name(pfe));
                }
                (false, None)
            }
            _ => (false, None),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationId {
    pub loc: SourcePointer,
    pub name: Identifier,
    pub ty: Option<Expression>,
}

impl From<&Identifier> for DeclarationId {
    fn from(value: &Identifier) -> Self {
        Self {
            loc: value.loc.clone(),
            name: value.clone(),
            ty: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionStatement {
    pub loc: SourcePointer,
    pub val: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValDeclEntry {
    pub loc: SourcePointer,
    pub id: DeclarationId,
    pub val: Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValDeclStatement {
    pub loc: SourcePointer,
    pub decls: Vec<ValDeclEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignStatement {
    pub loc: SourcePointer,
    pub id: Vec<PostfixExpression>,
    pub val: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOpEqStatement {
    pub loc: SourcePointer,
    pub id: PostfixExpression,
    pub op: AddEqSymbol,
    pub val: Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfCondPiece {
    pub loc: SourcePointer,
    pub expression: Box<Expression>,
    pub then: CodeBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElsePiece {
    pub loc: SourcePointer,
    pub then: CodeBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElsifPiece {
    pub content: IfCondPiece,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfPiece {
    pub content: IfCondPiece,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfStatement {
    pub loc: SourcePointer,
    pub iff: IfPiece,
    pub elsif: Vec<ElsifPiece>,
    pub els: Option<ElsePiece>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPatternComp {
    pub loc: SourcePointer,
    pub op: CompSymbol,
    pub expr: Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPatternRel {
    pub loc: SourcePointer,
    pub op: RelSymbol,
    pub expr: Expression,
}

impl MatchPatternComp {
    pub fn isa(loc: SourcePointer, expr: Expression) -> Self {
        Self {
            loc,
            op: CompSymbol::Isa,
            expr,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPatternEnumCase {
    pub loc: SourcePointer,
    pub case: Identifier,
    pub payload: Option<DeclarationId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchPattern {
    MatchPatternComp(MatchPatternComp),
    MatchPatternRel(MatchPatternRel),
    MatchPatternEnumCase(MatchPatternEnumCase),
}

impl MatchPattern {
    pub fn loc(&self) -> &SourcePointer {
        match self {
            Self::MatchPatternComp(e) => &e.loc,
            Self::MatchPatternRel(e) => &e.loc,
            Self::MatchPatternEnumCase(c) => &c.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchRule {
    pub loc: SourcePointer,
    pub patterns: Vec<MatchPattern>,
    pub then: CodeBlock,
}

impl MatchRule {
    pub fn enum_and_case(
        loc: SourcePointer,
        enumm: &str,
        case: &str,
        payload: Option<Identifier>,
        then: CodeBlock,
    ) -> Self {
        let enumm = Identifier {
            loc: loc.clone(),
            value: enumm.to_owned(),
        };
        let case = Identifier {
            loc: loc.clone(),
            value: case.to_owned(),
        };
        let payload = payload.map(|p| DeclarationId::from(&p));
        let case_pattern = MatchPattern::MatchPatternEnumCase(MatchPatternEnumCase {
            loc: enumm.loc.clone(),
            case,
            payload,
        });
        let isa_pattern = MatchPattern::MatchPatternComp(MatchPatternComp {
            loc: loc.clone(),
            op: CompSymbol::Isa,
            expr: From::from(&enumm),
        });
        Self {
            loc,
            patterns: vec![isa_pattern, case_pattern],
            then,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchStatement {
    pub loc: SourcePointer,
    pub expr: Expression,
    pub rules: Vec<MatchRule>,
    pub els: Option<ElsePiece>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhileStatement {
    pub loc: SourcePointer,
    pub cond: Expression,
    pub then: CodeBlock,
    pub els: Option<ElsePiece>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForStatement {
    pub loc: SourcePointer,
    pub id: Identifier,
    pub expr: Expression,
    pub then: CodeBlock,
    pub els: Option<ElsePiece>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnStatement {
    pub loc: SourcePointer,
    pub val: Option<Expression>,
}

impl From<&Expression> for ReturnStatement {
    fn from(val: &Expression) -> Self {
        Self {
            loc: val.loc().clone(),
            val: Some(val.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThrowStatement {
    pub loc: SourcePointer,
    pub val: Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertStatement {
    pub loc: SourcePointer,
    pub val: Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakStatement {
    pub loc: SourcePointer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinueStatement {
    pub loc: SourcePointer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TryBlock {
    pub loc: SourcePointer,
    pub body: CodeBlock,
    pub id: Identifier,
    pub catch: CodeBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Statement {
    ValDeclStatement(ValDeclStatement),
    AssignStatement(AssignStatement),
    WriteOpEqStatement(WriteOpEqStatement),
    IfStatement(IfStatement),
    MatchStatement(MatchStatement),
    WhileStatement(WhileStatement),
    ForStatement(ForStatement),
    CodeBlock(CodeBlock),
    ReturnStatement(ReturnStatement),
    ThrowStatement(ThrowStatement),
    TryBlock(TryBlock),
    AssertStatement(AssertStatement),
    ExpressionStatement(ExpressionStatement),
    BreakStatement(BreakStatement),
    ContinueStatement(ContinueStatement),
    StructDecl(StructDecl),
    EnumDecl(EnumDecl),
    FunctionDecl(FunctionDecl),
}

impl Statement {
    pub fn loc(&self) -> &SourcePointer {
        match self {
            Self::ValDeclStatement(a) => &a.loc,
            Self::AssignStatement(a) => &a.loc,
            Self::WriteOpEqStatement(a) => &a.loc,
            Self::IfStatement(a) => &a.loc,
            Self::MatchStatement(a) => &a.loc,
            Self::WhileStatement(a) => &a.loc,
            Self::ForStatement(a) => &a.loc,
            Self::CodeBlock(a) => &a.loc,
            Self::ReturnStatement(a) => &a.loc,
            Self::ThrowStatement(a) => &a.loc,
            Self::TryBlock(a) => &a.loc,
            Self::AssertStatement(a) => &a.loc,
            Self::ExpressionStatement(a) => &a.loc,
            Self::BreakStatement(a) => &a.loc,
            Self::ContinueStatement(a) => &a.loc,
            Self::StructDecl(s) => &s.loc,
            Self::EnumDecl(e) => &e.loc,
            Self::FunctionDecl(f) => &f.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    pub loc: SourcePointer,
    pub entries: Vec<Statement>,
}

impl From<&Statement> for CodeBlock {
    fn from(value: &Statement) -> Self {
        Self {
            loc: value.loc().clone(),
            entries: vec![value.clone()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentDecl {
    pub loc: SourcePointer,
    pub id: DeclarationId,
    pub deft: Option<Expression>,
}

impl ArgumentDecl {
    pub fn name(&self) -> &String {
        &self.id.name.value
    }

    pub fn type_info(&self) -> Option<&Expression> {
        self.id.ty.as_ref()
    }
}

impl From<&DeclarationId> for ArgumentDecl {
    fn from(value: &DeclarationId) -> Self {
        Self {
            loc: value.loc.clone(),
            id: value.clone(),
            deft: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentList {
    pub loc: SourcePointer,
    pub names: Vec<ArgumentDecl>,
    pub vararg: bool,
}

impl ArgumentList {
    pub fn empty(loc: SourcePointer) -> Self {
        Self {
            loc,
            names: vec![],
            vararg: false,
        }
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDecl {
    pub loc: SourcePointer,
    pub name: Identifier,
    pub args: ArgumentList,
    pub body: FunctionBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodAccess {
    Instance,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodDecl {
    pub loc: SourcePointer,
    pub access: MethodAccess,
    pub name: Identifier,
    pub args: ArgumentList,
    pub body: FunctionBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorSymbol {
    Plus,
    Minus,
    UnaryMinus,
    Star,
    Slash,
    Percent,
    LeftShift,
    RightShift,
    Equals,
    LessThanEqual,
    GreaterThanEqual,
    LessThan,
    GreaterThan,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    Call,
    GetSquareBrackets,
    SetSquareBrackets,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorDecl {
    pub loc: SourcePointer,
    pub reverse: bool,
    pub symbol: OperatorSymbol,
    pub args: ArgumentList,
    pub body: FunctionBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixinIncludeDecl {
    pub loc: SourcePointer,
    pub what: Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructEntry {
    Method(Box<MethodDecl>),
    Operator(Box<OperatorDecl>),
    Variable(Box<ValDeclStatement>),
    Struct(Box<StructDecl>),
    Enum(Box<EnumDecl>),
    MixinInclude(Box<MixinIncludeDecl>),
}

impl StructEntry {
    pub fn loc(&self) -> &SourcePointer {
        match self {
            Self::Method(m) => &m.loc,
            Self::Operator(o) => &o.loc,
            Self::Variable(v) => &v.loc,
            Self::Struct(s) => &s.loc,
            Self::Enum(e) => &e.loc,
            Self::MixinInclude(m) => &m.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDecl {
    pub loc: SourcePointer,
    pub name: Identifier,
    pub inherits: Vec<Expression>,
    pub body: Vec<StructEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixinDecl {
    pub loc: SourcePointer,
    pub name: Identifier,
    pub body: Vec<StructEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumCaseDecl {
    pub loc: SourcePointer,
    pub name: Identifier,
    pub payload: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum EnumDeclEntry {
    EnumCaseDecl(EnumCaseDecl),
    StructEntry(StructEntry),
}

impl EnumDeclEntry {
    pub fn loc(&self) -> &SourcePointer {
        match self {
            Self::EnumCaseDecl(e) => &e.loc,
            Self::StructEntry(s) => s.loc(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDecl {
    pub loc: SourcePointer,
    pub name: Identifier,
    pub body: Vec<EnumDeclEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionDecl {
    pub loc: SourcePointer,
    pub target: Expression,
    pub inherits: Vec<Expression>,
    pub body: Vec<StructEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPath {
    pub loc: SourcePointer,
    pub entries: Vec<Identifier>,
}

impl ImportPath {
    pub fn from_dotted_string(loc: SourcePointer, dotted: &str) -> Self {
        Self {
            loc: loc.clone(),
            entries: dotted
                .split('.')
                .map(|x| Identifier {
                    loc: loc.clone(),
                    value: x.to_owned(),
                })
                .collect(),
        }
    }

    pub fn to_dotted_string(&self) -> String {
        self.entries
            .iter()
            .map(|x| x.value.clone())
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn to_path_string(&self) -> String {
        self.entries
            .iter()
            .map(|x| x.value.clone())
            .collect::<Vec<_>>()
            .join("/")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportStatement {
    pub loc: SourcePointer,
    pub what: ImportPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportTarget {
    IdentifierList(IdentifierList),
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportFromStatement {
    pub loc: SourcePointer,
    pub what: ImportTarget,
    pub from: ImportPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleFlag {
    NoStandardLibrary,
    UsesDylib(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleFlags {
    pub flags: Vec<ModuleFlag>,
}

impl ModuleFlags {
    pub fn empty() -> Self {
        Self { flags: vec![] }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum TopLevelEntry {
    ExpressionStatement(ExpressionStatement),
    ValDeclStatement(ValDeclStatement),
    WriteOpEqStatement(WriteOpEqStatement),
    AssignStatement(AssignStatement),
    FunctionDecl(FunctionDecl),
    StructDecl(StructDecl),
    MixinDecl(MixinDecl),
    EnumDecl(EnumDecl),
    ExtensionDecl(ExtensionDecl),
    AssertStatement(AssertStatement),
    ImportStatement(ImportStatement),
    ImportFromStatement(ImportFromStatement),
    IfStatement(IfStatement),
    MatchStatement(MatchStatement),
    WhileStatement(WhileStatement),
    ForStatement(ForStatement),
    CodeBlock(CodeBlock),
    TryBlock(TryBlock),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedModule {
    pub loc: SourcePointer,
    pub flags: ModuleFlags,
    pub entries: Vec<TopLevelEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserError {
    pub loc: SourcePointer,
    pub msg: String,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

pub type ParserResult<T> = Result<T, ParserError>;

pub fn source_to_ast(source: &SourceBuffer) -> ParserResult<ParsedModule> {
    let input = &source.as_str();
    let parse_tree = HaxbyParser::parse(Rule::module, input);
    match parse_tree {
        Ok(mut pt) => Ok(<ParsedModule as derive::Derive>::from_parse_tree(
            pt.next_back().expect("invalid parse tree"),
            source,
        )),
        Err(err) => {
            let loc = From::from(&err.location);
            let ptr = source.pointer(loc);
            Err(ParserError {
                loc: ptr,
                msg: err.variant.message().to_string(),
            })
        }
    }
}
