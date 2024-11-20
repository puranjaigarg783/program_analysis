//! Intraprocedural integer constant propagation, with no pointer information.

use derive_more::Display;

use crate::commons::Valid;

use super::*;

use std::ops::Bound;
use std::ops::Bound::{Included, Excluded, Unbounded};
use std::cmp::*;

use std::fmt;

// SECTION: analysis interface

type IntInterval = (Bound<i64>, Bound<i64>);

fn new_int_interval(low: i64, high: i64) -> IntInterval {
    (Bound::Included(low), Bound::Included(high))
}

// fn widening(i1: IntInterval, i2: IntInterval) -> Int

fn int_from_bound(bound: Bound<i64>) -> Option<i64> {

    match bound {
        Included(n) => Some(n),
        Unbounded => None,
        _ => panic!("int_from_bound: expected included/unbounded")
    }
}

impl Value {
    fn interval(low: i64, high: i64) -> Self {
        V::R(new_int_interval(low, high))
    }

    fn top() -> Self {
        V::R((Bound::Unbounded, Bound::Unbounded))
    }

    fn widen(&self, widee: &Self) -> Self {

        match (self, widee) {
            // if either is bot
            (V::Bot, _) => *widee,
            (_, V::Bot) => *self,
            // else it is range
            (V::R((self_low, self_high)), V::R((widee_low, widee_high))) => {

                let i3_low: Bound<i64> = match (self_low, widee_low) {
                    (Included(x), Included(y)) => if x <= y { *self_low } else { Unbounded },
                    _ => Unbounded
                };

                let i3_high: Bound<i64> = match (self_high, widee_high) {
                    (Included(x), Included(y)) => if x >= y { *self_high } else { Unbounded },
                    _ => Unbounded
                };

                V::R((i3_low, i3_high))
            },
            _ => panic!("unexpected case in widening"),
        }
    }
}

// The constant lattice.  It represents the abstract value for an integer
// variable.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Value {
    R(IntInterval),
    Bot,
}

impl fmt::Display for Value {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::R((x, y)) => {
                let x_string = match x {
                    Included(num) => format!("[{}", num),
                    Excluded(num) => format!("({}", num),
                    Unbounded => "(NegInf".to_string(),
                };
                let y_string = match y {
                    Included(num) => format!("{}]", num),
                    Excluded(num) => format!("{})", num),
                    Unbounded => "PosInf)".to_string(),
                };
                write!(f, "{}, {}", x_string, y_string)
            },
            Value::Bot => write!(f, "Bot"),
        }
        
    }
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
            values.insert(param, Value::top());
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
            values.insert(param, Value::top());
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
            values.insert(param, Value::Bot);
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
        V::R(new_int_interval(val as i64, val as i64))
    }

    fn join(&self, rhs: &Self) -> Value {
        fn join_interval((i1_low, i1_high): IntInterval, (i2_low, i2_high): IntInterval) -> Value {
            
            let low_bound: Bound<i64> = if i1_low == Bound::Unbounded || i2_low == Bound::Unbounded {
                Bound::Unbounded
            } else {
                Bound::Included(min(int_from_bound(i1_low).unwrap(), int_from_bound(i2_low).unwrap()))
            };

            let high_bound: Bound<i64> = if i1_high == Bound::Unbounded || i2_high == Bound::Unbounded {
                Bound::Unbounded
            } else {
                Bound::Included(max(int_from_bound(i1_high).unwrap(), int_from_bound(i2_high).unwrap()))
            };
            
            Value::R((low_bound, high_bound))
        }

        match (self, rhs) {
            // if either are bot, return other
            (V::Bot, _) => *rhs,
            (_, V::Bot) => *self,

            // else integer value check
            (V::R(i1), V::R(i2)) => {
                join_interval(*i1, *i2)
            }
        }
    }
}

impl Env {
    fn value_from_op(&self, op: Operand) -> Value {
        match op {
            Operand::CInt(x) => V::alpha(x),
            Operand::Var(var) => *self.values.get(&var).unwrap_or(&V::top()),
            //Operand::Var(var) => self.get(&var),
        }
    }

    fn set_to_top(&mut self, vars: &Set<VarId>) {
        for var in vars {
            if var.typ().is_int() { self.insert(var, &V::top()); }
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
                    self.insert(v, &V::top())
                }
                break;
            }
        }
    }
}

impl AbstractEnv for Env {
    fn join_with(&mut self, rhs: &Self, _block: &BbId, join_type: i64) -> bool {
        let mut changed = false;

        for (var, self_val) in &mut self.values {
            let joined = match join_type {
                0 => self_val.join(&rhs.get(var)),
                1 => self_val.widen(&rhs.get(var)),
                _ => panic!("not a valid join type for integer_interval"),
            };

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
            fn interval_addition((i1_low, i1_high): IntInterval, (i2_low, i2_high): IntInterval) -> Value {
                
                let low_bound = if i1_low == Bound::Unbounded || i2_low == Bound::Unbounded {
                    Unbounded
                } else {
                    Included(int_from_bound(i1_low).unwrap() + int_from_bound(i2_low).unwrap())
                };

                let high_bound = if i1_high == Bound::Unbounded || i2_high == Bound::Unbounded {
                    Unbounded
                } else {
                    Included(int_from_bound(i1_high).unwrap() + int_from_bound(i2_high).unwrap())
                };
                
                Value::R((low_bound, high_bound))
            }

            fn interval_subtraction((i1_low, i1_high): IntInterval, (i2_low, i2_high): IntInterval) -> Value {
                
                let low_bound = if i1_low == Bound::Unbounded || i2_high == Bound::Unbounded {
                    Unbounded
                } else {
                    Included(int_from_bound(i1_low).unwrap() - int_from_bound(i2_high).unwrap())
                };

                let high_bound = if i1_high == Bound::Unbounded || i2_low == Bound::Unbounded {
                    Unbounded
                } else {
                    Included(int_from_bound(i1_high).unwrap() - int_from_bound(i2_low).unwrap())
                };

                Value::R((low_bound, high_bound))

            }

            fn interval_multiplication((i1_low, i1_high): IntInterval, (i2_low, i2_high): IntInterval) -> Value {
                fn b_mult((b1, i1): (Bound<i64>, i64), (b2, i2): (Bound<i64>, i64)) -> (Bound<i64>, i64) {

                    match (b1, b2) {
                        // if either is zero, return zero
                        (Included(0), _) => (Included(0), 0),
                        (_, Included(0)) => (Included(0), 0),
                        // if both are unbounded, calculate which way it is
                        (Unbounded, Unbounded) => (Unbounded, i1 * i2),
                        // if only one is unbounded, find out the sign
                        (Unbounded, Included(n)) => {
                            let n_sign = if n > 0 {1} else if n < 0 {-1} else {0};
                            (Unbounded, i1 * n_sign)
                        },
                        (Included(n), Unbounded) => {
                            let n_sign = if n > 0 {1} else if n < 0 {-1} else {0};
                            (Unbounded, n_sign * i2)
                        },
                        (Included(x), Included(y)) => (Included(x * y), 0),
                        _ => panic!("b_mult b1: {b1:?}, b2: {b2:?}")
                    }
                }

                let mut low_bound: Option<Bound<i64>> = None;
                let mut high_bound: Option<Bound<i64>> = None;

                if (i1_low, i1_high) == (Included(0), Included(0)) || (i2_low, i2_high) == (Included(0), Included(0)) {
                    return V::interval(0, 0);
                }

                let v1 = b_mult((i1_low, -1), (i2_low, -1));
                let v2 = b_mult((i1_low, -1), (i2_high, 1));
                let v3 = b_mult((i1_high, 1), (i2_low, -1));
                let v4 = b_mult((i1_high, 1), (i2_high, 1));

                let mut int_stack: Vec<i64> = vec![];

                for (b, i) in [v1, v2, v3, v4].iter() {
                    if *b == Unbounded {
                        if *i == 1 {
                            high_bound = Some(Unbounded);
                        } else if *i == -1 {
                            low_bound = Some(Unbounded);
                        } else {
                            unreachable!("mult bound signs wrong");
                        }
                    } else {
                        int_stack.push(int_from_bound(*b).unwrap());
                    }
                }

                if low_bound.is_none() {
                    let min_int = match int_stack.iter().min() {
                        Some(n) => *n,
                        _ => panic!("mult: none in int_stack but low_bound is none")
                    };

                    low_bound = Some(Included(min_int))
                }

                

                if high_bound.is_none() {
                    let max_int = match int_stack.iter().max() {
                        Some(n) => *n,
                        _ => panic!("mult: none in int_stack but high_bound is none")
                    };

                    high_bound = Some(Included(max_int))
                }

                match (low_bound, high_bound) {
                    (Some(lb), Some(hb)) => Value::R((lb, hb)),
                    _ => panic!("did not assign either lowbound or highbound in mult")
                }
                
            }

            fn interval_division((mut i1_low, mut i1_high): IntInterval, (mut i2_low, mut i2_high): IntInterval) -> Value {
                fn b_div((b1, i1): (Bound<i64>, i64), (b2, i2): (Bound<i64>, i64)) -> (Bound<i64>, i64) {

                    match (b1, b2) {
                        // div by zero which shouldnt happen
                        (_, Included(0)) => {
                            panic!("panicking even though it should work, b_div div by 0");
                            (Unbounded, i1)
                        },
                        // if both are unbounded, calculate which way it is
                        (Unbounded, Unbounded) => (Included(1), i1 / i2),
                        // if dividing by unbounded, should be 0
                        (_, Unbounded) => {
                            (Included(0), 0)
                        },
                        // if left side is unbounded, find out the sign
                        (Unbounded, Included(n)) => {
                            let n_sign = if n > 0 { 1 } else if n < 0 {-1} else { unreachable!("can't be zero") };
                            (Unbounded, i1 * n_sign)
                        },
                        (Included(x), Included(y)) => (Included(x / y), 0),
                        _ => panic!("b_div b1: {b1:?}, b2: {b2:?}")
                    }
                }

                let i1_low_intopt = int_from_bound(i1_low);
                let i1_high_intopt = int_from_bound(i1_high);
                let i2_low_intopt = int_from_bound(i2_low);
                let i2_high_intopt = int_from_bound(i2_high);

                match (i1_low_intopt, i1_high_intopt, i2_low_intopt, i2_high_intopt) {
                    (_, _, Some(0), Some(0)) => return Value::Bot,
                    // same priority
                    (_, _, Some(0), _) => {
                        i2_low = Included(1);
                    },
                    (_, _, _, Some(0)) => {
                        i2_high = Included(-1);
                    },
                    // interval crosses 0
                    (_, _, Some(i2l), Some(i2h)) => {
                        if i2l < 0 && i2h > 0 {
                            i2_low = Included(-1);
                            i2_high = Included(1);
                        }
                    },
                    // interval crosses 0 both ends unbounded, one end bounded should be checked earlier
                    (_, _, None, None) => {
                        i2_low = Included(-1);
                        i2_high = Included(1);
                    }
                    // everything else do as normal
                    _ => (),
                }

                let mut low_bound: Option<Bound<i64>> = None;
                let mut high_bound: Option<Bound<i64>> = None;

                if (i1_low, i1_high) == (Included(0), Included(0)) || (i2_low, i2_high) == (Included(0), Included(0)) {
                    return V::interval(0, 0);
                }

                let v1 = b_div((i1_low, -1), (i2_low, -1));
                let v2 = b_div((i1_low, -1), (i2_high, 1));
                let v3 = b_div((i1_high, 1), (i2_low, -1));
                let v4 = b_div((i1_high, 1), (i2_high, 1));

                let mut int_stack: Vec<i64> = vec![];

                for (b, i) in [v1, v2, v3, v4].iter() {

                    match *b {
                        Unbounded => {
                            match *i {
                                1 => { high_bound = Some(Unbounded); },
                                -1 => { low_bound = Some(Unbounded); },
                                _ => unreachable!("mult bound signs wrong"),
                            }
                        }
                        _ => { int_stack.push(int_from_bound(*b).unwrap()); }
                    }
                }

                if low_bound.is_none() {
                    let min_int = match int_stack.iter().min() {
                        Some(n) => *n,
                        _ => panic!("mult: none in int_stack but low_bound is none")
                    };

                    low_bound = Some(Included(min_int))
                }

                

                if high_bound.is_none() {
                    let max_int = match int_stack.iter().max() {
                        Some(n) => *n,
                        _ => panic!("mult: none in int_stack but high_bound is none")
                    };

                    high_bound = Some(Included(max_int))
                }

                match (low_bound, high_bound) {
                    (Some(lb), Some(hb)) => Value::R((lb, hb)),
                    _ => panic!("did not assign either lowbound or highbound in mult")
                }
                
            }
            

            match (v1, v2) {
                // if either are bot, return bot
                (V::Bot, _) => V::Bot,
                (_, V::Bot) => V::Bot,

                // else interval value check
                (V::R(i1), V::R(i2)) => {
                    match aop {
                        LirOp![+] => interval_addition(i1, i2),
                        LirOp![-] => interval_subtraction(i1, i2),
                        LirOp![*] => interval_multiplication(i1, i2),
                        LirOp![/] => interval_division( i1, i2),
                    }
                }
            }

            
        }

        fn cmp(rop: ComparisonOp, v1: Value, v2: Value) -> Value {
            fn gt((b1, i1): (Bound<i64>, i64), (b2, i2): (Bound<i64>, i64)) -> bool {
                match (b1, i1, b2, i2) {
                    (Unbounded, x, Unbounded, y) => x > y,
                    (Unbounded, x, Included(_), _) => x == 1,
                    (Included(_), _, Unbounded, y) => y == -1,
                    (Included(int1), _, Included(int2), _) => int1 > int2,
                    _ => panic!("cmp::eq unexpected case"),
                }
            }

            fn lt((b1, i1): (Bound<i64>, i64), (b2, i2): (Bound<i64>, i64)) -> bool {
                !gt((b1, i1), (b2, i2)) && !eq((b1, i1), (b2, i2))
            }

            fn eq((b1, i1): (Bound<i64>, i64), (b2, i2): (Bound<i64>, i64)) -> bool {
                match (b1, i1, b2, i2) {
                    (Unbounded, x, Unbounded, y) => x == y,
                    (Unbounded, _, Included(_), _) => false,
                    (Included(_), _, Unbounded, _) => false,
                    (Included(int1), _, Included(int2), _) => int1 == int2,
                    _ => panic!("cmp::eq unexpected case"),
                }
            }
            
            fn gte((b1, i1): (Bound<i64>, i64), (b2, i2): (Bound<i64>, i64)) -> bool {
                !lt((b1, i1), (b2, i2))
            }

            fn lte((b1, i1): (Bound<i64>, i64), (b2, i2): (Bound<i64>, i64)) -> bool {
                !gt((b1, i1), (b2, i2))
            }

            fn neq((b1, i1): (Bound<i64>, i64), (b2, i2): (Bound<i64>, i64)) -> bool {
                !eq((b1, i1), (b2, i2))
            }

            match (v1, v2) {
                
                // if either are bot, return bot
                (V::Bot, _) => V::Bot,
                (_, V::Bot) => V::Bot,

                // else interval value check
                (V::R((i1_low, i1_high)), V::R((i2_low, i2_high))) => {
                    let i1_low = (i1_low, -1);
                    let i1_high = (i1_high, 1);
                    let i2_low = (i2_low, -1);
                    let i2_high = (i2_high, 1);

                    match rop {
                        LirOp![==] => {
                            if eq(i1_low, i1_high) && eq(i1_low, i2_low) && eq(i1_low, i2_high) {
                                V::interval(1, 1)
                            } else if lt(i1_high, i2_low) || lt(i2_high, i1_low)  {
                                V::interval(0, 0)
                            } else {
                                V::interval(0, 1)
                            }
                        }
                        LirOp![!=] => {
                            if eq(i1_low, i1_high) && eq(i1_low, i2_low) && eq(i1_low, i2_high) {
                                V::interval(0, 0)
                            } else if gt(i1_low, i2_high) || gt(i2_low, i1_high)  {
                                V::interval(1, 1)
                            } else {
                                V::interval(0, 1)
                            }
                        }
                        LirOp![<] => {
                            if lt(i1_high, i2_low) {
                                V::interval(1, 1)
                            } else if lte(i2_high, i1_low) {
                                V::interval(0, 0)
                            } else {
                                V::interval(0, 1)
                            }
                        }
                        LirOp![<=] => {
                            if lte(i1_high, i2_low) {
                                V::interval(1, 1)
                            } else if lt(i2_high, i1_low) {
                                V::interval(0, 0)
                            } else {
                                V::interval(0, 1)
                            }
                        }
                        LirOp![>] => {
                            if gt(i1_low, i2_high) {
                                V::interval(1, 1)
                            } else if gte(i2_low, i1_high) {
                                V::interval(0, 0)
                            } else {
                                V::interval(0, 1)
                            }
                        }
                        LirOp![>=] => {
                            if gte(i1_low, i2_high) {
                                V::interval(1, 1)
                            } else if gt(i2_low, i1_high) {
                                V::interval(0, 0)
                            } else {
                                V::interval(0, 1)
                            }
                        }
                    }
                }
            }

            
        }

        match inst.clone() {
            AddrOf { lhs, op } => if lhs.typ().is_int() { self.insert(&lhs, &V::top()) },
            Alloc { lhs, num: _, id: _ } => if lhs.typ().is_int() { self.insert(&lhs, &V::top()) },
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
                self.call_update(args, cfg);

                if let Some(lhs) = lhs {
                    if lhs.typ().is_int() { self.insert(&lhs, &V::top()) }
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
            } =>  if lhs.typ().is_int() { self.insert(&lhs, &V::top()) },
            Gfp {
                lhs,
                src: _,
                field: _,
            } =>  if lhs.typ().is_int() { self.insert(&lhs, &V::top()) },
            Load { lhs, src: _ } =>  if lhs.typ().is_int() { self.insert(&lhs, &V::top()) },
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
                    Value::R((Included(0), Included(0))) => { skip_state.insert(tt.clone()); },
                    Value::R((Included(low), Included(high))) => { 
                        if (low < 0 && high < 0) || (low > 0 && high > 0) {
                            skip_state.insert(ff.clone());
                        }
                    },
                    Value::R((Included(low), Unbounded)) => { 
                        if low > 0 {
                            skip_state.insert(ff.clone());
                        }
                    },
                    Value::R((Unbounded, Included(high))) => {
                        if high < 0 {
                            skip_state.insert(ff.clone());
                        }
                    },
                    
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
                    if lhs.typ().is_int() { self.insert(lhs, &V::top()) }
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
                    if lhs.typ().is_int() { self.insert(lhs, &V::top()) }
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
