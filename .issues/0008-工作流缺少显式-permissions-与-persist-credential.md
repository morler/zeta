---
id: 8
title: '工作流缺少显式 permissions 与 persist-credentials: false'
status: open
priority: medium
labels:
    - security
relations:
    related-to:
        - 7
created: "2026-11-24"
updated: "2026-11-24"
---

## 证据

`qlty check --all` 中：
- zizmor/excessive-permissions：19 条（全部 6 个工作流均未声明 `permissions:` 块）
- zizmor/artipacked：14 条（`actions/checkout` 未设 `persist-credentials: false`）

位置示例：
- `.github/workflows/ci.yml:2, 16, 27, 37, 70`
- `.github/workflows/benchmarks.yml:5, 49, 83, 125, 167, 217`
- `.github/workflows/agent-branches.yml:1, 13, 18`
- `.github/workflows/agent-sync.yml:1, 10, 15, 156, 163`

## 分档结论（已核实，不宜一律按 high 处理）

**确实需要修：**

- 会 `upload-artifact` / `gh release` 的 job —— `persist-credentials` 未关闭时，
  checkout 写入的 token 可能被打进 artifact 或 release 资产。
  涉及 `benchmarks.yml`、`release.yml`、以及 `agent-sync.yml` 中使用 `secrets.GITHUB_TOKEN` 的 job。
- `agent-sync.yml` 的 merge/写操作 job，应显式收窄到 `contents: write`、`issues: write` 等实际所需权限。

**风险较低、可按提示级处理：**

- `ci.yml` 的各 job 只执行 cargo 命令，不调用 GitHub API，仓库默认 token 为只读。
  这里的 excessive-permissions 属「未显式最小化」，不是实际可利用面。

## 修复建议

1. 每个工作流顶部加最小 `permissions:` 块，例如 CI：
   ```yaml
   permissions:
     contents: read
   ```
2. 所有 `actions/checkout` 加 `with: persist-credentials: false`（需要写操作的 job 除外，且写操作改用显式 token）
