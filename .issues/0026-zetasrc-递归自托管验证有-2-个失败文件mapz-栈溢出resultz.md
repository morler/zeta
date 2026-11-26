---
id: 26
title: zeta_src 递归自托管验证有 2 个失败文件（map.z 栈溢出、result.z 静默崩溃），且 CI 只检查顶层 5 个
status: open
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
