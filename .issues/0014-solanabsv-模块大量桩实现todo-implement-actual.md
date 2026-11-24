---
id: 14
title: 'Solana/BSV 模块大量桩实现（TODO: Implement actual ...）'
status: open
priority: medium
labels:
    - feature
created: "2026-11-24"
updated: "2026-11-24"
---

## 证据

`qlty check --all` 的 ripgrep TODO 结果中，`src/blockchain/**` 集中出现"实际实现缺失"类待办。
逐条核实注释，确认这些函数确实是桩，而非普通待办：

| 位置 | TODO 内容 | 影响 |
|---|---|---|
| src/blockchain/solana/account.rs:111,208 | Implement actual existence check via RPC / actual Solana rent calculation | 账户存在性、租金计算为假值 |
| src/blockchain/solana/address.rs:150 | Implement actual PDA derivation | PDA 派生错误 |
| src/blockchain/solana/transaction.rs:53,91,138,258 | actual transaction creation / Ed25519 signing / verification / fee calculation | 交易构造与签名未实现 |
| src/blockchain/wallet/storage.rs:23,32 | wallet serialization and encrypted file storage / deserialization | 钱包持久化未实现（涉密钥安全） |
| src/blockchain/bsv/mining.rs, network.rs | 相关 network/mining 待办 | |

全部 80 条 ripgrep TODO 的完整清单可通过 `qlty check --all` 复现（输出中 `ripgrep:TODO` 行）。

## 为何单独登记

这些不是"注释风格"问题：
- `transaction.rs:91` 与 `:138` 是 **Ed25519 签名/验签未实现**——若上层误以为已实现，会导致签出无效或被伪造接受的交易。
- `wallet/storage.rs` 的序列化未实现，涉及私钥落盘。

其余 TODO（如 `src/frontend/macro_expand*.rs`、`src/package/workspace.rs`、`tests/**` 中的待办）属正常开发进度记录，已用 `related-to` 关联但不在本条范围。

## 修复建议

1. 对 `transaction.rs` 的签名/验签、`wallet/storage.rs` 的私钥落盘，在实现前应先返回明确错误（`todo!()` / `Err(Unimplemented)`），避免静默产出错误结果
2. 或在这些入口加 `#[deprecated]` / feature gate，防止被当作可用 API
3. 长线：将链上相关 TODO 按模块拆成独立 issue 跟踪
