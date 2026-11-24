---
id: 14
title: 'Solana/BSV 模块大量桩实现（TODO: Implement actual ...）'
status: closed
priority: medium
labels:
    - feature
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
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

---

## 处理记录（按 issue 建议 1 修复）

完整实现 Solana 交易/钱包需接入 `solana-sdk` 与 `ed25519_dalek`，属独立特性工作。
本次按 issue 中「修复建议 1」执行：**让桩函数明确报错，不再静默产出错误结果**。

### 逐函数核实与修改

| 函数 | 原行为 | 风险 | 现状 |
|---|---|---|---|
| `Solana_Transaction_verify` | 无条件 `Ok(true)` | **最严重**：任何签名都通过校验，伪造/损坏交易被接受 | `Err(not_implemented)` |
| `Solana_Transaction_sign` | 用 `SHA256(data‖private_key)` 冒充 Ed25519 签名，返回 `Ok(())` | 产出永远无法被链接受的"已签名"交易 | `Err(not_implemented)` |
| `Solana_Transaction_new` | 自造布局（原始 blockhash 字符串 + 两个长度字节），非 Solana 线格式 | 产出无法提交的交易 | `Err(not_implemented)` |
| `Solana_Transaction_fee` | 固定返回 `5000` lamports | compute 用量高的交易会低估费用、导致资金不足 | `Err(not_implemented)` |

修改前 verify 的实现：

```rust
log::warn!("Solana transaction verification not fully implemented - placeholder");
Ok(true)
```

这是 fail-open：调用方无从区分"校验通过"与"根本没校验"。

### 与 storage.rs 的一致性

`src/blockchain/wallet/storage.rs` 的两个桩原本就正确返回
`BlockchainError::not_implemented(...)`，无需改动。本次修改让 `transaction.rs`
与之一致，而非另立风格。

### 验证

| 检查 | 结果 |
|---|---|
| `cargo check --workspace` | exit 0 |
| `cargo check --workspace --all-targets` | exit 0 |
| `cargo clippy --workspace --all-targets --features blockchain` | 0 error |
| `rustfmt --check transaction.rs` | 干净 |
| 既有测试是否依赖旧行为 | 无引用（`grep` 确认 tests/ 与 src/ 均无调用方） |

### 保留事项

- TODO 注释**保留**并补充了各函数的具体实现路径，便于后续接入真实实现。
- `Solana_Transaction_serialize` 仍是原有实现，本次未改动（其输出依赖上面被移除的
  占位数据，实际使用前需一并实现）。
- Solana/BSV 其余模块（`account.rs` 的 rent 计算与存在性检查、`address.rs` 的 PDA 派生、
  `bsv/` 下的 TODO）本次未改：它们不产生"看似成功"的安全假象，属正常的功能缺口，
  建议后续按模块拆分为独立 issue。
