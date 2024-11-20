
use optimization::middle_end::{analysis::*, lir::*, analysis::constant_prop::Env};
use optimization::middle_end::optimization::constant_prop::*;
use optimization::commons::Valid;
use std::collections::BTreeMap as Map;
use pretty_assertions::assert_eq;

// cheesing the assignment might not have been worth it i should just write a parser

fn main() {

let binary_trees = "./test-inputs/complex/lambda";
let arith_non_div = "./test-inputs/simple/arith_non_div";

let input_string = read_from(&lir_path(binary_trees));
let expected = read_from(&soln_path(binary_trees));

let lir_parsed = parse_lir(&input_string);
let analyzed = constant_prop::analyze(&lir_parsed, func_id("main"));

let output = nicely(&analyzed.1);

println!("{}", output);

}

fn lir_path(path: &str) -> String {
    format!("{path}.lir")
}

fn soln_path(path: &str) -> String {
    format!("{path}.constants.soln")
}

fn read_from(path: &str) -> String {
    String::from_utf8(
        std::fs::read(path)
            .unwrap_or_else(|_| panic!("Could not read the input file {}", path)),
    )
    .expect("The input file does not contain valid utf-8 text")
}

fn nicely(result: &Map<InstId, Env>) -> String {
    
    // this is very lazy and bad haha !
    let mut s = String::new();
    let mut c: Map<BbId, Env> = Map::new();
    for ((bb, n), env) in result {
        c.insert(bb.clone(), env.clone());
    }
    for (bb, env) in c {
        s += &bb.to_string();
        s += ":\n";
        s += &env.to_string();
        s += "\n";
    }
    s
}


fn parse_lir(input: &str) -> Valid<Program> {
    input.parse::<Program>().unwrap().validate().unwrap()
}

fn optimizes_to(input: &str, expected: &str) {
    // parse & sanitize the inputs
    let input = input.parse::<Program>().unwrap().validate().unwrap();
    let expected = expected
        .parse::<Program>()
        .unwrap()
        .validate()
        .unwrap()
        .0
        .to_string();

    let actual = constant_prop(input).0;
    assert_eq!(actual.to_string(), expected);
}

fn test1() {
    optimizes_to(
        r#"
    fn test(top:int) -> _ {
    let a:int, b:int, c:int, d:int, e:int, f:int, g:int, h:int, i:int, j:int, k:int, l:int, bot:int
    entry:
      a = $arith add 1 1
      b = $arith sub 1 1
      c = $arith mul 2 2
      d = $arith div 4 2
      bot = $arith div 2 0
      e = $arith add top 1
      f = $arith sub top 1
      g = $arith mul 1 top
      h = $arith div 1 top
      i = $arith add bot 1
      j = $arith sub bot 1
      k = $arith mul 1 bot
      l = $arith div 1 bot
      i = $arith add bot top
      j = $arith sub bot top
      k = $arith mul top bot
      l = $arith div top bot
      $ret
    }
    
    fn main() -> int {
    entry:
      $ret 0
    }
    "#,
        r#"
    fn test(top:int) -> _ {
    let a:int, b:int, c:int, d:int, e:int, f:int, g:int, h:int, i:int, j:int, k:int, l:int, bot:int
    entry:
      a = $copy 2
      b = $copy 0
      c = $copy 4
      d = $copy 2
      bot = $arith div 2 0
      e = $arith add top 1
      f = $arith sub top 1
      g = $arith mul 1 top
      h = $arith div 1 top
      i = $arith add bot 1
      j = $arith sub bot 1
      k = $arith mul 1 bot
      l = $arith div 1 bot
      i = $arith add bot top
      j = $arith sub bot top
      k = $arith mul top bot
      l = $arith div top bot
      $ret
    }
    
    fn main() -> int {
    entry:
      $ret 0
    }
    "#,
        );
}