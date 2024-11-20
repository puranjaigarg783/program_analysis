use optimization::middle_end::{analysis::*, lir::*};
use optimization::middle_end::analysis_constraints::constraints_gen::{Env, analyze};
use optimization::middle_end::analysis_constraints::*;
use optimization::commons::Valid;
use std::collections::{BTreeMap as Map, BTreeSet as Set};
use pretty_assertions::assert_eq;
use std::env;

use optimization::middle_end::constraints::*;

// cheesing the assignment might not have been worth it i should just write a parser

pub fn run() {
    let args: Vec<String> = env::args().collect();

    let lir_file_name = &args[1];
    let json_file_name = &args[2];

    let output = constraints_gen_lir(lir_file_name, json_file_name);
    println!("{output}");
}

pub fn run_test() {

    let lir_file_name = "./test-inputs-03/gen/no_call3.lir";
    let json_file_name = "./test-inputs-03/gen/no_call3.lir.json";

    let lir_file_name = "./test-inputs-03/failed/02.lir";
    let json_file_name = "./test-inputs-03/failed/02.lir";

    let output = constraints_gen_lir(lir_file_name, json_file_name);
    println!("{output}");
}
fn main() {

    run();

}


fn final_path(package: &str, path: &str) -> String{
    format!("{package}{path}")
}


fn lir_path(path: &str) -> String {
    format!("{path}.lir")
}

fn constraints_gen_lir(lir_file_name: &str, json_file_name: &str) -> String {
    let input_string = read_from(lir_file_name);
    
    let lir_parsed = parse_lir(&input_string);

    let temp_set: Set<Constraint> = lir_parsed.0.functions.iter()
        .flat_map(|(funcid, _)| constraints_gen::analyze(&lir_parsed, funcid.clone()))
        .collect();

    nicely(&temp_set)
}

fn read_from(path: &str) -> String {
    String::from_utf8(
        std::fs::read(path)
            .unwrap_or_else(|_| panic!("Could not read the input file {}", path)),
    )
    .expect("The input file does not contain valid utf-8 text")
}

fn parse_lir(input: &str) -> Valid<Program> {
    input.parse::<Program>().unwrap().validate().unwrap()
}
    
fn nicely(result: &Set<Constraint>) -> String {
    use std::fmt::Write;

    result.iter().fold(String::new(), |mut output, b| {
        let _ = writeln!(output, "{b}");
        output
    })
}