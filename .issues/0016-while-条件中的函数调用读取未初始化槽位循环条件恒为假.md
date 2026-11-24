---
id: 16
title: while 条件中的函数调用读取未初始化槽位，循环条件恒为假
status: closed
priority: high
labels:
    - bug
relations:
    depends-on:
        - 17
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
---

## 证据

修复 #15 后动态数组主链路已通（alloc/push/get/set/len/free），但发现一个独立的循环 lowering 缺陷。

### 复现

```zeta
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
```

期望 3，实际 0。

### 生成的 IR

```llvm
while.cond:
  %22 = load i64, ptr %4        ; k
  %23 = load i64, ptr %13       ; <-- 从未被 store 过的栈槽
  %cmp_lt = icmp slt i64 %22, %23
```

`array_len` 的调用被放在 `while.body` 里：

```llvm
while.body:
  %24 = load i64, ptr %16       ; arr 指针
  %25 = call i64 @array_len(i64 %24)
  store i64 %25, ptr %13        ; 写进 %13
```

于是条件分支每次都读 %13 的未初始化值。

### 根因

`src/middle/mir/gen.rs:821-843` 的 `AstNode::While` lowering 把
`lower_expr(cond)` 产生的**全部**语句（包括产生条件值的 Call）都塞进了
`MirStmt::While { body }`，而 `cond` 只保留最终比较的表达式 ID。
条件若包含有副作用的子表达式（函数调用），其副作用只发生在 body 里，
条件先于副作用求值。

注释里说明了这么做是为了让 `p * p < n` 这类 SemiringFold 每次迭代重算，
但 Call 的返回值同样需要在条件求值前先落到槽位。

### 范围

任何在 while 条件中调用带返回值函数的写法都会命中，不只是 `arr.len()`。
`while i < limit()` 形式可正常工作（被内联成表达式），因此缺陷与具体调用是否内联有关。

### 建议

把 cond 的求值语句拆成两类：产生 cond 值的语句每轮迭代在条件前执行，
纯副作用语句才放进 body；或在 `MirStmt::While` 中显式区分
`cond_prelude` 与 `body`。

## 相关
- #15 修复过程中发现

## 修复结论（2026-11-24）

### 根因确认

`AstNode::While` 的 lowering 把 `lower_expr(cond)` 产生的**全部语句**都塞进了
`MirStmt::While.body`，而 codegen 的 `while.cond` 基本块只对 `cond` 表达式求值
（`gen_expr_safe(cond)`），不执行任何语句。于是：

- `array_len(arr)` 的调用体在 `while.body` 里，写进某个临时槽位；
- `while.cond` 每轮先读该槽位 —— 但它在这个块里从未被写；
- 槽位是 `alloca` 出来的未初始化内存，读到的值不确定，实测为 0，
  导致 `k < value` 恒假、循环一次都不执行。

这也解释了为什么 `while i < limit()`（用户函数）看起来正常：那种条件被内联成了表达式，
没有产生独立语句。

### 修法：脱糖为 `loop { ...; if C { B } else { break } }`

在 `src/middle/mir/gen.rs` 里把

```zeta
while C { B }
```

改写成

```
loop {
    <lowering C 产生的语句>
    if C { B } else { break }
}
```

选择脱糖而不是给 `MirStmt::While` 加 `cond_stmts` 字段的理由：

- 该变体在 12 处被构造/匹配（`mir.rs`、`gen.rs` ×5、`optimization.rs`、
  `monomorphize.rs`、`codegen.rs` ×4），加字段的改动面与出错面都更大；
- 脱糖直接复用了 codegen 里已实现并已验证的 `If` + `Break` 跳出路径，
  `if C { B } else { break }` 的语义与 `while C { B }` 完全等价；
- 条件语句落在 `if` **之前**、`while.body` 之内，因此每轮迭代都先执行、再判断。

关键细节：语句必须放在 `if` 之前，而不是 `if` 的 then 分支里。第一版草稿放在 then 分支，
测试仍然返回 0——因为 `If` 是先求值条件、后执行 then。

### 验证

| 检查 | 结果 |
|---|---|
| `test_array_index_while.z`（5 次 push 求和） | **0 → 2**（正确值） |
| `cargo test` | 32 个目标全部 ok |
| 新增 2 个回归测试（见下） | 通过 |
| 自举 `zeta_src/*.z` 全部编译 | 0 失败 |
| `qlty check` | 退出码 0 |

新增的回归测试在 `tests/array-parsing/dynamic_array_runtime.rs`：

- `while_condition_calls_function_each_iteration` —— 条件里调用 `arr.len()`
- `while_condition_sees_values_added_in_body` —— 条件依赖循环体内增长的状态

**已反证测试有效**：把 `src/middle/mir/gen.rs` 回退到修复前的版本，
`while_condition_calls_function_each_iteration` 立即 FAILED（5 passed, 1 failed），
恢复后全绿。

额外手工验证（临时 `.z`，未入库）：`break`、`continue`、嵌套 `while`、
条件里调用用户函数、嵌套循环里用 `arr.len()` —— 结果均符合预期。

### 顺带发现（已单独立项）

`tests/unit-tests/test_while_loop.z` 现在能走进更深的编译阶段，暴露出
`arr.push(i as u8)` 的实参未提升为 i64 的问题 → 登记为 **#18**。
已用 `git stash` 回退确认该错误在本次修复前就存在，与本 issue 无关。
