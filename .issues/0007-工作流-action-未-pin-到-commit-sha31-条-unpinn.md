---
id: 7
title: 工作流 action 未 pin 到 commit SHA（31 条 unpinned-uses）
status: closed
priority: medium
labels:
    - security
relations:
    related-to:
        - 6
        - 8
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
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

---

## 修复记录（已完成）

### 改动

全部 26 处 action 引用改为 commit SHA，并保留版本注释：

| action | SHA | 版本 |
|---|---|---|
| actions/checkout | `11d5960a326750d5838078e36cf38b85af677262` | v4（14 处） |
| actions/cache | `0057852bfaa89a56745cba8c7296529d2fc39830` | v4（5 处） |
| actions/github-script | `f28e40c7f34bde8b3046d885e986cb6290c5673b` | v7（4 处） |
| actions/upload-artifact | `ea165f8d65b6e75b540449e92b4886f43607fa02` | v4（2 处） |
| dtolnay/rust-toolchain | `683b588c487a69d01f6c6ddf40f036ff682cc2c8` | v1（5 处） |

SHA 均通过 `git ls-remote` 从上游仓库实际解析，未凭记忆填写。

### dtolnay/rust-toolchain 的特殊处理

该 action 原本用 `@master` / `@stable` **分支名**作为 toolchain 选择器。直接替换成 SHA 会丢失该语义，
因此：

- 查验其 README：明确要求 pin 到 **master 历史内**的 SHA，否则会被 GC 导致 workflow 失败
- `git merge-base --is-ancestor` 验证 `v1`（`683b588`）确实在 master 历史内；
  而 `stable` 分支头 `3900882` **不在** master 历史内（不可用）
- 因此 pin 到 `v1` 的 SHA，并为原本依赖 `@stable` 的那处显式补上 `toolchain: stable`：

```yaml
uses: dtolnay/rust-toolchain@683b588... # v1
with:
  toolchain: stable   # was implied by the @stable branch ref
```

benchmarks.yml 的 4 处本来就显式写了 `toolchain: nightly`，直接 pin 即可。

### 顺带修复：release.yml 的 inputs.tag 从未被使用

`workflow_dispatch.inputs.tag` 有定义但整个工作流没有任何地方引用它，
即手动发布时该输入完全无效。改用 `gh release create` 后一并处理：

```yaml
TAG="${GITHUB_REF_NAME}"
if [ "$GITHUB_EVENT_NAME" = "workflow_dispatch" ]; then
  if [ -n "$INPUT_TAG" ]; then TAG="$INPUT_TAG"
  else TAG="build-${GITHUB_SHA::12}"; fi
fi
```

同时这也解决了 `zizmor/superfluous-actions` —— 原 `softprops/action-gh-release`
的功能 runner 已内置，用 `gh release create --generate-notes --latest <files>` 等价且更简洁。

### 验证

| 检查 | 结果 |
|---|---|
| `zizmor/unpinned-uses` | 31 -> **0** ✅ |
| `zizmor/superfluous-actions` | 1 -> **0** ✅ |
| `qlty check --all --filter zizmor` | **✔ No issues**（原 58 条） |
| 未 pin 的 `uses:` | 0（`grep -vE '@[0-9a-f]{40}'` 确认） |
| `actionlint` 全部工作流 | exit 0 |
| SHA 来源 | `git ls-remote` 实际解析 |

### 后续维护建议

pin SHA 后不会自动跟随上游安全更新，建议启用 Dependabot 的 `github-actions` 生态，
让它定期提 PR 升 SHA（可配合 `pinact` 保持 SHA 形式）。

## 关闭说明（2026-11-24）

修复已在提交 `a34a5ae` 落地，本次仅核对后同步状态。

核实方式：

```
grep -rn 'uses:' .github/workflows/*.yml | grep -vE 'uses: [^@]+@[0-9a-f]{40}'
# -> 无输出，即没有未 pin 的 action
grep -rn 'uses:' .github/workflows/*.yml | grep -cE '@[0-9a-f]{40}'
# -> 30，全部 pin 到 40 位 commit SHA
```
