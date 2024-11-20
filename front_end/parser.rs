// ll(1) parser for cflat.
//
// You are free to change any function or type signature except for `parse` and
// `ParseError`.

use derive_more::Display;

use super::*;
use TokenKind::*;

// SECTION: interface

pub fn parse(code: &str) -> Result<Program, ParseError> {
    let mut parser = Parser::new(code)?;
    program_r(&mut parser)
}

// A parse error with explanatory message.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
pub struct ParseError(pub String);
impl std::error::Error for ParseError {}

// SECTION: parser functionality

#[derive(Clone, Debug)]
struct Parser<'a> {
    code: &'a str,      // the source code being parsed
    tokens: Vec<Token>, // the token stream
    pos: usize,         // the position in the token stream
}

// utility functions for traversing the token stream and creating error
// messages.
impl<'a> Parser<'a> {
    // always use this to create new Parsers.
    fn new(code: &'a str) -> Result<Self, ParseError> {
        let tokens = lex(code);
        if tokens.is_empty() {
            Err(ParseError("empty token stream".to_string()))
        } else {
            Ok(Parser {
                code,
                tokens,
                pos: 0,
            })
        }
    }

    // if the next token has the given kind advances the iterator and returns true,
    // otherwise returns false.
    fn eat(&mut self, kind: TokenKind) -> bool {
        match self.peek() {
            Some(k) if k == kind => {
                self.next();
                true
            }
            _ => false,
        }
    }

    // returns an Ok or Err result depending on whether the next token has the given
    // kind, advancing the iterator on an Ok result.
    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.eat(kind) {
            Ok(())
        } else {
            self.error_next(&format!("expected `{kind}`"))
        }
    }

    // advances the iterator and returns the next token in the stream, or None if
    // there are no more tokens.
    fn next(&mut self) -> Option<TokenKind> {
        if !self.end() {
            self.pos += 1;
            Some(self.tokens[self.pos - 1].kind)
        } else {
            None
        }
    }

    // returns the next token (if it exists) without advancing the iterator.
    fn peek(&self) -> Option<TokenKind> {
        if !self.end() {
            Some(self.tokens[self.pos].kind)
        } else {
            None
        }
    }

    // returns whether the next token has the given kind, without advancing the
    // iterator.
    fn next_is(&self, kind: TokenKind) -> bool {
        self.peek() == Some(kind)
    }

    // returns whether the next token is one of the given kinds.
    fn next_is_one_of(&self, kinds: &[TokenKind]) -> bool {
        matches!(self.peek(), Some(k) if kinds.contains(&k))
    }

    // returns whether we're at the end of the token stream.
    fn end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    // returns the lexeme of the token immediately prior to the current token.
    fn slice_prev(&self) -> &str {
        &self.code[self.tokens[self.pos - 1].span.clone()]
    }

    // returns a parse error knowing that the next token to be inspected causes an
    // error (based on a call to peek(), next_is(), etc).
    fn error_next<T>(&self, msg: &str) -> Result<T, ParseError> {
        // handle the case where we're at the end of the token stream.
        if self.pos >= self.tokens.len() {
            Err(ParseError(format!(
                "parse error: unexpected end of input ({msg})\n"
            )))
        } else {
            self.error(self.pos, msg)
        }
    }

    // constructs a parse error given the position of the error-causing token in the
    // token stream.
    fn error<T>(&self, pos: usize, msg: &str) -> Result<T, ParseError> {
        // the position of the error-causing lexeme in the source code.
        let span = &self.tokens[pos].span;

        // the row number and the index of the start of the row containing the
        // error-causing token.
        let (row, row_start) = {
            let mut row = 0;
            let mut row_start = 0;
            for (idx, _) in self.code.match_indices('\n') {
                if idx > span.start {
                    break;
                }
                row += 1;
                row_start = idx + 1;
            }
            (row, row_start)
        };

        // the column where the error-causing lexeme starts.
        let col = span.start - row_start;

        // the line containing the error-causing lexeme.
        let line = self.code.lines().nth(row).unwrap();

        Err(ParseError(format!(
            "parse error in line {row}, column {col}\n{line}\n{:width$}^\n{msg}\n",
            " ",
            width = col
        )))
    }
}

// SECTION: parsing functions

// the function names come from the production rules of the LL(1) cflat grammar.

// type.
fn type_r(parser: &mut Parser) -> Result<Type, ParseError> {
    if parser.eat(Address) {
        Ok(ptr_ty(type_r(parser)?))
    } else {
        type_ad_r(parser)
    }
}

// non-pointer type.
fn type_ad_r(parser: &mut Parser) -> Result<Type, ParseError> {
    if parser.eat(Int) {
        Ok(int_ty())
    } else if parser.eat(Id) {
        Ok(struct_ty(struct_id(parser.slice_prev())))
    } else if parser.eat(OpenParen) {
        Ok(type_op_r(parser)?)
    } else {
        // CATCH REST
        parser.error_next("type_ad_r: expected: Int, Id, OpenParen")
    }
}

// type in parentheses OR function type.
fn type_op_r(parser: &mut Parser) -> Result<Type, ParseError> {
    if parser.eat(CloseParen) {
        Ok(func_ty(type_ar_r(parser)?, vec![]))
    } else {
        let first_type = type_r(parser)?;
        if let Some((mut type_vec, ret)) = type_fp_r(parser)? {
            type_vec.insert(0, first_type);
            Ok(func_ty(ret, type_vec))
        } else {
            Ok(first_type)
        }
    }
}

// type in parentheses OR function type.
#[allow(clippy::type_complexity)]
fn type_fp_r(parser: &mut Parser) -> Result<Option<(Vec<Type>, Option<Type>)>, ParseError> {
    let mut return_vec: Vec<Type> = vec![];

    if parser.eat(CloseParen) {
        if parser.next_is(Arrow) {
            Ok(Some((return_vec, type_ar_r(parser)?)))
        } else {
            Ok(None)
        }
    } else if parser.eat(Comma) {
        loop {
            return_vec.push(type_r(parser)?);
            if !parser.eat(Comma) {
                break;
            }
        }
        parser.expect(CloseParen)?;
        Ok(Some((return_vec, type_ar_r(parser)?)))
    } else {
        parser.error_next("expected: CloseParen, Comma")
    }
}

// function return type.
fn type_ar_r(parser: &mut Parser) -> Result<Option<Type>, ParseError> {
    parser.expect(Arrow)?;
    rettyp_r(parser)
}

// function type.
fn funtype_r(parser: &mut Parser) -> Result<Type, ParseError> {
    parser.expect(OpenParen)?;
    let mut type_vec: Vec<Type> = vec![];

    if parser.next_is_one_of(&[Address, Int, Id, OpenParen]) {
        loop {
            type_vec.push(type_r(parser)?);

            if !parser.eat(Comma) {
                break;
            }
        }
    }

    parser.expect(CloseParen)?;
    parser.expect(Arrow)?;

    Ok(func_ty(rettyp_r(parser)?, type_vec))
}

// function return type.
fn rettyp_r(parser: &mut Parser) -> Result<Option<Type>, ParseError> {
    if parser.eat(Underscore) {
        Ok(None)
    } else {
        Ok(Some(type_r(parser)?))
    }
}

// cflat program.
fn program_r(parser: &mut Parser) -> Result<Program, ParseError> {
    let mut program = Program {
        globals: vec![],
        typedefs: vec![],
        externs: vec![],
        functions: vec![],
    };
    while parser.peek().is_some() {
        if parser.next_is(Let) {
            program.globals.extend(glob_r(parser)?);
        } else if parser.next_is(Struct) {
            program.typedefs.push(typedef_r(parser)?);
        } else if parser.next_is(Extern) {
            program.externs.push(extern_r(parser)?);
        } else if parser.next_is(Fn) {
            program.functions.push(fundef_r(parser)?);
        } else {
            parser.error_next("TOP LEVEL: expected: Let, Struct, Extern, Fn")?
        }
    }

    Ok(program)
}

// global variable declaration.
fn glob_r(parser: &mut Parser) -> Result<Vec<Decl>, ParseError> {
    parser.expect(Let)?;
    let ret_vec = decls_r(parser)?;
    parser.expect(Semicolon)?;

    Ok(ret_vec)
}

// struct type declaration.
fn typedef_r(parser: &mut Parser) -> Result<Typedef, ParseError> {
    parser.expect(Struct)?;
    parser.expect(Id)?;
    let id_name = parser.slice_prev().to_string();

    parser.expect(OpenBrace)?;
    let decls = decls_r(parser)?;

    parser.expect(CloseBrace)?;

    Ok(Typedef {
        name: id_name,
        fields: decls,
    })
}

// variable declaration.
fn decl_r(parser: &mut Parser) -> Result<Decl, ParseError> {
    parser.expect(Id)?;
    let id_name = parser.slice_prev().to_string();

    parser.expect(Colon)?;
    let type_var = type_r(parser)?;

    Ok(Decl {
        name: id_name,
        typ: type_var,
    })
}

// series of variable declarations.
fn decls_r(parser: &mut Parser) -> Result<Vec<Decl>, ParseError> {
    let mut decl_vec: Vec<Decl> = vec![];
    loop {
        decl_vec.push(decl_r(parser)?);

        if !parser.eat(Comma) {
            break;
        }
    }

    Ok(decl_vec)
}

// external function declaration.
fn extern_r(parser: &mut Parser) -> Result<Decl, ParseError> {
    parser.expect(Extern)?;
    parser.expect(Id)?;
    let id_name = parser.slice_prev().to_string();

    parser.expect(Colon)?;
    let funtype = funtype_r(parser)?;

    parser.expect(Semicolon)?;

    Ok(Decl {
        name: id_name,
        typ: funtype,
    })
}

// function definition.
fn fundef_r(parser: &mut Parser) -> Result<Function, ParseError> {
    parser.expect(Fn)?;
    parser.expect(Id)?;
    let name = parser.slice_prev().to_string();

    parser.expect(OpenParen)?;
    let mut params = vec![];
    if !parser.next_is(CloseParen) {
        params = decls_r(parser)?;
    }

    parser.expect(CloseParen)?;
    parser.expect(Arrow)?;

    let rettyp = rettyp_r(parser)?;

    parser.expect(OpenBrace)?;

    let mut decls = vec![];
    while parser.next_is(Let) {
        decls.extend(let_r(parser)?);
    }

    let mut stmts = vec![stmt_r(parser)?];

    while parser.next_is_one_of(&[
        If,    // FIRST(cond)
        While, // FIRST(loop)
        Star,  // FIRST(assign_or_call)
        Id,    // FIRST(assign_or_call)
        Break, Continue, Return,
    ]) {
        stmts.push(stmt_r(parser)?);
    }

    parser.expect(CloseBrace)?;

    let body = Body { decls, stmts };

    Ok(Function {
        name,
        params,
        rettyp,
        body,
    })
}

// internal variable declaration and possibly initialization.
fn let_r(parser: &mut Parser) -> Result<Vec<(Decl, Option<Exp>)>, ParseError> {
    parser.expect(Let)?;

    let mut decl_vec = vec![];
    loop {
        let decl = decl_r(parser)?;
        let mut exp = None;
        if parser.eat(Gets) {
            exp = Some(exp_r(parser)?);
        }

        decl_vec.push((decl, exp));

        if !parser.eat(Comma) {
            break;
        }
    }

    parser.expect(Semicolon)?;
    Ok(decl_vec)
}

// statement.
fn stmt_r(parser: &mut Parser) -> Result<Stmt, ParseError> {
    if parser.next_is(If) {
        Ok(cond_r(parser)?)
    } else if parser.next_is(While) {
        Ok(loop_r(parser)?)
    } else if parser.next_is_one_of(&[Star, Id]) {
        let stmt = assign_or_call_r(parser)?;
        parser.expect(Semicolon)?;

        Ok(stmt)
    } else if parser.eat(Break) {
        parser.expect(Semicolon)?;

        Ok(Stmt::Break)
    } else if parser.eat(Continue) {
        parser.expect(Semicolon)?;

        Ok(Stmt::Continue)
    } else if parser.eat(Return) {
        let mut ret_exp = None;

        if parser.next_is_one_of(&[Star, Dash, Bang, Num, Nil, OpenParen, Id]) {
            ret_exp = Some(exp_r(parser)?);
        }

        parser.expect(Semicolon)?;
        Ok(Stmt::Return(ret_exp))
    } else {
        parser.error_next("expected: If, While, Star, Id, Break, Continue, Return")
    }
}

// conditional statement.
fn cond_r(parser: &mut Parser) -> Result<Stmt, ParseError> {
    parser.expect(If)?;
    let cond_exp = exp_r(parser)?;
    let tt = block_r(parser)?;
    let mut ff = vec![];

    if parser.eat(Else) {
        ff = block_r(parser)?;
    }

    Ok(Stmt::If {
        guard: cond_exp,
        tt,
        ff,
    })
}

// while or for loop.
fn loop_r(parser: &mut Parser) -> Result<Stmt, ParseError> {
    parser.expect(While)?;
    let loop_exp = exp_r(parser)?;
    let body = block_r(parser)?;

    Ok(Stmt::While {
        guard: loop_exp,
        body,
    })
}

// sequence of statements.
fn block_r(parser: &mut Parser) -> Result<Vec<Stmt>, ParseError> {
    parser.expect(OpenBrace)?;
    let mut stmt_vec = vec![];

    while parser.next_is_one_of(&[
        If,    // FIRST(cond)
        While, // FIRST(loop)
        Star,  // FIRST(assign_or_call)
        Id,    // FIRST(assign_or_call)
        Break, Continue, Return,
    ]) {
        stmt_vec.push(stmt_r(parser)?);
    }

    parser.expect(CloseBrace)?;

    Ok(stmt_vec)
}

// assignment or call statement.
fn assign_or_call_r(parser: &mut Parser) -> Result<Stmt, ParseError> {
    let lval = lval_r(parser)?;
    if parser.eat(Gets) {
        Ok(Stmt::Assign {
            lhs: lval,
            rhs: rhs_r(parser)?,
        })
    } else if parser.eat(OpenParen) {
        let mut args = vec![];
        if parser.next_is_one_of(&[Star, Dash, Bang, Num, Nil, OpenParen, Id]) {
            args = args_r(parser)?
        }
        parser.expect(CloseParen)?;

        Ok(Stmt::Call { callee: lval, args })
    } else {
        parser.error_next("assign_or_call_r: expected: Gets, OpenParen")?
    }
}

// right-hand side of an assignment.
fn rhs_r(parser: &mut Parser) -> Result<Rhs, ParseError> {
    if parser.next_is_one_of(&[Star, Dash, Bang, Num, Nil, OpenParen, Id]) {
        Ok(Rhs::Exp(exp_r(parser)?))
    } else if parser.eat(New) {
        let typ = type_r(parser)?;
        let mut num = None;

        if parser.next_is_one_of(&[Star, Dash, Bang, Num, Nil, OpenParen, Id]) {
            num = Some(exp_r(parser)?);
        }

        Ok(Rhs::New { typ, num })
    } else {
        parser.error_next("rhs_r: expected: FIRST(Exp), New")
    }
}

// left-hand side of an assignment.
fn lval_r(parser: &mut Parser) -> Result<Lval, ParseError> {
    if parser.eat(Star) {
        Ok(Lval::Deref(Box::new(lval_r(parser)?)))
    } else if parser.eat(Id) {
        let mut base_lval = Lval::Id(parser.slice_prev().to_string());
        while parser.next_is_one_of(&[OpenBracket, Dot]) {
            base_lval = access_r(parser, base_lval)?;
        }

        Ok(base_lval)
    } else {
        parser.error_next("lval_r: expected: Star, Id")
    }
}

// access path.
fn access_r(parser: &mut Parser, base: Lval) -> Result<Lval, ParseError> {
    if parser.next_is(OpenBracket) {
        parser.expect(OpenBracket)?;
        let index = exp_r(parser)?;
        parser.expect(CloseBracket)?;

        Ok(Lval::ArrayAccess {
            ptr: Box::new(base),
            index,
        })
    } else if parser.next_is(Dot) {
        parser.expect(Dot)?;
        parser.expect(Id)?;
        let field = parser.slice_prev().to_string();

        Ok(Lval::FieldAccess {
            ptr: Box::new(base),
            field,
        })
    } else {
        parser.error_next("exp_ac_r: expected: OpenBracket, Dot")
    }
}

// call arguments.
fn args_r(parser: &mut Parser) -> Result<Vec<Exp>, ParseError> {
    let mut exp_vec = vec![exp_r(parser)?];

    while parser.eat(Comma) {
        exp_vec.push(exp_r(parser)?);
    }

    Ok(exp_vec)
}

// expression (precedence level 6).
fn exp_r(parser: &mut Parser) -> Result<Exp, ParseError> {
    let mut base_exp = exp_p5_r(parser)?;

    while parser.next_is_one_of(&[And, Or]) {
        if parser.eat(And) {
            base_exp = Exp::And(Box::new(base_exp), Box::new(exp_r(parser)?))
        } else if parser.eat(Or) {
            base_exp = Exp::Or(Box::new(base_exp), Box::new(exp_r(parser)?))
        } else {
            parser.error_next("this should literally never happen, something is really wrong")?
        }
    }
    Ok(base_exp)
}

// expression (precedence level 5).
fn exp_p5_r(parser: &mut Parser) -> Result<Exp, ParseError> {
    let mut base_exp = exp_p4_r(parser)?;

    while parser.next_is_one_of(&[Equal, NotEq, Lt, Lte, Gt, Gte]) {
        if parser.eat(Equal) {
            base_exp = Exp::Compare(
                Box::new(base_exp),
                CompareOp::Equal,
                Box::new(exp_p4_r(parser)?),
            );
        } else if parser.eat(NotEq) {
            base_exp = Exp::Compare(
                Box::new(base_exp),
                CompareOp::NotEq,
                Box::new(exp_p4_r(parser)?),
            );
        } else if parser.eat(Lt) {
            base_exp = Exp::Compare(
                Box::new(base_exp),
                CompareOp::Lt,
                Box::new(exp_p4_r(parser)?),
            );
        } else if parser.eat(Lte) {
            base_exp = Exp::Compare(
                Box::new(base_exp),
                CompareOp::Lte,
                Box::new(exp_p4_r(parser)?),
            );
        } else if parser.eat(Gt) {
            base_exp = Exp::Compare(
                Box::new(base_exp),
                CompareOp::Gt,
                Box::new(exp_p4_r(parser)?),
            );
        } else if parser.eat(Gte) {
            base_exp = Exp::Compare(
                Box::new(base_exp),
                CompareOp::Gte,
                Box::new(exp_p4_r(parser)?),
            );
        } else {
            parser.error_next("this should literally never happen, something is really wrong")?
        }
    }

    Ok(base_exp)
}

// expression (precedence level 4).
fn exp_p4_r(parser: &mut Parser) -> Result<Exp, ParseError> {
    let mut base_exp = exp_p3_r(parser)?;

    while parser.next_is_one_of(&[Plus, Dash]) {
        if parser.eat(Plus) {
            base_exp = Exp::Arith(
                Box::new(base_exp),
                ArithOp::Add,
                Box::new(exp_p3_r(parser)?),
            );
        } else if parser.eat(Dash) {
            base_exp = Exp::Arith(
                Box::new(base_exp),
                ArithOp::Subtract,
                Box::new(exp_p3_r(parser)?),
            );
        } else {
            parser.error_next("this should literally never happen, something is really wrong")?
        }
    }

    Ok(base_exp)
}

// expression (precedence level 3).
fn exp_p3_r(parser: &mut Parser) -> Result<Exp, ParseError> {
    let mut base_exp = exp_p2_r(parser)?;

    while parser.next_is_one_of(&[Star, Slash]) {
        if parser.eat(Star) {
            base_exp = Exp::Arith(
                Box::new(base_exp),
                ArithOp::Multiply,
                Box::new(exp_p2_r(parser)?),
            );
        } else if parser.eat(Slash) {
            base_exp = Exp::Arith(
                Box::new(base_exp),
                ArithOp::Divide,
                Box::new(exp_p2_r(parser)?),
            );
        } else {
            parser.error_next("this should literally never happen, something is really wrong")?
        }
    }

    Ok(base_exp)
}

// expression (precedence level 2).
fn exp_p2_r(parser: &mut Parser) -> Result<Exp, ParseError> {
    if parser.eat(Star) {
        Ok(Exp::Deref(Box::new(exp_p2_r(parser)?)))
    } else if parser.eat(Dash) {
        Ok(Exp::Neg(Box::new(exp_p2_r(parser)?)))
    } else if parser.eat(Bang) {
        Ok(Exp::Not(Box::new(exp_p2_r(parser)?)))
    } else {
        Ok(exp_p1_r(parser)?)
    }
}

// expression (precedence level 1).
fn exp_p1_r(parser: &mut Parser) -> Result<Exp, ParseError> {
    if parser.eat(Num) {
        match parser.slice_prev().parse::<i32>() {
            Ok(num) => Ok(Exp::Num(num)),
            Err(_) => parser.error_next("exp_p1_r: could not parse int or int too large"),
        }
    } else if parser.eat(Nil) {
        Ok(Exp::Nil)
    } else if parser.eat(OpenParen) {
        let ret_exp = exp_r(parser)?;
        parser.expect(CloseParen)?;

        Ok(ret_exp)
    } else if parser.eat(Id) {
        let mut base_exp = Exp::Id(parser.slice_prev().to_string());

        while parser.next_is_one_of(&[OpenBracket, Dot, OpenParen]) {
            base_exp = exp_ac_r(parser, base_exp)?;
        }

        Ok(base_exp)
    } else {
        parser.error_next("exp_p1_r: expected: Num, Nil, OpenParen, Id")?
    }
}

fn exp_ac_r(parser: &mut Parser, base: Exp) -> Result<Exp, ParseError> {
    if parser.eat(OpenBracket) {
        let index = exp_r(parser)?;
        parser.expect(CloseBracket)?;

        Ok(Exp::ArrayAccess {
            ptr: Box::new(base),
            index: Box::new(index),
        })
    } else if parser.eat(Dot) {
        parser.expect(Id)?;
        let field = parser.slice_prev().to_string();

        Ok(Exp::FieldAccess {
            ptr: Box::new(base),
            field,
        })
    } else if parser.eat(OpenParen) {
        let mut args = vec![];

        if parser.next_is_one_of(&[Star, Dash, Bang, Num, Nil, OpenParen, Id]) {
            args = args_r(parser)?
        }
        parser.expect(CloseParen)?;

        Ok(Exp::Call {
            callee: Box::new(base),
            args,
        })
    } else {
        parser.error_next("exp_ac_r: expected: OpenBracket, Dot, OpenParen")
    }
}
