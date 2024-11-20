//! Intraprocedural reaching definitions analysis.

use crate::commons::Valid;

use super::*;

// SECTION: analysis interface

// The powerset lattice.  It represents the definitions
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Value(pub Set<ProgramPoint>);

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let len = self.0.len();
        for (bb, pp) in self.0.iter().enumerate() {
            let (bb, n) = match pp {
                ProgramPoint::Instruction { bb, i } => (bb, i.to_string()),
                ProgramPoint::Terminal { bb } => (bb, "term".to_owned()),
            };

            if n == "term" {
                write!(f, "{bb}.{n}")?;
            } else {
                write!(f, "{bb}.{n}, ")?;
            }
        }
        write!(f, "}}")
    }
}

// Abstract environment
pub type Env = PointwiseEnv<Value>;

// Performs the analysis: use `forward_analysis` to implement this.
pub fn analyze(program: &Valid<Program>, func: FuncId, pts_to: Map<String, Set<String>>) -> (Map<FuncId, FuncId>) {
    let program = &program.0;
    let f = &program.functions[&func];

    let init_store = Env::new(Map::new());
    let mut soln: Map<FuncId, FuncId> = Map::new();
    forward_analysis(f, &Cfg::new(f, program.globals.clone(), program.structs.clone()), &init_store, &init_store, &mut soln)
}

// SECTION: analysis implementation

impl AbstractValue for Value {
    type Concrete = ProgramPoint;

    const BOTTOM: Self = Value(Set::new());

    fn alpha(def: ProgramPoint) -> Self {
        Value(Set::from([def]))
    }

    fn join(&self, rhs: &Self) -> Value {
        Value(self.0.union(&rhs.0).cloned().collect())
    }
}

fn join_sets(set1: Set<ProgramPoint>, set2: Set<ProgramPoint>) -> Set<ProgramPoint> {
    set1.union(&set2).cloned().collect()
}

fn get_vars(opset: Vec<&Operand>) -> Set<VarId> {
    let mut return_set: Set<VarId> = Set::new();
    for op in opset {
        if let Operand::Var(v) = op.clone() {
            return_set.insert(v);
        }
    }

    return_set
}

fn get_reachable(pts_to: Map<String, Set<String>>, v: VarId) -> Set<VarId> {
    todo!()
}

impl AbstractEnv for Env {
    fn join_with(&mut self, rhs: &Self, _block: &BbId, join_type: i64) -> bool {
        let mut changed = false;

        for (x, lhs) in self.values.iter_mut() {
            if let Some(rhs) = rhs.values.get(x) {
                let old = lhs.clone();
                *lhs = lhs.join(rhs);

                changed = changed || *lhs != old;
            }
        }

        for (x, rhs) in &rhs.values {
            let old = self.get(x);
            let lhs = old.join(rhs);
            self.insert(x, &lhs);

            changed = changed || lhs != old;
        }

        changed
    }

    fn analyze_inst(&mut self, inst: &Instruction, cfg: &Cfg, soln: &mut Map<FuncId, FuncId>, store: &mut Map<VarId, Set<FuncId>>) {
        let this_inst = &self.curr_inst.clone().unwrap();
        
        use Instruction::*;

        let mut used_vars: Set<VarId> = Set::new();
        let mut wdef: Option<Set<VarId>> = None;
        let this_pp = ProgramPoint::from_instid(this_inst.clone());
        let def = match inst {
            AddrOf { lhs, op: _ } => Some(lhs),
            Alloc { lhs, num, id } => {
                Some(lhs)
            },
            Arith { lhs, aop:_, op1, op2 } => {
                Some(lhs)
            },
            Cmp { lhs, rop:_, op1, op2 } => {
                Some(lhs)
            },
            CallExt { lhs, ext_callee, args } => {
                if let Some(x) = lhs {
                    if ext_callee.name().starts_with("src") {
                        let mut insert_set = Set::new();
                        insert_set.insert(ext_callee.clone());
                        store.insert(x.clone(), insert_set);

                        for arg in args {
                            if let Operand::Var(v) = arg.to_owned() {
                                store.entry(v).or_default().insert(ext_callee.clone());
                            }
                        }

                        println!("inserted to store, src: {}", ext_callee.name());
                    } else {
                        store.insert(x.clone(), Set::new());
                    }
                }

                
                lhs.as_ref()
            },
            Copy { lhs, op } => {
                Some(lhs)
            },
            Gep {
                lhs,
                src,
                idx,
            } => {
                Some(lhs)
            },
            Gfp { lhs, src, field } => {
                Some(lhs)
            }, 
            Load { lhs, src } => {
                Some(lhs)
            },
            Store { dst, op } => {
                None
            },
            Phi { .. } => unreachable!(),
        };

        // ppvalue
        let pp_value = &Value(Set::from([this_pp.clone()]));
    }

    fn analyze_term(&mut self, term: &Terminal, cfg: &Cfg, soln: &mut Map<FuncId, FuncId>, store: &mut Map<VarId, Set<FuncId>>) -> Set<BbId> {

        /*
            ReachViaArgs = ReachableTypes(type(<arg1>)) ∪ . . . ∪ ReachableTypes(type(<argN>))
            ReachViaGlobals = ReachableTypes(type(<global1>)) ∪ . . .
            WDEF = ⋃ {addr_taken[τ] | τ ∈ ReachViaArgs ∪ReachViaGlobals} ∪ Globals
        */
        fn calculate_call_wdef(cfg: &Cfg, ops: Set<VarId>) -> Set<VarId> {
            ops.iter()
                .map(|x| cfg.reachable_types(&x.typ()))
                .chain(cfg.globals.iter().map(|x| cfg.reachable_types(&x.typ())))
                .fold(Set::new(), |acc, x| acc.union(&x).cloned().collect()).iter()
                .filter_map(|a| cfg.addr_taken.get(a))
                .flatten()
                .chain(cfg.globals.iter())
                .cloned()
                .collect()
        }

        fn calculate_call_wdef_debug(cfg: &Cfg, ops: Set<VarId>) -> Set<VarId> {
            let ops_reachable_types: Vec<Set<Type>> = ops.iter()
                .map(|x| cfg.reachable_types(&x.typ()))
                .collect();
        
            let globals_reachable_types: Vec<Set<Type>> = cfg.globals.iter()
                .map(|x| cfg.reachable_types(&x.typ()))
                .collect();
        
            let combined_reachable_types: Vec<Set<Type>> = ops_reachable_types.into_iter()
                .chain(globals_reachable_types)
                .collect();

            dbg!(&combined_reachable_types);
        
            let folded_reachable_types: Set<Type> = combined_reachable_types.into_iter()
                .fold(Set::new(), |acc, x| acc.union(&x).cloned().collect());

            dbg!(&folded_reachable_types);

            let filtered_addr_taken: Vec<&VarId> = folded_reachable_types.iter()
                .filter_map(|a| cfg.addr_taken.get(a))
                .flatten()
                .collect();
        
            let final_set: Set<VarId> = filtered_addr_taken.into_iter()
                .chain(cfg.globals.iter())
                .cloned()
                .collect();
        
            final_set
        }
        use Terminal::*;

        let mut used_vars: Set<VarId> = Set::new();
        let mut wdef: Option<Set<VarId>> = None;
        let this_pp = ProgramPoint::from(self.curr_inst.clone().unwrap().0, None);
        let def = match term {
            CallDirect { lhs, callee, args, next_bb } => {
                let call_wdef = calculate_call_wdef(cfg, get_vars(args.iter().collect()));
                // {<arg>|<arg> is a variable}
                used_vars.extend(get_vars(args.iter().collect()).iter().cloned());
                // CALL_WDEF
                used_vars.extend(call_wdef.iter().cloned());

                wdef = Some(call_wdef.clone());

                lhs.as_ref()
            },
            CallIndirect { lhs, callee, args, next_bb } => {
                let call_wdef = calculate_call_wdef(cfg, get_vars(args.iter().collect()));
                // {fp}
                used_vars.insert(callee.clone());
                // {<arg>|<arg> is a variable}
                used_vars.extend(get_vars(args.iter().collect()).iter().cloned());
                // CALL_WDEF
                used_vars.extend(call_wdef.iter().cloned());

                wdef = Some(call_wdef.clone());

                lhs.as_ref()
            },
            Branch { cond, .. } => {
                used_vars = get_vars(vec![cond]);
                None
            },
            Ret(Some(op)) => {
                used_vars = get_vars(vec![op]);
                None
            },
            _ => None,
        };

        // skip branch
        // remnants from value analysis
        // and i quote, "Because we don’t do a value analysis, we consider both branches as viable when analyzing a $branch instruction."
        Set::new() 
    }

    fn analyze_bb(&self, bb: &BasicBlock, cfg: &Cfg, soln: &mut Map<FuncId, FuncId>, store: &mut Map<VarId, Set<FuncId>>) -> (Vec<Self>, Set<BbId>) {
        let mut v = vec![];
        let mut s = self.clone();

        for (i, inst) in bb.insts.iter().enumerate() {
            s.curr_inst = Some((bb.id.clone(), i));
            s.analyze_inst(inst, cfg, soln, store);
            v.push(s.clone());
        }

        s.curr_inst = Some((bb.id.clone(), bb.insts.len()));
        s.analyze_term(&bb.term, cfg, soln, store);
        v.push(s.clone());
        (v, Set::new())
    }
    
}