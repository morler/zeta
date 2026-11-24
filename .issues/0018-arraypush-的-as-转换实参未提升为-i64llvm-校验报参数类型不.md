---
id: 18
title: array_push 的 as 转换实参未提升为 i64，LLVM 校验报参数类型不匹配
status: closed
priority: medium
labels:
    - bug
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
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

## 修复结论（2026-11-24）

### 根因

`MirExpr::As` 在 codegen 里被降成**真正的窄整型值**：

```rust
Type::U8 => self.builder.build_int_truncate(int_val, self.context.i8_type(), "trunc_u8")
```

但调用点（`MirStmt::Call` 与 `VoidCall` 两处）拿到实参后直接塞进 `build_call`，
没有按被调函数的形参宽度做适配。而运行时 ABI（`array_push(ptr: i64, value: i64)`）
和绝大多数声明函数都收 `i64`，于是产生：

```
%trunc_u8 = trunc i64 %22 to i8
%23 = call i64 @array_push(i64 %21, i8 %trunc_u8)   ; ← LLVM 校验拒绝
```

### 修法

新增 `LLVMCodegen::coerce_arg_to_param(val, callee, index)`：按实参位置读取
被调函数的 `BasicMetadataTypeEnum`，仅在**两侧都是整型且位宽不同**时做
`z_ext`（变宽）或 `trunc`（变窄），其余情况（位宽相同、指针/浮点/向量形参）
原样返回。

选择变宽用 `z_ext` 而非 `s_ext`：`As` 已经先截断过，高位本就丢了，
零扩展与 cast 表达的语义一致（`300 as u8` → 44，再变宽仍是 44）。

接到两处调用点：`MirStmt::Call`（约 2796 行）与 `MirStmt::VoidCall`（约 3307 行）。

### 验证

| 检查 | 结果 |
|---|---|
| `test_while_loop.z`（原本触发该错误） | 编译通过，返回 0（该测试的"全部通过"标志） |
| 拆解三个子测试 | 5 / 5 / 2，与注释预期完全一致 |
| 多宽度 cast 手工验证 | `u8`(300→44→45)、`i32`、`u16` 全部正确 |
| `cargo test` | 32 个目标全部 ok |
| 自举 `zeta_src/*.z` | 0 失败 |
| `qlty check` | 退出码 0 |

新增 2 个回归测试（`tests/array-parsing/dynamic_array_runtime.rs`）：

- `push_of_cast_value_compiles_and_stores` —— `arr.push(i as u8)` 全链路
- `cast_argument_to_plain_function_is_widened` —— 普通函数调用的 `as` 实参

**已反证测试有效**：把 `src/backend/codegen/codegen.rs` 回退到修复前，
两个新测试均 FAILED（6 passed, 2 failed），恢复后 8 个全绿。

### 备注

`test_while_loop.z` 返回 0 容易误读为"失败"。该文件的设计是所有子测试
都符合预期时才返回 0，因此 0 是通过标志；我另用临时用例把三个子结果
按位拼成 `552` 单独确认过。
