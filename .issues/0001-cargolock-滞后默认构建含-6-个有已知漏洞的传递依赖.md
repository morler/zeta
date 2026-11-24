---
id: 1
title: Cargo.lock 滞后：默认构建含 6 个有已知漏洞的传递依赖
status: open
priority: critical
labels:
    - bug
    - security
relations:
    related-to:
        - 2
        - 3
created: "2026-11-24"
updated: "2026-11-24"
---

## 证据

运行 `qlty check --all` 报 371 issues（含 248 security），其中 osv-scanner 96 条集中在 Cargo.lock:0:0。
用 `cargo audit --json` 独立复核，确认 41 条唯一漏洞，并用 `cargo metadata` 做依赖可达性分析。

**默认构建（无 feature）即可达的漏洞包：**

| 包 | 版本 | 漏洞数 | 代表性 RUSTSEC |
|---|---|---|---|
| quinn-proto | 0.11.14 | 7 | RUSTSEC-2026-0185（远程内存耗尽）、-0340（peer ACK 可 panic） |
| rustls | 0.23.38 | 4 | RUSTSEC-2026-0285（TLS1.3 握手跨加密层）、-0348（Brotli 解压 1GiB） |
| rustls-webpki | 0.103.12 | 2 | RUSTSEC-2026-0382（接受不受信中间证书） |
| quinn-udp | 0.5.14 | 1 | RUSTSEC-2026-0538 |
| url | 2.5.8 | 1 | RUSTSEC-2026-0592（url::origin 栈溢出） |
| crossbeam-epoch | 0.9.18 | 1 | RUSTSEC-2026-0204 |

入口链：`reqwest -> hyper-rustls / tokio-rustls / rustls-platform-verifier`。

## 可修性（已实测）

`cargo update --dry-run` 显示均可升级：
- rustls 0.23.38 -> 0.23.46
- quinn-proto 0.11.14 -> 0.11.19，quinn-udp 0.5.14 -> 0.5.16
- rustls-webpki 0.103.12 -> 0.103.17
- crossbeam-epoch 0.9.18 -> 0.9.21

## 修复建议

1. `cargo update`（无需改 Cargo.toml）
2. 重跑 `cargo audit` 与 `qlty check --all` 确认归零
3. 在 CI 增加 `cargo audit` 门禁
