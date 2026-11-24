---
id: 16
title: while 条件中的函数调用读取未初始化槽位，循环条件恒为假
status: open
priority: high
labels:
    - bug
relations:
    depends-on:
        - 17
created: "2026-11-24"
updated: "2026-11-24"
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
