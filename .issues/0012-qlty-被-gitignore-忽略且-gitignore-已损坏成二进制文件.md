---
id: 12
title: .qlty 被 .gitignore 忽略，且 .gitignore 已损坏成二进制文件
status: open
priority: medium
labels:
    - bug
relations:
    related-to:
        - 3
created: "2026-11-24"
updated: "2026-11-24"
---

## 证据

### 1) qlty 配置未纳入版本控制

```
$ git check-ignore -v .qlty/qlty.toml
.gitignore:143:.qlty    .qlty/qlty.toml
```

`.gitignore` 末尾存在一行 `.qlty`。这与仓库 AGENTS.md 的 qlty 规范冲突：
「仓库未初始化 Qlty 时先运行 `qlty init`，并把生成的 `qlty.toml`、`.qlty/` 配置随代码一起提交」。

后果：`qlty check --all` 的插件集合（actionlint / bandit / clippy / osv-scanner /
radarlint-python / ripgrep / ruff / shellcheck / trufflehog / zizmor）没有任何版本控制记录，
其他人 clone 后行为可能不同，也无法 review 规则变更。

### 2) .gitignore 本身已损坏

```
$ git diff .gitignore
Binary files a/.gitignore and b/.gitignore differ

$ grep 'issues' .gitignore
grep: .gitignore: 匹配到二进制文件
```

文件内存在 NUL 字节，具体在 `PERFORMANCE_*.md` 之后：

```
PERFORMANCE_*.md\0zeta/\0 \0 \0 ...
```

即原第 133 行附近被插入了 `\0`，把 ".gitignore" 变成了二进制文件。
这会导致 git 对该文件的行为不可预期，且 `grep`/`rg` 等工具跳过它。

## 修复建议

1. 用文本编辑器重写 `.gitignore`，移除 NUL 字节（保留原意：`PERFORMANCE_*.md` 与 `zeta/` 各占一行）
2. 删除末尾的 `.qlty` 忽略规则
3. `git add .qlty/qlty.toml` 并随代码提交
4. 同时注意 `.issues/` 目前也是未跟踪状态（`?? .issues/`），应一并纳入

## 相关

`.gitignore` 损坏是本轮唯一需要"修工具配置本身"的问题，优先级高于其他风格项。
