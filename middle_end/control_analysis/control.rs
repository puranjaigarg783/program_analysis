use derive_more::Display;

use crate::commons::Valid;

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Value(pub Set<BbId>);

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let len = self.0.len();
        for (i, bb) in self.0.iter().enumerate() {
            if i + 1 == len {
                write!(f, "{bb}")?;
            } else {
                write!(f, "{bb}, ")?;
            }
        }
        write!(f, "}}")
    }
}


// Abstract environment
pub type Env = PointwiseEnv<Value>;

pub fn analyze2(program: &Valid<Program>, func: FuncId) -> (Map<BbId, Env>) {
    let program = &program.0;
    let f = &program.functions[&func];

    let init_store = Env::new(Map::new());
    let full_set: Set<BbId> = f.body.keys().cloned().collect();
    let full_set_val = Value(full_set);
    
    forward_analysis(f, &Cfg::new(f, program.globals.clone(), program.structs.clone()), &init_store, &init_store)
}

pub fn analyze(program: &Valid<Program>, func: FuncId) -> Map<BbId, Set<BbId>> {
    let program = &program.0;
    let f = &program.functions[&func];

    let init_store = Env::new(Map::new());
    let full_set: Set<BbId> = f.body.keys().cloned().collect();
    let full_set_val = Value(full_set);

    all_roads_lead_to_me(f, &Cfg::new(f, program.globals.clone(), program.structs.clone()))
}

pub fn analyze_postdom(program: &Valid<Program>, func: FuncId) -> Map<BbId, Set<BbId>> {
    let program = &program.0;
    let f = &program.functions[&func];

    let init_store = Env::new(Map::new());
    let full_set: Set<BbId> = f.body.keys().cloned().collect();
    let full_set_val = Value(full_set);

    all_roads_lead_to_me(f, &Cfg::new(f, program.globals.clone(), program.structs.clone()).reversed())
}

impl AbstractValue for Value {
    type Concrete = BbId;

    const BOTTOM: Self = Value(Set::new());

    fn alpha(def: BbId) -> Self {
        Value(Set::from([def]))
    }

    fn join(&self, rhs: &Self) -> Value {
        Value(self.0.intersection(&rhs.0).cloned().collect())
    }
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
    }

    fn analyze_term(&mut self, term: &Terminal, cfg: &Cfg) {
        let this_bbid = &self.curr_inst.clone().unwrap().0;
        use Terminal::*;

        let mut new_set: Set<BbId> = Set::new();
        new_set.insert(this_bbid.clone());

        match term {
            Branch {
                cond,
                tt,
                ff,
            } => {
                new_set.insert(tt.clone());
                new_set.insert(ff.clone());
                
            }
            CallDirect {
                lhs,
                callee: _,
                args,
                next_bb,
            } => {
                new_set.insert(next_bb.clone());
                
            },
            CallIndirect {
                lhs,
                callee: _,
                args,
                next_bb,
            } => {
                new_set.insert(next_bb.clone());
            },
            Jump(next_bb) => {
                new_set.insert(next_bb.clone());
            }
            Ret(_) => {
            }
            _ => (),
        }

        self.insert(&this_bbid, &Value(new_set));
    }

    fn analyze_bb(&self, bb: &BasicBlock, cfg: &Cfg) -> Self {
        let mut s = self.clone();

        s.curr_inst = Some((bb.id.clone(), bb.insts.len()));
        
        s.analyze_term(&bb.term, cfg);
        s
    }
}