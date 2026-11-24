---
id: 9
title: src/runtime/array.rs 栈数组初始化 HACK：恒定写入位置 0
status: open
priority: high
labels:
    - bug
created: "2026-11-24"
updated: "2026-11-24"
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
