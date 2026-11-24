# Dependency security scanning: how to read the numbers

This repo has two overlapping dependency scanners. They disagree, and the
disagreement is expected. This note records why, so nobody re-derives it.

Tracking issue: #3.

## Use `cargo audit` as the source of truth

```sh
cargo audit            # human readable
cargo audit --json     # machine readable
```

`cargo audit` reports against the resolved dependency graph and separates the
three categories that matter:

| category | meaning |
|---|---|
| `vulnerabilities` | Actually exploitable advisories (RUSTSEC/CVE/GHSA). |
| `warnings.unsound` | Soundness bugs in the crate; may or may not be reachable. |
| `warnings.unmaintained` | No longer maintained; no patch will ever ship. |

## Why the qlty/osv-scanner numbers are higher and lower at the same time

`qlty check` runs osv-scanner against `Cargo.lock`, which produces two biases:

**Over-counting.** One advisory is reported once per identifier it has. An
`openssl` advisory published as CVE + GHSA + RUSTSEC shows up two or three
times. Exact duplicates in the qlty output are expected; dedupe before
quoting any total.

**Under-counting.** The qlty osv-scanner integration only surfaces the
`vulnerable` category. The `unsound` and `unmaintained` advisories that
`cargo audit` prints in its warning summary are **not** in the qlty output at
all.

Net effect: a qlty security count is simultaneously inflated by duplicates and
deflated by missing categories. Do not quote it as a vulnerability total.

## Reachability matters more than the raw count

`Cargo.lock` is a superset of what actually gets compiled. Several advisories
only enter the graph under `--features blockchain` / `--features solana`.

Check reachability per build configuration:

```sh
cargo metadata --format-version 1 --filter-platform x86_64-unknown-linux-gnu \
  | jq -r '.resolve.nodes[].id' | sort -u
```

Compare that set against the packages named in `cargo audit --json`. A package
present in `Cargo.lock` but absent from the resolved graph for a given feature
set is not compiled and carries no runtime risk for that configuration.

Note that CI does build `--all-features` in some workflows, so feature-gated
advisories still need attention — they are just a separate bucket from the
default build.

## Current status

Known remaining advisories are tracked in #2, with the upstream constraints
that block each fix recorded there.
