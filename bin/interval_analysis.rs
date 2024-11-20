
use optimization::middle_end::{analysis::*, lir::*, analysis::integer_interval::Env};
use optimization::middle_end::optimization::constant_prop::*;
use optimization::commons::Valid;
use std::collections::BTreeMap as Map;
use pretty_assertions::assert_eq;
use std::env;

// cheesing the assignment might not have been worth it i should just write a parser

pub fn run() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        panic!("must have at least 3 args");
    }

    let lir_file_name = &args[1];
    let json_file_name = &args[2];
    let function_name = &args[3];

    let output = constants_analysis_lir(lir_file_name, json_file_name, function_name);
    println!("{output}");
}
fn main() {

    run();

}

fn constants_analysis_lir(lir_file_name: &str, json_file_name: &str, function_name: &str) -> String {
    let input_string = read_from(lir_file_name);
    
    let lir_parsed = parse_lir(&input_string);
    let analyzed = integer_interval::analyze(&lir_parsed, func_id(function_name));
    
    nicely(&analyzed.1)
}

fn read_from(path: &str) -> String {
    String::from_utf8(
        std::fs::read(path)
            .unwrap_or_else(|_| panic!("Could not read the input file {}", path)),
    )
    .expect("The input file does not contain valid utf-8 text")
}

fn nicely(result: &Map<InstId, Env>) -> String {
    
    // this is very lazy and bad haha !
    let mut s = String::new();
    let mut c: Map<BbId, Env> = Map::new();
    for ((bb, n), env) in result {
        c.insert(bb.clone(), env.clone());
    }
    for (bb, env) in c {
        s += &bb.to_string();
        s += ":\n";
        s += &env.to_string();
        s += "\n";
    }
    s
}

fn parse_lir(input: &str) -> Valid<Program> {
    input.parse::<Program>().unwrap().validate().unwrap()
}