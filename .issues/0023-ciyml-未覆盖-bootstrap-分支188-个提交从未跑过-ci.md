---
id: 23
title: ci.yml 未覆盖 bootstrap 分支，188 个提交从未跑过 CI
status: in-progress
priority: high
labels:
    - bug
relations:
    blocks:
        - 25
    related-to:
        - 24
created: "2026-11-26"
updated: "2026-11-26"
---

## 现象

`.github/workflows/ci.yml` 的触发配置：

```yaml
on:
  push:
    branches: [ main, dev ]
  pull_request:
    branches: [ main, dev ]
```

而实际开发发生在 `bootstrap` 分支：该分支领先 `origin/main` **188 个提交**（2026-04-17 → 2026-11-24），这些提交**没有任何 CI 记录**。`ci.yml` 最近一次运行是 2026-06-16 且结论为 failure。

## 影响

- check / clippy / fmt / test / zeta-selfhost 全部未在 bootstrap 上执行过，回归无法在上游被拦截。
- 与 #20、#21 叠加：分支上本来就存在的 fmt/clippy 失败直到合并前都无人发现。

## 建议修复

把 `bootstrap`（以及后续的活跃集成分支）加入 `push` 与 `pull_request` 的 `branches` 列表；若自建 runner 资源紧张，可对该分支只跑 check + test，跳过 zeta-selfhost 全量编译。

## 补充发现（2026-11-26，本机补齐 LLVM 21 后实测）

本机可以完整构建并发起验证后，确认 ci.yml 的实际覆盖比预期更弱：

1. **zeta-selfhost 只覆盖 5/51 个文件**
   `for f in zeta_src/*.z` 是顶层通配，实际命中 5 个；另 46 个在子目录中从未编译过。
   递归验证 `zeta_src/**/*.z` 后发现 2 个失败文件，详见 #26。

2. **Run known-good tests 步骤是死代码**
   该步骤遍历 `tests/test_hello.z`、`tests/test_values.z`、`tests/test_basic.z`，
   但仓库里 `tests/` 根目录**没有任何 `.z` 文件**（`ls tests/*.z` 报 No such file）。
   实际存在 477 个 `.z` 测试文件，全部在子目录中，从未被执行。

3. 该步骤用 `|| echo "⚠️ ..."` 吞掉失败，即使文件存在也不会让 job 失败。

因此「zeta_src 全部编译通过」这一结论目前并不被 CI 真实支撑，建议与本 issue 一并修正：

```yaml
- name: Compile all zeta_src/ files
  run: |
    find zeta_src -name '*.z' -print0 | xargs -0 -n1 ./target/release/zetac
```
