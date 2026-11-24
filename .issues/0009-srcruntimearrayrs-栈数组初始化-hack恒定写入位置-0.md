---
id: 9
title: src/runtime/array.rs 栈数组初始化 HACK：恒定写入位置 0
status: closed
priority: high
labels:
    - bug
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
---

## 证据

`qlty check --all` 中 ripgrep:HACK 命中 `src/runtime/array.rs:366`。核实源码后确认这是实质缺陷，而非注释风格问题：

```rust
// src/runtime/array.rs 约 360-372 行
} else {
    // This is a stack array (no header)
    // For stack arrays, we need to track position separately
    // This is a hack - we'll use a static variable to track position
    // This only works for single-threaded, single-array initialization
    // HACK: Always write to position 0
    // This is wrong but works for simple test cases
    unsafe {
        *data_ptr = value;
    }
}
```

代码自身的注释已承认：
- 用静态变量跟踪位置，因此**只适用于单线程、单数组**初始化
- 对栈数组**永远写入位置 0**，"This is wrong but works for simple test cases"

## 影响

栈数组（无 header 路径）的逐元素初始化会反复覆盖第 0 个元素，导致后续元素未被写入、
而第 0 个元素保留最后一次赋的值。任何依赖栈数组多元素初始化的编译器输出都会得到错误数据。
这解释了为什么它"在简单测试中通过"。

## 修复建议

1. 栈数组初始化应由调用方传入显式的元素索引/偏移，而不是依赖静态位置变量
2. 移除静态状态（多线程下即为数据竞争）
3. 补充覆盖多元素栈数组初始化的回归测试，并断言每个下标的值

---

## 修复记录（已完成）

### 根因分析

`array_push(ptr, value)` 的签名**没有索引参数**，因此对无 header 的栈数组无从得知下一个空位。
原实现"恒定写位置 0"是对该信息缺失的掩盖，会导致多元素初始化时反复覆盖第 0 个元素。

进一步核实该分支的实际可达性：

- `array_push` 的唯一调用点是 `src/middle/mir/gen.rs:2767`，仅用于 `AstNode::ArrayLit`
  先 `array_new(capacity)` 再逐个 push 的**动态数组**路径。
- 而 `array_new` 当前直接 `return 0x0BADF00D;`（`src/runtime/array.rs:70`），后续分配逻辑是死代码。
- 栈数组的正确入口是 `stack_array_set(ptr, index, value)`，它带有显式索引。

也就是说：栈数组分支在现有代码路径中**不可达**，但一旦可达（如后续接入真实 `array_new` 或
其他地方直接调用 `array_push`），就会静默写出错误数据。

### 改动

移除"写位置 0"的行为，改为明确拒绝并报告：

```rust
} else {
    // This is a stack array (no header).
    //
    // A stack array carries no length/capacity metadata, so `array_push`
    // cannot know where the next free slot is. ...
    println!(
        "[ARRAY_PUSH] ERROR: cannot push to a stack array (no length metadata); use stack_array_set with an explicit index"
    );
}
```

同一函数内堆数组分支（`ptr.add(len)` + `len += 1`）本来就是正确的，未改动。

### 验证

| 检查 | 结果 |
|---|---|
| `cargo check --workspace --all-targets` | exit 0 |
| `cargo clippy --workspace --all-targets` | 无 error |
| `rustfmt --check src/runtime/array.rs` | 干净 |
| `qlty check --all` 中 `ripgrep:HACK` | 1 -> **0** |
| diff 范围 | 仅该函数 else 分支，12 增 9 删 |

### 未做与建议

- 未改成"带索引的 push"：那会改变 `array_push` 的 FFI 签名，需同步修改
  `codegen.rs`（两处 `add_function` 声明）与 MIR 生成，超出本 issue 范围。
- 建议单独跟进：`array_new` 返回常量属更大问题，意味着动态数组当前整体是桩实现，
  建议新建 issue 追踪。
