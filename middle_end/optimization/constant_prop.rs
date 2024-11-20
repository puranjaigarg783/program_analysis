//! Constant folding & propagation optimization.

use super::*;
use crate::commons::*;
use crate::middle_end::analysis::{constant_prop::*, *};
use crate::middle_end::lir::*;

/// The actual optimization pass.
pub fn constant_prop(valid_program: Valid<Program>) -> Valid<Program> {
    let mut program = valid_program.0.clone();

    program.functions = program
        .functions
        .iter()
        .map(|(id, f)| {
            let (pre_bb, pre_inst) = analyze(&valid_program, id.clone());
            (id.clone(), constant_prop_func(pre_bb, pre_inst, f))
        })
        .collect();

    // Do not remove this validation check.  It is there to help you catch the
    // bugs early on.  The autograder uses an internal final validation check.
    program.validate().unwrap()
}

fn constant_prop_func(
    _pre_bb: Map<BbId, Env>,
    pre_inst: Map<InstId, Env>,
    func: &Function,
) -> Function {
    let mut opt_func = func.clone();

    for (bbid, bb) in &mut opt_func.body {
        let mut opt_insts: Vec<Instruction> = Vec::new();

        for (idx, inst) in bb.insts.iter().enumerate() {
            let instid: InstId = (bbid.clone(), idx + 1); 
            let env = pre_inst.get(&instid).unwrap(); // poststate?

            opt_insts.push(optimize_instruction(inst, env));
        }
        
        bb.insts = opt_insts;

        match bb.term.clone() {
            Terminal::Ret(Some(var)) => {
                let term_instid: InstId = (bbid.clone(), bb.insts.len());
                let env = pre_inst.get(&term_instid).unwrap();
                bb.term = Terminal::Ret(Some(collapse_op(&var, env)))
            }
            Terminal::Branch { cond, ff, tt } => {
                let term_instid: InstId = (bbid.clone(), bb.insts.len());
                let env = pre_inst.get(&term_instid).unwrap();
                bb.term = Terminal::Branch {
                    cond: collapse_op(&cond, env),
                    ff: ff.clone(),
                    tt: tt.clone(),
                }
            }
            Terminal::CallDirect {
                lhs,
                callee,
                args,
                next_bb,
            } => {
                let term_instid: InstId = (bbid.clone(), bb.insts.len());
                let env = pre_inst.get(&term_instid).unwrap();
                bb.term = Terminal::CallDirect {
                    lhs,
                    callee,
                    args: collapse_op_vec(&args, env),
                    next_bb,
                }
            }
            Terminal::CallIndirect {
                lhs,
                callee,
                args,
                next_bb,
            } => {
                let term_instid: InstId = (bbid.clone(), bb.insts.len());
                let env = pre_inst.get(&term_instid).unwrap();
                bb.term = Terminal::CallIndirect {
                    lhs,
                    callee,
                    args: collapse_op_vec(&args, env),
                    next_bb,
                }
            }
            _ => (),
        }
    }
    
    opt_func.clone()
}

fn copyable_check(var: &VarId, env: &Env) -> Option<Instruction> {
    match env.get(var) {
        Value::Top => None,
        Value::Int(n) => {
            let op = Operand::CInt(n.try_into().unwrap());
            Some(Instruction::Copy {
                lhs: var.clone(),
                op,
            })
        }
        Value::Bot => None,
    }
}

fn optimize_instruction(inst: &Instruction, env: &Env) -> Instruction {
    use Instruction::*;
    match inst {
        Alloc { lhs, num, id } => Alloc {
            lhs: lhs.clone(),
            num: collapse_op(num, env),
            id: id.clone(),
        },
        Arith { lhs, aop, op1, op2 } => {
            if let Some(copy_inst) = copyable_check(lhs, env) {
                copy_inst
            } else if op_is_not_bottom(Operand::Var(lhs.clone()), env) {
                Arith {
                    aop: *aop,
                    lhs: lhs.clone(),
                    op1: collapse_op(op1, env),
                    op2: collapse_op(op2, env),
                }
            } else {
                println!("sussy activity: lhs: {lhs}, aop: {aop}, op1: {op1}, op2: {op2}");
                inst.clone()
            }
        }
        CallExt {
            lhs,
            ext_callee,
            args,
        } => CallExt {
            lhs: lhs.clone(),
            ext_callee: ext_callee.clone(),
            args: collapse_op_vec(args, env),
        },
        Cmp { lhs, rop, op1, op2 } => {
            if let Some(copy_inst) = copyable_check(lhs, env) {
                copy_inst
            } else if op_is_not_bottom(Operand::Var(lhs.clone()), env) {
                Cmp {
                    rop: *rop,
                    lhs: lhs.clone(),
                    op1: collapse_op(op1, env),
                    op2: collapse_op(op2, env),
                }
            } else {
                inst.clone()
            }
        }
        Copy { lhs, op } => Copy {
            lhs: lhs.clone(),
            op: collapse_op(op, env),
        },
        Gep { lhs, src, idx } => Gep {
            lhs: lhs.clone(),
            src: src.clone(),
            idx: collapse_op(idx, env),
        },
        Gfp {
            lhs: _,
            src: _,
            field: _,
        } => inst.clone(),
        Load { lhs: _, src: _ } => inst.clone(),
        Store { dst, op } => Store {
            dst: dst.clone(),
            op: collapse_op(op, env),
        },
        _ => inst.clone(),
    }
}

fn collapse_op_vec(op_vec: &Vec<Operand>, env: &Env) -> Vec<Operand> {
    let mut collapsed_vec = vec![];

    for op in op_vec {
        collapsed_vec.push(collapse_op(op, env));
    }

    collapsed_vec
}

fn collapse_op(op: &Operand, env: &Env) -> Operand {
    match op {
        Operand::Var(var) => match env.get(var) {
            Value::Top => op.clone(),
            Value::Int(n) => Operand::CInt(n.try_into().unwrap()),
            Value::Bot => op.clone(),
        },
        Operand::CInt(_) => op.clone(),
    }
}

fn op_is_not_bottom(op: Operand, env: &Env) -> bool {
    match &op {
        Operand::Var(var) => match env.get(var) {
            Value::Top => true,
            Value::Int(_) => true,
            Value::Bot => false,
        },
        Operand::CInt(_) => true,
    }
}
