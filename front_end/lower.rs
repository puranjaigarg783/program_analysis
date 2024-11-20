// lower the AST to Lir. assumes the AST is valid; may panic if it is not.

use std::mem::swap;

use super::*;
use crate::middle_end::lir::{
    self, bb_id, field_id, func_id, struct_id, var_id, Instruction, LirOp,
};

// SECTION: public interface

pub fn lower(ast: &Valid<Program>) -> lir::Program {
    // initialize the variable information data structure with non-function-specific
    // info; everything else will be filled in per-function by lower_functions().
    let mut info = Lowering::new();
    info.externs = lower_externs(&ast.0.externs);
    info.structs = lower_structs(&ast.0.typedefs);
    info.globals = lower_globals(&ast.0.globals);

    // fills in more info.globals info too, so this needs to come before copying
    // info.globals into the Program.
    let functions = lower_functions(&ast.0.functions, &mut info);

    lir::Program {
        structs: info.structs,
        globals: info.globals,
        externs: info.externs,
        functions,
    }
}

// SECTION: utilities

#[derive(Clone, Debug)]
struct Lowering {
    externs: Map<lir::FuncId, Type>,                // external functions
    structs: Map<lir::StructId, Set<lir::FieldId>>, // struct type info
    globals: Set<lir::VarId>,                       // global variables
    func_names: Set<lir::FuncId>,                   // function names
    curr_func: Option<lir::FuncId>,                 // current function
    params: Vec<lir::VarId>,                        // per-function parameters
    locals: Set<lir::VarId>,                        // per-function locals
    loop_info: Vec<(lir::BbId, lir::BbId)>,         // stack of loop header and loop exit blocks.
    tmp_ctr: u32,                                   // for generating fresh temporary variables
    bb_ctr: u32,                                    // for generating fresh basic blocks
}

impl Lowering {
    fn new() -> Self {
        Lowering {
            externs: Map::new(),
            structs: Map::new(),
            globals: Set::new(),
            func_names: Set::new(),
            curr_func: None,
            params: vec![],
            locals: Set::new(),
            loop_info: vec![],
            tmp_ctr: 0,
            bb_ctr: 0,
        }
    }

    // reset everything that's function-specific.
    fn reset(&mut self) {
        self.curr_func = None;
        self.params.clear();
        self.locals.clear();
        self.loop_info = vec![];
        self.tmp_ctr = 0;
        self.bb_ctr = 0;
    }

    // creates a fresh temporary variable with the given prefix and records it in
    // self.locals.
    fn create_tmp(&mut self, typ: &Type, prefix: &str) -> lir::VarId {
        self.tmp_ctr += 1;
        let tmp = var_id(
            &(prefix.to_string() + &self.tmp_ctr.to_string()),
            typ.clone(),
            self.curr_func.clone(),
        );
        self.locals.insert(tmp.clone());
        tmp
    }

    // creates a fresh basic block label.
    fn create_bb(&mut self) -> lir::BbId {
        self.bb_ctr += 1;
        bb_id(&("bb".to_string() + &self.bb_ctr.to_string()))
    }

    // looks up name in locals, parameters, and globals (in that order) to get
    // the corresponding VarId.
    fn name_to_var(&self, name: &str) -> lir::VarId {
        match self.locals.iter().find(|v| v.name() == name) {
            Some(var) => var.clone(),
            None => match self.params.iter().find(|v| v.name() == name) {
                Some(var) => var.clone(),
                None => match self.globals.iter().find(|v| v.name() == name) {
                    Some(var) => var.clone(),
                    None => unreachable!("name_to_var: {name:#?}"),
                },
            },
        }
    }

    // returns whether name is an extern (consulting locals and parameters to make
    // sure the name hasn't been shadowed). we don't need to look in globals because
    // a valid program can't have any overlap between externs and globals.
    fn is_extern(&self, name: &str) -> bool {
        match self.locals.iter().find(|v| v.name() == name) {
            Some(_) => false,
            None => match self.params.iter().find(|v| v.name() == name) {
                Some(_) => false,
                None => self.externs.contains_key(&func_id(name)),
            },
        }
    }

    // returns whether id is a global function pointer with the same name as an
    // internal function.
    fn is_internal_func(&self, id: &lir::VarId) -> bool {
        id.typ().is_ptr()
            && id.typ().get_deref_type().unwrap().is_function()
            && id.scope().is_none()
            && self.func_names.contains(&func_id(id.name()))
    }

    // returns the field id with the given name of a given struct type.
    fn get_field_by_name(&self, struct_id: &lir::StructId, field_name: &str) -> lir::FieldId {
        match self.structs[struct_id]
            .iter()
            .find(|f| *f.name == field_name)
        {
            Some(field) => field.clone(),
            None => unreachable!(),
        }
    }
}

// add an instruction to the end of the curr_bb basic block.
fn add_inst(
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    curr_bb: &lir::BbId,
    inst: lir::Instruction,
) {
    body.get_mut(curr_bb).unwrap().insts.push(inst);
}

// set the terminal of the curr_bb basic block, which should be a sentinel
// value.
fn set_terminal(
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    curr_bb: &lir::BbId,
    term: lir::Terminal,
) {
    // the terminal should be a sentinel.
    // println!("{curr_bb:#?}, {term:#?}");
    assert!(
        matches!(&body[curr_bb].term, lir::Terminal::Jump(bb) if bb.name() == "_SENTINEL"),
        "terminal isn't a sentinel value: {:?}",
        &body[curr_bb].term
    );
    body.get_mut(curr_bb).unwrap().term = term;
}

// reset the terminal of the curr_bb basic block, which should not be a sentinel
// value.
fn reset_terminal(
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    curr_bb: &lir::BbId,
    term: lir::Terminal,
) {
    // the terminal should not be a sentinel.
    assert!(!matches!(&body[curr_bb].term, lir::Terminal::Jump(bb) if bb.name() == "_SENTINEL"));
    body.get_mut(curr_bb).unwrap().term = term;
}

// SECTION: lowering implementation

fn lower_structs(typedefs: &[Typedef]) -> Map<lir::StructId, Set<lir::FieldId>> {
    typedefs
        .iter()
        .map(|Typedef { name, fields }| {
            let id = struct_id(name);
            let fields = fields
                .iter()
                .map(|Decl { name, typ }| field_id(name, typ.clone()))
                .collect();
            (id, fields)
        })
        .collect()
}

fn lower_globals(globals: &[Decl]) -> Set<lir::VarId> {
    globals
        .iter()
        .map(|Decl { name, typ }| var_id(name, typ.clone(), None))
        .collect()
}

fn lower_externs(externs: &[Decl]) -> Map<lir::FuncId, Type> {
    externs
        .iter()
        .map(|Decl { name, typ }| (func_id(name), typ.clone()))
        .collect()
}

fn lower_functions(functions: &[Function], info: &mut Lowering) -> Map<lir::FuncId, lir::Function> {
    // record all internally-defined function names and create global function
    // pointers to all functions except main. translating function calls requires
    // this info, so it needs to be done before lowering each individual function.
    for func in functions {
        info.func_names.insert(func_id(&func.name));
        if func.name != "main" {
            info.globals.insert(var_id(
                &func.name,
                ptr_ty(func_ty(
                    func.rettyp.clone(),
                    func.params
                        .iter()
                        .map(|Decl { typ, .. }| typ.clone())
                        .collect(),
                )),
                None,
            ));
        }
    }

    functions
        .iter()
        .map(|func| {
            info.reset();

            // the function identifier.
            let id = func_id(&func.name);

            // initialize info with function-specific information.
            info.curr_func = Some(id.clone());
            info.params = lower_params(&func.params, id.clone());
            info.locals = lower_locals(&func.body.decls, id.clone());

            // eliminate local variable initializations.
            let stmts = eliminate_inits(&func.body);

            // lower the function body (assumes there are no local initializations or
            // logical operators, per the above transformations).
            let mut body = Map::new();
            let fin = lower_stmts(&stmts, &mut body, bb_id("entry"), info);
            println!("fin: {fin:#?}, func name: {:#?}", func.name);
            assert!(fin.is_none());

            // guarantee there is a single return statement.
            eliminate_multiple_ret(&mut body, &func.rettyp, info);

            // the lowered function, minus the parameters and locals.
            let mut lir_func = lir::Function {
                id: id.clone(),
                ret_ty: func.rettyp.clone(),
                params: vec![],
                locals: Set::new(),
                body,
            };

            // put the final versions of the parameters and locals into the lir function.
            swap(&mut lir_func.params, &mut info.params);
            swap(&mut lir_func.locals, &mut info.locals);

            (id, lir_func)
        })
        .collect()
}

fn lower_params(params: &[Decl], func: lir::FuncId) -> Vec<lir::VarId> {
    params
        .iter()
        .map(|Decl { name, typ }| var_id(name, typ.clone(), Some(func.clone())))
        .collect()
}

fn lower_locals(locals: &[(Decl, Option<Exp>)], func: lir::FuncId) -> Set<lir::VarId> {
    locals
        .iter()
        .map(|(Decl { name, typ }, _)| var_id(name, typ.clone(), Some(func.clone())))
        .collect()
}

// curr_bb is the basic block we're currently inserting instructions into; the
// function returns the id of the basic block ending the lowering of stmts
// unless that block is already terminal (i.e., cannot have any instruction or
// terminal added).
fn lower_stmts(
    stmts: &[Stmt],
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    mut curr_bb: lir::BbId,
    info: &mut Lowering,
) -> Option<lir::BbId> {
    // create the basic block that we're inserting instructions into by inserting a
    // basic block with the given label, using '$jump _SENTINEL' as a sentinel for
    // the terminal indicating it hasn't been given a real value yet.
    assert!(!body.contains_key(&curr_bb));
    body.insert(
        curr_bb.clone(),
        lir::BasicBlock {
            id: curr_bb.clone(),
            insts: vec![],
            term: lir::Terminal::Jump(bb_id("_SENTINEL")),
        },
    );

    // lower each statement in turn.
    for stmt in stmts {
        match stmt {
            Stmt::If { guard, tt, ff } => match lower_if(guard, tt, ff, body, &curr_bb, info) {
                Some(bb) => curr_bb = bb,
                None => return None,
            },
            Stmt::While {
                guard,
                body: while_body,
            } => curr_bb = lower_while(guard, while_body, body, &curr_bb, info),
            Stmt::Assign { lhs, rhs } => curr_bb = lower_assign(lhs, rhs, body, &curr_bb, info),
            Stmt::Call { callee, args } => curr_bb = lower_call(callee, args, body, &curr_bb, info),
            Stmt::Break => {
                println!(
                    "info.loop_info.last().unwrap().1.clone(): {:#?}",
                    info.loop_info.last().unwrap().1.clone()
                );
                set_terminal(
                    body,
                    &curr_bb,
                    lir::Terminal::Jump(info.loop_info.last().unwrap().1.clone()),
                );
                return None;
            }
            Stmt::Continue => {
                println!(
                    "info.loop_info.last().unwrap().0.clone(): {:#?}",
                    info.loop_info.last().unwrap().0.clone()
                );
                set_terminal(
                    body,
                    &curr_bb,
                    lir::Terminal::Jump(info.loop_info.last().unwrap().0.clone()),
                );
                return None;
            }
            Stmt::Return(op) => {
                match op {
                    Some(exp) => {
                        let (op, bb) = lower_exp_to_operand(exp, body, &curr_bb, info);
                        curr_bb = bb;
                        set_terminal(body, &curr_bb, lir::Terminal::Ret(Some(op)));
                    }
                    None => {
                        set_terminal(body, &curr_bb, lir::Terminal::Ret(None));
                    }
                }
                return None;
            }
        }
    }

    Some(curr_bb)
}

// returns the join basic block, or None if both branches of the If end in
// Break/Continue/Return and hence there is no join.
fn lower_if(
    guard: &Exp,
    tt: &[Stmt],
    ff: &[Stmt],
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    curr_bb: &lir::BbId,
    info: &mut Lowering,
) -> Option<lir::BbId> {
    let (cond, bb_after_eval) = lower_exp_to_operand(guard, body, curr_bb, info);

    let tt_bb = info.create_bb();
    let ff_bb = info.create_bb();
    let join_bb = info.create_bb();

    set_terminal(
        body,
        &bb_after_eval,
        lir::Terminal::Branch {
            cond,
            tt: tt_bb.clone(),
            ff: ff_bb.clone(),
        },
    );

    let mut no_join = true;

    if let Some(bb) = lower_stmts(tt, body, tt_bb.clone(), info) {
        set_terminal(body, &bb, lir::Terminal::Jump(join_bb.clone()));
        no_join = false;
    }

    if let Some(bb) = lower_stmts(ff, body, ff_bb.clone(), info) {
        set_terminal(body, &bb, lir::Terminal::Jump(join_bb.clone()));
        no_join = false;
    }

    if !no_join {
        assert!(!body.contains_key(&join_bb)); // this should never happen
        body.insert(
            join_bb.clone(),
            lir::BasicBlock {
                id: join_bb.clone(),
                insts: vec![],
                term: lir::Terminal::Jump(bb_id("_SENTINEL")),
            },
        );
        Some(join_bb)
    } else {
        None
    }
}

// returns the loop exit basic block.
fn lower_while(
    guard: &Exp,
    while_body: &[Stmt],
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    curr_bb: &lir::BbId,
    info: &mut Lowering,
) -> lir::BbId {
    let cond_bb = info.create_bb();
    set_terminal(body, curr_bb, lir::Terminal::Jump(cond_bb.clone())); // curr_bb jump to conditional block
    let while_bb = info.create_bb();
    let exit_bb = info.create_bb();
    body.insert(
        // create body for conditional block
        cond_bb.clone(),
        lir::BasicBlock {
            id: cond_bb.clone(),
            insts: vec![],
            term: lir::Terminal::Jump(bb_id("_SENTINEL")),
        },
    );

    // we do these after creating basic block so insts don't overwrite
    let (cond, bb_after_eval) = lower_exp_to_operand(guard, body, &cond_bb, info); // insert conditional calculation if any
    set_terminal(
        body,
        &bb_after_eval,
        lir::Terminal::Branch {
            cond,
            tt: while_bb.clone(),
            ff: exit_bb.clone(),
        },
    ); // bb_after_eval branch to loop body or exit

    info.loop_info.push((cond_bb.clone(), exit_bb.clone()));

    assert!(!body.contains_key(&exit_bb)); // this should never happen
    body.insert(
        // create exit block
        exit_bb.clone(),
        lir::BasicBlock {
            id: exit_bb.clone(),
            insts: vec![],
            term: lir::Terminal::Jump(bb_id("_SENTINEL")),
        },
    );

    if let Some(bb) = lower_stmts(while_body, body, while_bb.clone(), info) {
        set_terminal(body, &bb, lir::Terminal::Jump(cond_bb)); // jump back to check condition again
    }

    info.loop_info.pop();
    exit_bb
}

fn lower_assign(
    lhs: &Lval,
    rhs: &Rhs,
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    curr_bb: &lir::BbId,
    info: &mut Lowering,
) -> lir::BbId {
    // NOTE: in direct assignments, you should emit a $copy instruction, in
    // indirect assignments you should emit a $store instruction.
    match rhs {
        Rhs::Exp(exp) => {
            let (lhs, copy) = lower_lval(lhs, body, curr_bb, info);
            let (op, bbid_new) = lower_exp_to_operand(exp, body, curr_bb, info);
            if copy {
                let inst = lir::Instruction::Copy { lhs, op };

                add_inst(body, &bbid_new, inst);
            } else {
                let inst = lir::Instruction::Store { dst: lhs, op };

                add_inst(body, &bbid_new, inst);
            }

            bbid_new
        }
        Rhs::New { typ, num } => {
            let (lhs, copy) = lower_lval(lhs, body, curr_bb, info);

            let mut size = lir::Operand::CInt(1);
            let mut bbid_new = curr_bb.clone();

            if let Some(num_exp) = num {
                (size, bbid_new) = lower_exp_to_operand(num_exp, body, curr_bb, info);
            }

            //let tmp = info.create_tmp(typ, "_t");
            let alloc_var = info.create_tmp(typ, "_alloc");

            if copy {
                let inst_alloc = lir::Instruction::Alloc {
                    lhs,
                    num: size,
                    id: alloc_var,
                };

                add_inst(body, &bbid_new, inst_alloc);
            } else if let Some(deref_type) = lhs.typ().get_deref_type() {
                let deref_ptr = info.create_tmp(deref_type, "_t");
                let inst_alloc = lir::Instruction::Alloc {
                    lhs: deref_ptr.clone(),
                    num: size,
                    id: alloc_var,
                };
                add_inst(body, &bbid_new, inst_alloc);

                let inst_load = Instruction::Store {
                    dst: lhs.clone(),
                    op: lir::Operand::Var(deref_ptr.clone()),
                };
                add_inst(body, curr_bb, inst_load);
            }

            bbid_new
        }
    }
}

// returns the call-return basic block, or the current basic block if this is a
// call to an external function.
//
// NOTE: This is extremely similar to the call expression implementation.
fn lower_call(
    callee: &Lval,
    args: &[Exp],
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    curr_bb: &lir::BbId,
    info: &mut Lowering,
) -> lir::BbId {
    fn create_load(
        ptr: &lir::VarId,
        body: &mut Map<lir::BbId, lir::BasicBlock>,
        curr_bb: &lir::BbId,
        info: &mut Lowering,
    ) -> lir::VarId {
        if let Some(deref_type) = ptr.typ().get_deref_type() {
            let deref_ptr = info.create_tmp(deref_type, "_t");
            if !deref_ptr.typ().is_ptr() {
                panic!("create_load derefed into non-pointer, shouldn't happen?")
            }
            let inst_load = Instruction::Load {
                lhs: deref_ptr.clone(),
                src: ptr.clone(),
            };
            add_inst(body, curr_bb, inst_load);

            deref_ptr
        } else {
            panic!("Tried to create_load on non-pointer, shouldn't happen.")
        }
    }

    let curr_bb = curr_bb.clone();

    // lower the arguments and collect the resulting operands; this may update the
    // current basic block.
    let (args, curr_bb) = args
        .iter()
        .fold((Vec::new(), curr_bb), |(mut acc, bb), arg| {
            let (op, ret_bb) = lower_exp_to_operand(arg, body, &bb, info);
            acc.push(op);
            (acc, ret_bb)
        });

    // the extern check has to be done before calling lower_lval() because an extern
    // doesn't have a corresponding VarId.
    match callee {
        Lval::Id(name) if info.is_extern(name) => {
            let extern_type = info.externs.get(&func_id(name)).unwrap().clone();

            let ret_type = match &*extern_type.0 {
                lir::LirType::Function {
                    ret_ty,
                    param_ty: _,
                } => ret_ty,
                _ => unreachable!("extern not function type, something is wrong in lower_call"),
            };

            let ret_var = ret_type.as_ref().map(|typ| info.create_tmp(typ, "_t"));

            let inst_ext = Instruction::CallExt {
                // only used for calls to external functions (which can only be direct calls).
                lhs: ret_var,
                ext_callee: func_id(name),
                args,
            };

            add_inst(body, &curr_bb, inst_ext);

            return curr_bb;
        }
        _ => {}
    }

    // if we're here then it's a direct or indirect call and we'll need a
    // call-return basic block.
    let next_bb = info.create_bb();

    // lower the lval to a VarId and a boolean indicating whether the VarId holds
    // the final function pointer value or is a pointer to the final function
    // pointer.
    let (mut callee, copy) = lower_lval(callee, body, &curr_bb, info);

    if !copy {
        callee = create_load(&callee, body, &curr_bb, info);
    }

    // determine if this is a direct or indirect call. callee will always be a
    // function pointer due to lowering the lval, but if it's a global function
    // pointer with the same name as an internal function then it should be a direct
    // call to that function.
    if info.is_internal_func(&callee) {
        set_terminal(
            body,
            &curr_bb,
            lir::Terminal::CallDirect {
                lhs: None,
                callee: func_id(callee.name()),
                args,
                next_bb: next_bb.clone(),
            },
        );
    } else {
        set_terminal(
            body,
            &curr_bb,
            lir::Terminal::CallIndirect {
                lhs: None,
                callee,
                args,
                next_bb: next_bb.clone(),
            },
        );
    };

    // insert next_bb into the function body, using "$jump _SENTINEL" as a sentinel
    // value for the terminal indicating it hasn't been given a real value yet.
    body.insert(
        next_bb.clone(),
        lir::BasicBlock {
            id: next_bb.clone(),
            insts: vec![],
            term: lir::Terminal::Jump(bb_id("_SENTINEL")),
        },
    );

    next_bb
}

// evaluating an expression may require multiple basic blocks if the expression
// contains a call; we return the final basic block from evaluating the
// expression along with the final operand.
fn lower_exp_to_operand(
    exp: &Exp,
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    curr_bb: &lir::BbId,
    info: &mut Lowering,
) -> (lir::Operand, lir::BbId) {
    use lir::Operand::*;

    // for each instruction kind, emit the instructions to compute that
    // expression's value, then return the operand containing the value as well
    // as the new `curr_bb`.  Only `And`, `Or`, `Call` create new basic blocks,
    // but the subexpressions you have may create basic blocks too!
    match exp {
        Exp::Num(n) => (CInt(*n), curr_bb.clone()),
        Exp::Id(name) => (Var(info.name_to_var(name)), curr_bb.clone()),
        Exp::Nil => (CInt(0), curr_bb.clone()),
        Exp::Neg(e) => {
            let (op, bb_after_e) = lower_exp_to_operand(e, body, curr_bb, info);
            let tmp = info.create_tmp(&int_ty(), "_t");
            let inst = lir::Instruction::Arith {
                lhs: tmp.clone(),
                aop: LirOp![-],
                op1: lir::Operand::CInt(0),
                op2: op,
            };

            add_inst(body, &bb_after_e, inst);
            (Var(tmp), bb_after_e.clone())
        }
        Exp::Not(e) => {
            let (op, bb_after_e) = lower_exp_to_operand(e, body, curr_bb, info);
            let tmp = info.create_tmp(&int_ty(), "_t");
            let inst = lir::Instruction::Cmp {
                lhs: tmp.clone(),
                rop: LirOp![==],
                op1: lir::Operand::CInt(0),
                op2: op,
            };

            add_inst(body, &bb_after_e, inst);
            (Var(tmp), bb_after_e.clone())
        }
        Exp::Deref(e) => {
            println!("deref e: {:#?}", e);

            let (ptr, curr_bb) = match lower_exp_to_operand(e, body, curr_bb, info) {
                (Var(p), bb_after_ptr) => (p, bb_after_ptr),
                _ => panic!("exp: trying to deref non var"),
            };

            if let Some(deref_type) = ptr.typ().get_deref_type() {
                let tmp_val = info.create_tmp(deref_type, "_t");
                let inst_load = Instruction::Load {
                    lhs: tmp_val.clone(),
                    src: ptr,
                };
                add_inst(body, &curr_bb, inst_load);
                println!("returning deref with val: {tmp_val:#?}");
                (Var(tmp_val), curr_bb)
            } else {
                panic!("exp: trying to deref non pointer")
            }
        }
        Exp::Arith(e1, op, e2) => {
            let (op1, bb_after_e1) = lower_exp_to_operand(e1, body, curr_bb, info);
            let (op2, bb_after_e2) = lower_exp_to_operand(e2, body, &bb_after_e1, info);

            let tmp = info.create_tmp(&int_ty(), "_t");

            let aop = match *op {
                ArithOp::Add => LirOp![+],
                ArithOp::Subtract => LirOp![-],
                ArithOp::Divide => LirOp![/],
                ArithOp::Multiply => LirOp![*],
            };

            let inst = lir::Instruction::Arith {
                lhs: tmp.clone(),
                aop,
                op1,
                op2,
            };

            add_inst(body, &bb_after_e2, inst);
            (Var(tmp), bb_after_e2.clone())
        }
        Exp::Compare(e1, op, e2) => {
            let (op1, bb_after_e1) = lower_exp_to_operand(e1, body, curr_bb, info);
            let (op2, bb_after_e2) = lower_exp_to_operand(e2, body, &bb_after_e1, info);

            let tmp = info.create_tmp(&int_ty(), "_t");

            let rop = match *op {
                CompareOp::Equal => LirOp![==],
                CompareOp::NotEq => LirOp![!=],
                CompareOp::Lt => LirOp![<],
                CompareOp::Lte => LirOp![<=],
                CompareOp::Gt => LirOp![>],
                CompareOp::Gte => LirOp![>=],
            };

            let inst = lir::Instruction::Cmp {
                lhs: tmp.clone(),
                rop,
                op1,
                op2,
            };

            add_inst(body, &bb_after_e2, inst);
            (Var(tmp), bb_after_e2.clone())
        }
        Exp::ArrayAccess { ptr, index } => {
            if let (Var(ptr_id), bb_after_ptr) = lower_exp_to_operand(ptr, body, curr_bb, info) {
                let (idx, bb_after_idx) = lower_exp_to_operand(index, body, &bb_after_ptr, info);

                let tmp_ptr = info.create_tmp(&ptr_id.typ(), "_t");
                let inst_gep = Instruction::Gep {
                    lhs: tmp_ptr.clone(),
                    src: ptr_id,
                    idx,
                };

                add_inst(body, &bb_after_idx, inst_gep);

                if let Some(deref_type) = tmp_ptr.typ().get_deref_type() {
                    let tmp_val = info.create_tmp(deref_type, "_t");

                    let inst_load = Instruction::Load {
                        lhs: tmp_val.clone(),
                        src: tmp_ptr,
                    };

                    add_inst(body, &bb_after_idx, inst_load);

                    (Var(tmp_val), bb_after_idx)
                } else {
                    panic!("array access not a pointer")
                }
            } else {
                panic!("array access non var")
            }
        }
        Exp::FieldAccess { ptr, field } => {
            if let (Var(src), bb_after_ptr) = lower_exp_to_operand(ptr, body, curr_bb, info) {
                if let Some(deref_type) = src.typ().get_deref_type() {
                    if let lir::LirType::Struct(id) = deref_type.0.get() {
                        let field_id = info.get_field_by_name(&id.clone(), field);
                        let tmp_ptr = info.create_tmp(&ptr_ty(field_id.clone().typ), "_t");
                        let inst_gfp = Instruction::Gfp {
                            lhs: tmp_ptr.clone(),
                            src,
                            field: field_id.clone(),
                        };

                        add_inst(body, &bb_after_ptr, inst_gfp);
                        let ret_val = info.create_tmp(&field_id.clone().typ, "_t");

                        let inst_load = Instruction::Load {
                            lhs: ret_val.clone(),
                            src: tmp_ptr,
                        };

                        add_inst(body, &bb_after_ptr, inst_load);
                        (Var(ret_val), bb_after_ptr)
                    } else {
                        panic!("Exp: field accessing non struct, ptr: {ptr:?}, field: {field:?}\nsrc: {src:?}")
                    }
                } else {
                    panic!("Exp: field access var that is not pointer")
                }
            } else {
                panic!("Exp: field access non var, ptr: {ptr:?}, field: {field:?}")
            }
        }
        Exp::Call { callee, args } => {
            // println!("exp call: callee: {:#?}", callee);
            let curr_bb = curr_bb.clone();

            // lower the arguments and collect the resulting operands; this may update the
            // current basic block.
            let (args, curr_bb) = args
                .iter()
                .fold((Vec::new(), curr_bb), |(mut acc, bb), arg| {
                    let (op, ret_bb) = lower_exp_to_operand(arg, body, &bb, info);
                    acc.push(op);
                    (acc, ret_bb)
                });

            // handle extern calls.
            match &**callee {
                Exp::Id(name) if info.is_extern(name) => {
                    // SAFE to unwrap here as is_extern verifies existence
                    let extern_type = info.externs.get(&func_id(name)).unwrap().clone();

                    let ret_type = match &*extern_type.0 {
                        lir::LirType::Function {
                            ret_ty,
                            param_ty: _,
                        } => ret_ty,
                        _ => unreachable!(
                            "extern not function type, something is really wrong in lower_call"
                        ),
                    };

                    let ret_var = if let Some(typ) = ret_type {
                        info.create_tmp(typ, "_t")
                    } else {
                        panic!("trying to assign extern with no return type to variable")
                    };

                    let inst_ext = Instruction::CallExt {
                        // only used for calls to external functions (which can only be direct calls).
                        lhs: Some(ret_var.clone()),
                        ext_callee: func_id(name),
                        args,
                    };

                    add_inst(body, &curr_bb, inst_ext);

                    // TODO what should i return if extern has no return type
                    return (lir::Operand::Var(ret_var), curr_bb);
                }
                _ => {}
            }

            // emit lhs = $call_{dir, idr} callee(args)
            let (callee_op, curr_bb) = lower_exp_to_operand(callee, body, &curr_bb, info);
            // println!("exp call: callee_op: {:#?}", callee_op);

            // if we're here then it's a direct or indirect call and we'll need a
            // call-return basic block.
            let next_bb = info.create_bb();

            // the callee must be a VarId.
            let callee = match callee_op {
                lir::Operand::Var(var) => var,
                _ => unreachable!(),
            };

            // make a left-hand side variable to receive the function return value, based
            // on the function return type.
            //
            // then, emit the call instruction ($call_dir or $call_idr)
            let ret_lhs = if info.is_internal_func(&callee) {
                let ret_typ_option = match &*callee.typ().get_deref_type().unwrap().0 {
                    lir::LirType::Function {
                        ret_ty,
                        param_ty: _,
                    } => ret_ty.clone(),
                    _ => unreachable!("exp call: must be func"),
                };

                let lhs = ret_typ_option.map(|t| info.create_tmp(&t, "_t"));

                set_terminal(
                    body,
                    &curr_bb,
                    lir::Terminal::CallDirect {
                        lhs: lhs.clone(),
                        callee: func_id(callee.name()),
                        args,
                        next_bb: next_bb.clone(),
                    },
                );
                lhs.unwrap()
            } else {
                let ret_typ_option = match &*callee.typ().get_deref_type().unwrap().0 {
                    lir::LirType::Function {
                        ret_ty,
                        param_ty: _,
                    } => ret_ty.clone(),
                    _ => unreachable!("exp call: must be func"),
                };

                let lhs = ret_typ_option.map(|t| info.create_tmp(&t, "_t"));

                set_terminal(
                    body,
                    &curr_bb,
                    lir::Terminal::CallIndirect {
                        lhs: lhs.clone(),
                        callee,
                        args,
                        next_bb: next_bb.clone(),
                    },
                );
                lhs.unwrap()
            };

            // insert next_bb into the function body, using "$jump _SENTINEL" as a sentinel
            // value for the terminal indicating it hasn't been given a real value yet.
            body.insert(
                next_bb.clone(),
                lir::BasicBlock {
                    id: next_bb.clone(),
                    insts: vec![],
                    term: lir::Terminal::Jump(bb_id("_SENTINEL")),
                },
            );

            (lir::Operand::Var(ret_lhs), next_bb)
        }
        Exp::And(e1, e2) => {
            let (e1_op, curr_bb) = lower_exp_to_operand(e1, body, curr_bb, info);
            let ret = info.create_tmp(&int_ty(), "_t");
            let inst_e1 = lir::Instruction::Copy {
                lhs: ret.clone(),
                op: e1_op.clone(),
            };
            add_inst(body, &curr_bb, inst_e1);

            let eval_bb = info.create_bb();
            let ss_bb = info.create_bb();

            body.insert(
                eval_bb.clone(),
                lir::BasicBlock {
                    id: eval_bb.clone(),
                    insts: vec![],
                    term: lir::Terminal::Jump(bb_id("_SENTINEL")),
                },
            );

            body.insert(
                ss_bb.clone(),
                lir::BasicBlock {
                    id: ss_bb.clone(),
                    insts: vec![],
                    term: lir::Terminal::Jump(bb_id("_SENTINEL")),
                },
            );

            set_terminal(
                body,
                &curr_bb,
                lir::Terminal::Branch {
                    cond: e1_op,
                    tt: eval_bb.clone(),
                    ff: ss_bb.clone(),
                },
            );

            let (e2_op, curr_bb) = lower_exp_to_operand(e2, body, &eval_bb, info);
            let inst_e2 = lir::Instruction::Copy {
                lhs: ret.clone(),
                op: e2_op.clone(),
            };
            add_inst(body, &curr_bb, inst_e2);

            set_terminal(body, &curr_bb, lir::Terminal::Jump(ss_bb.clone()));

            (lir::Operand::Var(ret), ss_bb)

            // given 'e1 and e2' generate the following code:
            //
            //   _t <- eval(e1)
            //   // create basic blocks bb2 and bb3
            //   $branch _t bb2 bb3
            // bb2:
            //   _t' <- eval(e2)  // may create new basic blocks
            //   _t = $copy _t'
            //   $jump bb3
            // bb3:
            //   // empty
            //
            // then, curr_bb = bb3, result = _t
            //
        }
        Exp::Or(e1, e2) => {
            let (e1_op, curr_bb) = lower_exp_to_operand(e1, body, curr_bb, info);
            let ret = info.create_tmp(&int_ty(), "_t");
            let inst_e1 = lir::Instruction::Copy {
                lhs: ret.clone(),
                op: e1_op.clone(),
            };
            add_inst(body, &curr_bb, inst_e1);

            let eval_bb = info.create_bb();
            let ss_bb = info.create_bb();

            body.insert(
                eval_bb.clone(),
                lir::BasicBlock {
                    id: eval_bb.clone(),
                    insts: vec![],
                    term: lir::Terminal::Jump(bb_id("_SENTINEL")),
                },
            );

            body.insert(
                ss_bb.clone(),
                lir::BasicBlock {
                    id: ss_bb.clone(),
                    insts: vec![],
                    term: lir::Terminal::Jump(bb_id("_SENTINEL")),
                },
            );

            set_terminal(
                body,
                &curr_bb,
                lir::Terminal::Branch {
                    cond: e1_op,
                    tt: ss_bb.clone(),
                    ff: eval_bb.clone(),
                },
            );

            let (e2_op, curr_bb) = lower_exp_to_operand(e2, body, &eval_bb, info);
            let inst_e2 = lir::Instruction::Copy {
                lhs: ret.clone(),
                op: e2_op.clone(),
            };
            add_inst(body, &curr_bb, inst_e2);

            set_terminal(body, &curr_bb, lir::Terminal::Jump(ss_bb.clone()));

            (lir::Operand::Var(ret), ss_bb)

            // given 'e1 and e2' generate the following code:
            //
            //   _t <- eval(e1)
            //   // create basic blocks bb2 and bb3
            //   $branch _t bb3 bb2
            // bb2:
            //   _t' <- eval(e2)  // may create new basic blocks
            //   _t = $copy _t'
            //   $jump bb3
            // bb3:
            //   // empty
            //
            // then, curr_bb = bb3, result = _t
            //
        }
    }
}

// given an Lval (i.e., an expression indicating where to store a value) returns
// a VarId and a boolean indicating whether the VarId should directly hold the
// value (true) or it holds a pointer to where the value should be held (false).
fn lower_lval(
    lval: &Lval,
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    curr_bb: &lir::BbId,
    info: &mut Lowering,
) -> (lir::VarId, bool) {
    // helper function: creates a temporary and loads the ptr value into that
    // temporary, returning the created temporary. the temporary should itself be a
    // pointer.
    fn create_load(
        ptr: &lir::VarId,
        body: &mut Map<lir::BbId, lir::BasicBlock>,
        curr_bb: &lir::BbId,
        info: &mut Lowering,
    ) -> lir::VarId {
        if let Some(deref_type) = ptr.typ().get_deref_type() {
            let deref_ptr = info.create_tmp(deref_type, "_t");
            if !deref_ptr.typ().is_ptr() {
                panic!("create_load derefed into non-pointer, shouldn't happen?")
            }
            let inst_load = Instruction::Load {
                lhs: deref_ptr.clone(),
                src: ptr.clone(),
            };
            add_inst(body, curr_bb, inst_load);

            deref_ptr
        } else {
            panic!("Tried to create_load on non-pointer, shouldn't happen.")
        }
    }

    match lval {
        // var (a direct access to a variable)
        Lval::Id(var) => (info.name_to_var(var), true),
        // *ptr
        Lval::Deref(ptr) => {
            let (mut var_id, copy) = lower_lval(ptr, body, curr_bb, info);
            if !copy {
                var_id = create_load(&var_id, body, curr_bb, info);
            }
            (var_id, false)
        }

        // ptr[index]
        Lval::ArrayAccess { ptr, index } => {
            let (mut src, copy) = lower_lval(ptr, body, curr_bb, info);
            if !copy {
                src = create_load(&src, body, curr_bb, info);
            }
            let (idx, bb_after_idx) = lower_exp_to_operand(index, body, curr_bb, info);

            let tmp_ptr = info.create_tmp(&src.typ(), "_t");
            let inst_gep = Instruction::Gep {
                lhs: tmp_ptr.clone(),
                src,
                idx,
            };

            add_inst(body, &bb_after_idx, inst_gep);
            (tmp_ptr, false)
        }

        // ptr.field
        Lval::FieldAccess { ptr, field } => {
            let (mut src, copy) = lower_lval(ptr, body, curr_bb, info);
            if !copy {
                src = create_load(&src, body, curr_bb, info);
            }
            if let Some(deref_type) = src.typ().get_deref_type() {
                if let lir::LirType::Struct(id) = deref_type.0.get() {
                    let field_id = info.get_field_by_name(&id.clone(), field);
                    let tmp_ptr = info.create_tmp(&ptr_ty(field_id.clone().typ), "_t");
                    let inst_gfp = Instruction::Gfp {
                        lhs: tmp_ptr.clone(),
                        src,
                        field: field_id.clone(),
                    };

                    add_inst(body, curr_bb, inst_gfp);
                    (tmp_ptr, false)
                } else {
                    panic!("\nLval: field accessing non struct, ptr: {ptr:?}, field: {field:?}\nsrc: {src:?}, deref_type: {deref_type:?}\n")
                }
            } else {
                panic!("Lval: field access var that is not pointer")
            }
        }
    }
}

// SECTION: eliminating initializations and cleaning up $ret instructions

// takes a Body (containing declarations and statements) and returns the
// statements prepended with assignments implementing any initializations in the
// declarations.
fn eliminate_inits(body: &Body) -> Vec<Stmt> {
    let mut new_stmts: Vec<Stmt> = vec![];
    for (decl, init) in &body.decls {
        if let Some(exp) = init {
            new_stmts.push(Stmt::Assign {
                lhs: Lval::Id(decl.name.clone()),
                rhs: Rhs::Exp(exp.clone()),
            })
        }
    }
    new_stmts.extend(body.stmts.clone());
    new_stmts
}

// if there are multiple return statements, transform them so there is a single
// return statement.
fn eliminate_multiple_ret(
    body: &mut Map<lir::BbId, lir::BasicBlock>,
    rettyp: &Option<Type>,
    info: &mut Lowering,
) {
    // collect all basic blocks ending in a $ret.
    // if there's only one $ret, there's nothing else to do.

    // create a new basic block named "exit" containing the sole $ret in the
    // function. we rely on the fact that lowering a function does not create
    // any basic blocks named "exit" before this step.
    let exit_id = bb_id("exit");

    if let Some(typ) = rettyp {
        let tmp = info.create_tmp(typ, "_ret");

        for (bbid, bb) in body.clone() {
            //println!("rettyp: {rettyp:#?}, bb: {bb:#?}");
            if let lir::Terminal::Ret(ret_op) = bb.term {
                if let Some(ret_op) = ret_op {
                    add_inst(
                        body,
                        &bbid,
                        Instruction::Copy {
                            lhs: tmp.clone(),
                            op: ret_op,
                        },
                    );
                }
                reset_terminal(body, &bbid, lir::Terminal::Jump(exit_id.clone()));
            }
        }

        body.insert(
            exit_id.clone(),
            lir::BasicBlock {
                id: exit_id.clone(),
                insts: vec![],
                term: lir::Terminal::Ret(Some(lir::Operand::Var(tmp))),
            },
        );
    } else {
        // do the same thing except we don't need to return anything.
    }
}
