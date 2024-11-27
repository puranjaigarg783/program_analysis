use optimization::middle_end::{analysis::*, lir::*};
use optimization::middle_end::taint::taint_analysis::analyze;
use optimization::middle_end::taint::*;
use optimization::commons::Valid;
use std::collections::{BTreeMap as Map, BTreeSet as Set};
use pretty_assertions::assert_eq;
use std::env;

use optimization::middle_end::constraints::*;

// cheesing the assignment might not have been worth it i should just write a parser

pub fn run() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 5 {
        eprintln!("Usage: {} <lir_file> <json_file> <pts_to_file> <context-sensitivity>", args[0]);
        std::process::exit(1);
    }

    let lir_file_name = &args[1];
    let json_file_name = &args[2]; // Not used in your code but required for submission
    let pts_to_path = &args[3];
    let context_sensitivity = &args[4];

    // Since you're not using the function name from command-line, you can set it to "main" or adjust as needed
    let function_name = "main";

    // If you need to handle context sensitivity, you can pass `context_sensitivity` to `taint_lir`
    let output = taint_lir(lir_file_name, function_name, pts_to_path);

    println!("{output}");
}


pub fn run_test() {

    let lir_file_name = "./test-inputs-taint/tainted01.lir";
    let soln_path = "./test-inputs-taint/tainted01.lir.soln";
    let pts_to_path = "./test-inputs-taint/tainted01.lir.ptsto";

    let output = taint_lir(lir_file_name, "main", pts_to_path);
    println!("{output:#?}");
}

fn taint_lir(lir_file_name: &str, function_name: &str, pts_to_path: &str) -> String {
    let input_string = read_from(lir_file_name);
    let pts_to_str = read_from(pts_to_path);
    let pts_to = parse_pts_to(&pts_to_str);
    let lir_parsed = parse_lir(&input_string);
    let analyzed = analyze(&lir_parsed, func_id(function_name), pts_to);
    format!("{:?}", analyzed)
}

fn main() {

    run();

}

pub fn parse_pts_to(pts_to_str: &str) -> Map<String, Set<String>> {
    let mut pts_to_map: Map<String, Set<String>> = Map::new();
    for line in pts_to_str.split('\n') {
        let mut lr = line.split(" -> ");
        if let (Some(left), Some(right)) = (lr.next(), lr.next()) {
            let mut pts_to_set: Set<String> = Set::new();
            let mut right = right.to_string();
            right.remove(0);
            right.pop();

            for ptd_to_var in right.split(", ") {
                pts_to_set.insert(ptd_to_var.to_string());
            }

            pts_to_map.insert(left.to_string(), pts_to_set);
        }
    }
    pts_to_map
}

fn final_path(package: &str, path: &str) -> String{
    format!("{package}{path}")
}


fn lir_path(path: &str) -> String {
    format!("{path}.lir")
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