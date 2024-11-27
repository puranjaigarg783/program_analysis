use std::collections::{BTreeMap as Map, BTreeSet as Set, VecDeque};
use pretty_assertions::assert_eq;
use std::env;


use optimization::middle_end::constraints::*;
use constraint_solve::solve;

pub fn main() {
    run();
}

fn run() {
    let args: Vec<String> = env::args().collect();
    let constraint_file_name = &args[1];

    let input_string = read_from(constraint_file_name);

    let output = solve(&input_string);

    println!("{}", output);
}

fn run_test() {
    let constraint_file_name = "./test-inputs-03/solve/no_proj.lir.constraints";
    let constraint_file_name = "./test-inputs-03/solve/proj4.lir.constraints";
    // let constraint_file_name = "./test-inputs-03/solve/failed/01.lir.constraints";

    let input_string = read_from(constraint_file_name);

    let output = solve(&input_string);

    println!("{}", output);
}

fn read_from(path: &str) -> String {
    String::from_utf8(
        std::fs::read(path)
            .unwrap_or_else(|_| panic!("Could not read the input file {}", path)),
    )
    .expect("The input file does not contain valid utf-8 text")
}

fn preds_to_string(pred_edges: &Map<ConstraintExp, Set<ConstraintExp>>) -> String {
    /*
    for x in var node in the graph in lexicographical ordering:
    if pred(x) contains a ref:
       ptsto = {c.args[0] for c in x.pred if c is a ref constructor}
	print("$x -> $ptsto")
    */
    use ConstraintExp::*;

    let mut return_string = String::from("");

    for (node, pred_set) in pred_edges {
        let mut refs: Vec<String> = vec![];
        for pred in pred_set.clone() {
            if let Ref(v1, v2) = pred {
                refs.push(v1.to_string());
            }
        }

        if !refs.is_empty() {
            return_string = format!("{}{node} -> {{{}}}\n", return_string, refs.join(", "));
        }
    }
    
    return_string
}