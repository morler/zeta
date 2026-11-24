---
id: 6
title: 工作流使用已归档的 actions-rs/toolchain@v1 与过旧的 actions/github-script@v6
status: closed
priority: medium
labels:
    - bug
relations:
    related-to:
        - 4
        - 7
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
---

## 证据

### 1) 已归档仓库（高危维护性风险）

`qlty check --all` 报 zizmor/archived-uses（1 条）：

```
.github/workflows/agent-branches.yml:38
  uses: actions-rs/toolchain@v1
```

核实：`actions-rs/toolchain` 仓库已归档，不再接收包括安全在内的任何更新。
同时 zizmor/actionlint 也对该 step 报「runner 过旧」。

### 2) 过旧 action（4 处）

actionlint 报 `the runner of "actions/github-script@v6" action is too old`：

- `.github/workflows/agent-branches.yml:174`
- `.github/workflows/agent-dashboard.yml:43, 220`
- `.github/workflows/agent-sync.yml:140`

## 修复建议

- `actions-rs/toolchain@v1` -> `dtolnay/rust-toolchain@stable`（同时可用 `with: components: clippy, rustfmt`）
- `actions/github-script@v6` -> `actions/github-script@v7`

## 相关

与 #4 同一批文件，建议合并为一次 PR 处理，避免重复改动工作流。

---

## 修复记录（已完成）

### 改动

1. **归档 action 替换**（`agent-branches.yml`）：
   `actions-rs/toolchain@v1` -> `dtolnay/rust-toolchain@stable`，`with.components` 保留
2. **github-script 升级** v6 -> v7，共 4 处：
   - `agent-branches.yml:174`
   - `agent-dashboard.yml:43, 220`
   - `agent-sync.yml:140`

### 验证

| 检查 | 结果 |
|---|---|
| `actionlint:action`（runner 过旧） | 5 条 -> **0** |
| `zizmor/archived-uses` | 1 条 -> **0** |
| 残留 `actions/github-script@v6` | 0（`grep` 确认） |
| 残留 `actions-rs/toolchain` | 0 |

### 未处理（属 #7）

这些 action 仍未 pin 到 commit SHA，`zizmor/unpinned-uses` 仍在，见 #7。
