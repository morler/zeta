---
id: 10
title: vsr_cluster_test.py：rid 死代码、裸 except 吞异常、部分安全告警为误报
status: open
priority: low
labels:
    - bug
created: "2026-11-24"
updated: "2026-11-24"
---

## 证据

`qlty check --all` 在 `zetas/capybara/vsr_cluster_test.py` 上共报 20 条（qlty 第 3-101 行区间）。
单独跑 `ruff check zetas/capybara/vsr_cluster_test.py` 复核：20 errors，其中 8 条可自动修复。

## 已核实为真实问题

### 1) 死代码（ruff F841 / radarlint S1481）

`vsr_cluster_test.py:242`
```python
# Start fresh replica for status test
rid = 0
d = f"/tmp/capybara_status"
```
`rid` 赋值后从未使用。同函数后续 `start_replica(0)` 直接传字面量 0，说明这里是想复用变量但漏了。

### 2) 裸 except 吞异常（ruff E722 + bandit B110 + radarlint S5754）

`:340` 与 `:345` 两处：
```python
except:
    p.kill()      # :344-345 附近
...
except:
    pass          # :345-346
```
清理阶段会静默吞掉所有异常，包括 `KeyboardInterrupt` / `SystemExit`。

### 3) 风格项（可 --fix 自动修）

- ruff E401 多 import 同行（:18）
- ruff F541 无占位符 f-string 共 5 处（:149, :151, :153, :243, :290）
- radarlint S3457 同 F541 共 5 条
- radarlint S1192 字面量 "  PASS" 重复 3 次（:165）
- radarlint S5713 冗余 Exception（:61）

## 已核实为误报或降级

- **bandit B101 共 21 条（assert 使用）**：这是测试脚本，assert 就是断言手段，属正常用法。
  注意副作用：`python -O` 会移除 assert，应确保此脚本不以 `-O` 运行。
- **bandit B108（/tmp 硬编码）7 条、B603/B607（子进程完整路径）8 条**：
  脚本设计即 `CAPYBARA = "/tmp/capybara"` 启动本机集群，属测试工具固有形态，
  风险等级应按测试工具而非生产代码评估。

## 修复建议

1. `ruff check --fix zetas/capybara/vsr_cluster_test.py`
2. 手工删除 `rid = 0`（或改为真正使用该变量）
3. 两处裸 `except:` 改为 `except Exception:`，至少加日志
4. 保留 assert，但在文件头或 CI 中注明禁止 `-O`
