---
id: 20
title: cargo fmt --check 在已提交代码上有 29 处差异，CI check job 必红
status: open
priority: medium
labels:
    - bug
relations:
    blocks:
        - 25
created: "2026-11-26"
updated: "2026-11-26"
---

## 现象

`cargo fmt --all -- --check` 在已提交代码上即有失败，共 29 处差异，涉及 6 个文件：

| 文件 | 差异数 |
|---|---|
| `src/backend/codegen/codegen.rs` | 17 |
| `src/middle/resolver/module_resolver.rs` | 6 |
| `src/middle/mir/gen.rs` | 2 |
| `src/frontend/cfg.rs` | 2 |
| `src/middle/optimization.rs` | 1 |
| `src/frontend/mod.rs` | 1 |

多为多余空行/缩进等纯格式差异。

## 影响

`.github/workflows/ci.yml` 的 `check` job 顺序为 `cargo check` → `cargo clippy -D warnings` → `cargo fmt --all -- --check`。fmt 失败会让整个 check job 变红，进而阻塞 `test` 与 `zeta-selfhost` 两个下游 job。

## 建议修复

```sh
cargo fmt --all
cargo fmt --all -- --check   # 确认无差异
```

提交前按项目约定运行 `qlty check`。
