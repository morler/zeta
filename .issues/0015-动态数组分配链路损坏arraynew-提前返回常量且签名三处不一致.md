---
id: 15
title: 动态数组分配链路损坏：array_new 提前返回常量且签名三处不一致
status: closed
priority: high
labels:
    - bug
    - security
relations:
    depends-on:
        - 17
    related-to:
        - 9
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
---

## 证据

在修复 #9（`array_push` 栈数组分支）时追溯调用链发现的问题，范围大于 #9。

### 1) `array_new` 提前返回，后续正确实现是死代码

`src/runtime/array.rs:68-71`：

```rust
#[allow(unreachable_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn array_new(capacity: usize) -> i64 {
    // Just return a constant, no function calls
    return 0x0BADF00D;                 // <-- 函数在此返回

    // 以下约 50 行是完整、正确的实现（header + 数据 + canary + 内存填充）
    // 但永远不可达，靠 #[allow(unreachable_code)] 压掉告警
    ...
}
```

吊诡之处：**死代码本身是正确实现**，而生效的 `return` 是一个魔数。
`#[allow(unreachable_code)]` 使这个问题不产生任何编译告警。

### 2) 三处签名互不一致（ABI 不匹配）

| 位置 | 签名 |
|---|---|
| `src/runtime/array.rs:68`（Rust 实现） | `array_new(capacity: usize) -> i64` —— **1 参数** |
| `src/backend/codegen/codegen.rs:401`（LLVM 声明） | `i64 @array_new(i64, i64)` —— **2 参数** |
| `src/middle/mir/gen.rs:2536,2616,2753`（调用） | `args: vec![capacity_id]` —— 实际传 **1 个** |
| `zeta_src/runtime/array.z:98`（自举实现） | `fn array_new() -> i64` —— **0 参数** |
| `zeta_src/backend/codegen/codegen.z:41`（自举声明） | `declare i64 @array_new()` —— **0 参数** |

codegen 声明 2 参、MIR 只传 1 参、Rust 实现收 1 参、自举侧两侧都是 0 参——
四者两两不一致。按 LLVM 语义，`call` 指令实参个数必须与被调函数的 `FunctionType` 一致；
不一致时 IR 校验（`llvm-as` / `verifyModule`）会直接报错。若此处未在生成阶段被校验出来，
运行时 ABI 也是错位的（被调方按 1 参约定读取寄存器/栈，调用方按 2 参声明准备）。

> 待确认：`cargo test` 在本机因缺 LLVM 21 无法运行（见 #1），
> 因此这三种签名组合中哪一种真正走通到执行阶段，尚未实测。

### 3) 同一函数被重复声明且返回类型冲突

`src/backend/codegen/codegen.rs` 的 `LLVMCodegen::new()` 内，`array_push` 声明了两次：

```rust
// 436 行
module.add_function("array_push",
    i64_type.fn_type(&[i64_type.into(), i64_type.into()], false), ...);   // 返回 i64

// 668 行
module.add_function("array_push",
    void_type.fn_type(&[i64_type.into(), i64_type.into()], false), ...);  // 返回 void
```

而 `src/runtime/array.rs:321` 的 Rust 实现返回 `()`。
第二处附近的注释 `// Note: array_new already declared above (line 312)` 引用的行号也是错的
（实际在 401）。

### 4) 后果：动态数组整体不可用，且失败是静默的

若 `array_new` 注入生效，JIT 会返回 `0x0BADF00D`（`src/backend/codegen/jit.rs:243`
把 Rust 函数映射进去）。该值随后作为"数据指针"传给 `array_get` / `array_set` / `array_push`：

- `get_header()` 会去 `0x0BADF00D - 32` 处读 `magic`，属**野指针读取**
- 若该地址恰好可读且 `magic` 不匹配，会被判定为"栈数组"，进入无边界检查分支
- `array_push` 在栈数组分支现已改为报错（#9），但 `array_set` 仍会写 `data_ptr.offset(index)`，
  即写入**任意地址**

也就是说：动态数组的失败模式不是"报错"或"数据不对"，而是**向不可预测的地址读写**。

### 5) 现有测试无法发现

`tests/array-parsing/test_array_init.z` 等用例使用 `[dynamic]u8{}` + `arr.push()`，
经 MIR 生成 `array_new` → `array_push`。但：

- 本机 `cargo test` 因缺 LLVM 21 无法运行（见 #1 的验证记录）
- 即使能跑，这些是 `.z` 源文件，是否被 CI 实际执行未确认

## 影响

| 项 | 状态 |
|---|---|
| 动态数组字面量 `[dynamic]T{...}` | 依赖 `array_new` |
| 大数组（超过栈阈值）的堆分配路径 | 依赖 `array_new`（`mir/gen.rs:2531,2610`） |
| `arr.push()` | 依赖 `array_new` 返回有效指针 |
| 失败模式 | 野指针读写，非受控错误 |

## 修复建议

1. **先删掉 `return 0x0BADF00D;` 与 `#[allow(unreachable_code)]`**，让死代码真正生效。
   注意：死代码实现内部仍有 `println!` 调试输出，需一并清理。
2. **统一 `array_new` 签名**，四处对齐。建议保留 `capacity` 参数（与 MIR 调用一致），
   修改 codegen 声明为 1 参，自举侧同步补上参数。
3. 删除 `codegen.rs` 中 `array_push` 的重复声明，保留与 Rust 实现一致的 `void` 版本。
4. 修正 `// Note: array_new already declared above (line 312)` 的行号。
5. 让 `array_set` 的栈数组分支也做防御（当前无边界检查即写），或至少校验指针有效性，
   避免 #9 修好 push 之后 set 仍是任意地址写。
6. 在 `tests/` 增加能被 CI 实际执行的动态数组用例，覆盖 alloc → push → get → free 全链路。

## 相关

- #9 修复了 `array_push` 的栈数组分支，但未触及分配链路本身

## 修复结论（2026-11-24）

实际根因比 issue 中列的更宽一层，四件事一起改才跑通。

### 实际根因

1. **`array_new` 返回魔数**：`return 0x0BADF00D;` 让所有动态数组指向同一伪造地址。
2. **LLVM 声明与调用不一致**：codegen 声明 `i64 @array_new(i64, i64)`，而 MIR 只传 1 个参数。
   codegen 的 `get_or_declare_function` 按实参个数找不到匹配声明，于是新建了
   `@array_new.10` 这个未定义符号；JIT 跳进去直接崩溃（已用 `./zetac /tmp/dyn1.z`
   复现 SIGSEGV）。这才是"动态数组完全不可用"的即时原因。
3. **`array_push` 返回类型三处不一致**：Rust 实现返回 `()`、第一处声明返回 `i64`、
   第二处声明返回 `void`，而 MIR 按有返回值调用。
4. **容量为 0 时永远无法 push**：`[dynamic]T{}` 分配容量 0，`array_push` 只报错不扩容，
   于是最基础的"空数组 + push"（仓库里 `tests/array-parsing/*.z` 的主流写法）必然失败。

### 落地的修改

- `src/runtime/array.rs`
  - 删除 `return 0x0BADF00D;` 与 `#[allow(unreachable_code)]`，清理 3 处 `[ARRAY_NEW]` 调试输出。
  - `array_new` 加溢出检查（`checked_mul` / `checked_add`），失败返回 0。
  - 新增 `block_layout` + `grow_array`：用 `realloc` 扩容，成功返回新的数据指针。
  - `array_push` 改为 `-> i64`，按 `new = old * 1.5 + 1` 几何扩容并返回新数据指针；
    扩容失败/栈数组/头损坏一律返回原指针且不写入。
  - `array_set` 删除重复的入参检查与日志块；栈数组分支拒绝负索引。
  - 新增 5 个单元测试：生命周期、分配地址互异、越界拒绝、几何扩容后旧数据完整、free(NULL)。
- `src/backend/codegen/codegen.rs`
  - `array_new` 声明改为 1 参，与运行时一致（修复 `@array_new.10` 幽灵符号）。
  - 删除第二组重复的 `array_push` / `array_len` / `array_get` 声明及错误行号注释。
- `src/middle/mir/gen.rs`
  - 动态数组字面量的 `array_push` 结果写回数组槽位（扩容可能搬移内存块）。
  - 方法调用形式 `arr.push(x)` 改为返回 `i64` 并写回接收者变量，不再当成 void。
- `src/middle/resolver/resolver.rs`：`array_push` 内建签名改为返回 `i64`。
- 自举侧同步：`zeta_src/runtime/array.z`（`array_new(capacity)`、`reserve`、`array_push -> i64`）、
  `zeta_src/backend/codegen/codegen.z`、`zeta_src/middle/mir/gen.z`。
- 新增 `tests/array-parsing/dynamic_array_runtime.rs`（已在 `Cargo.toml` 注册），
  4 个用例走完整 parse → MIR → LLVM → JIT 管线，CI 的 `cargo test` 会实际执行。

### 验证

| 检查 | 结果 |
|---|---|
| `cargo test`（LLVM 21.1.8） | 32 个目标全部 ok，0 failed |
| `cargo test --lib` | 121 passed |
| 自举 `zeta_src/*.z` 全部经 `zetac` 编译 | 0 失败 |
| `tests/array-parsing/*.z` 数组用例 | push/len/access/complete/set/ops/basic 结果全部符合注释预期 |

### 遗留（不在本 issue 范围）

- `test_array_index_while.z` 仍返回 0，但根因是 while 条件中调用带返回值函数会读未初始化槽位，
  已单独立项 **#16**；该用例在本 issue 修复前是直接段错误，现在至少不会崩。
- `qlty check` 仍报 49 条既有问题（39 条在 `zetas/capybara/vsr_cluster_test.py`），
  本次改动涉及的文件零新增问题。
