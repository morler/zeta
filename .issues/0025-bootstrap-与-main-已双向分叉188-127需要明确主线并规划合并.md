---
id: 25
title: bootstrap 与 main 已双向分叉（188 / 127），需要明确主线并规划合并
status: open
priority: high
labels:
    - bug
relations:
    depends-on:
        - 23
        - 19
        - 20
        - 21
        - 22
created: "2026-11-26"
updated: "2026-11-26"
---

## 现象

`bootstrap` 与 `main` 已深度双向分叉：

| 方向 | 提交数 | 时间范围 |
|---|---|---|
| `bootstrap` 领先 `main` | 188 | 2026-04-17 → 2026-11-24 |
| `main` 领先 `bootstrap` | 127 | → 2026-06-16 |

merge-base 停在 2026-04-15 的 v0.6.0。也就是说两边的分叉从半年前的基线开始，此后各自独立演进。

具体冲突点举例：

- `main` 上有 v1.0.22 / v1.0.23 的「LLVM X86 backend argument register dedup bug」修复（commit 987d6fe 等），`bootstrap` 上没有。
- `bootstrap` 上有 capybara、issue 门禁修复、运行时/编译器缺陷修复（#9、#11、#15、#16、#18 等）全量工作，`main` 上没有。
- 版本号三方不一致：`Cargo.toml` 为 `0.11.0`，README 讲 v0.11.0，仓库内已有 v0.14.7 / v0.14.8 / v1.0.19 的 release notes，`main` 已到 v1.0.23。
- `main` 根目录无 `Cargo.toml`/`zeta_src`（编译器工程只在 bootstrap 侧），直接影响基准与 CI 的运行环境（见 #23、#24）。

## 影响

- 修复无法双向流动：main 上的 X86 修复到不了 bootstrap，反之亦然。
- 分叉每延长一天，合并成本越高；127 × 188 的交叉改动已超出可安全审阅的规模。
- CI 与基准都建立在错误的分支假设上。

## 建议修复顺序

1. 先清干净门禁：#20（fmt）、#21（clippy）、#22（qlty TODO）、#19（build.z 截断），确保 bootstrap 自身是绿的。
2. 让 CI 覆盖 bootstrap（#23）。
3. 决定主线形态：若 `bootstrap` 才是主开发线，应把 `main` 的 127 个提交（含 X86 修复）逐项 cherry-pick / 分批合并回 bootstrap，而不是反向合并。
4. 统一版本号与 release notes 的归属，明确哪一个分支发布。
5. 合并后执行 `issues check --fix` 校验关系一致性。
