---
id: 22
title: qlty check 仍有 10 个 TODO/NOTE 问题；#14 关闭后桩实现缺口已无追踪
status: open
priority: medium
labels:
    - bug
relations:
    blocks:
        - 25
    related-to:
        - 21
created: "2026-11-26"
updated: "2026-11-26"
---

## 现象

`qlty check` 当前报 **10 个 issue**：9 条来自 `ripgrep:TODO` 标记，1 条来自 `ripgrep:NOTE`。

```
src/blockchain/bsv/mining.rs:162,163            TODO: Get from config
src/blockchain/solana/transaction.rs:53,85,126,250  TODO: Implement actual ...
src/blockchain/wallet/storage.rs:23,32          TODO: Implement wallet serialization ...
src/frontend/proc_macro.rs:707                  TODO: Handle fields in enum variants
src/middle/ctfe/evaluator_complete.rs:4         NOTE: This file is intentionally minimal
```

## 追踪缺口

这批 TODO 多数属于已关闭的 #14「Solana/BSV 模块大量桩实现」。#14 的处置是**改为 fail-closed**（调用即报错，而非静默返回错误数据），并未真正实现；关闭后这部分功能缺口在 issue 表里已无任何记录。

## 补充：该门禁实际不阻塞提交（2026-11-26 复核）

`qlty check` 的真实退出码为 **0**。这 10 条均为 `note` 级别（ripgrep TODO/NOTE 标记），不是 error，因此不阻塞 `git commit` / `jj commit` / push。

本 issue 因此**降级为技术债与追踪缺口**，不再是门禁阻塞项。真正会卡住 CI check job 的是 #20（fmt）与 #21（clippy -D warnings）。

## 影响

- `qlty check` 非 0 退出，按项目约定阻塞一切提交与推送。
- 功能缺口失去追踪，容易误判为已完成。

## 建议修复

1. 先拆分追踪：把「fail-closed 已完成」与「真正实现交易构造/签名/验签/费用计算、钱包序列化、BSV RPC 凭据配置」分别记录为独立 issue。
2. 对确实未实现的路径，统一保留可检索标记（如 `TODO(#NN)` 指向对应 issue），避免门禁与追踪互相矛盾。
3. 对 `evaluator_complete.rs:4` 的 NOTE，评估是否改用不触发 TODO 扫描的措辞，或纳入 qlty 已知问题清单。
