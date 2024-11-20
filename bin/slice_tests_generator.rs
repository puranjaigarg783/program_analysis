use optimization::middle_end::lir::*;
use optimization::commons::Valid;
use std::fs;
use std::fs::File;
use std::io::Write;

fn write_to_file(data: &str, path: &str) {
    let mut f = File::create(path).expect("unable to create file");
    f.write_all(data.as_bytes()).expect("unable to write data");
}

fn main() {

    use optimization::middle_end::lir::Instruction::*;

    let paths = fs::read_dir("./test-inputs-slice/to_query/").unwrap()
        .filter_map(|res| res.ok())
        .map(|dir_entry| dir_entry.path())
        .filter_map(|path| {
            if path.extension().map_or(false, |ext| ext == "lir") {
                Some(path)
            } else {
                None
            }
        });
    for path in paths {
        let path_name = path.into_os_string().into_string().unwrap();

        let valid_program = parse_lir(&read_from(&path_name));
        let program = &valid_program.0;
        let functions = &program.functions;

        // check how many lines of code in each function although i'll prob just do everything
        let function_names = functions.iter().map(|(fid, f)| {
            let number_of_lines = f.body.iter().fold(0, |acc, (_bbid, bb)| acc + bb.insts.len() + 1);
            format!("[{} | {}]", fid.name(), number_of_lines)
        }).collect::<Vec<String>>().join(", ");

        // generate the query stuff
        let mut write_data = String::from("");
        for (fid, f) in functions {
            let func_name = fid.name();
            for (bbid, bb) in &f.body {
                let block_name = bbid.name();

                for (inst_idx, inst) in bb.insts.iter().enumerate() {
                    let idx_name = inst_idx.to_string();
                    let inst_name = match inst {
                        AddrOf { lhs: _, op: _ } => {
                            "addrof"
                        },
                        Alloc { lhs: _, num: _, id: _ } => {
                            "alloc"
                        },
                        Arith { lhs: _, aop:_, op1: _, op2: _ } => {
                            "arith"
                        },
                        Cmp { lhs: _, rop:_, op1: _, op2: _ } => {
                            "cmp"
                        },
                        CallExt { lhs: _, ext_callee: _, args: _ } => {
                            "callext"
                        },
                        Copy { lhs: _, op: _ } => {
                            "copy"
                        },
                        Gep {
                            lhs: _,
                            src: _,
                            idx: _,
                        } => {
                            "gep"
                        },
                        Gfp { lhs: _, src: _, field: _ } => {
                            "gfp"
                        }, 
                        Load { lhs: _, src: _ } => {
                            "load"
                        },
                        Store { dst: _, op: _ } => {
                            "store"
                        },
                        Phi { .. } => unreachable!(),
                    };

                    write_data += &format!("{func_name}#{block_name}#{idx_name},{inst_name}\n");
                }

                // check term
                let term_name = match &bb.term {
                    Terminal::Branch { cond: _, tt: _, ff: _ } => {
                        "branch"
                    },
                    Terminal::CallDirect {
                        lhs: _,
                        callee: _,
                        args: _,
                        next_bb: _,
                    } => {
                        "calldirect"
                    },
                    Terminal::CallIndirect {
                        lhs: _,
                        callee: _,
                        args: _,
                        next_bb: _,
                    } => {
                        "callindirect"
                    },
                    Terminal::Jump(_) => {
                        "jump"
                    },
                    Terminal::Ret(Some(_)) => {
                        "ret_some"
                    },
                    Terminal::Ret(None) => {
                        "ret_none"
                    }
                };
                write_data += &format!("{func_name}#{block_name}#term,{term_name}\n");
            }
        }
        
        // actually write the query files
        let query_file_path = &format!("{path_name}.queries");
        write_to_file(&write_data, query_file_path);

        // print function names and lines of code
        println!("{path_name}\nfunctions: {{{function_names}}}")
    }
}

fn read_from(path: &str) -> String {
    String::from_utf8(
        std::fs::read(path)
            .unwrap_or_else(|_| panic!("Could not read the input file {}", path)),
    )
    .expect("The input file does not contain valid utf-8 text")
}


fn parse_lir(input: &str) -> Valid<Program> {
    input.parse::<Program>().unwrap().validate().unwrap()
}