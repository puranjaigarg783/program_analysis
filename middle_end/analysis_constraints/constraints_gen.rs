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
pub fn analyze(program: &Valid<Program>, func: FuncId) -> Set<Constraint> {
    fn get_func_rets(functions: &Map<FuncId, Function>) -> Map<FuncId, Option<Operand>> {
        let mut func_rets: Map<FuncId, Option<Operand>> = Map::new();
        for (fid, f) in functions {
            func_rets.insert(fid.clone(), None);
            for bb in f.body.values() {
                if let Terminal::Ret(ret) = &bb.term {
                    func_rets.insert(fid.clone(), ret.clone());
                }
            }
        };

        func_rets
    }

    let program = &program.0;
    let f = &program.functions[&func];

    let init_store = Env::new(Map::new());
    let mut soln: Set<Constraint> = Set::new();

    let func_rets = get_func_rets(&program.functions);
    for global in &program.globals {
        if global.typ().base_typ().is_function() {
            let function = &program.functions[&func_id(&global.0.name)];

            soln.insert(Constraint(
                ConstraintExp::Lam {
                    name: global.name().to_string(),
                    param_ty: function.params.iter().map(|x| x.typ()).collect(),
                    ret_ty: function.ret_ty.clone(),
                    ret_op: func_rets[&func_id(&global.0.name)].clone(),
                    args: function.params.clone(),
                },
                ConstraintExp::Var(global.clone())
            ));
        }
    }
    forward_analysis(f, &Cfg::new(f, program.globals.clone(), program.structs.clone(), program, func_rets), &init_store, &init_store, &mut soln)
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

    fn analyze_inst(&mut self, inst: &Instruction, cfg: &Cfg, soln: &mut Set<Constraint>, store: &mut Map<VarId, Set<InstId>>) {
        use Instruction::*;

        let this_inst = &self.curr_inst.clone().unwrap();
        let this_pp = ProgramPoint::from_instid(this_inst.clone());
        match inst {
            AddrOf { lhs, op } => {
                if lhs.typ().is_ptr() {
                    soln.insert(Constraint(
                        ConstraintExp::Ref(op.clone(), op.clone()),
                        ConstraintExp::Var(lhs.clone())
                    ));
                }
            },
            Alloc { lhs, num, id } => {
                if lhs.typ().is_ptr() {
                    soln.insert(Constraint(
                        ConstraintExp::Ref(id.clone(), id.clone()),
                        ConstraintExp::Var(lhs.clone())
                    ));
                }
            },
            Arith { lhs, aop:_, op1, op2 } => {
                // Can be ignored, does not affect pointers.
            },
            Cmp { lhs, rop:_, op1, op2 } => {
                // Can be ignored, does not affect pointers.
            },
            CallExt { lhs, ext_callee, args } => {
                // IGNORE
            },
            Copy { lhs, op } => {
                
                if lhs.typ().is_ptr() {
                    if let Operand::Var(var) = op {
                        soln.insert(Constraint(
                            ConstraintExp::Var(var.clone()),
                            ConstraintExp::Var(lhs.clone())
                        ));
                    };
                }
            },
            Gep {
                lhs,
                src,
                idx,
            } => {
                if lhs.typ().is_ptr() {
                    soln.insert(Constraint(
                        ConstraintExp::Var(src.clone()),
                        ConstraintExp::Var(lhs.clone())
                    ));
                }
            },
            Gfp { lhs, src, field } => {
                if lhs.typ().is_ptr() {
                    soln.insert(Constraint(
                        ConstraintExp::Var(src.clone()),
                        ConstraintExp::Var(lhs.clone())
                    ));
                }
            }, 
            Load { lhs, src } => {
                if lhs.typ().is_ptr() {
                    soln.insert(Constraint(
                        ConstraintExp::Proj(src.clone()),
                        ConstraintExp::Var(lhs.clone())
                    ));
                }
            },
            Store { dst, op } => {
                if let Operand::Var(var) = op {
                    if op.typ().is_ptr() {
                        soln.insert(Constraint(
                            ConstraintExp::Var(var.clone()),
                            ConstraintExp::Proj(dst.clone())
                        ));
                    }
                };
            },
            Phi { .. } => unreachable!(),
        };
    }

    fn analyze_term(&mut self, term: &Terminal, cfg: &Cfg, soln: &mut Set<Constraint>, store: &mut Map<VarId, Set<InstId>>) -> Set<BbId> {
        use Terminal::*;
        let this_pp = ProgramPoint::from(self.curr_inst.clone().unwrap().0, None);
        match term {
            CallDirect { lhs, callee, args, next_bb } => {

                // function return is subset of lhs
                if let Some(lhs_var) = lhs {
                    if lhs_var.typ().is_ptr() {
                        if let Some(Operand::Var(ret_var)) = &cfg.func_rets[callee] {
                            soln.insert(Constraint(
                                ConstraintExp::Var(ret_var.clone()),
                                ConstraintExp::Var(lhs_var.clone())
                            ));
                        }
                    }
                } else {
                    // nothing?
                }

                // each arg is subset of function param
                let callee_args = &cfg.program.functions[callee].params;
                for (param, arg) in callee_args.iter().zip(args) {
                    if param.typ().is_ptr() {
                        if let Operand::Var(arg_var) = arg {
                            soln.insert(Constraint(
                                ConstraintExp::Var(arg_var.clone()),
                                ConstraintExp::Var(param.clone())
                            ));
                        }
                    }
                }
            },
            CallIndirect { lhs, callee, args, next_bb } => {
                if let LirType::Function{ ret_ty, param_ty} = callee.typ().base_typ().0.get().clone() {
                    soln.insert(Constraint(
                        ConstraintExp::Var(callee.clone()),
                        ConstraintExp::Lam {
                            name: "_DUMMY".to_string(),
                            param_ty,
                            ret_ty: ret_ty.clone(),
                            ret_op: {
                                if let Some(ret_var) = lhs {
                                    Some(Operand::Var(ret_var.clone()))
                                } else {
                                    Some(Operand::Var(var_id("_DUMMY", ret_ty.clone().unwrap(), None)))
                                }
                            },
                            args: args.iter().filter_map(|x| if let Operand::Var(arg_var) = x {
                                Some(arg_var.clone())
                            } else {
                                None
                            }).collect(),
                        }
                    ));
                };

                
            },
            Branch { cond, .. } => {
                // Can be ignored, flow-insensitive.
            },
            Ret(Some(op)) => {
                // IGNORE
            },
            _ => (), // IGNORE
        };

        Set::new() // relic from flow analysis
    }

    fn analyze_bb(&self, bb: &BasicBlock, cfg: &Cfg, soln: &mut Set<Constraint>, store: &mut Map<VarId, Set<InstId>>) -> (Vec<Self>, Set<BbId>) {
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