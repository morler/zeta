---
id: 7
title: 工作流 action 未 pin 到 commit SHA（31 条 unpinned-uses）
status: open
priority: medium
labels:
    - security
relations:
    related-to:
        - 6
        - 8
created: "2026-11-24"
updated: "2026-11-24"
---

## 证据

`qlty check --all` 中 zizmor/unpinned-uses 共 31 条 high（含重复位置），涉及全部 6 个工作流：

| 文件 | 代表性位置 |
|---|---|
| agent-branches.yml | 19, 38, 44, 128, 174 |
| agent-dashboard.yml | 15, 43, 208, 220 |
| agent-sync.yml | 16, 140, 164 |
| benchmarks.yml | 54, 69, 74, 88, 94, 98, 130, 136, 140, 172, 178, 182, 222 |
| ci.yml | 19, 31, 41, 74 |
| release.yml | 23, 49 |

涉及 `actions/checkout@v4`、`actions/cache@v4`、`actions/github-script@v6`、`softprops/action-gh-release@v2` 等。

## 风险

tag（如 `@v4`）可被上游强制移动。对自建 runner（`runs-on: [self-hosted, galaxy, wsl2, x86-64]`）尤其危险：
runner 常驻可信内网，被劫持的 action 可直接访问本地凭据。

## 修复建议

1. 全部改为 `@<commit-sha> # vX.Y.Z` 形式
2. 用 `pinact run` 或 Dependabot 的 `pin` 策略自动维护
3. 与 #6 合并处理（升版本的同时 pin SHA）
