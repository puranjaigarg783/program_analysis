

use optimization::middle_end::{analysis::*, lir::*, control_analysis::control::Env};
use optimization::middle_end::{control_analysis::control::{analyze, analyze2}};

use optimization::commons::Valid;
use std::collections::{BTreeMap as Map, BTreeSet as Set};
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

    let output = dominance_lir(lir_file_name, json_file_name, function_name);
    println!("{output}");
}

pub fn run_test() {

    let lir_file_name = "./test-inputs-02/complex/binary_trees.lir";
    let json_file_name = "./test-inputs-02/complex/binary_trees.lir.json";
    let function_name = "main";

    let output = dominance_lir(lir_file_name, json_file_name, function_name);
    println!("{output}");
}
fn main() {

    run();

}

fn dominance_lir(lir_file_name: &str, json_file_name: &str, function_name: &str) -> String {
    let input_string = read_from(lir_file_name);
    
    let lir_parsed = parse_lir(&input_string);
    let analyzed = analyze(&lir_parsed, func_id(function_name));
    
    nicely2(&analyzed)
}

fn read_from(path: &str) -> String {
    String::from_utf8(
        std::fs::read(path)
            .unwrap_or_else(|_| panic!("Could not read the input file {}", path)),
    )
    .expect("The input file does not contain valid utf-8 text")
}

fn nicely2(result: &Map<BbId, Set<BbId>>) -> String {
    let mut s = String::new();
    for (bb, env) in result {
        s += &bb.to_string();
        s += " -> {";
        for bbid in env {
            s += &bbid.to_string();
            s += ",";
        }
        if s.pop() == Some('{') {
            s.push('{');
        };
        s += "}\n";
    }
    s += "\n";
    s
}

fn nicely(result: &Map<BbId, Env>) -> String {
    // this is very lazy and bad haha !
    let mut s = String::new();
    for (bb, env) in result {
        
        s += &env.to_string();
    }
    s
}

fn parse_lir(input: &str) -> Valid<Program> {
    input.parse::<Program>().unwrap().validate().unwrap()
}