---
id: 8
title: '工作流缺少显式 permissions 与 persist-credentials: false'
status: closed
priority: medium
labels:
    - security
relations:
    related-to:
        - 7
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
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

---

## 修复记录（已完成）

### 改动原则

按 issue 中已核实的分档处理：写操作 job 显式收窄权限，只读 job 设为 `contents: read`；
所有 `actions/checkout` 关闭凭据持久化（确需 push 的除外）。

### permissions（顶层只读，写权限下移到具体 job）

| 工作流 | 权限 |
|---|---|
| ci.yml | `contents: read` |
| benchmarks.yml | `contents: read` |
| release.yml | `contents: write`（发布 GitHub Release 所必需） |
| agent-dashboard.yml | `contents: read`、`actions: read`（列 workflow/run）、`issues: write`（commit comment 走 issues API） |
| agent-branches.yml | 顶层 `contents: read`；`pr-comment` job 追加 `pull-requests: write`、`issues: write` |
| agent-sync.yml | 顶层 `contents: read`；`sync-and-merge` job `issues: write`；`merge-to-main` job `contents: write` |

初版把写权限放在工作流顶层，zizmor 仍报 `excessive-permissions`（"overly broad at the
workflow level"），因此改为 job 级，符合最小权限。

### persist-credentials: false

13 处 `actions/checkout` 全部设置 `persist-credentials: false`，唯一例外是
`agent-sync.yml` 的 `merge-to-main` job —— 它要 `git push origin v0.3.9`，
保留 `persist-credentials: true` 并加注释说明原因。

### 验证

| 检查 | 结果 |
|---|---|
| `zizmor/artipacked` | 14 -> **0** ✅ |
| `zizmor/excessive-permissions` | 19 -> **0** ✅ |
| `actionlint` 全部工作流 | exit 0 |
| 各工作流权限结构 | 逐项核对需求（见上表） |

### 未处理

`unpinned-uses`（31 条）属于 #7，本次未动。

## 关闭说明（2026-11-24）

修复已在提交 `4e82ae9` 落地，本次仅核对后同步状态。

核实方式：

```
for f in .github/workflows/*.yml; do grep -q '^permissions:' "$f" || echo "MISSING: $f"; done
# -> 无输出，6 个工作流均声明了显式 permissions
```

`persist-credentials` 已逐处设置：所有 checkout 默认 `false`，唯一例外是
`agent-sync.yml:198` 的 `true`，该处需要把合并结果推回分支，且已就地注释说明。
