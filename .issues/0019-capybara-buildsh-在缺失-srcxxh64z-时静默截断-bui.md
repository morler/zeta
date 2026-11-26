---
id: 19
title: capybara build.sh 在缺失 src/xxh64.z 时静默截断 build.z，产物丢失 12 个模块
status: open
priority: high
labels:
    - bug
relations:
    blocks:
        - 25
created: "2026-11-26"
updated: "2026-11-26"
---

## 现象

`zetas/capybara/build.z`（tracked 文件）当前工作区显示 3301 行 → 312 行（-2998/+9），被截断到 `os.z` 之后、`// === xxh64.z ===` 注释为止。runtime/ledger/client_session/bloom/memtable/sstable/manifest/lsm/vsr/main 共 12 个模块的源码全部丢失。

## 根因

`zetas/capybara/build.sh` 第 15-16 行：

```bash
echo "// === xxh64.z ==="
cat "$SRC/xxh64.z"
```

但 `zetas/capybara/src/xxh64.z` 不存在（HEAD 中不存在，git 也未跟踪；xxh64 内容目前只存在于 build.z 里）。

脚本第 4 行虽然声明了 `set -euo pipefail`，但拼接发生在 `{ ... } > "$DIR/build.z"` 重定向块内：块中命令失败时会先截断并写入部分内容再退出，**不会阻止残缺文件落盘**。

最小复现结论：缺少被 cat 的文件时脚本退出码为 1，但 build.z 仍保留了截断后的内容。

## 影响

- 直接阻塞 capybara 构建（缺 main.z 等 12 个模块）。
- build.z 是 tracked 文件，残缺产物一旦提交会进入仓库历史。

## 注意

工作区当前的这处未提交改动正是这次截断产物，**不要原样提交**。

## 建议修复

1. 补回 `src/xxh64.z`：可从 `git show HEAD:zetas/capybara/build.z` 的 `// === xxh64.z ===` 段提取，并纳入源码目录管理；或
2. 让 build.sh 显式校验每个输入文件是否存在（如 `[[ -f "$f" ]] || exit 1`），并改为先写临时文件、成功后再覆盖 build.z。

修复后重新生成，确认行数回到约 3301。

## 备注

工作区已存在的两处真实修复（`guard()` 编译屏障、io_uring `idx * 4` 下标）已在 `src/os.z` 中同步，重新拼接不会丢失。
