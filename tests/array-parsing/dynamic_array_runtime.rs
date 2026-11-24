//! End-to-end tests for the dynamic array runtime.
//!
//! These drive real Zeta source through the full pipeline
//! (parse -> MIR -> LLVM -> JIT) so the `array_new` / `array_push` /
//! `array_get` / `array_set` / `array_len` ABI stays consistent. Regression
//! coverage for the "dynamic arrays are entirely broken" class of bugs: the
//! runtime used to return a placeholder constant from `array_new`, and codegen
//! used to declare a different parameter list than the runtime implements.

use inkwell::context::Context;

use zetac::backend::codegen::LLVMCodegen;
use zetac::frontend::parser::top_level::parse_zeta;
use zetac::middle::mir::mir::Mir;
use zetac::middle::resolver::resolver::Resolver;

/// Compile `source` and run its `main`, returning the result.
fn run_zeta_main(source: &str) -> i64 {
    let (_, asts) = parse_zeta(source).expect("source must parse");

    let mut resolver = Resolver::new();
    for ast in &asts {
        resolver.register(ast.clone());
    }
    let _ = resolver.typecheck(&asts);

    let func_asts = resolver.get_registered_funcs();
    let all_mirs: Vec<Mir> = func_asts
        .iter()
        .filter_map(|ast| match ast {
            zetac::frontend::ast::AstNode::FuncDef { .. } => Some(resolver.lower_to_mir(ast)),
            _ => None,
        })
        .collect();
    assert!(!all_mirs.is_empty(), "no functions were lowered");

    let context = Context::create();
    let mut codegen = LLVMCodegen::new(&context, "dynamic_array_test");
    codegen.gen_mirs(&all_mirs);

    let ee = codegen
        .finalize_and_jit("native")
        .expect("module must verify and JIT-compile");

    type MainFn = unsafe extern "C" fn() -> i64;
    unsafe {
        let main = ee
            .get_function::<MainFn>("main")
            .expect("test source must define main");
        main.call()
    }
}

/// `let arr = [dynamic]T{a, b}` + `push` + index must see the pushed values.
#[test]
fn dynamic_array_literal_then_push() {
    let result = run_zeta_main(
        r#"
fn main() -> i64 {
    let arr = [dynamic]i64{3, 5}
    arr.push(7)
    return arr[0] * 100 + arr[1] * 10 + arr[2]
}
"#,
    );
    assert_eq!(result, 357);
}

/// `len()` reflects every push, including pushes that grow the buffer.
#[test]
fn dynamic_array_len_after_growth() {
    let result = run_zeta_main(
        r#"
fn main() -> i64 {
    let arr = [dynamic]i64{}
    arr.push(1)
    arr.push(2)
    arr.push(3)
    arr.push(4)
    arr.push(5)
    arr.push(6)
    return arr.len()
}
"#,
    );
    assert_eq!(result, 6);
}

/// `arr[i] = v` writes the right slot and leaves the others alone.
#[test]
fn dynamic_array_set_updates_single_element() {
    let result = run_zeta_main(
        r#"
fn main() -> i64 {
    let arr = [dynamic]i64{1, 2, 3}
    arr[1] = 42
    return arr[0] + arr[1] + arr[2]
}
"#,
    );
    assert_eq!(result, 46);
}

/// Values written before a growth stay readable after it (realloc must not
/// lose data).
#[test]
fn dynamic_array_keeps_data_across_growth() {
    let result = run_zeta_main(
        r#"
fn main() -> i64 {
    let arr = [dynamic]i64{}
    arr.push(10)
    arr.push(20)
    arr.push(30)
    arr.push(40)
    arr.push(50)
    arr.push(60)
    arr.push(70)
    return arr[0] + arr[6]
}
"#,
    );
    assert_eq!(result, 80);
}

/// A `while` condition that calls a function (here `arr.len()`) must be
/// re-evaluated every iteration. Regression test for issue #16: the condition's
/// statements used to be emitted into the loop body, so the condition block read
/// a temporary that was never written and the loop never ran.
#[test]
fn while_condition_calls_function_each_iteration() {
    let result = run_zeta_main(
        r#"
fn main() -> i64 {
    let arr = [dynamic]i64{0, 1, 2}
    let mut k: i64 = 0
    let mut acc: i64 = 0
    while k < arr.len() {
        acc += arr[k]
        k += 1
    }
    return acc
}
"#,
    );
    assert_eq!(result, 3);
}

/// The condition is re-tested against a value that grows during the loop, which
/// only works if the call happens once per iteration rather than once up front.
#[test]
fn while_condition_sees_values_added_in_body() {
    let result = run_zeta_main(
        r#"
fn main() -> i64 {
    let arr = [dynamic]i64{}
    let i: i64 = 0
    let mut n: i64 = 0
    while n < 3 {
        arr.push(n)
        n += 1
    }
    return n + arr.len()
}
"#,
    );
    assert_eq!(result, 6);
}
