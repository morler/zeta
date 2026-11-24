---
id: 3
title: qlty 的 osv-scanner 结果重复计数且漏报 unsound/unmaintained
status: open
priority: medium
labels:
    - bug
relations:
    related-to:
        - 1
        - 12
created: "2026-11-24"
updated: "2026-11-24"
---

## 问题

`qlty check --all` 中 osv-scanner 报 96 条，但去重后只有 **72 条**唯一问题：

- 24 条被同一漏洞的多个 ID 重复上报（CVE + GHSA + RUSTSEC 各算一次）。
  例：`openssl@0.10.77` 的 RUSTSEC-2026-0473 同时以 `CVE-2026-41676`、`GHSA-pqf5-4pqq-29f5` 出现，共 2 次。
  验证：`cargo audit --json` 对同一 lock 只给出 41 条唯一漏洞。

反向漏报更值得注意：

- `cargo audit` 报告 **17 条 unsound**（anyhow、tokio、rustix、rand、memmap2、event-listener、atty、zstd-safe 等）
  和 **9 条 unmaintained**（atty、bincode、chrono、derivative、humantime、libsecp256k1、number_prefix、paste、rand_os），
  这些在 qlty 输出中**完全不可见**，只保留了 vulnerable 类别。

影响：`248 security issues` 这个数字既被夸大（重复），又低估了真实风险面（漏掉 unsound/unmaintained）。

## 修复建议

- 判定风险时以 `cargo audit --json` 为准，不要直接引用 qlty 的 96/248 计数
- 若要让 qlty 也覆盖 unsound/unmaintained，检查 osv-scanner 插件参数或改用 `cargo audit` 作为独立 CI 门禁
