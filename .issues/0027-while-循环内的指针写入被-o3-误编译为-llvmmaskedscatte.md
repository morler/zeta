---
id: 27
title: while 循环内的指针写入被 -O3 误编译为 llvm.masked.scatter，JIT 报未解析符号
status: open
priority: high
labels:
    - bug
created: "2026-11-26"
updated: "2026-11-26"
---

## 现象

在 while 循环体内通过指针写入（`*p = v`）时，JIT 会引用一个不存在的符号：

```
Error: "cannot execute: unresolved external symbol(s): llvm.masked.scatter.v8i64.v8p0"
```

该 intrinsic 并未在模块中声明，是 -O3 向量化后残留的引用。

## 最小复现

```zeta
fn f(c: i64) -> i64 {
    let mut i: i64 = 0;
    while i < c {
        let p = i as *mut i64;
        *p = 0;
        i = i + 1;
    }
    return 0;
}
fn main() -> i64 { f(3); 0 }
```

- 等价的递归写法同样触发。
- 把循环体换成 `println_i64(i)`（无指针写）则不触发。
- 循环体内先调用一个做指针写的函数，效果相同。

## 影响

`zeta_src/runtime/actor/map.z` 原先尝试用循环把槽位清零时踩中此缺陷。绕开方式见该文件注释。

## 排查方向

`optimize_module` 用 `default<O3>` 跑新 PM。生成 masked.scatter 说明标量 store 被提升成向量操作，但相应的 intrinsic 声明未进入模块。需要确认：

1. codegen 对 `*p = v` 生成的 IR 是否缺少对齐/nonnull 信息，导致向量化误判；
2. 用 `-O2` 或关闭 loop vectorizer 对照，确认是否只在向量化开启时出现；
3. `optimize_module` 当前只在 debug 构建打印 pass 错误（`#[cfg(debug_assertions)]`），release 下会静默忽略失败。
