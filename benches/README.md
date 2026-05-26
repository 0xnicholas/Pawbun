# Pawbun Performance Benchmark Report

> Environment: macOS / Apple M3 / Rust 1.75+
> Command: `cargo bench --workspace`
> Date: 2026-05-26

## pawbun-toolkit

| Benchmark | Time | Throughput | Target Met |
|-----------|------|------------|------------|
| registry_lookup/100 | ~33ns | — | ✅ |
| registry_lookup/1000 | ~38ns | — | ✅ |
| tool_execute_overhead | ~277ns | — | ✅ |
| tool_register | ~840ns | — | ✅ |
| tool_descriptions/100 | ~67µs | — | ✅ |
| schema_build/10_params | ~2µs | — | ✅ |

## pawbun-files

| Benchmark | Time | Throughput | Target Met |
|-----------|------|------------|------------|
| load_local/text | ~152µs | — | [ ] |
| provider_format/openai/text | ~542ns | — | ✅ |

## pawbun-mcp-server

| Benchmark | Time | Throughput | Target Met |
|-----------|------|------------|------------|
| handler_initialize | ~4.7µs | — | ✅ |
| handler_tools_list/1 | ~5.0µs | — | ✅ |
| handler_tools_call | ~2.9µs | — | ✅ |

> **注意**：`load_local/text` 的目标未标记，因为 Spec 中目标为 `< 2× std::fs::read`，需要与纯 std 操作对比后确定。
