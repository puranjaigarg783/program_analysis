use std::collections::{BTreeMap as Map, BTreeSet as Set};
use crate::middle_end::analysis::*;
use crate::middle_end::lir::*;
use std::fmt::Display;
use crate::commons::Valid;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaintValue(pub Set<FuncId>);

impl Display for TaintValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{")?;
        let mut srcs: Vec<_> = self.0.iter().collect();
        srcs.sort_by(|a, b| a.name().cmp(b.name()));
        for (i, src) in srcs.iter().enumerate() {
            if i > 0 { write!(f, ", ")? }
            write!(f, "{}", src.name())?;
        }
        write!(f, "}}")
    }
}

pub type TaintEnv = PointwiseEnv<TaintValue>;

impl AbstractValue for TaintValue {
    type Concrete = FuncId;
    
    const BOTTOM: Self = TaintValue(Set::new());
    
    fn alpha(src: FuncId) -> Self {
        let mut set = Set::new();
        set.insert(src);
        TaintValue(set)
    }
    
    fn join(&self, rhs: &Self) -> Self {
        TaintValue(self.0.union(&rhs.0).cloned().collect())
    }
}

// Global state for tracking taint flows
#[derive(Clone, Debug, Default)]
struct TaintState {
    sources: Set<FuncId>,
    sinks: Set<FuncId>,
    sink_map: Map<FuncId, Set<FuncId>>
}

impl AbstractEnv for TaintEnv {
    fn join_with(&mut self, rhs: &Self, _block: &BbId, _join_type: i64) -> bool {
        let mut changed = false;
        for (x, rhs_val) in &rhs.values {
            let lhs_val = self.values.entry(x.clone()).or_insert(TaintValue::BOTTOM);
            let old = lhs_val.clone();
            *lhs_val = lhs_val.join(rhs_val);
            changed |= *lhs_val != old;
        }
        changed
    }

    fn analyze_inst(&mut self, inst: &Instruction, cfg: &Cfg) {
        use Instruction::*;
        match inst {
            Copy { lhs, op } => {
                if let Operand::Var(v) = op {
                    let val = self.get(v);
                    self.values.insert(lhs.clone(), val);
                }
            }
            Load { lhs, src } => {
                let val = self.get(src);
                self.values.insert(lhs.clone(), val);
            }
            Store { dst, op } => {
                if let Operand::Var(v) = op {
                    let val = self.get(v);
                    self.values.insert(dst.clone(), val);
                }
            }
            CallExt { lhs, ext_callee, args } => {
                if ext_callee.name().starts_with("src") {
                    // Source call - taint the result
                    if let Some(l) = lhs {
                        self.values.insert(l.clone(), TaintValue::alpha(ext_callee.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    fn analyze_term(&mut self, term: &Terminal, cfg: &Cfg) -> Set<BbId> {
        use Terminal::*;
        match term {
            CallDirect { lhs, callee: _, args, next_bb: _ }
            | CallIndirect { lhs, callee: _, args, next_bb: _ } => {
                if let Some(l) = lhs {
                    let mut combined = TaintValue::BOTTOM;
                    for arg in args {
                        if let Operand::Var(v) = arg {
                            let val = self.get(v);
                            combined = combined.join(&val);
                        }
                    }
                    self.values.insert(l.clone(), combined);
                }
            }
            _ => {}
        }
        Set::new()
    }

    fn analyze_bb(&self, bb: &BasicBlock, cfg: &Cfg) -> (Vec<Self>, Set<BbId>) {
        let mut states = vec![];
        let mut curr_state = self.clone();

        for (i, inst) in bb.insts.iter().enumerate() {
            curr_state.curr_inst = Some((bb.id.clone(), i));
            curr_state.analyze_inst(inst, cfg);
            states.push(curr_state.clone());
        }

        curr_state.curr_inst = Some((bb.id.clone(), bb.insts.len()));
        curr_state.analyze_term(&bb.term, cfg);
        states.push(curr_state);

        (states, Set::new())
    }
}

// Helper function to collect sources and sinks from program
fn collect_external_funcs(func: &Function) -> (Set<FuncId>, Set<FuncId>) {
    let mut sources = Set::new();
    let mut sinks = Set::new();

    for bb in func.body.values() {
        for inst in &bb.insts {
            if let Instruction::CallExt { ext_callee, .. } = inst {
                if ext_callee.name().starts_with("src") {
                    sources.insert(ext_callee.clone());
                } else if ext_callee.name().starts_with("snk") {
                    sinks.insert(ext_callee.clone());
                }
            }
        }
    }

    (sources, sinks)
}

pub fn analyze(program: &Valid<Program>, func: FuncId, pts_to: Map<String, Set<String>>) -> String {
    let function = &program.0.functions[&func];
    let (sources, sinks) = collect_external_funcs(function);
    
    let mut taint_state = TaintState {
        sources,
        sinks: sinks.clone(),
        sink_map: Map::new()
    };

    // Initialize environment with explicit type annotation
    // We specify TaintValue as the type parameter since that's our concrete implementation
    let taint_env: PointwiseEnv<TaintValue> = PointwiseEnv {
        values: Map::new(),
        curr_inst: None
    };

    let cfg = Cfg::new(function, program.0.globals.clone(), program.0.structs.clone());
    
    // Add explicit type parameter to forward_analysis call
    forward_analysis::<TaintEnv>(
        function,
        &cfg,
        &taint_env,
        &taint_env
    );

    // Rest of the formatting code remains the same...
    let mut result = String::new();
    let mut sorted_sinks: Vec<_> = taint_state.sinks.iter().collect();
    sorted_sinks.sort_by(|a, b| a.name().cmp(b.name()));

    for sink in sorted_sinks {
        if let Some(sources) = taint_state.sink_map.get(sink) {
            let mut sorted_sources: Vec<_> = sources.iter().collect();
            sorted_sources.sort_by(|a, b| a.name().cmp(b.name()));
            
            result.push_str(&format!("{} -> {{", sink.name()));
            result.push_str(&sorted_sources.iter().map(|s| s.name()).collect::<Vec<_>>().join(", "));
            result.push_str("}\n");
        }
    }

    result.trim().to_string()
}