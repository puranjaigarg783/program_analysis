use optimization::middle_end::{analysis::*, lir::*};
use optimization::middle_end::slice::slice_solve::{slice, slice_ptrs};
use optimization::middle_end::slice::*;
use optimization::commons::Valid;
use std::collections::{BTreeMap as Map, BTreeSet as Set};
use pretty_assertions::assert_eq;
use std::env;

use optimization::middle_end::constraints::*;

// cheesing the assignment might not have been worth it i should just write a parser

pub fn run() {
    let args: Vec<String> = env::args().collect();

    let lir_file_name = &args[1];
    let mut target = args[3].split('#'); // <function>#<basicblock>#{<index> | term}
    let pts_to_path = &args[4];
    let function = target.next().unwrap();
    let basicblock = target.next().unwrap();
    let idx = target.next().unwrap();

    let idx = match idx {
        "term" => None,
        _ => Some(idx.parse::<usize>().unwrap())
    };

    let output = slice_lir(lir_file_name, function, basicblock, idx);
    println!("{output}");
}

pub fn run_test() {

    let lir_file_name = "./test-inputs-slice/failed/01.lir";
    let target_path = "./test-inputs-slice/failed/01.target";

    let target_str = read_from(target_path);
    let mut target = target_str.split('#');
    let function = target.next().unwrap();
    let basicblock = target.next().unwrap();
    let idx = target.next().unwrap();

    let idx = match idx {
        "term" => None,
        _ => Some(idx.parse::<usize>().unwrap())
    };

    let output = slice_lir(lir_file_name, function, basicblock, idx);
    println!("{output}");
}

fn slice_lir(lir_file_name: &str, function: &str, basicblock: &str, term: Option<usize>) -> String {
    let input_string = read_from(lir_file_name);
    
    let lir_parsed = parse_lir(&input_string);
    let analyzed = slice(&lir_parsed, function, basicblock, term);
    analyzed
}

fn slice_lir_ptrs(lir_file_name: &str, function: &str, basicblock: &str, term: Option<usize>, pts_to_path: &str) -> String {
    let input_string = read_from(lir_file_name);
    let pts_to_str = read_from(pts_to_path);

    let lir_parsed = parse_lir(&input_string);
    let analyzed = slice_ptrs(&lir_parsed, function, basicblock, term, &pts_to_str);
    analyzed
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