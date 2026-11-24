---
id: 5
title: agent-sync.yml 的 merge-to-main job 引用了未定义的 workflow_dispatch 输入，分支合并流程必然失败
status: closed
priority: high
labels:
    - bug
relations:
    related-to:
        - 4
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
---

## 证据

`qlty check --all` 报 `actionlint:expression`（1 条）：

```
.github/workflows/agent-sync.yml:169
  169:169  medium  property "branch" is not defined in object type {}  actionlint:expression
```

核实源码后确认这是真实缺陷，而非误报：

```yaml
# agent-sync.yml 顶部
on:
  push:
    branches:
      - 'feat/**'
  workflow_dispatch:        # <-- 没有定义任何 inputs
```

```yaml
# agent-sync.yml:160-175
  merge-to-main:
    if: github.event_name == 'workflow_dispatch'
    ...
      - name: Merge feature branch to v0.3.9
        run: |
          BRANCH="${{ github.event.inputs.branch }}"
          if [ -z "$BRANCH" ]; then
            echo "Error: Branch not specified"
            exit 1
          fi
```

由于 `workflow_dispatch` 未声明 `inputs.branch`，`github.event.inputs.branch` 恒为空字符串，
脚本必然走到 `exit 1`。也就是说**手动触发的「merge 到 v0.3.9」流程无法工作**。

## 修复建议

1. 在 `on.workflow_dispatch` 下补上输入定义：
   ```yaml
   workflow_dispatch:
     inputs:
       branch:
         description: 'Branch to merge into v0.3.9'
         required: true
         type: string
   ```
2. 同时按 #4 的方式改为 `env: BRANCH: ${{ inputs.branch }}`，避免注入

---

## 修复记录（已完成）

### 改动

在 `on.workflow_dispatch` 下补充输入定义：

```yaml
  workflow_dispatch:  # Manual trigger for testing
    inputs:
      branch:
        description: 'Feature branch to merge into v0.3.9'
        required: true
        type: string
```

并将该 job 中的取值改为 `env:` + 环境变量（同时解决注入）：

```yaml
      - name: Merge feature branch to v0.3.9
        env:
          BRANCH: ${{ inputs.branch }}
        run: |
          if [ -z "$BRANCH" ]; then ...
```

`git merge --no-ff "origin/$BRANCH"` 增加引号。

### 验证

- 修改前 `actionlint:expression` 报 `property "branch" is not defined in object type {}`
- 修改后 **该告警消失**，`qlty check --all --filter actionlint` 只剩 15 条 runner-label 误报（见 #13）
- `run:` 块 `bash -n` 语法检查通过

### 行为变化

该流程此前必然在 `exit 1` 处失败；现在手动触发时可由用户提供分支名并真正执行合并。
