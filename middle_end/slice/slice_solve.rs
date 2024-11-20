use super::*;
use crate::commons::Valid;
use analysis_rdef::{ProgramPoint, reaching_defs};
use analysis_rdef_ptrs::reaching_defs_ptrs;

use control_analysis::control;
use std::collections::{BTreeMap as Map, BTreeSet as Set};

pub fn slice(valid_program: &Valid<Program>, function: &str, block: &str, index: Option<usize>) -> String {
    let program = &valid_program.0;
    let mut debug_string = String::from("");
    let fid = &func_id(function);

    let f = program.functions.get(fid).unwrap();

    let rdef_f =  reaching_defs::analyze(valid_program, fid.clone());
    let control_f = control::analyze_postdom(valid_program, fid.clone());
    let target = ProgramPoint::from(bb_id(block), index);

    let mut dependencies = rdef_f.clone();

    for (bbid, bb_set) in &control_f {
        for depended_on in bb_set {
            let pp_depended = ProgramPoint::from(depended_on.clone(), None);
            for idx in 0..f.body.get(bbid).unwrap().insts.len() {
                let pp = ProgramPoint::from(bbid.clone(), Some(idx));
                dependencies.entry(pp).or_default().insert(pp_depended.clone());
            }
            let pp = ProgramPoint::from(bbid.clone(), None);
            dependencies.entry(pp).or_default().insert(pp_depended);
        }
    }

    let mut slice_set: Set<&ProgramPoint> = Set::new();
    slice_set.insert(&target);
    let mut worklist: Vec<&ProgramPoint> =  vec![&target];

    while let Some(pp) = worklist.pop() {
        if let Some(pp_set) = dependencies.get(pp) {
            for target_pp in pp_set {
                if slice_set.insert(target_pp) {
                    worklist.push(target_pp);
                }
            }
        }
    }

    print_slice(program.functions.get(&func_id(function)).unwrap(), &slice_set)
}

pub fn slice_ptrs(valid_program: &Valid<Program>, function: &str, block: &str, index: Option<usize>, pts_to_str: &str) -> String {
    use analysis_rdef_ptrs::ProgramPoint;
    let program = &valid_program.0;
    let mut debug_string = String::from("");
    let pts_to = parse_pts_to(pts_to_str);
    let fid = &func_id(function);

    let f = program.functions.get(fid).unwrap();

    let rdef_f =  reaching_defs_ptrs::analyze(valid_program, fid.clone(), pts_to);
    let control_f = control::analyze_postdom(valid_program, fid.clone());
    let target = ProgramPoint::from(bb_id(block), index);

    let mut dependencies = rdef_f.clone();

    for (bbid, bb_set) in &control_f {
        for depended_on in bb_set {
            let pp_depended = ProgramPoint::from(depended_on.clone(), None);
            for idx in 0..f.body.get(bbid).unwrap().insts.len() {
                let pp = ProgramPoint::from(bbid.clone(), Some(idx));
                dependencies.entry(pp).or_default().insert(pp_depended.clone());
            }
            let pp = ProgramPoint::from(bbid.clone(), None);
            dependencies.entry(pp).or_default().insert(pp_depended);
        }
    }

    let mut slice_set: Set<&ProgramPoint> = Set::new();
    slice_set.insert(&target);
    let mut worklist: Vec<&ProgramPoint> =  vec![&target];

    while let Some(pp) = worklist.pop() {
        if let Some(pp_set) = dependencies.get(pp) {
            for target_pp in pp_set {
                if slice_set.insert(target_pp) {
                    worklist.push(target_pp);
                }
            }
        }
    }

    print_slice_ptrs(program.functions.get(&func_id(function)).unwrap(), &slice_set)
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

fn print_slice(f: &Function, slice_info: &Set<&ProgramPoint>) -> String {
    let mut output = String::from("");
    let bb_visit: Set<&BbId> = slice_info.into_iter().map(|a| a.get_bb()).collect();
    for (bbid, bb) in f.body.clone() {
        if bb_visit.contains(&bbid) {
            output = output + bbid.name() +":\n";
            for (idx, inst) in bb.insts.iter().enumerate() {
                let pp = ProgramPoint::from(bbid.clone(), Some(idx));
                if slice_info.contains(&pp) {
                    output = output + "  " + &inst.to_string() + "\n";
                }
            }

            let pp_term = ProgramPoint::from(bbid.clone(), None);
            if slice_info.contains(&pp_term) {
                output = output + "  " +  &bb.term.to_string() + "\n";
            }
            output += "\n"
        }
    }
    output
}

fn print_slice_ptrs(f: &Function, slice_info: &Set<&analysis_rdef_ptrs::ProgramPoint>) -> String {
    let mut output = String::from("");
    let bb_visit: Set<&BbId> = slice_info.into_iter().map(|a| a.get_bb()).collect();
    for (bbid, bb) in f.body.clone() {
        if bb_visit.contains(&bbid) {
            output = output + bbid.name() +":\n";
            for (idx, inst) in bb.insts.iter().enumerate() {
                let pp = analysis_rdef_ptrs::ProgramPoint::from(bbid.clone(), Some(idx));
                if slice_info.contains(&pp) {
                    output = output + "  " + &inst.to_string() + "\n";
                }
            }

            let pp_term = analysis_rdef_ptrs::ProgramPoint::from(bbid.clone(), None);
            if slice_info.contains(&pp_term) {
                output = output + "  " +  &bb.term.to_string() + "\n";
            }
            output += "\n"
        }
    }
    output
}

fn print_rdef(result: &Map<ProgramPoint, Set<ProgramPoint>>) -> String {
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

fn print_dominance(result: &Map<BbId, Set<BbId>>) -> String {
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