---
id: 24
title: Benchmarks workflow 每日失败：main 根目录没有 Cargo.toml
status: open
priority: medium
labels:
    - bug
relations:
    related-to:
        - 23
created: "2026-11-26"
updated: "2026-11-26"
---

## 现象

`Benchmarks` workflow 每日调度（cron `0 2 * * *`）**连续失败**，本地可见的最近 10 次调度无一成功。

失败根因（取自 run 45598534150）：三个基准 job 全部倒在 “Build ...” 步骤：

```
error: could not find `Cargo.toml` in `/home/runner/work/zeta/zeta` or any parent directory
Process completed with exit code 101.
```

受影响 job：`compiler-benchmarks`、`runtime-benchmarks`、`regression-tests`（均为 `ubuntu-latest` 托管 runner）；`setup` 与 `benchmark-report` 步骤本身成功。

## 根因

workflow 假定仓库根存在 Rust 工程，但默认分支 `main` 的根目录**没有 `Cargo.toml`**（编译器工程只存在于 `bootstrap` 分支）。另外仓库也没有 `benches/` 目录，`Cargo.toml` 中无 `[[bench]]` 段，`cargo build --bench compiler_bench` 本就无目标。

## 影响

- 每日产生一条红色调度记录，掩盖真实回归信号。
- 基准数据长期缺失，性能回退无法度量。

## 建议修复

任选其一：

1. 在 `bootstrap` 分支上运行基准（加 `branches` 或在 checkout 时指定 ref）；
2. 改为下载已发布二进制做基准，去掉对仓库内 `Cargo.toml` 的依赖；
3. 补上 `benches/` 与对应 `[[bench]]` 定义；
4. 若基准暂不维护，直接停用该 workflow，避免每日噪音。
