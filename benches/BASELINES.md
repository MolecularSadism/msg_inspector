# Benchmark baselines

Criterion baseline `base`, captured 2026-08-26.

| | |
|---|---|
| Commit | `9c3b24818cc9` |
| Branch | `claude/card-list-and-bitmask-field` |
| Toolchain | rustc 1.98.0 (88d9e12ae 2026-08-18) |
| Host | Linux x86_64 container (shared/virtualised) |

## Results

Mean with 95% confidence interval.

| Benchmark | Mean | 95% CI |
|---|---:|---|
| `bitmask_field_full_draw` | 41.25 µs | 40.91 µs – 41.62 µs |
| `named_bits_32_named` | 162 ns | 158.7 ns – 165.7 ns |

## Reproducing

```sh
cargo bench -- --save-baseline base   # capture
cargo bench -- --baseline base        # compare against it
```

These were taken in a shared virtualised container, so absolute figures carry
more run-to-run noise than a dedicated machine. Comparisons made with
`--baseline base` on the same host are meaningful; comparing these absolute
numbers against a different machine is not.
