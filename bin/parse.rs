// The compiler for CFlat code.

use ::optimization::front_end::*;

use std::io::Read;

pub fn main() {
    let mut input_string = String::new();
    std::io::stdin().read_to_string(&mut input_string).unwrap();

    let program: ast::Program =
        parse(&input_string).unwrap_or_else(|e| panic!("Syntax error: {e}"));
    let output = serde_json::to_string_pretty(&program).unwrap();

    println!("{output}");
}
