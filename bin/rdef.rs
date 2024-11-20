use optimization::middle_end::{analysis::*, lir::*};
use optimization::middle_end::{analysis_rdef::reaching_defs::{Env, analyze}};
use optimization::middle_end::analysis_rdef::ProgramPoint;
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

    let output = rdef_lir(lir_file_name, json_file_name, function_name);
    println!("{output}");
}

pub fn run_test() {

    let lir_file_name = "./test-inputs-02/complex/binary_trees.lir";
    let json_file_name = "./test-inputs-02/complex/binary_trees.lir.json";
    let function_name = "main";

    let lir_file_name = "./test-inputs-02/simple/call_ext.lir";
    let json_file_name = "./test-inputs-02/simple/call_ext.lir.json";
    let function_name = "main";

    let output = rdef_lir(lir_file_name, json_file_name, function_name);
    println!("{output}");
}
fn main() {

    run();

}

fn rdef_lir(lir_file_name: &str, json_file_name: &str, function_name: &str) -> String {
    let input_string = read_from(lir_file_name);
    
    let lir_parsed = parse_lir(&input_string);
    let analyzed = analyze(&lir_parsed, func_id(function_name));
    nicely_v3(&analyzed)
}

fn read_from(path: &str) -> String {
    String::from_utf8(
        std::fs::read(path)
            .unwrap_or_else(|_| panic!("Could not read the input file {}", path)),
    )
    .expect("The input file does not contain valid utf-8 text")
}

fn nicely_v2(result: &Map<ProgramPoint, Set<ProgramPoint>>) -> String {
    let mut s = String::new();
    for (bb, soln) in result {
        if soln.is_empty() {
            continue;
        }
        s += &bb.to_string();
        s += " -> {";
        for pp in soln {
            s += &format!("{pp}, ");
        }
        s.pop();
        s.pop();
        s += "}\n";
    }
    s += "\n";
    s
}

fn nicely_v3(result: &Map<ProgramPoint, Set<ProgramPoint>>) -> String {
    let mut output = result.iter()
        .filter(|(_, soln)| !soln.is_empty())
        .map(|(bb, soln)| {
            let soln_str = soln.iter()
                .map(|pp| pp.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{bb} -> {{{soln_str}}}\n")
        })
        .collect::<Vec<_>>()
        .join("");

    if output.is_empty() {
        output += "\n";
    }

    output
}


fn parse_lir(input: &str) -> Valid<Program> {
    input.parse::<Program>().unwrap().validate().unwrap()
}