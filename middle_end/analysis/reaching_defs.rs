
//! Intraprocedural reaching definitions analysis.

use crate::commons::Valid;

use super::*;

// SECTION: analysis interface

// The powerset lattice.  It represents the definitions
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Value(pub Set<InstId>);

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let len = self.0.len();
        for (i, (bb, n)) in self.0.iter().enumerate() {
            if i + 1 == len {
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
pub fn analyze(program: &Valid<Program>, func: FuncId) -> (Map<BbId, Env>, Map<InstId, Env>) {
    let program = &program.0;
    let f = &program.functions[&func];

    let init_store = Env::new(Map::new());

    forward_analysis(f, &Cfg::new(f, program.globals.clone(), program.structs.clone()), &init_store, &init_store)
}

// SECTION: analysis implementation

impl AbstractValue for Value {
    type Concrete = InstId;

    const BOTTOM: Self = Value(Set::new());

    fn alpha(def: InstId) -> Self {
        Value(Set::from([def]))
    }

    fn join(&self, rhs: &Self) -> Value {
        Value(self.0.union(&rhs.0).cloned().collect())
    }
}

fn join_sets(set1: Set<InstId>, set2: Set<InstId>) -> Set<InstId> {
    set1.union(&set2).cloned().collect()
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

    fn analyze_inst(&mut self, inst: &Instruction, cfg: &Cfg) {
        let this_inst = &self.curr_inst.clone().unwrap();
        
        let get_vars = |opset: Vec<&Operand>| -> Vec<VarId> {
            let mut return_set: Vec<VarId> = Vec::new();
            for op in opset {
                if let Operand::Var(v) = op.clone() {
                    return_set.push(v);
                }
            }

            return_set
        };
        
        use Instruction::*;

        let mut use_set: Vec<VarId> = Vec::new();

        let def = match inst {
            AddrOf { lhs, op: _ } => Some(lhs),
            Alloc { lhs, .. } => Some(lhs),
            Arith { lhs, aop:_, op1, op2 } => {
                use_set = get_vars(vec![op1, op2]);

                Some(lhs)
            },
            Cmp { lhs, .. } => Some(lhs),
            CallExt { lhs, .. } => lhs.as_ref(),
            Copy { lhs, op: _ } => Some(lhs),
            Gep {
                lhs,
                src: _,
                idx: _,
            } => Some(lhs),
            Gfp { lhs, .. } => Some(lhs),
            Load { lhs, src: _ } => Some(lhs),
            Store { dst: _, op: _ } => None,
            Phi { .. } => unreachable!(),
        };

        let mut used_lines: Set<InstId> = Set::new();
        for used in use_set {
            let Value(set) = self.get(&used);
            used_lines = join_sets(used_lines, set);
            
        }

        if let Some(lhs) = def {
            self.values.insert(
                lhs.clone(),
                Value(Set::from([self.curr_inst.clone().unwrap()])),
            );
        }
    }

    fn analyze_term(&mut self, term: &Terminal, cfg: &Cfg) -> Set<BbId> {
        use Terminal::*;

        let def = match term {
            CallDirect { lhs, .. } => lhs.as_ref(),
            CallIndirect { lhs, .. } => lhs.as_ref(),
            _ => None,
        };
        
        

        if let Some(lhs) = def {
            self.values.insert(
                lhs.clone(),
                Value(Set::from([self.curr_inst.clone().unwrap()])),
            );
        }

        Set::new()
    }

    fn analyze_bb(&self, bb: &BasicBlock, cfg: &Cfg) -> (Vec<Self>, Set<BbId>) {
        let mut v = vec![];
        let mut s = self.clone();

        for (i, inst) in bb.insts.iter().enumerate() {
            s.curr_inst = Some((bb.id.clone(), i));
            s.analyze_inst(inst, cfg);
            v.push(s.clone());
        }

        s.curr_inst = Some((bb.id.clone(), bb.insts.len()));
        s.analyze_term(&bb.term, cfg);
        v.push(s.clone());

        (v, Set::new())
    }
}
