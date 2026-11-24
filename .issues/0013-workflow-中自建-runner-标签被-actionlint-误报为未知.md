---
id: 13
title: workflow 中自建 runner 标签被 actionlint 误报为未知标签（15 条）
status: open
priority: low
labels:
    - docs
created: "2026-11-24"
updated: "2026-11-24"
---

## 证据

`qlty check --all` 报 actionlint:runner-label 共 15 条：

```
.github/workflows/ci.yml:17,28,38,71
.github/workflows/release.yml:21
  medium  label "galaxy" is unknown
  medium  label "wsl2" is unknown
  medium  label "x86-64" is unknown
```

## 核实结论：误报

`ci.yml` 第 1 行已写明用途：

```yaml
# CI for Zeta — runs on self-hosted Galaxy runner
```

所有位置均为 `runs-on: [self-hosted, galaxy, wsl2, x86-64]`，
`galaxy`/`wsl2`/`x86-64` 是自建 runner 的标签，actionlint 只校验 GitHub 官方托管标签列表。

## 修复建议

二选一：

1. 在仓库加 `.github/actionlint.yaml` 声明自定义标签：
   ```yaml
   self-hosted-runner:
     labels:
       - galaxy
       - wsl2
       - x86-64
   ```
2. 或在 qlty 配置中把 `actionlint:runner-label` 降级为不报

方案 1 更优：保留对其他 actionlint 规则（如 #5 的 expression 检查）的敏锐度。

## 说明

此条登记的目的是防止误报长期污染 `qlty check` 输出，导致真问题（如 #5）被淹没。
