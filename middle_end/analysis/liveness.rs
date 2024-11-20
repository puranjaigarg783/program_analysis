//! Intraprocedural liveness analysis.
//!
//! Remember that this is a backwards analysis!

use std::rc::Rc;

use crate::commons::Valid;

use super::*;

// SECTION: analysis interface

// The abstract environment.
#[derive(Clone, Debug)]
pub struct Env {
    // Result of reaching definitions analysis (pre sets).
    //
    // You should treat the `Rc` object just like a `Box` you cannot modify.
    reaching_defs: Rc<Map<InstId, super::reaching_defs::Env>>,
    // result of this analysis
    pub live_defs: Set<InstId>,
}

// Performs the analysis: use `backward_analysis` to implement this.
pub fn analyze(_program: &Valid<Program>, _func: FuncId) -> (Map<BbId, Env>, Map<InstId, Env>) {
    // @clippydodge let reaching_def_results = Rc::new(super::reaching_defs::analyze(program, func.clone()).0);
    // @clippydodge let program = &program.0;

    // @clippydodge let init_store = todo!("map everything to empty set");

    todo!("call backward_analysis");
}

// SECTION: analysis implementation

impl AbstractEnv for Env {
    fn join_with(&mut self, _rhs: &Self, _block: &BbId, join_type: i64) -> bool {
        todo!()
    }

    fn analyze_inst(&mut self, _inst: &Instruction, cfg: &Cfg) {
        todo!()
    }

    fn analyze_term(&mut self, _term: &Terminal, cfg: &Cfg) -> Set<BbId> {
        todo!()
    }

    fn analyze_bb(&self, _bb: &BasicBlock, cfg: &Cfg) -> (Vec<Self>, Set<BbId>) {
        todo!()
    }
}
