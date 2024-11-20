use super::*;
use std::collections::{BTreeMap as Map, BTreeSet as Set, VecDeque};
use pretty_assertions::assert_eq;
use std::env;

pub fn solve(constraint_string: &str) -> String {
    use ConstraintExp::*;

    let pairs = constraint_string.parse::<Constraints>().unwrap();

    assert_eq!(pairs.to_string(), constraint_string);
    
    let mut pred_edges: Map<ConstraintExp, Set<ConstraintExp>> = Map::new();
    let mut succ_edges: Map<ConstraintExp, Set<ConstraintExp>> = Map::new();

    let mut worklist: VecDeque<ConstraintExp> = VecDeque::new();

    for constraint in pairs.0 {
        let (e1, e2) = constraint.as_tuple();
        add_edge(&mut worklist, &mut pred_edges, &mut succ_edges, e1.clone(), e2.clone());
    }
    dbg!(&worklist);
    while let Some(node) = worklist.pop_front() {

        preds_to_succs(&node, &mut worklist, &mut pred_edges, &mut succ_edges);
        match node.clone() {
            Var(var) => {
                let projected = Proj(var.clone());
                let preds = pred_edges.entry(node.clone()).or_default().clone();
                let succs = succ_edges.entry(projected.clone()).or_default().clone();

                for succ in succs.clone() {
                    for pred in preds.clone() {
                        if let Ref(v1, v2) = pred {
                            add_edge(&mut worklist, &mut pred_edges, &mut succ_edges, Var(v1), succ.clone());

                        }
                    }
                }
            },
            Proj(var) => {
                let varred = Var(var.clone());
                let preds = pred_edges.entry(node.clone()).or_default().clone();
                let succs = succ_edges.entry(varred.clone()).or_default().clone();

                for succ in succs.clone() {
                    for pred in preds.clone() {
                        if let Ref(v1, v2) = pred {
                            add_edge(&mut worklist, &mut pred_edges, &mut succ_edges, Var(v1), succ.clone());

                        }
                    }
                }
            },
            _ => {
                ()
            }
        }
    }

    preds_to_string(&pred_edges)
}

fn preds_to_succs(
    node: &ConstraintExp,
    worklist: &mut VecDeque<ConstraintExp>,
    pred_edges: &mut Map<ConstraintExp, Set<ConstraintExp>>,
    succ_edges: &mut Map<ConstraintExp, Set<ConstraintExp>>
) {
    let preds = pred_edges.entry(node.clone()).or_default().clone();
    let succs = succ_edges.entry(node.clone()).or_default().clone();
    for succ in succs.clone() {
        for pred in preds.clone() {
            add_edge(worklist, pred_edges, succ_edges, pred, succ.clone());
        }
    }
}

fn read_from(path: &str) -> String {
    String::from_utf8(
        std::fs::read(path)
            .unwrap_or_else(|_| panic!("Could not read the input file {}", path)),
    )
    .expect("The input file does not contain valid utf-8 text")
}

fn add_edge(
    worklist: &mut VecDeque<ConstraintExp>,
    pred_edges: &mut Map<ConstraintExp, Set<ConstraintExp>>,
    succ_edges: &mut Map<ConstraintExp, Set<ConstraintExp>>,
    e1: ConstraintExp, e2: ConstraintExp
) {
    use ConstraintExp::*;
    if e1.get_name() == "main.id16" {
        dbg!(&e1, &e2);
    }
    match (&e1, &e2) {
        // both var
        (Var(v1), Var(v2)) => {
            if succ_edges.entry(e1.clone()).or_default().insert(e2.clone()) && !worklist.contains(&e1) {
                worklist.push_back(e1);
            }
        },
        // one proj
        (Proj(v1), Var(v2)) => {
            if succ_edges.entry(e1.clone()).or_default().insert(e2.clone()) && !worklist.contains(&e1) {
                pred_edges.entry(e2.clone()).or_default().insert(e1.clone());
                worklist.push_back(e1);
            }
        },
        (Var(v1), Proj(v2)) => {
            if succ_edges.entry(e1.clone()).or_default().insert(e2.clone()) && !worklist.contains(&e1) {
                pred_edges.entry(e2.clone()).or_default().insert(e1.clone());
                worklist.push_back(e1);
            }
        },
        (Ref(v1, v2), Var(v3)) => {
            if pred_edges.entry(e2.clone()).or_default().insert(e1.clone()) && !worklist.contains(&e2) {
                succ_edges.entry(Var(v1.clone())).or_default().insert(Proj(v3.clone()));
                worklist.push_back(e2);
            }
        },
        (Proj(v1), Proj(v2)) => {
            if succ_edges.entry(e1.clone()).or_default().insert(e2.clone()) && !worklist.contains(&e1) {
                pred_edges.entry(e2.clone()).or_default().insert(e1.clone());
                worklist.push_back(e1.clone());
            }
        },
        (Ref(v1, v2), Proj(v3)) => {
            

            /* 
            let var = Var(v3.clone());
            let var_pred = pred_edges.entry(var).or_default().clone();
            for pred in var_pred {
                if let Ref(b1, b2) = pred {
                    if pred_edges.entry(Var(b1.clone())).or_default().insert(e1.clone()) && !worklist.contains(&Var(b1.clone())) {
                        worklist.push_back(Var(b1.clone()));
                    }
                }
            }*/
        },
        _ => { println!("============================================skipping for now e1: {e1} | e2: {e2}") }
    }
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
    return_string += "\n";
    return_string
}