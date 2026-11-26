---
id: 23
title: ci.yml 未覆盖 bootstrap 分支，188 个提交从未跑过 CI
status: open
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
