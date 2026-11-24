---
id: 3
title: qlty 的 osv-scanner 结果重复计数且漏报 unsound/unmaintained
status: in-progress
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

---

## 处理记录（已完成）

本条不是可"修"的代码缺陷，而是**工具口径问题**——结论已固化为仓库文档，避免后续反复推断。

### 产出

新增 `docs/DEPENDENCY-SECURITY-SCANNING.md`，记录：

1. **以 `cargo audit` 为准**，它按 `vulnerabilities` / `warnings.unsound` /
   `warnings.unmaintained` 三类分开报告。
2. **qlty/osv-scanner 的双向偏差**：
   - 高估：一条公告按 CVE + GHSA + RUSTSEC 分别计数，输出中存在重复
   - 低估：只呈现 `vulnerable` 类，完全不含 `unsound` 与 `unmaintained`
   - 净效果：既被重复放大、又漏掉部分类别，**不能当作漏洞总数引用**
3. **可达性比总数更重要**：给出用 `cargo metadata --filter-platform` 遍历依赖图、
   与 `cargo audit --json` 的包集合求交的方法；`Cargo.lock` 是超集，
   仅在特定 feature 下进入图的包对该配置不产生运行时风险。
4. 提示 CI 中有 `--all-features` 构建，feature 门控的公告仍需单独对待。

### 修复后的实测数据（与文档一致）

| 口径 | 数值 |
|---|---|
| `cargo audit` 漏洞 | 12（原 41） |
| `qlty check --all` 中 osv-scanner 行数 | 41（含重复） |
| 其中唯一包数 | 26 |
| 默认构建可达漏洞 | 0 |

### 建议

未改动 qlty 配置中的 osv-scanner 插件（如关闭它会一并丢掉真实漏洞检测）。
若希望 CI 强制失败，应加 `cargo audit` 独立门禁，而非依赖 qlty 的计数。
