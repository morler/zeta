---
id: 2
title: 区块链 feature 引入的传递依赖含 8 条 openssl 等已知漏洞
status: open
priority: high
labels:
    - bug
    - security
relations:
    related-to:
        - 1
created: "2026-11-24"
updated: "2026-11-24"
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
