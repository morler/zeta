---
id: 18
title: array_push 的 as 转换实参未提升为 i64，LLVM 校验报参数类型不匹配
status: open
priority: medium
labels:
    - bug
created: "2026-11-24"
updated: "2026-11-24"
---

## 证据

`tests/unit-tests/test_while_loop.z` 无法编译：

```zeta
let arr: [dynamic]u8 = [dynamic]u8{}
arr.push(i as u8)
```

报错：

```
Error: "Call parameter type does not match function signature!
  %trunc_u8 = trunc i64 %18 to i8
 i64  %19 = call i64 @array_push(i64 %17, i8 %trunc_u8)"
```

## 根因方向

MIR 把 `i as u8` 降成 `trunc i64 -> i8`，但调用点没有把实参提升回 i64。
数组运行时统一按 i64 槽位存元素（`array_push(ptr: i64, value: i64)`），
所以窄类型实参需要在调用前 `z_ext` / `s_ext` 到 i64。

## 影响

任何 `arr.push(x as u8)` / `arr[i] = x as u8` 的写法都无法编译。
这类写法在 `tests/` 里很常见（`[dynamic]u8` 是默认示例类型）。

## 与 #16 无关（已核实）

在 #16 修复前的 HEAD 上用 `git stash` 回退 `src/middle/mir/gen.rs`，
复现同样的报错。也就是说 #16 只是让该测试文件走到更远的阶段才暴露它。

## 建议

在 codegen 处理 `MirStmt::Call` / `VoidCall` 实参时，按被调函数的形参宽度
对窄整型做提升；或在 MIR lowering 阶段就把 `as` 的结果统一表示为 i64。
