---
id: 26
title: zeta_src 递归自托管验证有 2 个失败文件（map.z 栈溢出、result.z 静默崩溃），且 CI 只检查顶层 5 个
status: in-progress
priority: high
labels:
    - bug
created: "2026-11-26"
updated: "2026-11-26"
---

## 现象

本机补齐 LLVM 21 后，按 `ci.yml` 的 zeta-selfhost 口径递归验证 `zeta_src/**/*.z` 全部 51 个文件，**2 个失败**：

```
FAIL zeta_src/runtime/actor/map.z      -> 栈溢出崩溃（exit 134）
FAIL zeta_src/runtime/actor/result.z   -> 静默失败（exit 139）
```

`map.z` 报错原文：

```
thread 'main' (424867) has overflowed its stack
fatal runtime error: stack overflow, aborting
```

其余 49 个文件通过。

## 不是本次改动引入

已用修复前的提交 `03a7356` 单独构建对照版本复验，`map.z` exit 134、`result.z` exit 139，**与当前 HEAD 完全一致**，属既有缺陷。

## 为什么 CI 没发现

`ci.yml` 的 zeta-selfhost 步骤只遍历 `zeta_src/*.z`（顶层通配，实际只命中 5 个文件）：

```yaml
for f in zeta_src/*.z; do
```

子目录里的 46 个文件从未被验证，因此「51 个文件全部通过」的说法（README 与 release notes 均有表述）与 CI 实际覆盖范围不符。应改为 `find zeta_src -name '*.z'`。

## 影响

- 自托管完整度被高估；actor runtime 的 map/result 仍未打通。
- 栈溢出属于编译器崩溃，比普通编译错误更严重，需要单独定位。

## 建议修复

1. CI 改为递归遍历并把这些文件纳入门禁（与 #23 一并处理）。
2. `map.z`：定位栈溢出点，判断是递归下降无终止还是类型/泛型实例化循环。
3. `result.z`：当前静默 exit 139，先让编译错误可输出（不吞 stderr），再定位失败原因。

## 修复进展（2026-11-26）

### 已修复

1. **JIT 遇到未解析符号时不再段错误**
   `finalize_and_jit` 原先对未声明函数直接生成裸 external 声明，LLVM 惰性解析导致 `main()` 执行时段错误（exit 139）。
   现记录所有 `add_global_mapping` 的绑定名，建 JIT 引擎后静态比对 module 中的无函数体声明，
   有缺口就返回 `cannot execute: unresolved external symbol(s): ...`。
   同时支持按基名匹配，避免 `println_i64.6` 这类带后缀的声明被误报。

2. **`is_null` 方法调用错误**
   方法调用降级时函数名会带唯一后缀（`is_null_1`），而内联拦截只匹配 `"is_null"`，
   于是漏判为普通调用，生成未定义的 `@is_null`。新增 `base_intrinsic_name()` 去后缀匹配。

3. **补齐缺失的运行时映射**
   - `panic` → 新增 `zeta_panic`（打印消息后 abort）
   - `print_str` → `runtime::io::print_str`
   - `runtime_malloc` / `runtime_free` → 已有实现，补映射

4. **`result.z` 现在编译执行通过**（exit 0）。

### 仍失败（Rust 专属 API，非 Zeta 语义）

`map.z` 报：

```
unresolved external symbol(s): Box__from_raw, Box__new, Box__into_raw, &, cloned, unwrap_or
```

根因是 `zeta_src/runtime/actor/*.z` 里写的是 Rust 代码（`Box::new`、`HashMap::get().cloned().unwrap_or(0)`、
`impl` 泛型等），不是 Zeta 语法。这些构造在 Zeta 里没有对应语义，需要改写为 Zeta 等价实现。

全量统计：`zeta_src` 51 个文件中 39 通过、12 失败。之前统计的「49 通过」是把段错误当成成功的误判——
那 12 个在修复前同样是 139/134 崩溃，现在至少是明确诊断。

### 待办

- 改写 `map.z`（及同类 `*_test`/parser 文件）为 Zeta 语法
- 决定 `zeta_src/runtime/actor/*.z` 的真实定位：是 Zeta 源码还是 Rust 参考实现
