---
id: 4
title: agent-sync.yml 将 github.ref_name/inputs 直接插入 run 与 github-script，存在命令注入
status: closed
priority: high
labels:
    - bug
    - security
relations:
    related-to:
        - 6
        - 5
created: "2026-11-24"
updated: "2026-11-24"
closed: "2026-11-24"
---

## 证据

`qlty check --all` 中 zizmor/template-injection 共 14 条 high。逐条核实后，以下位置确实把不可信输入直接拼进可执行上下文：

### 1) 直接拼进 shell（run: |）

- `.github/workflows/agent-sync.yml:28` — `BRANCH="${{ github.ref_name }}"`，随后参与分支判断与 `git` 操作
- `.github/workflows/agent-sync.yml:170` — `BRANCH="${{ github.event.inputs.branch }}"`
- `.github/workflows/agent-branches.yml:24` — `BRANCH_NAME="${{ github.ref_name }}"`
- `.github/workflows/agent-branches.yml:100,103,106,109` — `[[ "${{ github.ref_name }}" == *"const"* ]]` 等字符串比较

### 2) 拼进 JS 字符串（actions/github-script）

- `.github/workflows/agent-sync.yml:141-143,146,153-154,157`
  ```yaml
  const agent  = '${{ steps.agent-info.outputs.agent }}';
  const branch = '${{ steps.agent-info.outputs.feature_branch }}';
  ```
  单引号包裹并不能阻止注入：分支名中的 `'` 即可闭合字符串并执行任意 JS。
  该 job 使用 `secrets.GITHUB_TOKEN`（见同文件 `token: ${{ secrets.GITHUB_TOKEN }}`），影响面为其权限范围。

## 风险

`feat/**` 分支名可控。任何能推送 `feat/**` 的人可借此在上述 job 执行任意命令 / 任意 GitHub API 调用。

## 修复建议

1. 用 `env:` 传值，脚本内引用环境变量：
   ```yaml
   env:
     BRANCH: ${{ github.ref_name }}
   run: |
     if [[ "$BRANCH" == *"const"* ]]; then ...
   ```
2. github-script 改为 `process.env.AGENT` / `process.env.BRANCH`，不使用 `${{ }}` 拼接

---

## 修复记录（已完成）

### 改动

将所有 shell / JS 上下文中的 `${{ }}` 插值改为通过 `env:` 传值，脚本内引用环境变量。

**agent-sync.yml**
- `Extract agent info`：`BRANCH="${{ github.ref_name }}"` -> `env: BRANCH`
- `Check if ready for merge`、`Sync with v0.3.9`、`Resolve conflicts`、`Create sync report`：
  `feature_branch` / `agent` / `merge_status` 全部改由 `env:` 传入
- `git checkout "$FEATURE_BRANCH"` 等命令增加引号，防止分词
- `Notify agent on conflict`（github-script）：
  `const agent = '${{ ... }}'` -> `const agent = process.env.AGENT`
  （原写法可用分支名里的单引号闭合 JS 字符串）
- 同时升 `actions/github-script@v6` -> `v7`

**agent-branches.yml**
- `Extract agent from branch name`：`github.ref_name` 改 `env: BRANCH_NAME`
- `Check commit tagging`：`steps.extract-agent.outputs.agent` 改 `env: AGENT`，
  正则中的插值一并替换为 `$AGENT`
- `Generate agent report`：5 处 `github.ref_name` 改 `env: REF_NAME`
- `Create summary comment`：`job.status` 及 5 个 step 输出改由 `env:` 传入

**agent-dashboard.yml**
- `Generate dashboard`：`steps.workflow-status.outputs.branch_status`、
  `steps.get-branches.outputs.branches`（共 6 处）改由 `env:` 传入

**benchmarks.yml**
- 3 处 `needs.setup.outputs.llvm_prefix` 改由 `env: LLVM_PREFIX` 传入

### 验证

| 检查 | 结果 |
|---|---|
| `qlty check --all` 中 zizmor template-injection | 14 -> **0** ✅ |
| `qlty check --all` 中 actionlint 非 runner-label 告警 | **0**（原 5 条含 1 条 expression） |
| 各工作流 `run:` 块 `bash -n` 语法检查 | 26 块，0 错误 |
| YAML 可被 actionlint 完整解析 | 通过（无解析错误） |

### 说明

`name:` 字段与 `env:` 值中的 `${{ github.ref_name }}` 保持不变，这些不是 shell 求值上下文，无注入风险。
