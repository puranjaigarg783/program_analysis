use super::super::constant_prop::*;
use super::*;
use crate::middle_end::lir::*;

fn run_on_file(test_name: &str) {
    run_test(test_name, constant_prop, "const_prop");
}

// Check if the input program optimizes to the expected output program
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

#[test]
fn simple_arith1() {
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

#[test]
fn simple_arith2() {
    optimizes_to(
        r#"
    fn test(top:int) -> _ {
    let a:int, b:int, c:int, d:int, e:int, f:int, g:int, h:int, i:int, j:int, k:int, l:int, bot:int
    entry:
      a = $arith add 1 1
      b = $arith sub a 4
      a = $arith mul b 2
      d = $arith div a 3
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
      b = $copy -2
      a = $copy -4
      d = $copy -1
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

#[test]
fn debugtimus() {
    optimizes_to(
        r#"
        fn main() -> int {
            let _105:int, _116:int, _119:int, _135:&&int, _144:&&int, _146:&&int, _152:&&int, _159:int, _170:&int, _20:&&int, _23:int, _26:&&int, _32:&&int, _47:int, _61:int, _64:int, _67:int, _78:int, _81:int, _86:int, _ret176:int, _t1:&&&int, _t10:&int, _t100:int, _t101:int, _t102:int, _t103:&&int, _t104:int, _t106:&int, _t107:&&&int, _t108:&&int, _t109:&int, _t11:&int, _t110:int, _t111:int, _t112:int, _t113:int, _t114:int, _t115:&int, _t117:&int, _t118:&&&int, _t12:int, _t120:&int, _t121:int, _t122:int, _t123:int, _t124:int, _t125:int, _t126:int, _t127:int, _t128:&&&&&int, _t129:&&&&int, _t13:int, _t130:&&&&int, _t131:&&&int, _t132:int, _t133:&&&int, _t134:&&int, _t136:&&&int, _t137:int, _t138:&int, _t139:int, _t14:int, _t140:int, _t141:int, _t142:int, _t143:int, _t145:&&&int, _t147:&&&int, _t148:&&&&int, _t149:&&&int, _t15:int, _t150:&&int, _t151:&int, _t153:&&&int, _t154:&&&int, _t155:&&int, _t156:int, _t157:int, _t158:&&int, _t16:int, _t160:&int, _t161:&int, _t162:int, _t163:int, _t164:int, _t165:&int, _t166:int, _t167:&int, _t168:int, _t169:int, _t17:&int, _t171:&&int, _t172:int, _t173:int, _t174:int, _t175:int, _t18:int, _t19:int, _t2:&&&int, _t21:&&&int, _t22:&int, _t24:&int, _t25:&&&int, _t27:&&&int, _t28:int, _t29:int, _t3:&&int, _t30:int, _t31:int, _t33:&&&int, _t34:int, _t35:&&&int, _t36:&&int, _t37:&&int, _t38:&int, _t39:int, _t4:&int, _t40:&&int, _t41:&int, _t42:int, _t43:int, _t44:&int, _t45:int, _t46:&&int, _t48:&int, _t49:int, _t5:&&&int, _t50:int, _t51:int, _t52:int, _t53:int, _t54:int, _t55:int, _t56:int, _t57:&&&int, _t58:&&int, _t59:int, _t6:&&int, _t60:int, _t62:&int, _t63:&int, _t65:&int, _t66:&&int, _t68:&int, _t69:int, _t7:&int, _t70:int, _t71:int, _t72:int, _t73:int, _t74:int, _t75:int, _t76:&&int, _t77:int, _t79:&int, _t8:&&&&int, _t80:&int, _t82:&int, _t83:int, _t84:int, _t85:int, _t87:&int, _t88:&int, _t89:int, _t9:&&int, _t90:int, _t91:int, _t92:&int, _t93:int, _t94:&int, _t95:int, _t96:int, _t97:&int, _t98:int, _t99:int, id0:int, id1:&&int, id10:&&&int, id11:&&&int, id12:int, id13:&&&int, id14:&int, id15:int, id16:&&&int, id17:&int, id18:&&int, id19:&int, id2:int, id20:&int, id21:&&&int, id22:&&&int, id23:&&int, id24:&&int, id25:&int, id26:int, id27:int, id28:&&int, id29:&&&&&int, id3:&int, id30:&&&int, id31:int, id32:&&&&int, id33:&&&&int, id4:int, id5:&&&int, id6:&&&&int, id7:&&int, id8:&int, id9:&int
            bb1:
              $branch 1 bb2 bb3
            
            bb10:
              _t172 = $arith sub 0 3
              _t173 = $arith sub 0 _t172
              id26 = $copy _t173
              _t174 = $load id3
              _t175 = $cmp gt _t174 id15
              _ret176 = $copy _t175
              $jump exit
            
            bb11:
              _t49 = $cmp eq 0 3
              _t50 = $copy _t49
              $branch _t50 bb12 bb13
            
            bb12:
              _t50 = $copy 3
              $jump bb13
            
            bb13:
              _t51 = $cmp gte _t50 id0
              $store id14 _t51
              _t52 = $copy 1
              $branch _t52 bb14 bb15
            
            bb14:
              _t52 = $copy 4
              $jump bb15
            
            bb15:
              _t53 = $load id8
              _t54 = $cmp gt _t52 _t53
              _t55 = $cmp eq 8 _t54
              id15 = $copy _t55
              $jump bb10
            
            bb16:
              _t56 = $load id17
              _t57 = $gep id16 _t56
              _t58 = $load _t57
              _t59 = $arith sub 0 3
              _t60 = $arith sub 0 _t59
              _t62 = $alloc _t60 [_61]
              $store _t58 _t62
              _t63 = $load id1
              $store id18 _t63
              _t65 = $alloc 2 [_64]
              id19 = $copy _t65
              $branch 7 bb18 bb38
            
            bb17:
              $store id23 id3
              _t171 = $alloc 1 [_170]
              id7 = $copy _t171
              id9 = $copy id25
              $jump bb10
            
            bb18:
              $branch 0 bb20 bb23
            
            bb19:
              _t101 = $arith sub 0 9
              _t102 = $copy _t101
              $branch _t102 bb27 bb26
            
            bb2:
              $jump bb4
            
            bb20:
              _t66 = $gep id23 0
              _t68 = $alloc 1 [_67]
              $store _t66 _t68
              _t69 = $arith add id0 3
              _t70 = $arith mul 7 _t69
              _t71 = $load id9
              _t72 = $cmp eq _t71 4
              _t73 = $cmp lte _t70 _t72
              id12 = $copy _t73
              _t74 = $arith sub 0 9
              _t75 = $copy _t74
              $branch _t75 bb22 bb21
            
            bb21:
              _t75 = $copy 3
              $jump bb22
            
            bb22:
              _t76 = $gep id23 _t75
              _t77 = $arith sub 0 id4
              _t79 = $alloc _t77 [_78]
              $store _t76 _t79
              $jump bb19
            
            bb23:
              _t80 = $load id1
              $store id24 _t80
              _t82 = $alloc 1 [_81]
              id25 = $copy _t82
              _t83 = $arith sub 0 2
              _t84 = $arith div 10 _t83
              _t85 = $cmp lt _t84 2
              id26 = $copy _t85
              _t87 = $alloc 1 [_86]
              id8 = $copy _t87
              _t88 = $gep id3 0
              _t89 = $load id3
              _t90 = $copy 4
              $branch _t90 bb25 bb24
            
            bb24:
              _t90 = $copy 8
              $jump bb25
            
            bb25:
              _t91 = $arith add _t89 _t90
              _t92 = $gep id17 _t91
              _t93 = $load _t92
              $store _t88 _t93
              _t94 = $load id23
              _t95 = $cmp neq _t94 id25
              id2 = $copy _t95
              _t96 = $arith sub 0 6
              _t97 = $gep id3 _t96
              _t98 = $load id25
              _t99 = $arith sub 0 4
              _t100 = $cmp eq _t98 _t99
              $store _t97 _t100
              $jump bb19
            
            bb26:
              _t102 = $copy 3
              $jump bb27
            
            bb27:
              _t103 = $gep id1 _t102
              _t104 = $arith sub 0 id26
              _t106 = $alloc _t104 [_105]
              $store _t103 _t106
              $jump bb28
            
            bb28:
              _t107 = $gep id16 9
              _t108 = $load _t107
              _t109 = $load _t108
              _t110 = $load _t109
              $branch _t110 bb29 bb30
            
            bb29:
              _t111 = $arith sub 0 id2
              _t112 = $copy _t111
              $branch _t112 bb31 bb32
            
            bb3:
              _t46 = $gep id1 7
              _t48 = $alloc 4 [_47]
              $store _t46 _t48
              $branch 8 bb11 bb16
            
            bb30:
              _t118 = $gep id11 5
              $store _t118 id18
              $jump bb33
            
            bb31:
              _t113 = $cmp eq 0 8
              _t112 = $copy _t113
              $jump bb32
            
            bb32:
              _t114 = $cmp gt 8 _t112
              $store id17 _t114
              _t115 = $gep id19 id15
              $store _t115 10
              _t117 = $alloc 1 [_116]
              id8 = $copy _t117
              $jump bb28
            
            bb33:
              $branch 4 bb34 bb35
            
            bb34:
              _t120 = $alloc 1 [_119]
              id14 = $copy _t120
              _t121 = $arith sub 0 6
              _t122 = $cmp gte 9 10
              _t123 = $cmp gt _t121 _t122
              _t124 = $cmp gte _t123 9
              id27 = $copy _t124
              _t125 = $arith sub 0 9
              _t126 = $copy _t125
              $branch _t126 bb37 bb36
            
            bb35:
              $jump bb17
            
            bb36:
              _t126 = $copy 3
              $jump bb37
            
            bb37:
              id12 = $copy _t126
              $jump bb33
            
            bb38:
              _t127 = $arith sub 0 3
              $branch _t127 bb40 bb47
            
            bb39:
              _t160 = $alloc 1 [_159]
              id8 = $copy _t160
              _t161 = $gep id3 0
              _t162 = $load id3
              _t163 = $copy 4
              $branch _t163 bb49 bb48
            
            bb4:
              $branch 4 bb5 bb6
            
            bb40:
              _t128 = $gep id29 5
              _t129 = $load _t128
              _t130 = $gep _t129 10
              _t131 = $load _t130
              _t132 = $arith sub 0 2
              _t133 = $gep _t131 _t132
              _t134 = $load _t133
              id28 = $copy _t134
              _t136 = $alloc 1 [_135]
              id30 = $copy _t136
              _t137 = $copy 3
              $branch _t137 bb42 bb41
            
            bb41:
              _t137 = $copy id2
              $jump bb42
            
            bb42:
              _t138 = $gep id17 _t137
              _t139 = $load _t138
              _t140 = $copy _t139
              $branch _t140 bb44 bb43
            
            bb43:
              _t141 = $copy 5
              $branch _t141 bb46 bb45
            
            bb44:
              id31 = $copy _t140
              $jump bb39
            
            bb45:
              _t141 = $copy 3
              $jump bb46
            
            bb46:
              _t142 = $cmp gte 0 10
              _t143 = $cmp neq _t141 _t142
              _t140 = $copy _t143
              $jump bb44
            
            bb47:
              _t145 = $alloc 1 [_144]
              id16 = $copy _t145
              id12 = $copy 5
              _t147 = $alloc 4 [_146]
              id21 = $copy _t147
              id4 = $copy 3
              _t148 = $gep id32 id12
              _t149 = $load _t148
              _t150 = $load _t149
              _t151 = $load _t150
              id19 = $copy _t151
              _t153 = $alloc 1 [_152]
              id13 = $copy _t153
              _t154 = $gep id11 8
              _t155 = $load _t154
              _t156 = $arith sub 0 2
              _t157 = $arith div 10 _t156
              _t158 = $gep _t155 _t157
              $store _t158 id25
              $jump bb39
            
            bb48:
              _t163 = $copy 8
              $jump bb49
            
            bb49:
              _t164 = $arith add _t162 _t163
              _t165 = $gep id17 _t164
              _t166 = $load _t165
              $store _t161 _t166
              _t167 = $load id1
              _t168 = $cmp neq _t167 id25
              _t169 = $cmp eq _t168 7
              id26 = $copy _t169
              id0 = $copy 4
              $jump bb17
            
            bb5:
              _t17 = $load id1
              _t18 = $cmp neq _t17 id3
              $branch _t18 bb8 bb9
            
            bb6:
              _t39 = $cmp eq 8 3
              _t40 = $gep id1 _t39
              _t41 = $load _t40
              _t42 = $cmp gt 6 7
              _t43 = $cmp eq 0 _t42
              _t44 = $gep _t41 _t43
              _t45 = $load _t44
              id12 = $copy _t45
              _ret176 = $copy 10
              $jump exit
            
            bb7:
              _t33 = $alloc 1 [_32]
              id10 = $copy _t33
              _t34 = $arith sub 0 3
              _t35 = $gep id13 _t34
              _t36 = $load _t35
              _t37 = $gep _t36 2
              _t38 = $load _t37
              id8 = $copy _t38
              $jump bb4
            
            bb8:
              id4 = $copy 5
              _t19 = $arith sub 0 id4
              _t21 = $alloc _t19 [_20]
              id5 = $copy _t21
              $jump bb7
            
            bb9:
              _t22 = $load id1
              $store id7 _t22
              _t24 = $alloc 2 [_23]
              id8 = $copy _t24
              id2 = $copy 9
              _t25 = $gep id11 5
              $store _t25 id1
              $store id3 7
              _t27 = $alloc 1 [_26]
              id11 = $copy _t27
              _t28 = $arith sub 0 6
              _t29 = $cmp gte 9 10
              _t30 = $cmp gt _t28 _t29
              _t31 = $cmp gte _t30 9
              id12 = $copy _t31
              $jump bb7
            
            entry:
              id2 = $copy 2
              id3 = $copy id3
              id4 = $copy 9
              id6 = $copy id6
              _t1 = $load id6
              id5 = $copy _t1
              id11 = $copy id11
              id10 = $copy id11
              _t2 = $gep id10 2
              _t3 = $load _t2
              _t4 = $load _t3
              id9 = $copy _t4
              id8 = $copy id9
              id12 = $copy 8
              id13 = $copy id10
              id15 = $copy 7
              id16 = $copy id10
              id22 = $copy id11
              id21 = $copy id22
              _t5 = $gep id21 2
              _t6 = $load _t5
              _t7 = $load _t6
              id20 = $copy _t7
              id19 = $copy id20
              id23 = $copy 0
              id25 = $copy id3
              id27 = $copy 8
              id28 = $copy id24
              id29 = $copy id29
              id31 = $copy 3
              _t8 = $load id29
              id33 = $copy _t8
              id32 = $copy id33
              _t9 = $gep id1 8
              _t10 = $load _t9
              _t11 = $gep _t10 8
              _t12 = $load _t11
              _t13 = $arith sub 0 id0
              _t14 = $cmp gt 6 7
              _t15 = $arith mul _t13 _t14
              _t16 = $arith div _t12 _t15
              id0 = $copy _t16
              id2 = $copy 6
              id2 = $copy 1
              $jump bb1
            
            exit:
              $ret _ret176
            }
    "#,
        r#"
    fn test(top:int) -> _ {
    let a:int, b:int, c:int, d:int, e:int, f:int, g:int, h:int, i:int, j:int, k:int, l:int, bot:int
    entry:
      a = $copy 2
      b = $copy -2
      a = $copy -4
      d = $copy -1
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

// These tests are in alphabetical order

#[test]
fn arith_div_by_zero() {
    run_on_file("arith-div-by-zero");
}

#[test]
fn arith0() {
    run_on_file("arith0");
}

#[test]
fn arith1() {
    run_on_file("arith1");
}

#[test]
fn arith2() {
    run_on_file("arith2");
}

#[test]
fn assign_and_arith() {
    run_on_file("assign_and_arith");
}

#[test]
fn assign_basic() {
    run_on_file("assign_basic");
}

#[test]
fn assign_basic2() {
    run_on_file("assign_basic2");
}

#[test]
fn assign_compare_nil() {
    run_on_file("assign_compare_nil");
}

#[test]
fn call() {
    run_on_file("call");
}

#[test]
fn call_direct1() {
    run_on_file("call_direct1");
}

#[test]
fn call_direct2() {
    run_on_file("call_direct2");
}

#[test]
fn call_extern() {
    run_on_file("call_extern");
}

#[test]
fn call_indirect() {
    run_on_file("call_indirect");
}

#[test]
fn compare1() {
    run_on_file("compare1");
}

#[test]
fn compare2() {
    run_on_file("compare2");
}

#[test]
fn fib1() {
    run_on_file("fib1");
}

#[test]
fn fib2() {
    run_on_file("fib2");
}

#[test]
fn global() {
    run_on_file("global");
}

#[test]
fn hw5() {
    run_on_file("hw5");
}

#[test]
fn hw6() {
    run_on_file("hw6");
}

#[test]
fn if1() {
    run_on_file("if1");
}

#[test]
fn if2() {
    run_on_file("if2");
}

#[test]
fn if3() {
    run_on_file("if3");
}

#[test]
fn if4() {
    run_on_file("if4");
}

#[test]
fn in_class_example() {
    run_on_file("in_class_example");
}

#[test]
fn linked_list() {
    run_on_file("linked_list");
}

#[test]
fn nested_field() {
    run_on_file("nested_field");
}

#[test]
fn nested_ptr() {
    run_on_file("nested_ptr");
}

#[test]
fn new_array() {
    run_on_file("new_array");
}

#[test]
fn new_deref() {
    run_on_file("new_deref");
}

#[test]
fn new_field() {
    run_on_file("new_field");
}

#[test]
fn not1() {
    run_on_file("not1");
}

#[test]
fn not2() {
    run_on_file("not2");
}

#[test]
fn not3() {
    run_on_file("not3");
}

#[test]
fn not_and_compare() {
    run_on_file("not_and_compare");
}

#[test]
fn tortoise_and_hare() {
    run_on_file("tortoise_and_hare");
}

#[test]
fn while1() {
    run_on_file("while1");
}

#[test]
fn while2() {
    run_on_file("while2");
}

#[test]
fn while3() {
    run_on_file("while3");
}
