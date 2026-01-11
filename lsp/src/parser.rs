// SPDX-License-Identifier: Apache-2.0
use crate::lexer::{self, SyntaxKind};
use SyntaxKind::*;
use core::ops::Range;
use line_index::LineIndex;
use rowan::{GreenNode, GreenNodeBuilder, TextSize};
use std::cell::Cell;

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {}
impl rowan::Language for Lang {
    type Kind = SyntaxKind;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        assert!(raw.0 <= Eof as u16);
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    expected: SyntaxKind,
    pos: Option<Range<usize>>,
}

pub struct Parse {
    green_node: GreenNode,
    errors: Vec<ParseError>,
}

impl Parse {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green_node.clone())
    }

    pub fn errors(&self) -> &Vec<ParseError> {
        &self.errors
    }
}

impl ParseError {
    pub fn expected(&self) -> SyntaxKind {
        self.expected
    }
    pub fn pos(&self) -> Option<Range<usize>> {
        self.pos.clone()
    }
}

pub fn parse(text: &str) -> Parse {
    #[derive(Debug)]
    enum Event {
        Open { kind: SyntaxKind },
        Close,
        Advance,
    }

    struct MarkOpened {
        index: usize,
    }

    struct MarkClosed {
        index: usize,
    }

    struct Parser<'a> {
        tokens: Vec<(SyntaxKind, &'a str, logos::Span)>,
        pos: usize,
        fuel: Cell<u32>,
        events: Vec<Event>,
        errors: Vec<ParseError>,
        line_index: LineIndex,
    }

    fn is_trivia(kind: SyntaxKind) -> bool {
        matches!(kind, SyntaxKind::Whitespace | SyntaxKind::LineComment)
    }

    fn prefix_binding_power(op: SyntaxKind) -> Option<((), u8)> {
        use SyntaxKind::*;
        match op {
            Not | Minus => Some(((), 23)),
            _ => None,
        }
    }

    fn is_expr_start(kind: SyntaxKind) -> bool {
        use SyntaxKind::*;
        matches!(
            kind,
            Identifier
                | HexIntLiteral
                | OctIntLiteral
                | BinIntLiteral
                | DecIntLiteral
                | FloatLiteral
                | StringLiteral
                | TrueKwd
                | FalseKwd
                | LeftParen
                | LeftBracket
                | Pipe
                | LogicalOr
                | Not
                | Minus
        )
    }

    fn postfix_binding_power(op: SyntaxKind) -> Option<(u8, ())> {
        use SyntaxKind::*;
        match op {
            LeftParen | LeftBracket | Dot | DoubleColon => Some((25, ())),
            _ => None,
        }
    }

    fn infix_binding_power(op: SyntaxKind) -> Option<(u8, u8)> {
        use SyntaxKind::*;
        match op {
            // Assignment operators (right-associative, lowest precedence)
            Assign | PlusAssign | MinusAssign | StarAssign | SlashAssign | PercentAssign
            | LeftShiftAssign | RightShiftAssign => Some((2, 1)),

            LogicalOr => Some((3, 4)),
            LogicalAnd => Some((5, 6)),
            BitwiseXor => Some((7, 8)),
            BitwiseAnd => Some((9, 10)),
            Pipe => Some((11, 12)),

            Equal | NotEqual | IsaKwd => Some((13, 14)),

            Less | LessEqual | Greater | GreaterEqual => Some((15, 16)),

            LeftShift | RightShift => Some((17, 18)),

            Plus | Minus => Some((19, 20)),

            Star | Slash | Percent => Some((21, 22)),

            _ => None,
        }
    }

    impl Parser<'_> {
        fn file(&mut self) {
            let m: MarkOpened = self.open();
            self.trivia();

            while !self.eof() {
                match self.nth(0) {
                    ImportKwd => self.stmt_import(),
                    FlagKwd => self.module_flag(),
                    StructKwd => self.decl_struct_or_ext(Struct, StructKwd),
                    MixinKwd => self.decl_mixin(),
                    EnumKwd => self.decl_enum(),
                    ExtensionKwd => self.decl_struct_or_ext(Extension, ExtensionKwd),
                    FuncKwd => self.decl_func(),
                    _ => self.stmt(),
                }
            }
            self.trivia();
            self.close(m, File);
        }

        fn mixin_include(&mut self) {
            assert!(self.at(IncludeKwd));
            let m = self.open();

            self.expect(IncludeKwd);
            let _ = self.expr();

            self.close(m, MixinInclude);
        }

        fn decl_struct_or_ext(&mut self, kind: SyntaxKind, kwd: SyntaxKind) {
            assert!(self.at(kwd));
            let m = self.open();

            self.expect(kwd);
            if kwd == StructKwd {
                self.qualified_ident();
            } else {
                let _ = self.expr();
            }
            self.expect(LeftBrace);

            while !self.at(RightBrace) && !self.eof() {
                self.entry(StructEntry);
            }

            self.expect(RightBrace);
            self.close(m, kind);
        }

        fn entry(&mut self, kind: SyntaxKind) {
            let m = self.open();
            match self.nth(0) {
                FuncKwd => self.decl_func(),
                OperatorKwd | ReverseKwd => self.decl_operator(),
                StructKwd => self.decl_struct_or_ext(Struct, StructKwd),
                EnumKwd => self.decl_enum(),
                IncludeKwd => self.mixin_include(),
                TypeKwd | InstanceKwd => {
                    if self.nth(1) == FuncKwd {
                        self.decl_func();
                    } else {
                        self.decl_val();
                    }
                }
                _ => self.advance_with_error(kind),
            }
            self.close(m, StructEntry);
        }

        fn decl_enum(&mut self) {
            assert!(self.at(EnumKwd));
            let m = self.open();

            self.expect(EnumKwd);
            self.qualified_ident();
            self.expect(LeftBrace);

            while !self.at(RightBrace) && !self.eof() {
                match self.nth(0) {
                    CaseKwd => self.enum_case(),
                    _ => self.enum_entry(),
                }

                if self.at(Comma) {
                    self.expect(Comma);
                }
            }

            self.expect(RightBrace);
            self.close(m, Enum);
        }

        fn module_flag(&mut self) {
            assert!(self.at(FlagKwd));
            let m = self.open();

            self.expect(FlagKwd);
            self.expect(Colon);
            self.expect(Identifier);

            if self.at(LeftParen) {
                self.expect(LeftParen);
                // Value must be a string literal per grammar
                if self.at(StringLiteral) {
                    self.expect(StringLiteral);
                } else {
                    // fall back to generic expr to recover if not a string
                    let _ = self.expr();
                }
                self.expect(RightParen);
            }

            self.expect(Semicolon);
            self.close(m, ModuleFlag);
        }

        fn enum_case(&mut self) {
            assert!(self.at(CaseKwd));
            let m = self.open();

            self.expect(CaseKwd);
            self.expect(Identifier);

            if self.at(LeftParen) {
                self.expect(LeftParen);
                let _ = self.expr();
                self.expect(RightParen);
            }

            self.close(m, EnumCase);
        }

        fn enum_entry(&mut self) {
            let m = self.open();
            self.entry(EnumEntry);
            self.close(m, EnumEntry);
        }

        fn decl_mixin(&mut self) {
            assert!(self.at(MixinKwd));
            let m = self.open();

            self.expect(MixinKwd);
            self.qualified_ident();
            self.expect(LeftBrace);

            while !self.at(RightBrace) && !self.eof() {
                self.entry(MixinEntry);
            }

            self.expect(RightBrace);
            self.close(m, Mixin);
        }

        fn decl_operator(&mut self) {
            let m = self.open();

            if self.at(ReverseKwd) {
                self.expect(ReverseKwd);
            }

            self.expect(OperatorKwd);

            // Operator symbol - need to handle various operator tokens
            match self.nth(0) {
                Plus | Minus | UnaryMinus | Star | Slash | Percent | LeftShift | RightShift
                | Equal | LessEqual | GreaterEqual | Less | Greater | BitwiseAnd | Pipe
                | BitwiseXor => {
                    self.advance();
                }
                LeftParen => {
                    self.advance();
                    self.expect(RightParen);
                }
                LeftBracket => {
                    self.advance();
                    self.expect(RightBracket);
                    if self.at(Assign) {
                        self.expect(Assign)
                    }
                }
                _ => self.advance_with_error(Operator),
            }

            if self.at(LeftParen) {
                self.param_list(LeftParen, RightParen);
            }

            self.func_body();

            self.close(m, Operator);
        }

        fn decl_func(&mut self) {
            let m = self.open();

            self.parse_access_modifier();

            self.expect(FuncKwd);
            self.expect(Identifier);

            if self.at(LeftParen) {
                self.param_list(LeftParen, RightParen);
            }

            self.func_body();

            self.close(m, Func);
        }

        fn func_body(&mut self) {
            if self.at(LeftBrace) {
                self.block();
            } else if self.at(Assign) {
                self.expect(Assign);
                let _ = self.expr();
                self.expect(Semicolon);
            }
        }

        fn parse_access_modifier(&mut self) {
            if self.at(TypeKwd) {
                self.expect(TypeKwd);
            }

            if self.at(InstanceKwd) {
                self.expect(InstanceKwd);
            }
        }

        fn param_list(&mut self, left_delim: SyntaxKind, right_delim: SyntaxKind) {
            self.assert_tok(left_delim);
            let m = self.open();

            self.expect(left_delim);
            while !self.at(right_delim) && !self.eof() {
                if self.at(Identifier) || self.at(Ellipsis) {
                    self.param(self.nth(0), right_delim);
                } else {
                    break;
                }
            }
            self.expect(right_delim);

            self.close(m, ParamList);
        }

        fn param(&mut self, kind: SyntaxKind, right_delim: SyntaxKind) {
            assert!(self.at(kind));
            let m = self.open();

            self.expect(kind);

            if self.at(Colon) {
                self.type_annotation();
            }

            // Support default value for optional parameters: name [: type] = expr
            if self.at(Assign) {
                self.expect(Assign);
                let _ = self.expr();
            }

            if !self.at(right_delim) {
                self.expect(Comma);
            }

            self.close(m, Param);
        }

        fn type_annotation(&mut self) {
            self.assert_tok(Colon);
            self.expect(Colon);
            let m = self.open();
            while self.at(Identifier) {
                self.expect(Identifier);
                if self.at(Dot) {
                    self.expect(Dot);
                }
                if self.at(Pipe) {
                    self.expect(Pipe);
                }
                if self.at(BitwiseAnd) {
                    self.expect(BitwiseAnd);
                }
            }
            self.close(m, ExprType);
        }

        fn block(&mut self) {
            self.assert_tok(LeftBrace);
            let m = self.open();

            self.expect(LeftBrace);
            while !self.at(RightBrace) && !self.eof() {
                self.stmt();
            }
            self.expect(RightBrace);

            self.close(m, Block);
        }

        fn stmt(&mut self) {
            match self.nth(0) {
                AssertKwd => self.stmt_kwd_with_expr(AssertKwd),
                BreakKwd => self.stmt_single_token(BreakKwd),
                ContinueKwd => self.stmt_single_token(ContinueKwd),
                ValKwd => self.decl_val(),
                IfKwd => self.stmt_if(),
                MatchKwd => self.stmt_match(),
                WhileKwd => self.stmt_while(),
                ForKwd => self.stmt_for(),
                ThrowKwd => self.stmt_kwd_with_expr(ThrowKwd),
                ReturnKwd => self.stmt_return(),
                LeftBrace => self.block(),
                TryKwd => self.try_catch(),
                StructKwd => self.decl_struct_or_ext(Struct, StructKwd),
                EnumKwd => self.decl_enum(),
                FuncKwd => self.decl_func(),
                _ => self.stmt_expr(),
            }
        }

        fn stmt_if(&mut self) {
            assert!(self.at(IfKwd));
            let m = self.open();

            // if piece
            self.expect(IfKwd);
            let _ = self.expr();
            self.block();

            // elsif pieces
            while self.at(ElsifKwd) {
                self.expect(ElsifKwd);
                let _ = self.expr();
                self.block();
            }

            // optional else piece
            if self.at(ElseKwd) {
                self.expect(ElseKwd);
                self.block();
            }

            self.close(m, StmtIf);
        }

        fn stmt_for(&mut self) {
            assert!(self.at(ForKwd));
            let m = self.open();

            self.expect(ForKwd);
            self.expect(Identifier);
            self.expect(InKwd);
            let _ = self.expr();
            self.block();

            // optional else piece
            if self.at(ElseKwd) {
                self.expect(ElseKwd);
                self.block();
            }

            self.close(m, StmtFor);
        }

        fn stmt_match(&mut self) {
            assert!(self.at(MatchKwd));
            let m = self.open();

            self.expect(MatchKwd);
            let _ = self.expr();
            self.expect(LeftBrace);

            // Parse match rules
            if !self.at(RightBrace) {
                self.match_rule();

                while (self.at(Comma) || !self.at(RightBrace)) && !self.eof() {
                    if self.at(Comma) {
                        self.expect(Comma);
                    }
                    if !self.at(RightBrace) {
                        self.match_rule();
                    }
                }

                if self.at(Comma) {
                    self.expect(Comma);
                }
            }

            self.expect(RightBrace);

            // optional else piece
            if self.at(ElseKwd) {
                self.expect(ElseKwd);
                self.block();
            }

            self.close(m, StmtMatch);
        }

        fn match_rule(&mut self) {
            let m = self.open();

            self.match_pattern();

            // Handle "and" patterns
            while self.at(AndKwd) {
                self.expect(AndKwd);
                self.match_pattern();
            }

            self.expect(Arrow); // "=>"
            self.block();

            self.close(m, MatchRule);
        }

        fn match_pattern(&mut self) {
            let m = self.open();

            match self.nth(0) {
                CaseKwd => {
                    self.expect(CaseKwd);
                    self.expect(Identifier);
                    if self.at(LeftParen) {
                        self.expect(LeftParen);
                        self.expect(Identifier);
                        if self.at(Colon) {
                            self.expect(Colon);
                            let _ = self.expr();
                        }
                        self.expect(RightParen);
                    }
                }
                Equal | NotEqual | IsaKwd => {
                    self.advance(); // comparison operator
                    let _ = self.expr();
                }
                Less | LessEqual | Greater | GreaterEqual => {
                    self.advance(); // relational operator
                    let _ = self.expr();
                }
                _ => self.advance_with_error(MatchPattern),
            }

            self.close(m, MatchPattern);
        }

        fn stmt_while(&mut self) {
            assert!(self.at(WhileKwd));
            let m = self.open();

            self.expect(WhileKwd);
            let _ = self.expr();
            self.block();

            // optional else piece
            if self.at(ElseKwd) {
                self.expect(ElseKwd);
                self.block();
            }

            self.close(m, StmtWhile);
        }

        fn try_catch(&mut self) {
            assert!(self.at(TryKwd));
            let m = self.open();

            self.expect(TryKwd);
            self.block();
            self.expect(CatchKwd);
            self.expect(Identifier);
            self.block();

            self.close(m, TryBlock);
        }

        fn stmt_import(&mut self) {
            assert!(self.at(ImportKwd));
            let m = self.open();

            self.expect(ImportKwd);

            let mut is_from_import = false;
            let mut temp_pos = self.pos;

            while temp_pos < self.tokens.len()
                && self.tokens[temp_pos].0 != FromKwd
                && self.tokens[temp_pos].0 != Semicolon
            {
                temp_pos += 1;
            }

            if temp_pos < self.tokens.len() && self.tokens[temp_pos].0 == FromKwd {
                is_from_import = true;
            }

            if is_from_import {
                if self.at(Star) {
                    self.expect(Star);
                } else {
                    self.ident_list();
                }

                self.expect(FromKwd);
            }

            self.import_path();
            self.expect(Semicolon);

            self.close(m, StmtImport);
        }

        fn decl_val(&mut self) {
            let m = self.open();

            self.parse_access_modifier();

            self.expect(ValKwd);

            loop {
                self.expect(Identifier);

                if self.at(Colon) {
                    self.type_annotation();
                }

                self.expect(Assign);
                let _ = self.expr();

                if self.at(Comma) {
                    self.expect(Comma);
                } else {
                    break;
                }
            }

            self.expect(Semicolon);

            self.close(m, StmtVal);
        }

        fn stmt_single_token(&mut self, kind: SyntaxKind) {
            assert!(self.at(kind));
            let m = self.open();

            self.expect(kind);
            self.expect(Semicolon);

            self.close(m, StmtReturn);
        }

        fn stmt_return(&mut self) {
            assert!(self.at(ReturnKwd));
            let m = self.open();

            self.expect(ReturnKwd);

            if !self.at(Semicolon) {
                let _ = self.expr();
            }

            self.expect(Semicolon);
            self.close(m, StmtReturn);
        }

        fn stmt_kwd_with_expr(&mut self, kind: SyntaxKind) {
            assert!(self.at(kind));
            let m = self.open();

            self.expect(kind);
            let _ = self.expr();
            self.expect(Semicolon);

            self.close(m, StmtAssert);
        }

        fn init_list(&mut self) {
            // Recognize an initializer block only when the brace starts with a field or index init
            if self.at(LeftBrace) {
                let ahead = self.nth(1);
                if ahead != Dot && ahead != LeftBracket {
                    return;
                }
            } else {
                return;
            }

            self.assert_tok(LeftBrace);
            self.expect(LeftBrace);

            // Parse a comma-separated list of mixed field/index writes, optional trailing comma
            while !self.at(RightBrace) && !self.eof() {
                if self.at(Dot) {
                    // Field write: .identifier [= expr]?
                    self.expect(Dot);
                    self.expect(Identifier);
                    if self.at(Assign) {
                        self.expect(Assign);
                        let _ = self.expr();
                    }
                } else if self.at(LeftBracket) {
                    // Index write: [expr_list?] = expr
                    self.expr_list(LeftBracket, RightBracket);
                    self.expect(Assign);
                    let _ = self.expr();
                } else {
                    // Recovery: unexpected token inside init block
                    self.report_error(RightBrace);
                    break;
                }

                if self.at(Comma) {
                    self.expect(Comma);
                    // allow trailing comma; loop condition will exit on RightBrace
                } else {
                    // no more entries
                    break;
                }
            }

            self.expect(RightBrace);
        }

        fn ident_list(&mut self) {
            let m = self.open();

            self.expect(Identifier);
            while self.at(Comma) {
                self.expect(Comma);
                if self.at(Identifier) {
                    self.expect(Identifier);
                }
            }

            self.close(m, IdentList);
        }

        fn qualified_ident(&mut self) {
            let m = self.open();
            self.expect(Identifier);
            while self.at(Dot) {
                self.expect(Dot);
                self.expect(Identifier);
            }
            self.close(m, QualifiedIdent);
        }

        fn import_path(&mut self) {
            let m = self.open();
            self.qualified_ident();
            self.close(m, ImportPath);
        }

        fn stmt_expr(&mut self) {
            let m = self.open();

            if !self.at(Semicolon) {
                let _ = self.expr();

                // Allow multiple comma-separated expressions in a single
                // expression statement (e.g. multi-write patterns).
                while self.at(Comma) {
                    self.expect(Comma);

                    // Allow a trailing comma before semicolon, but don't
                    // try to parse another expression in that case.
                    if self.at(Semicolon) {
                        break;
                    }

                    let _ = self.expr();
                }
            }

            self.expect(Semicolon);

            self.close(m, StmtExpr);
        }

        fn expr(&mut self) -> MarkClosed {
            if self.at(Pipe) || self.at(LogicalOr) {
                self.decl_lambda()
            } else {
                self.expr_bp(0)
            }
        }

        fn decl_lambda(&mut self) -> MarkClosed {
            let m = self.open();

            if self.at(LogicalOr) {
                // Empty parameter list in the form of '||'
                self.expect(LogicalOr);
            } else {
                // Standard parameter list in the form of '|x, y|'
                self.param_list(Pipe, Pipe);
            }
            self.expect(Arrow);

            if self.at(LeftBrace) {
                self.block();
            } else {
                let _ = self.expr();
            }

            self.close(m, Lambda)
        }

        fn expr_bp(&mut self, min_bp: u8) -> MarkClosed {
            let mut lhs = self.expr_primary();

            loop {
                let op = self.nth(0);

                // Support initializer blocks as a postfix after any expression
                // e.g. "expr{ .field = value, [key] = value }"
                // Only consume if the brace actually starts an init block (next is '.' or '[')
                if op == LeftBrace {
                    let ahead = self.nth(1);
                    if ahead == Dot || ahead == LeftBracket {
                        self.init_list();
                        continue;
                    }
                }

                if let Some((l_bp, ())) = postfix_binding_power(op) {
                    if l_bp < min_bp {
                        break;
                    }

                    lhs = match op {
                        LeftParen => {
                            let m = self.open_before(lhs);
                            self.arg_list();
                            self.init_list();
                            self.close(m, ExprCall)
                        }
                        LeftBracket => {
                            let m = self.open_before(lhs);
                            self.expr_list(LeftBracket, RightBracket);
                            self.close(m, ExprIndex)
                        }
                        Dot => {
                            let m = self.open_before(lhs);
                            self.expect(Dot);
                            self.expect(Identifier);
                            self.close(m, ExprMember)
                        }
                        DoubleColon => {
                            let m = self.open_before(lhs);
                            self.expect(DoubleColon);
                            self.expect(Identifier);
                            if self.at(LeftParen) {
                                self.expect(LeftParen);
                                let _ = self.expr_bp(0);
                                self.expect(RightParen);
                            }
                            self.close(m, ExprMember)
                        }
                        _ => break,
                    };
                    continue;
                }

                if op == Not {
                    let l_bp = 25u8;
                    if l_bp < min_bp {
                        break;
                    }
                    let m = self.open_before(lhs);
                    self.expect(Not);
                    lhs = self.close(m, ExprNonNull);
                    continue;
                }

                if op == Question && !self.is_ternary_question() {
                    let l_bp = 25u8;
                    if l_bp < min_bp {
                        break;
                    }
                    let m = self.open_before(lhs);
                    self.expect(Question);
                    lhs = self.close(m, ExprNullish);
                    continue;
                }

                if op == Question {
                    let (l_bp, r_bp) = (1, 2);
                    if l_bp < min_bp {
                        break;
                    }

                    let m = self.open_before(lhs);
                    self.expect(Question);
                    let _ = self.expr_bp(r_bp);
                    self.expect(Colon);
                    let _ = self.expr_bp(r_bp);
                    lhs = self.close(m, ExprTernary);
                    continue;
                }

                if let Some((l_bp, r_bp)) = infix_binding_power(op) {
                    if l_bp < min_bp {
                        break;
                    }

                    let m = self.open_before(lhs);
                    self.advance(); // consume operator
                    let _ = self.expr_bp(r_bp);

                    // Use ExprAssign for assignment operators, ExprBinary for others
                    let node_kind = match op {
                        Assign | PlusAssign | MinusAssign | StarAssign | SlashAssign
                        | PercentAssign | LeftShiftAssign | RightShiftAssign => ExprAssign,
                        _ => ExprBinary,
                    };

                    lhs = self.close(m, node_kind);
                    continue;
                }

                break;
            }

            lhs
        }

        fn is_ternary_question(&self) -> bool {
            let mut idx = self.pos + 1;
            let mut first_kind = None;
            while let Some(tok) = self.tokens.get(idx) {
                if is_trivia(tok.0) {
                    idx += 1;
                    continue;
                }
                first_kind = Some(tok.0);
                break;
            }

            let Some(first_kind) = first_kind else {
                return false;
            };
            if !is_expr_start(first_kind) {
                return false;
            }

            let mut paren_depth = 0i32;
            let mut bracket_depth = 0i32;
            let mut brace_depth = 0i32;
            let mut scan_idx = idx;

            while let Some(tok) = self.tokens.get(scan_idx) {
                if is_trivia(tok.0) {
                    scan_idx += 1;
                    continue;
                }

                match tok.0 {
                    LeftParen => paren_depth += 1,
                    RightParen => {
                        if paren_depth == 0 {
                            return false;
                        }
                        paren_depth -= 1;
                    }
                    LeftBracket => bracket_depth += 1,
                    RightBracket => {
                        if bracket_depth == 0 {
                            return false;
                        }
                        bracket_depth -= 1;
                    }
                    LeftBrace => brace_depth += 1,
                    RightBrace => {
                        if brace_depth == 0 {
                            return false;
                        }
                        brace_depth -= 1;
                    }
                    Colon if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                        return true;
                    }
                    Comma | Semicolon
                        if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 =>
                    {
                        return false;
                    }
                    _ => {}
                }
                scan_idx += 1;
            }

            false
        }

        fn expr_primary(&mut self) -> MarkClosed {
            let m = self.open();

            match self.nth(0) {
                HexIntLiteral | OctIntLiteral | BinIntLiteral | DecIntLiteral | FloatLiteral
                | StringLiteral | TrueKwd | FalseKwd => {
                    self.advance();
                    self.close(m, ExprLiteral)
                }

                Identifier => {
                    self.advance();
                    self.init_list();
                    self.close(m, ExprName)
                }

                LeftParen => {
                    self.expect(LeftParen);
                    let _ = self.expr_bp(0);
                    self.expect(RightParen);
                    self.close(m, ExprParen)
                }

                LeftBracket => {
                    self.expr_list(LeftBracket, RightBracket);
                    self.close(m, ListLiteral)
                }

                op if prefix_binding_power(op).is_some() => {
                    let ((), r_bp) = prefix_binding_power(op).unwrap();
                    self.advance();
                    let _ = self.expr_bp(r_bp);
                    self.close(m, ExprUnary)
                }

                _ => {
                    if !self.eof() {
                        self.advance();
                    }
                    self.close(m, ErrorTree)
                }
            }
        }

        fn expr_list(&mut self, left_delim: SyntaxKind, right_delim: SyntaxKind) {
            self.expect(left_delim);

            if self.at(right_delim) {
                self.expect(right_delim);
                return;
            }

            self.expr();

            while self.at(Comma) && !self.eof() {
                self.expect(Comma);
                if !self.at(right_delim) {
                    self.expr();
                }
            }
            self.expect(right_delim);
        }

        fn arg_list(&mut self) {
            assert!(self.at(LeftParen));
            let m = self.open();
            self.expr_list(LeftParen, RightParen);
            self.close(m, ArgList);
        }

        fn open(&mut self) -> MarkOpened {
            let mark = MarkOpened {
                index: self.events.len(),
            };
            self.events.push(Event::Open { kind: ErrorTree });
            mark
        }

        fn close(&mut self, m: MarkOpened, kind: SyntaxKind) -> MarkClosed {
            self.events[m.index] = Event::Open { kind };
            self.events.push(Event::Close);
            MarkClosed { index: m.index }
        }

        fn open_before(&mut self, m: MarkClosed) -> MarkOpened {
            let mark = MarkOpened { index: m.index };
            // TODO: do something to avoid element shifting
            self.events.insert(m.index, Event::Open { kind: ErrorTree });
            mark
        }

        fn advance(&mut self) {
            // Before consuming a significant token, emit any leading trivia
            while let Some(tok) = self.tokens.get(self.pos) {
                if is_trivia(tok.0) {
                    self.events.push(Event::Advance);
                    self.pos += 1;
                } else {
                    break;
                }
            }

            assert!(!self.eof());
            self.fuel.set(256);
            self.events.push(Event::Advance);
            self.pos += 1;
        }

        fn eof(&self) -> bool {
            // true if there are no more non-trivia tokens
            let mut idx = self.pos;
            while let Some(tok) = self.tokens.get(idx) {
                if is_trivia(tok.0) {
                    idx += 1;
                    continue;
                }
                return false;
            }
            true
        }

        fn nth(&self, lookahead: usize) -> SyntaxKind {
            if self.fuel.get() == 0 {
                panic!("parser is stuck")
            }
            self.fuel.set(self.fuel.get() - 1);

            let mut idx = self.pos;
            let mut remaining = lookahead;
            while let Some(tok) = self.tokens.get(idx) {
                if is_trivia(tok.0) {
                    idx += 1;
                    continue;
                }
                if remaining == 0 {
                    return tok.0;
                }
                remaining -= 1;
                idx += 1;
            }
            Eof
        }

        fn nth_token(&self, lookahead: usize) -> Option<&(SyntaxKind, &str, logos::Span)> {
            self.tokens.get(self.pos + lookahead)
        }

        fn at(&self, kind: SyntaxKind) -> bool {
            self.nth(0) == kind || (kind == Identifier && self.is_keyword(self.nth(0)))
        }

        fn eat(&mut self, kind: SyntaxKind) -> bool {
            if self.at(kind) {
                self.advance();
                true
            } else {
                false
            }
        }

        fn is_keyword(&self, kind: SyntaxKind) -> bool {
            matches!(
                kind,
                AssertKwd
                    | BreakKwd
                    | CaseKwd
                    | CatchKwd
                    | ContinueKwd
                    | ElseKwd
                    | ElsifKwd
                    | EnumKwd
                    | ExtensionKwd
                    | FlagKwd
                    | ForKwd
                    | FromKwd
                    | FuncKwd
                    | IfKwd
                    | ImportKwd
                    | InKwd
                    | IncludeKwd
                    | InstanceKwd
                    | IsaKwd
                    | MatchKwd
                    | MixinKwd
                    | OperatorKwd
                    | ReturnKwd
                    | ReverseKwd
                    | StructKwd
                    | ThrowKwd
                    | TryKwd
                    | TypeKwd
                    | ValKwd
                    | WhileKwd
                    | AndKwd
                    | TrueKwd
                    | FalseKwd
            )
        }

        fn expect(&mut self, kind: SyntaxKind) {
            if self.eat(kind) {
                return;
            }
            self.report_error(kind);
        }

        fn advance_with_error(&mut self, token: SyntaxKind) {
            let m = self.open();
            self.report_error(token);
            self.advance();
            self.close(m, ErrorTree);
        }

        fn report_error(&mut self, expected: SyntaxKind) {
            let pos = self.nth_token(0).map(|tok| tok.2.clone());

            self.errors.push(ParseError { expected, pos });
        }

        fn trivia(&mut self) {
            while let Some(tok) = self.tokens.get(self.pos) {
                if is_trivia(tok.0) {
                    self.events.push(Event::Advance);
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }

        fn assert_tok(&mut self, kind: SyntaxKind) {
            assert!(
                self.at(kind),
                "expected {:?} but found {:?} at {:?}",
                kind,
                self.nth(0),
                self.line_index.line_col(TextSize::from(self.pos as u32))
            );
        }

        fn build_tree(self) -> Parse {
            {
                let mut opens = 0usize;
                let mut closes = 0usize;
                let mut advances = 0usize;
                let mut depth = 0isize;
                for (i, ev) in self.events.iter().enumerate() {
                    match ev {
                        Event::Open { .. } => {
                            opens += 1;
                            depth += 1;
                        }
                        Event::Close => {
                            closes += 1;
                            depth -= 1;
                            assert!(
                                depth >= 0,
                                "Unbalanced Close at event {i} (more closes than opens)"
                            );
                        }
                        Event::Advance => {
                            advances += 1;
                        }
                    }
                }
                assert_eq!(
                    opens, closes,
                    "Mismatched Open/Close count: opens={opens} closes={closes}"
                );
                let total_tokens = self.tokens.len();
                assert_eq!(
                    advances, total_tokens,
                    "Advance count {advances} does not match tokens len {total_tokens}"
                );
            }

            // TODO: add a shared node cache
            let mut builder = GreenNodeBuilder::new();
            let mut tokens = self.tokens.into_iter();

            for (idx, event) in self.events.into_iter().enumerate() {
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match event {
                    Event::Open { kind } => {
                        builder.start_node(kind.into());
                    }
                    Event::Close => {
                        builder.finish_node();
                    }
                    Event::Advance => {
                        let (token, slice, _) = tokens.next().unwrap();
                        builder.token(token.into(), slice);
                    }
                }));
                if res.is_err() {
                    eprintln!("Rowan builder panic at event index {idx}");
                    std::panic::resume_unwind(res.err().unwrap());
                }
            }

            assert!(tokens.next().is_none());

            let res_finish =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| builder.finish()));
            match res_finish {
                Ok(gn) => Parse {
                    green_node: gn,
                    errors: self.errors,
                },
                Err(p) => {
                    eprintln!("Rowan builder.finish() panic");
                    std::panic::resume_unwind(p)
                }
            }
        }
    }

    let lex = lexer::lex(text);
    let tokens = lex.into_iter().map(|res| res.unwrap()).collect();
    let mut parser = Parser {
        tokens,
        pos: 0,
        fuel: Cell::new(256),
        events: Vec::new(),
        errors: Vec::new(),
        line_index: LineIndex::new(text),
    };
    parser.file();
    parser.build_tree()
}

pub type SyntaxNode = rowan::SyntaxNode<Lang>;

#[allow(unused)]
pub type SyntaxToken = rowan::SyntaxToken<Lang>;

#[allow(unused)]
pub type SyntaxElement = rowan::NodeOrToken<SyntaxNode, SyntaxToken>;

#[cfg(test)]
mod tests {
    use super::*;
    use line_index::LineIndex;
    use pretty_assertions::assert_eq;

    fn tree_to_string(node: SyntaxNode) -> String {
        let mut result = Vec::new();
        let index = CompressedIndex::new(&node);
        tree_to_string_impl(&node, 0, &index, &mut result);
        result.join("\n")
    }

    // Build a mapping from original token ranges to compressed coordinates
    // that exclude trivia.
    struct CompressedIndex {
        // (original_text_range, compressed_start, compressed_end)
        entries: Vec<(rowan::TextRange, usize, usize)>,
    }

    impl CompressedIndex {
        fn new(root: &SyntaxNode) -> Self {
            use crate::lexer::SyntaxKind as K;
            let mut entries = Vec::new();
            let mut comp = 0usize;
            for el in root.descendants_with_tokens() {
                if let rowan::NodeOrToken::Token(tok) = el {
                    if matches!(tok.kind(), K::Whitespace | K::LineComment) {
                        continue;
                    }
                    let len = tok.text().len();
                    let start = comp;
                    let end = comp + len;
                    entries.push((tok.text_range(), start, end));
                    comp = end;
                }
            }
            Self { entries }
        }

        fn token_range(&self, tok: &SyntaxToken) -> Option<(usize, usize)> {
            let r = tok.text_range();
            self.entries
                .iter()
                .find_map(|(rr, s, e)| if *rr == r { Some((*s, *e)) } else { None })
        }

        fn node_range(&self, node: &SyntaxNode) -> Option<(usize, usize)> {
            use crate::lexer::SyntaxKind as K;
            let mut first: Option<usize> = None;
            let mut last: Option<usize> = None;
            for el in node.descendants_with_tokens() {
                if let rowan::NodeOrToken::Token(tok) = el {
                    if matches!(tok.kind(), K::Whitespace | K::LineComment) {
                        continue;
                    }
                    if let Some((s, e)) = self.token_range(&tok) {
                        if first.is_none() {
                            first = Some(s);
                        }
                        last = Some(e);
                    }
                }
            }
            match (first, last) {
                (Some(s), Some(e)) => Some((s, e)),
                _ => None,
            }
        }
    }

    fn tree_to_string_impl(
        node: &SyntaxNode,
        depth: usize,
        index: &CompressedIndex,
        result: &mut Vec<String>,
    ) {
        let indent = "  ".repeat(depth);
        if let Some((s, e)) = index.node_range(node) {
            result.push(format!("{}{:?}@{}..{}", indent, node.kind(), s, e));
        } else {
            // empty node
            result.push(format!("{}{:?}@0..0", indent, node.kind()));
        }

        for child in node.children_with_tokens() {
            match child {
                rowan::NodeOrToken::Node(child_node) => {
                    tree_to_string_impl(&child_node, depth + 1, index, result);
                }
                rowan::NodeOrToken::Token(token) => {
                    use crate::lexer::SyntaxKind as K;
                    if matches!(token.kind(), K::Whitespace | K::LineComment) {
                        continue;
                    }
                    let token_indent = "  ".repeat(depth + 1);
                    if let Some((s, e)) = index.token_range(&token) {
                        result.push(format!(
                            "{}{:?}@{}..{} {:?}",
                            token_indent,
                            token.kind(),
                            s,
                            e,
                            token.text()
                        ));
                    }
                }
            }
        }
    }

    fn expect_tree(input: &str, lines: &[&str]) {
        let node = parse(input).syntax();
        let tree_str = tree_to_string(node);
        let expected = lines.join("\n");
        assert_eq!(
            expected, tree_str,
            "tree should be\n{}\nbut is\n{}",
            expected, tree_str
        );
    }

    #[test]
    fn test_empty() {
        expect_tree("", &["File@0..0"])
    }

    #[test]
    fn test_empty_function() {
        expect_tree(
            "func empty_func() {}",
            &[
                "File@0..18",
                "  Func@0..18",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..14 \"empty_func\"",
                "    ParamList@14..16",
                "      LeftParen@14..15 \"(\"",
                "      RightParen@15..16 \")\"",
                "    Block@16..18",
                "      LeftBrace@16..17 \"{\"",
                "      RightBrace@17..18 \"}\"",
            ],
        )
    }

    #[test]
    fn test_param_list() {
        expect_tree(
            "func empty_func(x, y) {}",
            &[
                "File@0..21",
                "  Func@0..21",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..14 \"empty_func\"",
                "    ParamList@14..19",
                "      LeftParen@14..15 \"(\"",
                "      Param@15..17",
                "        Identifier@15..16 \"x\"",
                "        Comma@16..17 \",\"",
                "      Param@17..18",
                "        Identifier@17..18 \"y\"",
                "      RightParen@18..19 \")\"",
                "    Block@19..21",
                "      LeftBrace@19..20 \"{\"",
                "      RightBrace@20..21 \"}\"",
            ],
        )
    }

    #[test]
    fn test_val() {
        expect_tree(
            "func test() { val x = 5; }",
            &[
                "File@0..19",
                "  Func@0..19",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..19",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..18",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ExprLiteral@16..17",
                "          DecIntLiteral@16..17 \"5\"",
                "        Semicolon@17..18 \";\"",
                "      RightBrace@18..19 \"}\"",
            ],
        )
    }

    #[test]
    fn test_binary_expr() {
        expect_tree(
            "func test() { val x = 1 + 2 * 3; }",
            &[
                "File@0..23",
                "  Func@0..23",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..23",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..22",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ExprBinary@16..21",
                "          ExprLiteral@16..17",
                "            DecIntLiteral@16..17 \"1\"",
                "          Plus@17..18 \"+\"",
                "          ExprBinary@18..21",
                "            ExprLiteral@18..19",
                "              DecIntLiteral@18..19 \"2\"",
                "            Star@19..20 \"*\"",
                "            ExprLiteral@20..21",
                "              DecIntLiteral@20..21 \"3\"",
                "        Semicolon@21..22 \";\"",
                "      RightBrace@22..23 \"}\"",
            ],
        )
    }

    #[test]
    fn test_list_literal_empty() {
        expect_tree(
            "func test() { val x = []; }",
            &[
                "File@0..20",
                "  Func@0..20",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..20",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..19",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ListLiteral@16..18",
                "          LeftBracket@16..17 \"[\"",
                "          RightBracket@17..18 \"]\"",
                "        Semicolon@18..19 \";\"",
                "      RightBrace@19..20 \"}\"",
            ],
        )
    }

    #[test]
    fn test_list_literal_with_elements() {
        expect_tree(
            "func test() { val x = [1, 2, 3]; }",
            &[
                "File@0..25",
                "  Func@0..25",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..25",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..24",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ListLiteral@16..23",
                "          LeftBracket@16..17 \"[\"",
                "          ExprLiteral@17..18",
                "            DecIntLiteral@17..18 \"1\"",
                "          Comma@18..19 \",\"",
                "          ExprLiteral@19..20",
                "            DecIntLiteral@19..20 \"2\"",
                "          Comma@20..21 \",\"",
                "          ExprLiteral@21..22",
                "            DecIntLiteral@21..22 \"3\"",
                "          RightBracket@22..23 \"]\"",
                "        Semicolon@23..24 \";\"",
                "      RightBrace@24..25 \"}\"",
            ],
        )
    }

    #[test]
    fn test_list_literal_nested() {
        expect_tree(
            "func test() { val x = [[1, 2], [3]]; }",
            &[
                "File@0..29",
                "  Func@0..29",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..29",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..28",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ListLiteral@16..27",
                "          LeftBracket@16..17 \"[\"",
                "          ListLiteral@17..22",
                "            LeftBracket@17..18 \"[\"",
                "            ExprLiteral@18..19",
                "              DecIntLiteral@18..19 \"1\"",
                "            Comma@19..20 \",\"",
                "            ExprLiteral@20..21",
                "              DecIntLiteral@20..21 \"2\"",
                "            RightBracket@21..22 \"]\"",
                "          Comma@22..23 \",\"",
                "          ListLiteral@23..26",
                "            LeftBracket@23..24 \"[\"",
                "            ExprLiteral@24..25",
                "              DecIntLiteral@24..25 \"3\"",
                "            RightBracket@25..26 \"]\"",
                "          RightBracket@26..27 \"]\"",
                "        Semicolon@27..28 \";\"",
                "      RightBrace@28..29 \"}\"",
            ],
        )
    }

    #[test]
    fn test_unary_expr() {
        expect_tree(
            "func test() { val x = -5; }",
            &[
                "File@0..20",
                "  Func@0..20",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..20",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..19",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ExprUnary@16..18",
                "          Minus@16..17 \"-\"",
                "          ExprLiteral@17..18",
                "            DecIntLiteral@17..18 \"5\"",
                "        Semicolon@18..19 \";\"",
                "      RightBrace@19..20 \"}\"",
            ],
        )
    }

    #[test]
    fn test_member_access() {
        expect_tree(
            "func test() { val x = obj.field; }",
            &[
                "File@0..27",
                "  Func@0..27",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..27",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..26",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ExprMember@16..25",
                "          ExprName@16..19",
                "            Identifier@16..19 \"obj\"",
                "          Dot@19..20 \".\"",
                "          Identifier@20..25 \"field\"",
                "        Semicolon@25..26 \";\"",
                "      RightBrace@26..27 \"}\"",
            ],
        )
    }

    #[test]
    fn test_array_access() {
        expect_tree(
            "func test() { val x = arr[0]; }",
            &[
                "File@0..24",
                "  Func@0..24",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..24",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..23",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ExprIndex@16..22",
                "          ExprName@16..19",
                "            Identifier@16..19 \"arr\"",
                "          LeftBracket@19..20 \"[\"",
                "          ExprLiteral@20..21",
                "            DecIntLiteral@20..21 \"0\"",
                "          RightBracket@21..22 \"]\"",
                "        Semicolon@22..23 \";\"",
                "      RightBrace@23..24 \"}\"",
            ],
        )
    }

    #[test]
    fn test_function_call() {
        expect_tree(
            "func test() { val x = foo(1, 2); }",
            &[
                "File@0..26",
                "  Func@0..26",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..26",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..25",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ExprCall@16..24",
                "          ExprName@16..19",
                "            Identifier@16..19 \"foo\"",
                "          ArgList@19..24",
                "            LeftParen@19..20 \"(\"",
                "            ExprLiteral@20..21",
                "              DecIntLiteral@20..21 \"1\"",
                "            Comma@21..22 \",\"",
                "            ExprLiteral@22..23",
                "              DecIntLiteral@22..23 \"2\"",
                "            RightParen@23..24 \")\"",
                "        Semicolon@24..25 \";\"",
                "      RightBrace@25..26 \"}\"",
            ],
        )
    }

    #[test]
    fn test_chained_postfix() {
        expect_tree(
            "func test() { val x = obj.method().field[0]; }",
            &[
                "File@0..39",
                "  Func@0..39",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..39",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..38",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ExprIndex@16..37",
                "          ExprMember@16..34",
                "            ExprCall@16..28",
                "              ExprMember@16..26",
                "                ExprName@16..19",
                "                  Identifier@16..19 \"obj\"",
                "                Dot@19..20 \".\"",
                "                Identifier@20..26 \"method\"",
                "              ArgList@26..28",
                "                LeftParen@26..27 \"(\"",
                "                RightParen@27..28 \")\"",
                "            Dot@28..29 \".\"",
                "            Identifier@29..34 \"field\"",
                "          LeftBracket@34..35 \"[\"",
                "          ExprLiteral@35..36",
                "            DecIntLiteral@35..36 \"0\"",
                "          RightBracket@36..37 \"]\"",
                "        Semicolon@37..38 \";\"",
                "      RightBrace@38..39 \"}\"",
            ],
        )
    }

    fn test_files_in_directory_parse(dir: &str, dir_should_error: Vec<String>) {
        use std::fs;
        use std::path::Path;

        let test_dir = Path::new(dir);

        if !test_dir.exists() {
            println!("Directory not found, skipping test: {}", dir);
            return;
        }

        let expect_err =
            dir_should_error.contains(&test_dir.file_name().unwrap().to_str().unwrap().to_owned());

        let entries = fs::read_dir(test_dir).expect(&format!("Failed to read directory: {}", dir));

        for entry in entries {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("aria") {
                let filename = path.file_name().unwrap().to_str().unwrap();

                println!("Parsing {}", filename);

                let content =
                    fs::read_to_string(&path).expect(&format!("Failed to read file: {}", filename));

                let parse_result = parse(&content);

                if expect_err && parse_result.errors.is_empty() {
                    println!("\n{} should have parse errors but did not:", filename);
                    let line_index = LineIndex::new(&content);
                    for error in &parse_result.errors {
                        let line = line_index
                            .line_col(error.pos.as_ref().unwrap().start.try_into().unwrap());
                        println!("  {:?} at {:?}", error, line);
                    }

                    println!("\n\n{}", tree_to_string(parse_result.syntax()));

                    panic!("Parse errors found in {}", filename);
                } else if !expect_err && !parse_result.errors.is_empty() {
                    println!("\n{} has parse errors:", filename);
                    let line_index = LineIndex::new(&content);
                    for error in &parse_result.errors {
                        let line = line_index
                            .line_col(error.pos.as_ref().unwrap().start.try_into().unwrap());
                        println!("  {:?} at {:?}", error, line);
                    }

                    println!("\n\n{}", tree_to_string(parse_result.syntax()));

                    panic!("Parse errors found in {}", filename);
                }
            }

            if path.is_dir() {
                test_files_in_directory_parse(
                    path.to_path_buf().to_str().unwrap(),
                    dir_should_error.clone(),
                );
            }
        }
    }

    #[test]
    fn test_assignment_basic() {
        expect_tree(
            "func test() { x = 5; }",
            &[
                "File@0..16",
                "  Func@0..16",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..16",
                "      LeftBrace@10..11 \"{\"",
                "      StmtExpr@11..15",
                "        ExprAssign@11..14",
                "          ExprName@11..12",
                "            Identifier@11..12 \"x\"",
                "          Assign@12..13 \"=\"",
                "          ExprLiteral@13..14",
                "            DecIntLiteral@13..14 \"5\"",
                "        Semicolon@14..15 \";\"",
                "      RightBrace@15..16 \"}\"",
            ],
        )
    }

    #[test]
    fn test_assignment_compound() {
        expect_tree(
            "func test() { x += 5; }",
            &[
                "File@0..17",
                "  Func@0..17",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..17",
                "      LeftBrace@10..11 \"{\"",
                "      StmtExpr@11..16",
                "        ExprAssign@11..15",
                "          ExprName@11..12",
                "            Identifier@11..12 \"x\"",
                "          PlusAssign@12..14 \"+=\"",
                "          ExprLiteral@14..15",
                "            DecIntLiteral@14..15 \"5\"",
                "        Semicolon@15..16 \";\"",
                "      RightBrace@16..17 \"}\"",
            ],
        )
    }

    #[test]
    fn test_assignment_all_compound_operators() {
        expect_tree(
            "func test() { x -= 1; y *= 2; z /= 3; w %= 4; }",
            &[
                "File@0..32",
                "  Func@0..32",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..32",
                "      LeftBrace@10..11 \"{\"",
                "      StmtExpr@11..16",
                "        ExprAssign@11..15",
                "          ExprName@11..12",
                "            Identifier@11..12 \"x\"",
                "          MinusAssign@12..14 \"-=\"",
                "          ExprLiteral@14..15",
                "            DecIntLiteral@14..15 \"1\"",
                "        Semicolon@15..16 \";\"",
                "      StmtExpr@16..21",
                "        ExprAssign@16..20",
                "          ExprName@16..17",
                "            Identifier@16..17 \"y\"",
                "          StarAssign@17..19 \"*=\"",
                "          ExprLiteral@19..20",
                "            DecIntLiteral@19..20 \"2\"",
                "        Semicolon@20..21 \";\"",
                "      StmtExpr@21..26",
                "        ExprAssign@21..25",
                "          ExprName@21..22",
                "            Identifier@21..22 \"z\"",
                "          SlashAssign@22..24 \"/=\"",
                "          ExprLiteral@24..25",
                "            DecIntLiteral@24..25 \"3\"",
                "        Semicolon@25..26 \";\"",
                "      StmtExpr@26..31",
                "        ExprAssign@26..30",
                "          ExprName@26..27",
                "            Identifier@26..27 \"w\"",
                "          PercentAssign@27..29 \"%=\"",
                "          ExprLiteral@29..30",
                "            DecIntLiteral@29..30 \"4\"",
                "        Semicolon@30..31 \";\"",
                "      RightBrace@31..32 \"}\"",
            ],
        )
    }

    #[test]
    fn test_assignment_precedence() {
        expect_tree(
            "func test() { x = y + z; }",
            &[
                "File@0..18",
                "  Func@0..18",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..18",
                "      LeftBrace@10..11 \"{\"",
                "      StmtExpr@11..17",
                "        ExprAssign@11..16",
                "          ExprName@11..12",
                "            Identifier@11..12 \"x\"",
                "          Assign@12..13 \"=\"",
                "          ExprBinary@13..16",
                "            ExprName@13..14",
                "              Identifier@13..14 \"y\"",
                "            Plus@14..15 \"+\"",
                "            ExprName@15..16",
                "              Identifier@15..16 \"z\"",
                "        Semicolon@16..17 \";\"",
                "      RightBrace@17..18 \"}\"",
            ],
        )
    }

    #[test]
    fn test_assignment_right_associative() {
        expect_tree(
            "func test() { x = y = z; }",
            &[
                "File@0..18",
                "  Func@0..18",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..18",
                "      LeftBrace@10..11 \"{\"",
                "      StmtExpr@11..17",
                "        ExprAssign@11..16",
                "          ExprName@11..12",
                "            Identifier@11..12 \"x\"",
                "          Assign@12..13 \"=\"",
                "          ExprAssign@13..16",
                "            ExprName@13..14",
                "              Identifier@13..14 \"y\"",
                "            Assign@14..15 \"=\"",
                "            ExprName@15..16",
                "              Identifier@15..16 \"z\"",
                "        Semicolon@16..17 \";\"",
                "      RightBrace@17..18 \"}\"",
            ],
        )
    }

    #[test]
    fn test_simple_binary_expr() {
        expect_tree(
            "func test() { val x = y-1; }",
            &[
                "File@0..21",
                "  Func@0..21",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..21",
                "      LeftBrace@10..11 \"{\"",
                "      StmtVal@11..20",
                "        ValKwd@11..14 \"val\"",
                "        Identifier@14..15 \"x\"",
                "        Assign@15..16 \"=\"",
                "        ExprBinary@16..19",
                "          ExprName@16..17",
                "            Identifier@16..17 \"y\"",
                "          Minus@17..18 \"-\"",
                "          ExprLiteral@18..19",
                "            DecIntLiteral@18..19 \"1\"",
                "        Semicolon@19..20 \";\"",
                "      RightBrace@20..21 \"}\"",
            ],
        )
    }

    #[test]
    fn test_assert_statement() {
        expect_tree(
            "func test() { assert x > 0; }",
            &[
                "File@0..22",
                "  Func@0..22",
                "    FuncKwd@0..4 \"func\"",
                "    Identifier@4..8 \"test\"",
                "    ParamList@8..10",
                "      LeftParen@8..9 \"(\"",
                "      RightParen@9..10 \")\"",
                "    Block@10..22",
                "      LeftBrace@10..11 \"{\"",
                "      StmtAssert@11..21",
                "        AssertKwd@11..17 \"assert\"",
                "        ExprBinary@17..20",
                "          ExprName@17..18",
                "            Identifier@17..18 \"x\"",
                "          Greater@18..19 \">\"",
                "          ExprLiteral@19..20",
                "            DecIntLiteral@19..20 \"0\"",
                "        Semicolon@20..21 \";\"",
                "      RightBrace@21..22 \"}\"",
            ],
        )
    }

    #[test]
    fn test_example_files_parse_without_errors() {
        test_files_in_directory_parse("../examples", vec![]);
    }

    #[test]
    fn test_files_parse_without_errors() {
        test_files_in_directory_parse("../tests", vec![]);
    }

    #[test]
    fn test_std_lib_files_parse_without_errors() {
        test_files_in_directory_parse("../lib", vec![]);
    }

    #[test]
    fn test_std_lib_test_files_parse_without_errors() {
        test_files_in_directory_parse("../lib-test", vec!["with_err".to_string()]);
    }
}
