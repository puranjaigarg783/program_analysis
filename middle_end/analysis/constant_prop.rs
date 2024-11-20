//! Intraprocedural integer constant propagation, with no pointer information.

use derive_more::Display;

use crate::commons::Valid;

use super::*;

// SECTION: analysis interface

// The constant lattice.  It represents the abstract value for an integer
// variable.
#[derive(Copy, Clone, Debug, Display, Eq, PartialEq)]
pub enum Value {
    Top,
    Int(i64),
    Bot,
}

// Abstract environment
pub type Env = PointwiseEnv<Value>;

// Performs the analysis: use `forward_analysis` to implement this.
pub fn analyze(program: &Valid<Program>, func: FuncId) -> (Map<BbId, Env>, Map<InstId, Env>) {
    let program = &program.0;
    let f = &program.functions[&func];
    // println!("{:#?}", f); // TODO Remove
    let init_store = create_init_store_no_ptrs(f, program);
    let bottom_store = create_bottom_store_no_ptrs(f, program);
    forward_analysis(f, &Cfg::new(f, program.globals.clone(), program.structs.clone()), &init_store, &bottom_store)
}

// SECTION: helpers

fn create_init_store(f: &Function, program: &Program) -> Env {
    let mut values = Map::new();

    for local in f.clone().locals {
        if is_int_or_ptr(&local.typ()) {
            values.insert(local, Value::Bot);
        }
    }
    
    for param in f.clone().params {
        if is_int_or_ptr(&param.typ()) {
            values.insert(param, Value::Bot);
        }
    }

    for global in program.globals.clone() {
        if is_int_or_ptr(&global.typ()) {
            values.insert(global.clone(), Value::Bot);
        }
    }

    Env::new(values)
}

fn create_bottom_store(f: &Function, program: &Program) -> Env {
    let mut values = Map::new();

    for local in f.clone().locals {
        if is_int_or_ptr(&local.typ()) {
            values.insert(local, Value::Bot);
        }
    }

    for param in f.clone().params {
        if is_int_or_ptr(&param.typ()) {
            values.insert(param, Value::Bot);
        }
    }

    for global in program.globals.clone() {
        if is_int_or_ptr(&global.typ()) {
            values.insert(global.clone(), Value::Bot);
        }
    }

    Env::new(values)
}

fn create_init_store_no_ptrs(f: &Function, program: &Program) -> Env {
    let mut values = Map::new();

    for local in f.clone().locals {
        if local.typ().is_int() {
            values.insert(local, Value::Bot);
        }
    }
    
    for param in f.clone().params {
        if param.typ().is_int() {
            values.insert(param.clone(), Value::Top);
        }
    }

    for global in program.globals.clone() {
        if global.typ().is_int() {
            values.insert(global.clone(), Value::Bot);
        }
    }


    Env::new(values)
}

fn create_bottom_store_no_ptrs(f: &Function, program: &Program) -> Env {
    let mut values = Map::new();

    for local in f.clone().locals {
        if local.typ().is_int() {
            values.insert(local, Value::Bot);
        }
    }

    for param in f.clone().params {
        if param.typ().is_int() {
            values.insert(param.clone(), Value::Bot);
        }
    }

    for global in program.globals.clone() {
        if global.typ().is_int() {
            values.insert(global.clone(), Value::Bot);
        }
    }

    Env::new(values)
}

fn is_int_or_ptr(typ: &Type) -> bool {
    typ.is_int() || typ.is_ptr()
}

// SECTION: analysis implementation

use Value as V;

impl AbstractValue for Value {
    type Concrete = i32;

    const BOTTOM: Self = V::Bot;

    fn alpha(val: i32) -> Self {
        V::Int(val as i64)
    }

    fn join(&self, rhs: &Self) -> Value {
        match (self, rhs) {
            // if either are bot, return bot
            (V::Bot, _) => *rhs,
            (_, V::Bot) => *self,
            // if no bot, either are top, return top
            (V::Top, _) => V::Top,
            (_, V::Top) => V::Top,

            // else integer value check
            (V::Int(x), V::Int(y)) => {
                if x == y {
                    V::Int(*x)
                } else {
                    V::Top
                }
            }
        }
    }
}

impl Env {
    fn value_from_op(&self, op: Operand) -> Value {
        match op {
            Operand::CInt(x) => V::alpha(x),
            Operand::Var(var) => *self.values.get(&var).unwrap_or(&V::Top),
            //Operand::Var(var) => self.get(&var),
        }
    }

    fn set_to_top(&mut self, vars: &Set<VarId>) {
        for var in vars {
            if var.typ().is_int() { self.insert(var, &V::Top); }
        }
    }

    fn call_update(&mut self, args: Vec<Operand>, cfg: &Cfg) {

        let mut vars_to_check: Set<VarId> = cfg.globals.clone();

        // top globals
        self.set_to_top(&cfg.globals);

        for arg in args {
            match arg {
                Operand::CInt(_) => (),
                Operand::Var(id) => { vars_to_check.insert(id); },
            }
        }

        for g in vars_to_check {
            if cfg.var_reaches_int(&g) {
                for v in &cfg.addr_taken_ints {
                    self.insert(v, &V::Top)
                }
                break;
            }
        }
        
    }
}

impl AbstractEnv for Env {
    fn join_with(&mut self, rhs: &Self, _block: &BbId, _: i64) -> bool {
        let mut changed = false;

        for (var, self_val) in &mut self.values {
            let joined = self_val.join(&rhs.get(var));

            if &joined != self_val {
                *self_val = joined; // overwrite self with joined value
                changed = true;
            }
        }

        changed
    }

    fn analyze_inst(&mut self, inst: &Instruction, cfg: &Cfg) {
        use Instruction::*;

        fn arith(aop: ArithmeticOp, v1: Value, v2: Value) -> Value {

            match (v1, v2) {
                // if either are bot, return bot
                (V::Bot, _) => V::Bot,
                (_, V::Bot) => V::Bot,
                // if no bot, either are top, return top
                (V::Top, V::Top) => V::Top,
                (V::Top, V::Int(high)) => {
                    match aop {
                        LirOp![+] => V::Top,
                        LirOp![-] => V::Top,
                        LirOp![*] => match high {
                            0 => V::Int(0),
                            _ => V::Top,
                        },
                        LirOp![/] => match high {
                            0 => V::Bot,
                            _ => V::Top,
                        },
                    }
                },
                (V::Int(low), V::Top) => {
                    match aop {
                        LirOp![+] => V::Top,
                        LirOp![-] => V::Top,
                        LirOp![*] => match low {
                            0 => V::Int(0),
                            _ => V::Top,
                        },
                        LirOp![/] => match low {
                            0 => V::Int(0),
                            _ => V::Top,
                        },
                    }
                },

                // else integer value check
                (V::Int(x), V::Int(y)) => {
                    match aop {
                        LirOp![+] => V::Int(x + y),
                        LirOp![-] => V::Int(x - y),
                        LirOp![*] => V::Int(x * y),
                        LirOp![/] => match y {
                            0 => V::Bot,
                            _ => V::Int(x / y),
                        },
                    }
                }
            }

            
        }

        fn cmp(rop: ComparisonOp, v1: Value, v2: Value) -> Value {
            let int1;
            let int2;

            match (v1, v2) {
                // if either are bot, return bot
                (V::Bot, _) => return V::Bot,
                (_, V::Bot) => return V::Bot,
                // if no bot, either are top, return top
                (V::Top, _) => return V::Top,
                (_, V::Top) => return V::Top,

                // else integer value check
                (V::Int(x), V::Int(y)) => {
                    int1 = x;
                    int2 = y;
                }
            }

            match rop {
                LirOp![==] => {
                    if int1 == int2 {
                        V::Int(1)
                    } else {
                        V::Int(0)
                    }
                }
                LirOp![!=] => {
                    if int1 != int2 {
                        V::Int(1)
                    } else {
                        V::Int(0)
                    }
                }
                LirOp![<] => {
                    if int1 < int2 {
                        V::Int(1)
                    } else {
                        V::Int(0)
                    }
                }
                LirOp![<=] => {
                    if int1 <= int2 {
                        V::Int(1)
                    } else {
                        V::Int(0)
                    }
                }
                LirOp![>] => {
                    if int1 > int2 {
                        V::Int(1)
                    } else {
                        V::Int(0)
                    }
                }
                LirOp![>=] => {
                    if int1 >= int2 {
                        V::Int(1)
                    } else {
                        V::Int(0)
                    }
                }
            }
        }

        match inst.clone() {
            AddrOf { lhs, op } => if lhs.typ().is_int() { self.insert(&lhs, &V::Top) },
            Alloc { lhs, num: _, id: _ } => if lhs.typ().is_int() { self.insert(&lhs, &V::Top) },
            Arith { lhs, aop, op1, op2 } => {
                let v1 = self.value_from_op(op1);
                let v2 = self.value_from_op(op2);

                let lhs_val = arith(aop, v1, v2);

                if lhs.typ().is_int() { self.insert(&lhs, &lhs_val) };
            }
            CallExt {
                lhs,
                ext_callee: _,
                args,
            } =>  {
                self.call_update(args.clone(), cfg);

                if let Some(lhs) = lhs {
                    if lhs.typ().is_int() { self.insert(&lhs, &V::Top) }
                }
            },
            Cmp { lhs, rop, op1, op2 } => {
                let v1 = self.value_from_op(op1);
                let v2 = self.value_from_op(op2);

                let lhs_val = cmp(rop, v1, v2);

                if lhs.typ().is_int() { self.insert(&lhs, &lhs_val) };
            }
            Copy { lhs, op } => {
                if lhs.typ().is_int() { 
                    self.insert(&lhs, &self.value_from_op(op))
                };
            }
            Gep {
                lhs,
                src: _,
                idx: _,
            } =>  if lhs.typ().is_int() { self.insert(&lhs, &V::Top) },
            Gfp {
                lhs,
                src: _,
                field: _,
            } =>  if lhs.typ().is_int() { self.insert(&lhs, &V::Top) },
            Load { lhs, src: _ } =>  if lhs.typ().is_int() { self.insert(&lhs, &V::Top) },
            Store { dst: _, op } => {
                if op.typ().is_int() {
                    let op_value = self.value_from_op(op);
                    for var in &cfg.addr_taken_ints {
                        
                        let var_value = self.value_from_op(Operand::Var(var.clone()));
                        let new_value = var_value.join(&op_value);

                        self.insert(var, &new_value );
                    }
                }
            },
            _ => (),                       // phi is here
        }

        // lhs var will always be in self as part of local/params (OR WILL IT?)
        /*
        match inst.clone() {
            Alloc { lhs, num: _, id: _ } => self.insert(&lhs, &V::Top),
            Arith { lhs, aop, op1, op2 } => {
                let v1 = self.value_from_op(op1);
                let v2 = self.value_from_op(op2);

                let lhs_val = arith(aop, v1, v2);

                self.insert(&lhs, &lhs_val);
            }
            CallExt {
                lhs: Some(lhs),
                ext_callee: _,
                args: _,
            } => self.insert(&lhs, &V::Top),
            Cmp { lhs, rop, op1, op2 } => {
                let v1 = self.value_from_op(op1);
                let v2 = self.value_from_op(op2);

                let lhs_val = cmp(rop, v1, v2);

                self.insert(&lhs, &lhs_val);
            }
            Copy { lhs, op } => {
                self.insert(&lhs, &self.value_from_op(op));
            }
            Gep {
                lhs,
                src: _,
                idx: _,
            } => self.insert(&lhs, &V::Top),
            Gfp {
                lhs,
                src: _,
                field: _,
            } => self.insert(&lhs, &V::Top),
            Load { lhs, src: _ } => self.insert(&lhs, &V::Top),
            Store { dst: _, op: _ } => (), // store does nothing?
            _ => (),                       // phi is here
        }*/
    }

    fn analyze_term(&mut self, term: &Terminal, cfg: &Cfg) -> Set<BbId> {
        use Terminal::*;
        let mut skip_state = Set::new();
        match term {
            Branch {
                cond,
                tt,
                ff,
            } => {
                let cv = self.value_from_op(cond.clone());
                match cv {
                    Value::Int(0) => { skip_state.insert(tt.clone()); },
                    Value::Int(_) => { skip_state.insert(ff.clone()); },
                    Value::Bot => { skip_state.insert(tt.clone()); skip_state.insert(ff.clone()); },
                    _ => (),
                }
            }
            CallDirect {
                lhs,
                callee: _,
                args,
                next_bb: _,
            } => {
                self.call_update(args.clone(), cfg);

                if let Some(lhs) = lhs {
                    if lhs.typ().is_int() { self.insert(lhs, &V::Top) }
                }
            },
            CallIndirect {
                lhs,
                callee: _,
                args,
                next_bb: _,
            } => {
                self.call_update(args.clone(), cfg);

                if let Some(lhs) = lhs {
                    if lhs.typ().is_int() { self.insert(lhs, &V::Top) }
                }
                
            },
            Jump(_) => {
                // doesnt update value
            }
            Ret(_) => {
                // doesnt update value
            }
            _ => (),
        }

        skip_state
    }

    fn analyze_bb(&self, bb: &BasicBlock, cfg: &Cfg) -> (Vec<Self>, Set<BbId>) {
        // println!("analyze_bb: {:#?}", bb.id);
        let mut state_vec: Vec<Self> = vec![];
        let mut curr_state = self.clone();
        // println!("curr_state: {:#?}", curr_state);
        state_vec.push(curr_state.clone()); // push block PRE-STATE

        // loop through instructions
        // add all instructions' POST-STATE to vector
        for inst in &bb.insts {
            curr_state.analyze_inst(inst, cfg);
            // println!("curr_state: {:#?}", curr_state);
            state_vec.push(curr_state.clone());
        }

        let skip_state = curr_state.analyze_term(&bb.term, cfg);
        state_vec.push(curr_state.clone());
        // println!("state_vec: {:#?}", state_vec);
        (state_vec, skip_state)
    }
}
