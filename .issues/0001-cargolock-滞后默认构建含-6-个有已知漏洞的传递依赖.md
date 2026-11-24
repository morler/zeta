---
id: 1
title: Cargo.lock 滞后：默认构建含 6 个有已知漏洞的传递依赖
status: closed
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
closed: "2026-11-24"
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

---

## 修复记录（已完成）

### 改动

采用**定向升级**而非全量 `cargo update`，只动受影响的包，避免一次改动 228 个依赖：

```
cargo update -p rustls@0.23.38 -p quinn-proto -p quinn-udp \
             -p rustls-webpki@0.103.12 -p url@2.5.8 -p crossbeam-epoch
```

升级结果：
- rustls 0.23.38 -> 0.23.46
- quinn-proto 0.11.14 -> 0.11.19
- quinn-udp 0.5.14 -> 0.5.16
- rustls-webpki 0.103.12 -> 0.103.17
- url 2.5.8 -> 2.5.9
- crossbeam-epoch 0.9.18 -> 0.9.21

`Cargo.lock` 变更 175 增 / 130 删，`Cargo.toml` 未改动。

### 验证结果

| 检查 | 结果 |
|---|---|
| `cargo audit` 漏洞总数 | 41 -> **25**（消除 16 条） |
| 默认构建可达的漏洞包 | 6 个 -> **0 个** ✅ |
| `cargo check --workspace --all-targets` | exit 0 |
| `cargo clippy --workspace --all-targets` | exit 0 |
| `qlty check --all` 中 osv-scanner 告警 | 96 -> **41 条**（剩余为 unsound/unmaintained，见 #3） |
| 默认构建可达的漏洞 | **0** ✅ |
| `cargo fmt --check`（monomorphize.rs） | 干净 |

默认构建可达性用 `cargo metadata --filter-platform x86_64-unknown-linux-gnu` 逐节点遍历依赖图确认。

### 附带修复：既有编译错误

升级后 `cargo check --all-targets` 暴露 2 处 `monomorphize.rs` 测试中 `Mir` 初始化缺少 `is_extern` 字段（`cf8ef86` 给 `Mir` 加字段时漏改测试）。已确认该错误**在升级前就存在**（用原始 lock 复现），补 `is_extern: false` 修复。

### 未能验证的部分

`cargo test` 全部无法运行：本机未安装 LLVM 21，测试二进制加载 `libLLVM.so.23.1` 失败（`llvm-sys` 需要 `LLVM_SYS_211_PREFIX=/usr/lib/llvm-21`，但本机无此路径）。尝试用 rustlib 自带的 `libLLVM.so.23.1-rust-*` 替代，因缺少 `LLVMInitializeXCoreTargetMC` 符号失败。这是**既有环境限制**，与依赖升级无关，需在装有 LLVM 21 的 CI 环境复核。

### 剩余

25 条漏洞全部仅在 `--features blockchain/solana` 下可达，归 #2 处理。
