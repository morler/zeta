---
id: 2
title: 区块链 feature 引入的传递依赖含 8 条 openssl 等已知漏洞
status: closed
priority: high
labels:
    - bug
    - security
relations:
    related-to:
        - 1
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
---

## 证据

与 #1 同一来源（`qlty check --all` 的 osv-scanner 96 条），但用 `cargo metadata --features blockchain` 做可达性分析后，以下漏洞包**仅在启用 blockchain/solana feature 时进入依赖图**（默认构建不可达）：

| 包 | 版本 | 漏洞数 | 说明 |
|---|---|---|---|
| openssl | 0.10.77 | 8 | RUSTSEC-2026-0463/-0473..-0479，含 aes::unwrap_key 越界写、Deriver 缓冲区溢出 |
| rustls-webpki | 0.101.7 | 4 | 名称约束校验缺陷 |
| rustls | 0.21.12 | 2 | 见 RUSTSEC-2026-0346/-0349 |
| zstd-safe | 7.2.4 | 2 | RUSTSEC-2026-0398/-0403 |
| zstd / zstd-sys / ctutils | 0.13.3 / 2.0.16 / 0.4.2 | 各 1 | |
| curve25519-dalek / ed25519-dalek | 3.2.0 / 1.0.1 | 各 1 | RUSTSEC-2024-0344、RUSTSEC-2022-0093 |
| webpki-roots | 0.24.0 / 0.25.4 / 1.0.7 | 各 1 | RUSTSEC-2026-0381 |

来源链（Cargo.lock 内实际依赖）：
- `solana-secp256r1-program 2.2.4 -> openssl`
- `solana-account-decoder-client-types 2.3.13 -> zstd`
- `solana-secp256k1-program/recover -> libsecp256k1`

## 为何仍要修

`.github/workflows/agent-branches.yml:78,82` 与 `agent-sync.yml:104` 均以 `--all-features` 运行 clippy/build/test，因此这些依赖在 **CI 中实际被编译**。

## 修复建议

1. `cargo update` 后覆盖到 feature 下的间接依赖（openssl 0.10.77 -> 0.10.82 已确认可用）
2. 若无法升级，在 `deny.toml` 中显式记录 ignore 并注明理由，而非默认忽略
3. 长期：评估 solana 依赖是否为 bootstrap 分支所必需

---

## 修复记录（部分完成）

### 已完成

定向升级以下包（`Cargo.toml` 未改动）：

```
cargo update -p openssl -p zstd -p zstd-safe -p ctutils -p cmov
```

- openssl 0.10.77 -> 0.10.82（消除全部 8 条 RUSTSEC-2026-0463/-0473..-0479）
- zstd 0.13.3 -> 0.13.5、zstd-safe 7.2.4 -> 7.4.1、zstd-sys 2.0.16 -> 2.1.3（消除 4 条）
- ctutils 0.4.2 -> 0.4.8（消除 1 条）
- cmov 0.5.3 -> 0.5.4

漏洞总数 25 -> **12**，消除 13 条。

### 验证

| 检查 | 结果 |
|---|---|
| `cargo check --workspace --features blockchain` | exit 0 |
| `cargo clippy --workspace --features blockchain --all-targets` | 0 error |
| `qlty check`（默认模式，变更文件）中 osv-scanner 告警 | 0 条 |
| `qlty check --all` 中 osv-scanner 告警 | 96 -> **41 条** |

### 剩余 12 条：受上游约束，当前无法消除

| 包 | 版本 | 条数 | 上游约束 |
|---|---|---|---|
| rustls-webpki | 0.101.7 | 4 | 由 `rustls 0.21.12` 锁定，后者由 `tokio-tungstenite 0.20.1` 引入 |
| rustls | 0.21.12 | 2 | 同上，`tokio-tungstenite 0.20.1` <- `solana-pubsub-client 2.3.13` |
| webpki-roots | 0.24.0 / 0.25.4 | 2 | 分别由 `tungstenite 0.20.1`、`tokio-tungstenite 0.20.1` 引入 |
| webpki-roots | 1.0.7 | 1 | 仅 feature 路径可达 |
| ed25519-dalek | 1.0.1 | 1 | 由 `solana-keypair 2.2.3`、`solana-signature 2.3.0`、`solana-ed25519-program 2.2.3` 锁定 |
| curve25519-dalek | 3.2.0 | 1 | 由上述 `ed25519-dalek 1.0.1` 传递引入 |
| webpki-root-certs | 1.0.7 | 1 | **默认与 feature 构建均不可达**（见下） |

已确认 `cargo update -p` 对这些包无可用升级：`solana-sdk` 已处于 2.3.1（Cargo.toml 约束 `2.0.0`，2.x 上限），
`tokio-tungstenite 0.20.1` 是 `solana-pubsub-client 2.3.13` 的固定依赖。

`rustls-platform-verifier 0.6.2`（引入 `webpki-root-certs`）由 `reqwest 0.13.2` 锁定，无法单独升级。

### 关于 webpki-root-certs

可达性分析显示它既不在默认构建、也不在 `--features blockchain` 的依赖图中
（`cargo metadata` 逐节点遍历确认），属 osv-scanner 对 lock 文件的全量扫描结果，实际不参与编译。

### 后续建议

剩余 12 条需等待 solana 上游升级 `tokio-tungstenite` / `ed25519-dalek` 依赖，
或在 `deny.toml` 中显式记录 ignore（附理由），避免每次 `cargo audit` 都需人工判断。
