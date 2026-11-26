---
id: 21
title: clippy -D warnings 有 13 个 error，CI check job 无法通过
status: closed
priority: high
labels:
    - bug
relations:
    blocks:
        - 25
    related-to:
        - 22
created: "2026-11-26"
updated: "2026-11-26"
closed: "2026-11-26"
---

## 现象

按 CI 口径执行 `RUSTFLAGS='-Dwarnings' cargo clippy --workspace --all-targets`，编译失败：**13 个 error**，lint 分布：

| lint | 数量 |
|---|---|
| `for_kv_map` | 5 |
| `unneeded_wildcard_pattern` | 4 |
| `redundant_field_names` | 1 |
| `question_mark` | 1 |
| `len_zero` | 1 |

具体位置：

```
src/middle/mir/gen.rs:1943:25             redundant_field_names
src/middle/optimization.rs:32:35          unneeded_wildcard_pattern
src/middle/optimization.rs:325:43         unneeded_wildcard_pattern
src/middle/optimization.rs:343:33         unneeded_wildcard_pattern
src/middle/optimization.rs:355:33         unneeded_wildcard_pattern
src/middle/resolver/module_resolver.rs:278:17   unused_mut
src/backend/codegen/codegen.rs:991:30     for_kv_map
src/backend/codegen/codegen.rs:4227:28    for_kv_map
src/middle/mir/gen.rs:1570:65             len_zero
src/middle/resolver/resolver.rs:524:21    question_mark
src/middle/types/identity/inference.rs:75:47    for_kv_map
src/package/enhanced.rs:248:26            for_kv_map
src/package/enhanced.rs:269:26            for_kv_map
```

## 影响

`ci.yml` 设定了 `RUSTFLAGS: "-Dwarnings"`，这些 warning 被提升为 error，`check` job 必红。

## 建议修复

`cargo clippy --fix --workspace --all-targets --allow-dirty` 可自动修掉大部分（for_kv_map / unneeded_wildcard_pattern / redundant_field_names / len_zero / unused_mut），`question_mark` 与剩余项手工处理；随后用上面的命令复验到 0 error。
