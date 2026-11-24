---
id: 4
title: agent-sync.yml 将 github.ref_name/inputs 直接插入 run 与 github-script，存在命令注入
status: open
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
