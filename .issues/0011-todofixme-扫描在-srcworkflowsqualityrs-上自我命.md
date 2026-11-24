---
id: 11
title: TODO/FIXME 扫描在 src/workflows/quality.rs 上自我命中，产生 3 条假问题
status: open
priority: low
labels:
    - bug
created: "2026-11-24"
updated: "2026-11-24"
---

## 证据

`qlty check --all` 中 ripgrep 规则共 81 条（80 TODO + 1 FIXME + 1 HACK）。
其中 `src/workflows/quality.rs` 的 3 条是**自指误报**——该文件本身就是"TODO 注释检查器"的源码：

```
src/workflows/quality.rs:165:165  note  // Check for TODO comments                              ripgrep:TODO
src/workflows/quality.rs:175:175  note  &format!("TODO/FIXME comment found: {}", todo_text),    ripgrep:TODO
src/workflows/quality.rs:175:175  note  &format!("TODO/FIXME comment found: {}", todo_text),    ripgrep:FIXME
src/workflows/quality.rs:178:178  note  .with_suggestion("Address the TODO comment or remove it")  ripgrep:TODO
```

`quality.rs:165-179` 核实内容为检查器实现本身：

```rust
// Check for TODO comments
if *self.enabled_checks.get("todo_comments").unwrap_or(&true) {
    for (line_num, line) in content.lines().enumerate() {
        if line.to_lowercase().contains("todo") || line.to_lowercase().contains("fixme") {
            ...
            &format!("TODO/FIXME comment found: {}", todo_text),
```

匹配到的都是**字符串字面量与注释里的规则描述**，不是待办事项。

## 顺带发现的真实缺陷（与误报无关，但在这段代码里）

`quality.rs` 用 `line.to_lowercase().contains("todo")` 做子串匹配，会把
`mastodon`、`todos`、`TODO_LIST` 等无关词全部误判为待办，且忽略大小写判断后又重复调用两次 `to_lowercase()`（每行分配两次 String）。

## 修复建议

1. 在 qlty 配置中排除 `src/workflows/quality.rs`，或改用 `ripgrep` 的规则级 filter
2. 检查器本身改为词边界匹配（如 `\b(TODO|FIXME)\b`），并复用一次小写化结果
