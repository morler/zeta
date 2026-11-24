---
id: 11
title: TODO/FIXME 扫描在 src/workflows/quality.rs 上自我命中，产生 3 条假问题
status: in-progress
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

---

## 修复记录（已完成）

分两部分：修复检查器本身的缺陷，以及消除 qlty 对该文件的自指误报。

### 1) 检查器的子串匹配缺陷（真实 bug）

原文用 `line.to_lowercase().contains("todo")` 做子串匹配，会把 `mastodon`、`todos`、
`todos_count` 等无关词判为待办；且每行调用两次 `to_lowercase()`，每次分配一个 `String`。

改为词边界匹配，并复用一次小写化结果：

```rust
let lower = line.to_lowercase();
if contains_marker(&lower, "todo") || contains_marker(&lower, "fixme") {
```

```rust
fn contains_marker(haystack: &str, marker: &str) -> bool {
    haystack.match_indices(marker).any(|(idx, _)| {
        let before = haystack[..idx].chars().next_back();
        let after = haystack[idx + marker.len()..].chars().next();
        let is_word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        !is_word(before) && !is_word(after)
    })
}
```

新增单元测试覆盖两类情形：`// todo: fix this` 应命中；`mastodon` / `todos` / `todos_count`
不应命中。

### 2) qlty 自指误报

检查器的源码必然包含它要搜索的标记字符串，因此 ripgrep 会把它当成待办。
在 `.qlty/qlty.toml` 的 `exclude_patterns` 中排除该文件：

```toml
  # The TODO/FIXME checker's own source necessarily contains the marker
  # strings it searches for, which makes ripgrep report it against itself.
  # See issue #11.
  "src/workflows/quality.rs",
```

排除范围精确到单文件，而非整个 `src/workflows/`，因此 `src/workflows/` 下其他文件
仍照常检查（已验证 `src/middle/resolver/typecheck.rs` 的 TODO 仍被报出）。

### 验证

| 检查 | 结果 |
|---|---|
| `qlty check src/workflows/quality.rs` | **✔ No issues**（原 3 条自指） |
| `qlty check src/` 其余文件 | 仍报出，未被误排除 |
| `cargo check --workspace --all-targets` | exit 0 |
| `rustfmt --check src/workflows/quality.rs` | 干净 |
| 词边界逻辑 | 独立脚本验证 7 组用例全部符合预期 |
| `qlty check --all` 中 ripgrep 告警 | 82 -> 78 |

### 未能验证

新增的 2 个单元测试无法在本机运行：测试二进制需 `libLLVM.so.23.1`，本机缺 LLVM 21。
逻辑本身已用等价脚本逐例验证，但**测试未实际执行**，需在有 LLVM 21 的环境跑
`cargo test --lib workflows::quality`。
