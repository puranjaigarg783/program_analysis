use super::*;

use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;

#[derive(pest_derive::Parser)]
#[grammar_inline = r#"
WHITESPACE = _{ " " }
COMMENT = _{ "//" ~ (!NEWLINE ~ ANY)* ~ &NEWLINE }

constraints = { SOI ~ (constraint ~ NEWLINE)+ ~ EOI }

constraint = { constraint_expr ~ "<=" ~ constraint_expr }

constraint_expr = { reff | lam | proj | var }

var = { ( ASCII_ALPHANUMERIC+ ~ "." )? ~ "_"? ~ ASCII_ALPHANUMERIC+ }
reff = { "ref(" ~ var ~ "," ~ var ~ ")" }
proj = { "proj(ref,1," ~ var ~ ")" }
lam = { "lam_[(" ~ type_arr? ~ ")->" ~ ret_type ~ "](" ~ var_arr? ~ ")" }

ret_type = { type | "_" }

type_arr = { type ~ ("," ~ type)* }
var_arr = { var ~ ("," ~ var)* }
type = { "&"* ~ ASCII_ALPHANUMERIC+ }
"#]
struct ConstraintParser;

use derive_more::Display;
#[derive(Clone, Debug, Display, Eq, PartialEq)]
pub enum Errors {
    Parse(Error<Rule>),
    ContextSensitive(String),
}

impl std::str::FromStr for Constraints {
    type Err = Errors;

    fn from_str(prog_str: &str) -> Result<Self, Self::Err> {
        match ConstraintParser::parse(Rule::constraints, prog_str) {
            Ok(mut parse_tree) => create_constraints(parse_tree.next().unwrap()),
            Err(err) => Err(Errors::Parse(err)),
        }
    }
}

fn create_constraints(parse_tree: Pair<Rule>) -> Result<Constraints, Errors> {
    let mut constraints_set: Set<Constraint> = Set::new();

    for constraint in parse_tree.into_inner() {
        let mut inner = constraint.into_inner();
        if inner.len() == 2 {
            match (inner.next().clone(), inner.next().clone()) {
                (Some(expr1), Some(expr2)) => {
                    // dbg!(expr1.as_str().trim(), expr2.as_str().trim());

                    constraints_set.insert(Constraint(
                        parse_expr(expr1),
                        parse_expr(expr2)
                    ));
                    //println!("{} <= {}", expr1.as_str(), expr2.as_str());
                },
                _ => unreachable!("constraint pair doesn't have two constraints")
            }
        }
    }

    Ok(Constraints(constraints_set))
}

fn parse_expr(expr: Pair<Rule>) -> ConstraintExp {
    let expr = expr.into_inner().next().unwrap();
    match expr.as_rule() {
        Rule::var => parse_var(expr),
        Rule::proj => parse_proj(expr),
        Rule::reff => parse_ref(expr),
        Rule::lam => parse_lam(expr),
        _ => unreachable!("not a constraintexpr: {:#?}", expr),
    }
}

// giving everything &int type
fn dummy_var(str: &str) -> VarId {
    var_id(str.trim(), ptr_ty(int_ty()), None)
}

fn parse_var(expr: Pair<Rule>) -> ConstraintExp {
    ConstraintExp::Var(dummy_var(expr.as_str()))
}

fn parse_proj(expr: Pair<Rule>) -> ConstraintExp {
    let expr = expr.into_inner().next().clone().unwrap();
    ConstraintExp::Proj(dummy_var(expr.as_str()))
}

fn parse_ref(expr: Pair<Rule>) -> ConstraintExp {
    let expr = expr.into_inner().next().clone().unwrap();
    ConstraintExp::Ref(dummy_var(expr.as_str()), dummy_var(expr.as_str()))
}

fn parse_lam(expr: Pair<Rule>) -> ConstraintExp {
    let mut expr = expr.into_inner();
    let type_arr = expr.next().clone().unwrap();
    let ret_ty = expr.next().clone().unwrap();
    let var_arr = expr.next().clone().unwrap();

    //let mut param_ty: Vec<Type> = todo!();
    //let mut args: Vec<VarId> = todo!();

    ConstraintExp::LamSimple {
        params: type_arr.as_str().trim().to_owned(),
        ret_ty: ret_ty.as_str().trim().to_owned(),
        args: var_arr.as_str().trim().to_owned(),
    }
}