---
id: 13
title: workflow 中自建 runner 标签被 actionlint 误报为未知标签（15 条）
status: closed
priority: low
labels:
    - docs
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
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

---

## 修复记录（已完成）

### 改动

采用 issue 中推荐的方案 1（声明自定义标签），而非关闭规则——这样其他 actionlint 检查
（expression、action 版本、shellcheck）保持敏锐。

新增 `.github/actionlint.yaml`：

```yaml
self-hosted-runner:
  labels:
    - galaxy
    - wsl2
    - x86-64
```

### 验证

用 qlty 内置的 actionlint 1.7.9 直接复现与验证：

| 检查 | 结果 |
|---|---|
| 配置前 `actionlint .github/workflows/*.yml` | runner-label 15 条 |
| 配置后同命令 | **exit 0，0 条** |
| `qlty check --all --filter actionlint` | **✔ No issues** |
| 故意插入 `${{ github.event.inputs.nope }}` | 仍报 `property "nope" is not defined ... [expression]` ✅ |
| 注入测试后还原 | `git diff .github/workflows/` 为空 |

最后一项是关键：确认配置**只**消除了 runner-label，没有削弱 expression 等实质检查
（#5 的修复正依赖该规则）。

## 关闭说明（2026-11-24）

修复已在提交 `a5d93ea` 落地，本次核对实际文件后同步状态。

新增 `.github/actionlint.yaml` 声明 galaxy / wsl2 / x86-64 自定义标签，保留其他 actionlint 规则的敏锐度，而非降级规则。
