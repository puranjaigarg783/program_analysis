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
pub fn analyze(program: &Valid<Program>, func: FuncId) -> (Map<ProgramPoint, Set<ProgramPoint>>) {
    let program = &program.0;
    let f = &program.functions[&func];

    let init_store = Env::new(Map::new());
    let mut soln: Map<ProgramPoint, Set<ProgramPoint>> = Map::new();
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

    fn analyze_inst(&mut self, inst: &Instruction, cfg: &Cfg, soln: &mut Map<ProgramPoint, Set<ProgramPoint>>, store: &mut Map<VarId, Set<InstId>>) {

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

        let this_inst = &self.curr_inst.clone().unwrap();
        
        use Instruction::*;

        let mut used_vars: Set<VarId> = Set::new();
        let mut wdef: Option<Set<VarId>> = None;
        let this_pp = ProgramPoint::from_instid(this_inst.clone());
        let def = match inst {
            AddrOf { lhs, op: _ } => Some(lhs),
            Alloc { lhs, num, id } => {
                used_vars = get_vars(vec![num]);
                Some(lhs)
            },
            Arith { lhs, aop:_, op1, op2 } => {
                used_vars = get_vars(vec![op1, op2]);
                Some(lhs)
            },
            Cmp { lhs, rop:_, op1, op2 } => {
                used_vars = get_vars(vec![op1, op2]);
                Some(lhs)
            },
            CallExt { lhs, ext_callee, args } => {
                let call_wdef = calculate_call_wdef(cfg, get_vars(args.iter().collect()));
                // {<arg>|<arg> is a variable}
                used_vars.extend(get_vars(args.iter().collect()).iter().cloned());
                // CALL_WDEF
                used_vars.extend(call_wdef.iter().cloned());

                wdef = Some(call_wdef.clone());

                lhs.as_ref()
            },
            Copy { lhs, op } => {
                used_vars = get_vars(vec![op]);
                Some(lhs)
            },
            Gep {
                lhs,
                src,
                idx,
            } => {
                used_vars = get_vars(vec![idx]);
                used_vars.insert(src.clone());
                Some(lhs)
            },
            Gfp { lhs, src, field } => {
                used_vars.insert(src.clone());
                Some(lhs)
            }, 
            Load { lhs, src } => {
                let typ = &lhs.typ();
                used_vars.insert(src.clone());
                used_vars.append(cfg.addr_taken.clone().entry(typ.clone()).or_default());
                /* 
                if this_pp == ProgramPoint::from(bb_id("bb10"), Some(2)) {
                    println!("========================");
                    dbg!(&used_vars);
                    for used_var in &used_vars {
                        dbg!(self.get(used_var));
                    }
                }
                */
                Some(lhs)
            },
            Store { dst, op } => {
                // DEF = addr_taken[type(<op>)]
                wdef = cfg.addr_taken.get(&op.typ()).cloned();
                // USE = {x} ∪ {<op> | <op> is a variable}
                used_vars.insert(dst.clone());
                used_vars.extend(get_vars(vec![op]));
                None
            },
            Phi { .. } => unreachable!(),
        };

        // ppvalue
        let pp_value = &Value(Set::from([this_pp.clone()]));
        // ∀v ∈ USE, soln[pp] ← soln[pp] ∪ σ[v]
        soln.entry(this_pp.clone())
            .or_default()
            .extend(
                used_vars.iter()
                    .map(|used| self.get(used))
                    .map(|Value(set)| set)
                    .fold(Set::new(), |acc, set| acc.union(&set).cloned().collect())
            );

        // σ[x] ← {pp}
        if let Some(lhs) = def {
            self.insert(
                lhs,
                pp_value,
            );
        }

        // ∀x ∈ WDEF, σ[x] ← σ[x] ∪ {pp}
        if let Some(wdefs) = wdef {
            for w in wdefs {
                self.insert(&w, &self.get(&w).join(pp_value));
            }
        }
    }

    fn analyze_term(&mut self, term: &Terminal, cfg: &Cfg, soln: &mut Map<ProgramPoint, Set<ProgramPoint>>, store: &mut Map<VarId, Set<InstId>>) -> Set<BbId> {

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

        // ppvalue
        let pp_value = &Value(Set::from([this_pp.clone()]));
        // ∀v ∈ USE, soln[pp] ← soln[pp] ∪ σ[v]
        soln.entry(this_pp)
            .or_default()
            .extend(
                used_vars.iter()
                    .map(|used| self.get(used))
                    .map(|Value(set)| set)
                    .fold(Set::new(), |acc, set| acc.union(&set).cloned().collect())
            );

        // σ[x] ← {pp}
        if let Some(lhs) = def {
            self.insert(
                lhs,
                pp_value,
            );
        }
        // ∀x ∈ WDEF, σ[x] ← σ[x] ∪ {pp}
        if let Some(wdefs) = wdef {
            for w in wdefs {
                self.insert(&w, &self.get(&w).join(pp_value));
            }
        }

        // skip branch
        // remnants from value analysis
        // and i quote, "Because we don’t do a value analysis, we consider both branches as viable when analyzing a $branch instruction."
        Set::new() 
    }

    fn analyze_bb(&self, bb: &BasicBlock, cfg: &Cfg, soln: &mut Map<ProgramPoint, Set<ProgramPoint>>, store: &mut Map<VarId, Set<InstId>>) -> (Vec<Self>, Set<BbId>) {
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