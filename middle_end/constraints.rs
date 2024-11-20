pub mod fromstr_impl;
pub mod constraint_node;
pub mod constraint_solve;

use super::lir::*;
use std::cmp::Ordering;
use std::fmt;

use std::collections::{BTreeMap as Map, BTreeSet as Set};

#[derive(Clone)]
pub enum ConstraintExp {
    Var(VarId),
    Ref(VarId, VarId),
    Proj(VarId),
    Lam {
       name: String,
       param_ty: Vec<Type>,
       ret_ty: Option<Type>,
       ret_op: Option<Operand>,
       args: Vec<VarId>
    },
    LamSimple {
        params: String,
        ret_ty: String,
        args: String,
    }
}

impl ConstraintExp {
    fn dummy_var() -> ConstraintExp {
        ConstraintExp::Var(var_id("_GENERATED_DUMMY_PTR_TO_INT", ptr_ty(int_ty()), None))
    }

    fn get_name(&self) -> &str {
        match self {
            Self::Var(v) => v.name(),
            Self::Ref(v, _) => v.name(),
            Self::Proj(v) => v.name(),
            _ => panic!("LAMBDA THING DONT ASK"),
        }
    }
}

impl PartialOrd for ConstraintExp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConstraintExp {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl PartialEq for ConstraintExp {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for ConstraintExp {}


impl PartialOrd for Constraint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Constraint {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_string().cmp(&other.to_string())
    }
}

impl PartialEq for Constraint {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for Constraint {}

impl VarId {
    pub fn with_funcid(&self) -> String {
        if let Some(funcid) = &self.0.scope {
            format!("{}.{}", funcid, &self)
        } else {
            self.to_string()
        }
    }
}

impl fmt::Display for ConstraintExp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        match self {
            ConstraintExp::Var(var) => {
                write!(f, "{}", var.with_funcid())
            },
            ConstraintExp::Ref(var1, var2) => {
                write!(f, "ref({},{})", var1.with_funcid(), var2.with_funcid())
            },
            ConstraintExp::Proj(var) => write!(f, "proj(ref,1,{})", var.with_funcid()),
            ConstraintExp::Lam{
                name,
                param_ty,
                ret_ty,
                ret_op,
                args,
            } => {
                write!(f, "lam_[(")?;
                
                // write param type e.g (int,&int,&st)->&int
                write!(f, "{}", param_ty.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","))?;
                write!(f, ")->")?;

                // write return type if exist
                if let Some(ret_var) = ret_ty {
                    write!(f, "{}", ret_var)?;
                } else {
                    write!(f, "_")?;
                }
                write!(f, "](")?;

                // write name first
                write!(f, "{}", name)?;

                // write ret if ret exists and is ptr
                if let Some(Operand::Var(ret_var)) = ret_op {
                    if ret_var.typ().is_ptr() {
                        write!(f, ",{}", ret_var.with_funcid())?;
                    }
                }

                // write args which are pointers
                let args_vec = args.iter().filter(|x| x.typ().is_ptr()).map(|x| x.with_funcid()).collect::<Vec<String>>();

                if !args_vec.is_empty() {
                    write!(f, ",{}", args_vec.join(","))?;
                }
                

                write!(f, ")")
            },
            ConstraintExp::LamSimple {
                params,
                ret_ty,
                args,
            } => {
                write!(f, "lam_[({})->{}]({})", params, ret_ty, args)
            },
            _ => unreachable!(),
        }
        
    }
}

pub struct ConstraintVar(Option<FuncId>, VarId);

impl fmt::Debug for ConstraintExp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Clone)]
pub struct Constraint(pub ConstraintExp, pub ConstraintExp);

impl Constraint {
    pub fn as_tuple(&self) -> (&ConstraintExp, &ConstraintExp) {
        (&self.0, &self.1)
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} <= {}", self.0, self.1)
    }
}

impl fmt::Debug for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} <= {}", self.0, self.1)
    }
}

pub struct Constraints(pub Set<Constraint>);

impl fmt::Display for Constraints {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for constraint in self.0.clone() {
            writeln!(f, "{}", constraint)?;
        }

        fmt::Result::Ok(())
    }
}

impl fmt::Debug for Constraints {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}