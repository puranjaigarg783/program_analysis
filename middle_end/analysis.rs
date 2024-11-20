//! Static analysis of lir programs.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::collections::{BTreeMap as Map, BTreeSet as Set};
use std::fmt::Display;

use super::lir::*;

pub mod call_graph;
pub mod constant_prop;
pub mod integer_interval;
// pub mod copy_prop;
pub mod liveness;
pub mod reaching_defs;


#[cfg(test)]
mod tests;

/// Instruction IDs: this is just a combination of the basic block ID and the
/// index of the instruction in the block.
pub type InstId = (BbId, usize);

/// The control-flow graph *for a function* (abstracted so that we can easily
/// get successors and predecessors and also perform forward analyses on the
/// actual cfg or backwards analyses by reversing the edges to get a backwards
/// cfg).
#[derive(Clone, Debug)]
pub struct Cfg {
    pub entry: BbId,
    pub exit: BbId,
    succ_edges: Map<BbId, Set<BbId>>,
    pred_edges: Map<BbId, Set<BbId>>,
    pub loop_headers: Set<BbId>,
    pub addr_taken: Set<VarId>,
    pub addr_taken_ints: Set<VarId>,
    structs: Map<StructId, Set<FieldId>>,
    pub globals: Set<VarId>,
    pub structs_that_reach_int: Set<StructId>,
}

impl Cfg {
    // construct a Cfg from the given function's basic blocks.
    pub fn new(function: &Function, globals: Set<VarId>, structs: Map<StructId, Set<FieldId>>) -> Self {
        fn insert_edge(map: &mut Map<BbId, Set<BbId>>, key_bbid: &BbId, value_bbid: &BbId) {
            map.entry(key_bbid.clone())
                .and_modify(|s| {
                    s.insert(value_bbid.clone());
                })
                .or_insert([value_bbid.clone()].into());
        }
        let entry = bb_id("entry");
        let mut exit = bb_id("exit");
        let mut succ_edges: Map<BbId, Set<BbId>> = Map::new();
        let mut pred_edges: Map<BbId, Set<BbId>> = Map::new();

        pred_edges.insert(entry.clone(), [].into());

        for (bbid, bb) in function.body.clone() {
            match bb.term {
                Terminal::Branch { cond: _, tt, ff } => {
                    insert_edge(&mut succ_edges, &bbid, &tt);
                    insert_edge(&mut succ_edges, &bbid, &ff);

                    insert_edge(&mut pred_edges, &tt, &bbid);
                    insert_edge(&mut pred_edges, &ff, &bbid);
                }
                Terminal::CallDirect {
                    lhs: _,
                    callee: _,
                    args: _,
                    next_bb,
                } => {
                    insert_edge(&mut succ_edges, &bbid, &next_bb);
                    insert_edge(&mut pred_edges, &next_bb, &bbid);
                }
                Terminal::CallIndirect {
                    lhs: _,
                    callee: _,
                    args: _,
                    next_bb,
                } => {
                    insert_edge(&mut succ_edges, &bbid, &next_bb);
                    insert_edge(&mut pred_edges, &next_bb, &bbid);
                }
                Terminal::Jump(next_bb) => {
                    insert_edge(&mut succ_edges, &bbid, &next_bb);
                    insert_edge(&mut pred_edges, &next_bb, &bbid);
                }
                Terminal::Ret(_) => {
                    succ_edges.insert(bbid.clone(), [].into());
                    exit = bbid;
                }
            }
        }

        /*
        println!("{:#?}", Cfg {
            entry: entry.clone(),
            exit: exit.clone(),
            succ_edges: succ_edges.clone(),
            pred_edges: pred_edges.clone(),
        });
        */

        let mut return_cfg = Cfg {
            entry,
            exit,
            succ_edges,
            pred_edges,
            addr_taken: Set::new(),
            addr_taken_ints: Set::new(),
            loop_headers : Set::new(),
            globals,
            structs: structs.clone(),
            structs_that_reach_int: Set::new(),
        };

        return_cfg.loop_headers();
        return_cfg.get_addr_takens(&function.body);
        return_cfg.calculate_structs_that_reach_int(&structs);

        return_cfg
    }

    // an iterator over the successor edges of bb.
    pub fn succ(&self, bb: &BbId) -> impl Iterator<Item = &BbId> {
        self.succ_edges[bb].iter()
    }

    // an iterator over the predecessor edges of bb.
    pub fn pred(&self, bb: &BbId) -> impl Iterator<Item = &BbId> {
        self.pred_edges[bb].iter()
    }

    // get all addr_taken variables analysis
    fn get_addr_takens(&mut self, body: &Map<BbId, BasicBlock>) {

        for bb in body.values() {
            for inst in &bb.insts {
                match inst {
                    Instruction::AddrOf { lhs: _, op } => {
                        let op = op.clone();
                        self.addr_taken.insert(op.clone());

                        if op.typ().is_int() {
                            self.addr_taken_ints.insert(op);
                        }
                    },
                    Instruction::CallExt { lhs: _, ext_callee: _, args: _ } => (),
                    _ => (),
                }
            }
        }

    }

    fn calculate_structs_that_reach_int(&mut self, structs: &Map<StructId, Set<FieldId>>) {

        for (structid, fields) in structs {
            for field in fields {
                if field.typ.base_typ_is(int_ty()) { // could go deeper? TODO
                    self.structs_that_reach_int.insert(structid.clone());
                    break;
                }
            }
        }
    }

    fn reachable_types(&self, typ: &Type) -> Set<Type> {
        use LirType::*;

        fn reachable_inner(typ_stack: &mut Vec<Type>, reachables: &mut Set<Type>, structs: &Map<StructId, Set<FieldId>>) {
            while let Some(t) = typ_stack.pop() {
                if reachables.contains(&t) {
                    continue;
                }
                
                match &*(t.clone()).0 {
                    Int => {
                        reachables.insert(t.clone());
                    },
                    Struct(struct_id) => {
                        reachables.insert(t.clone());
    
                        let mut field_types: Vec<Type> = structs[struct_id].iter().map(|a| a.typ.clone()).collect();
                        typ_stack.append(&mut field_types);
                    },
                    Pointer(inner_typ) => {
                        reachables.insert(t.clone());
                        typ_stack.push(inner_typ.clone());
                    },
                    Function{ret_ty, param_ty} => (),
    
                }
            }
        }

        let mut reachables = Set::new();
        let mut typ_stack: Vec<Type> = vec![];
        match &*(typ.clone()).0 {
            Struct(struct_id) => {
                let mut field_types: Vec<Type> = self.structs[struct_id].iter().map(|a| a.typ.clone()).collect();
                typ_stack.append(&mut field_types);
            },
            Pointer(inner_typ) => {
                typ_stack.push(inner_typ.clone());
            },
            _ => return reachables,
        }

        reachable_inner(&mut typ_stack, &mut reachables, &self.structs);
        reachables
    }

    pub fn var_reaches_int(&self, var: &VarId) -> bool {
        let var_type = var.typ();

        self.reachable_types(&var_type).contains(&int_ty())
    }

    

    // returns all loop headers for widening
    fn loop_headers(&mut self) {

        let curr_block: &BbId = &self.entry;
        let mut visited: Set<&BbId> = Set::new();
        visited.insert(curr_block);
        let mut headers: Set<BbId> = Set::new();
        
        self.recursive_search(curr_block, visited, &mut headers);

        self.loop_headers = headers;
    }

    fn recursive_search(&self, curr_block: &BbId, visited: Set<&BbId>, headers: &mut Set<BbId>) {
        let succ_blocks = self.succ(curr_block);
        for succ in succ_blocks {
            if visited.contains(succ) {
                headers.insert(succ.clone());
            } else if *succ == self.exit {

            } else {
                let mut new_visited = visited.clone();
                new_visited.insert(succ);
                self.recursive_search(succ, new_visited, headers);
            }
        }
    }
}

/// An abstract value from an abstract lattice.
///
/// Any abstract domain for a variable implements this.
pub trait AbstractValue: Clone + Display + Eq + PartialEq {
    /// The concrete values we're abstracting.
    ///
    /// This is a generic type, basically.
    type Concrete;

    /// The bottom value of the join semi-lattice.
    const BOTTOM: Self;

    /// The abstraction of a concrete value.
    fn alpha(val: Self::Concrete) -> Self;

    /// The join of two abstract values.
    fn join(&self, rhs: &Self) -> Self;
}

/// The abstract environment (the abstract state) used for any dfa.  It needs to
/// know how to combine with other stores and how to modify itself when
/// processing an instruction or terminal.
pub trait AbstractEnv: Clone {
    // compute self = self ⊔ rhs
    //
    // `block` is the basic block self belongs to.
    //
    // Return whether the block has changed as the result of this operation.
    fn join_with(&mut self, rhs: &Self, block: &BbId, join_type: i64) -> bool;
    
    // Transfer function for instructions.  Emulates what an instruction would
    // do.  Note that this function changes the current state!
    fn analyze_inst(&mut self, inst: &Instruction, cfg: &Cfg);

    // Transfer function for terminals.  Emulates what a terminal would do.
    // Note that this function changes the current state!
    fn analyze_term(&mut self, inst: &Terminal, cfg: &Cfg) -> Set<BbId>;

    // Transfer function for basic blocks.
    //
    // If this environment is part of a forward analysis, `self` is the pre
    // state for the basic block, and this function should return all the post
    // states for all instructions and the terminal in the block.
    //
    // If this environment is part of a backward analysis, `self` is the post
    // state for the basic block, and this function should return all the pre
    // states for all instructions and the terminal in the block.
    fn analyze_bb(&self, bb: &BasicBlock, cfg: &Cfg) -> (Vec<Self>, Set<BbId>);
}

/// An abstract environment built as a pointwise extension of the abstract
/// domain `A`.  It is a map from variables to abstract values.
///
/// To use this in an analysis, we need to provide the abstract domain `A` for
/// each variable.
#[derive(Clone, Debug)]
pub struct PointwiseEnv<A: AbstractValue> {
    pub values: Map<VarId, A>,
    pub curr_inst: Option<InstId>,
}

impl<A: AbstractValue> PointwiseEnv<A> {
    fn new(values: Map<VarId, A>) -> Self {
        Self {
            values,
            curr_inst: None,
        }
    }

    // get the value of a variable, or bottom if it isn't present.
    pub fn get(&self, key: &VarId) -> A {
        self.values.get(key).unwrap_or(&A::BOTTOM).clone()
    }

    // insert a value for a variable.
    fn insert(&mut self, key: &VarId, val: &A) {
        self.values.insert(key.clone(), val.clone());
    }

    // get a mutable reference to the value of a variable, which will be inserted
    // with value bottom if not already present.
    fn get_mut(&mut self, key: &VarId) -> &mut A {
        self.values.entry(key.clone()).or_insert(A::BOTTOM)
    }

    fn get_env(self) -> Map<VarId, A> {
        self.values
    }
}
/*
impl<A: AbstractValue> Display for PointwiseEnv<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let str = self
            .values
            .iter()
            .filter(|(x, _)| x.scope().is_some())
            .fold("".to_string(), |acc, (var, val)| {
                if *val == A::BOTTOM {
                    acc
                } else {
                    format!("{acc}{var} -> {val}\n")
                }
            });
        write!(f, "{str}")
    }
}*/

impl<A: AbstractValue> Display for PointwiseEnv<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let str = self
            .values
            .iter()
            .fold("".to_string(), |acc, (var, val)| {
                if *val == A::BOTTOM {
                    acc
                } else {
                    format!("{acc}{var} -> {val}\n")
                }
            });
        write!(f, "{str}")
    }
}

// SECTION: intraprocedural dataflow analysis framework

/// Analyze the given function.  Assumes that the function is from a valid
/// program.
///
/// This function starts from the entry, end performs a forward analysis.  It
/// returns:
///
/// (1) the pre state for each basic block
/// (2) the pre state for each instruction
///
/// bottom_state is the bottom value for the abstract state `A`.  You should use
/// it as the starting state for the analysis.
///
/// Hint: You can compute (1) first, then process each block only once to compute
/// (2).
pub fn forward_analysis<A: AbstractEnv>(
    f: &Function,
    cfg: &Cfg,
    entry_state: &A,
    bottom_state: &A,
) -> (Map<BbId, A>, Map<InstId, A>) {
    // 1. Create an initial solution that maps entry block → entry state.
    // 2. Create a worklist containing entry.
    // 3. Implement the worklist algorithm.
    // 4. Compute per-instruction pre states.
    let mut bb_pre_states = Map::new();
    let mut inst_pre_states = Map::new();
    let mut worklist = VecDeque::new();

    let mut visited = Set::new();

    // Initialize
    for bbid in f.body.keys() {
        bb_pre_states.insert(bbid.clone(), bottom_state.clone());
        // if !visited.insert(bbid) {unreachable!("double bbid in forward_analysis visited")};
        // inst_pre_states.insert((bbid.clone(), 0), bottom_state.clone());
    }

    bb_pre_states.insert(cfg.entry.clone(), entry_state.clone());
    worklist.push_back(cfg.entry.clone());
    // Worklist algorithm
    while let Some(bb_id) = worklist.pop_front() {
        let state = bb_pre_states.get(&bb_id).unwrap_or(bottom_state).clone();
        let bb = f.body[&bb_id].clone();
        let (post_states, skip_state) = state.analyze_bb(&bb, cfg);
        // unncessary here
        for (i, post_state) in post_states.iter().enumerate() {
            inst_pre_states.insert((bb_id.clone(), i), post_state.clone());
        }
        
        for succ in cfg.succ(&bb_id) {
            if skip_state.contains(succ) {
                continue;
            }
            let succ_state = bb_pre_states.get_mut(succ).unwrap();

            let join_type: i64 = if cfg.loop_headers.contains(succ) { 1 } else { 0 };

            if succ_state.join_with(post_states.last().unwrap(), succ, join_type) || !visited.contains(succ) {
                visited.insert(succ);
                worklist.push_back(succ.clone());
            }
        }
    }
    (bb_pre_states, inst_pre_states)
}

/// Analyze the given function.  Assumes that the function is from a valid
/// program.
///
/// This function starts from the exit block in the CFG (which may have a name
/// other than "exit"), end performs a backward analysis.  It returns:
///
/// (1) the post state for each basic block
/// (2) the post state for each instruction
///
/// bottom_state is the bottom value for the abstract state `A`.  You should use
/// it as the starting state for the analysis.
///
/// Hint: You can compute (1) first, then process each block only once to compute
/// (2).
pub fn backward_analysis<A: AbstractEnv>(
    _f: &Function,
    _cfg: &Cfg,
    _exit_state: &A,
    _bottom_state: &A,
) -> (Map<BbId, A>, Map<InstId, A>) {
    // 1. Create an initial solution that maps exit block → exit state.
    // 2. Create a worklist containing exit.
    // 3. Implement the worklist algorithm.
    // 4. Compute per-instruction pre states.

    todo!()
}
