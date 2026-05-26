# Pawbun 性能、API 审计与文档 Specification

> Version: 0.3.0-draft
> Status: Design
> Date: 2026-05-26
> Scope: 全 workspace（pawbun-toolkit + pawbun-files + pawbun-mcp-core + pawbun-mcp-server）

---

## 1. 目标与范围

0.2.0 完成了生态集成：适配器示例、ToolError 链式错误、MCP Server 配置化、SSE 稳定性。但 Pawbun 在成为社区可信赖的依赖之前，还需要三个方面的成熟：

- **性能基线缺失**：核心操作（工具注册、查找、执行）的开销未知，无法向用户承诺性能指标。
- **公共 API 边界模糊**：部分模块过度暴露内部实现（如 `pawbun-toolkit::json_utils`、`pawbun-mcp-server::tool_bridge` 中的辅助函数），不利于 semver 合规。
- **文档与示例不足**：新用户难以快速上手；docs.rs 上存在未文档化的 `pub` 项。

### 1.1 核心目标

- **建立性能基线**：通过 Criterion 基准测试量化核心操作的开销，为后续优化提供参照。
- **精简公共 API**：审计所有 `pub`/`pub(crate)` 项，移除不必要的暴露，为 1.0 的 API 冻结做准备。
- **文档与示例完善**：每个 crate 至少 2 个可运行示例；所有 `pub` API 有完整 doc comment + `# Example`。
- **兼容性矩阵验证**：确保不同 feature 组合（最小依赖集到 `full`）全部编译通过。
- **依赖精简**：评估重型依赖（`image`、`reqwest` 等）的 feature-gate 粒度，降低最小依赖负担。

### 1.2 非目标

- **不引入新功能**：0.3.0 是纯"打磨"版本，不添加新工具、新协议方法、新 Provider 格式。
- **不做性能优化**：先建立基线、测量现状，优化留到 0.3.x 或 0.4.0。
- **不改 trait 定义**：`Tool`、`AsyncTool`、`ToolKit` 等核心 trait 的签名保持 0.2.0 不变。
- **不改 MCP 协议实现**：`RequestHandler` 的方法路由逻辑不变，仅做配置项审计。
- **不发布 1.0**：0.3.0 是为 1.0 做准备的中间版本，允许仍有 breaking changes。

---

## 2. 架构概述

0.3.0 的改动是横向的——覆盖所有 crate，但深度浅：

```
┌──────────────────────────────────────────────────────────────┐
│  质量层（本 Spec 新增）                                        │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  性能基准（benches/）                                   │ │
│  │  - pawbun-toolkit: 注册/查找/执行/schema 构建           │ │
│  │  - pawbun-files: 加载/格式化吞吐                        │ │
│  │  - pawbun-mcp-server: handler 响应时间                  │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  API 审计                                               │ │
│  │  - 过度暴露 → pub(crate) / #[doc(hidden)]               │ │
│  │  - 缺失文档 → 补 doc comment + # Example                │ │
│  │  - 命名不一致 → 统一风格（最后一次机会）                │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  文档与示例（examples/）                                │ │
│  │  - 每个 crate 2 个可运行示例                            │ │
│  │  - cookbook 风格指南                                    │ │
│  │  - benches/README.md 基准报告                           │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  兼容性矩阵                                             │ │
│  │  - 脚本化 feature 组合测试                              │ │
│  │  - 最小依赖集验证                                       │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  依赖精简                                               │ │
│  │  - image → image-meta 降级评估                          │ │
│  │  - reqwest vs ureq 评估                                 │ │
│  │  - tokio feature set 最小化                             │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────────┐
│  0.2.0 核心层（不变）                                         │
│  pawbun-toolkit / pawbun-files / pawbun-mcp-core /           │
│  pawbun-mcp-server / pawbun-toolkit-macros                   │
└──────────────────────────────────────────────────────────────┘
```

---

## 3. 性能基准测试

### 3.1 设计原则

| 原则 | 说明 |
|---|---|
| **测量而非优化** | 0.3.0 只写基准、不优化实现。优化在基线建立后评估。 |
| **稳定性优先** | 使用 `criterion` 的统计采样（≥100 次迭代），排除 JVM 预热类问题。 |
| **对比参照** | 基准结果与纯 std 操作对比（如 `ToolKit::get` vs `HashMap::get`），给出"overhead 倍数"。 |
| **可重现** | 基准脚本固定 `target-cpu` 和线程数，输出到 `benches/results/`。 |

### 3.2 pawbun-toolkit 基准

扩展现有 `crates/pawbun-toolkit/benches/toolkit.rs`：

```rust
fn benchmark_registry_lookup_100(c: &mut Criterion) {
    // 100 个工具的注册表查找
}

fn benchmark_registry_lookup_1000(c: &mut Criterion) {
    // 1000 个工具的注册表查找
}

fn benchmark_tool_execute_overhead(c: &mut Criterion) {
    // 空工具 execute 的端到端开销
}

fn benchmark_tool_descriptions(c: &mut Criterion) {
    // descriptions() 生成描述字符串
}

fn benchmark_schema_build(c: &mut Criterion) {
    // 从 10 个 ToolParameter 构建 JSON Schema
}
```

### 3.3 pawbun-files 基准

新建 `crates/pawbun-files/benches/loader.rs`：

```rust
fn benchmark_load_local(c: &mut Criterion) {
    // DefaultFileLoader::load_file 本地文件
}

fn benchmark_load_url_mock(c: &mut Criterion) {
    // wiremock 模拟 HTTP 下载
}

fn benchmark_provider_format(c: &mut Criterion) {
    // File → OpenAI / Anthropic / Gemini / Azure 格式化
}
```

### 3.4 pawbun-mcp-server 基准

新建 `crates/pawbun-mcp-server/benches/handler.rs`：

```rust
fn benchmark_initialize(c: &mut Criterion) {
    // handle_initialize 响应时间
}

fn benchmark_tools_list(c: &mut Criterion) {
    // handle_list_tools（100 个工具）
}

fn benchmark_tools_call(c: &mut Criterion) {
    // handle_call_tool（echo 工具）
}
```

### 3.5 基准目标

| 基准项 | 目标值 | 说明 |
|---|---|---|
| `toolkit_lookup` | < 100ns | `BTreeMap::get` 理论上 O(log N)，目标接近 HashMap 水平 |
| `tool_execute_overhead` | < 1μs | 不含工具内部逻辑，仅注册表查找 + trait 分发 |
| `tool_descriptions` | < 1ms/100 tools | 描述字符串拼接 |
| `schema_build` | < 10μs | 从 ToolParameter 构建 JSON Schema |
| `file_load_local` | < 2× std::fs::read | 含路径校验和元数据提取 |
| `handler_tools_list` | < 1ms/100 tools | JSON 序列化 + schema 构建 |

> **⚠️ 目标值说明**：上述数值为开发目标（development targets），非硬性 SLA。实际结果受硬件平台（x86_64 vs ARM）、编译器版本、优化级别（`-C opt-level=3` vs dev profile）影响。基准报告应注明测试环境，并与纯 std 操作对比给出"overhead 倍数"。

### 3.6 报告格式

基准结果写入 `benches/README.md`：

```markdown
# Pawbun 性能基准报告

> 测试环境: macOS 14 / Apple M3 / Rust 1.75
> 运行命令: `cargo bench --workspace`
> 日期: 2026-05-26

## pawbun-toolkit

| Benchmark | Time | Throughput | 目标达成 |
|---|---|---|---|
| registry_lookup/100 | 45ns | — | ✅ |
| registry_lookup/1000 | 65ns | — | ✅ |
| tool_execute_overhead | 850ns | — | ✅ |
| ... | ... | ... | ... |
```

---

## 4. pub API 审计

### 4.1 审计方法

逐 crate 运行 `cargo doc --document-private-items` 并审查生成的文档：

1. **过度暴露**：不应出现在 docs.rs 上的项（内部辅助函数、测试工具）标记为 `pub(crate)` 或 `#[doc(hidden)]`。
2. **缺失文档**：所有 `pub` 项必须有 doc comment；`#![deny(missing_docs)]` 在 crate root 启用。
3. **命名不一致**：最后一次统一风格的机会。
4. **trait 签名完整性**：检查是否所有方法都有合理的默认实现或强制实现。

### 4.2 预期改动清单

#### pawbun-toolkit

| 项 | 当前 | 目标 | 理由 | Breaking? |
|---|---|---|---|---|
| `json_utils` 模块 | `pub mod json_utils` | `pub(crate) mod json_utils` | 内部 JSON 解析辅助，不应暴露 | **是** |
| `json_utils::parse` | `pub` | `pub(crate)` | 同上 | **是** |
| `tools::url_utils` | `pub mod url_utils` | `pub(crate) mod url_utils` | SSRF 校验内部工具 | **是** |
| `tools::path_utils` | `pub mod path_utils` | `pub(crate) mod path_utils` | 路径校验内部工具 | **是** |
| `mcp::dynamic_tool` 的 `McpSession` | `pub` | `pub(crate)` | 内部会话管理 | **是** |
| `ToolKit` 的 `tools` 字段 | `pub` (隐含) | 保持 private | 通过 `get`/`list` 访问，不暴露内部 BTreeMap | 否（已是 private）|
| `ToolError::Io` | `{ message, kind }` | 考虑添加 `#[source]` | 补充链式错误追溯 | **是**（结构体字段变化）|

#### pawbun-files

| 项 | 当前 | 目标 | 理由 | Breaking? |
|---|---|---|---|---|
| `loader` 子模块的辅助函数 | 部分 `pub` | 降级为 `pub(crate)` | 内部文件类型检测等 | **是** |
| `provider` 子模块的 format 实现细节 | 部分 `pub` | 评估是否需要 `pub` | 用户只需 `to_provider_format`，不需内部结构 | 可能 |

#### pawbun-mcp-server

| 项 | 当前 | 目标 | 理由 | Breaking? |
|---|---|---|---|---|
| `handler::RequestHandler` | `pub(crate)` | 保持 | ✅ 已正确限制 | 否 |
| `tool_bridge` 模块 | `pub mod tool_bridge` | `pub(crate) mod tool_bridge` | 内部 bridge 实现，用户通过 builder 间接使用 | **是** |
| `transport::sse` 的 `SseSession` | `pub` (在 sse.rs 内) | `pub(crate)` | 内部结构 | 否（模块已是 `#[cfg(http)]` 限制）|

#### pawbun-mcp-core

| 项 | 当前 | 目标 | 理由 | Breaking? |
|---|---|---|---|---|
| `schema_convert` 的辅助函数 | 部分 `pub` | 降级 | 用户只需 `schema_to_parameters`/`parameters_to_schema` | **是** |

#### pawbun-toolkit-macros

| 项 | 当前 | 目标 | 理由 | Breaking? |
|---|---|---|---|---|
| `pawbun_tool` 宏文档 | 可能不完整 | 补全 doc comment | 宏是用户主要入口，需完整说明属性参数和生成的代码 | 否 |
| 宏内部辅助函数 | 部分 `pub` | `pub(crate)` | 内部实现细节不应暴露 | **是** |

### 4.3 命名一致性审计

最后一次调整机会（0.3.0 后进入 1.0 冻结期）：

| 当前 | 提议 | 涉及位置 |
|---|---|---|
| `ToolParameter::schema` | 保持 | 与 MCP `input_schema` 对应，一致性好 |
| `ToolKit::with_timeout` | 保持 | 关联函数风格，Rust 惯用 |
| `McpServerBuilder::capabilities(Value)` | 保持 | 向后兼容 |
| `SseServerConfig::with_session_ttl` | 保持 | Builder 风格一致 |

**结论**：0.2.0 的命名已较为一致，0.3.0 仅做微调，不做大范围重命名。

---

## 5. 文档与示例

### 5.1 文档标准

所有 `pub` API 必须满足：

```rust
/// 简短描述（一行）。
///
/// 详细说明（可选）。解释适用场景、限制、注意事项。
///
/// # Example
///
/// ```
/// use crate_name::TypeName;
///
/// let instance = TypeName::new();
/// assert_eq!(instance.method(), expected);
/// ```
pub fn method(&self) -> ReturnType;
```

启用 `#![deny(missing_docs)]` 于每个 crate 的 `lib.rs`。

### 5.2 示例规划

每个 crate 2 个示例，存于 `crates/<crate>/examples/`：

#### pawbun-toolkit

- `examples/basic_toolkit.rs`：创建 ToolKit → 注册 FileReadTool → 执行 → 打印结果
- `examples/custom_tool.rs`：手写 `Tool` trait 实现（不依赖宏）→ 注册 → 执行

#### pawbun-files

- `examples/load_image.rs`：加载本地图片 → 检测媒体类型 → 格式化为 OpenAI 格式
- `examples/batch_load.rs`：批量加载多个文件 → 应用约束（大小限制、类型白名单）

#### pawbun-mcp-server

- `examples/stdio_server.rs`：创建 McpServer → 注册 ToolKit → 通过 stdio 启动
- `examples/sse_server.rs`：创建 McpServer → 配置 CORS → 通过 SSE 启动（需 `http` feature）

#### pawbun-mcp-core

- `examples/schema_convert.rs`：serde_json::Value schema → Vec<ToolParameter> → 再转回 schema
- `examples/custom_transport.rs`：实现最简单的 `Transport` trait（内存队列版，基于 `std::sync::mpsc`）

#### pawbun-toolkit-macros

- `examples/basic_macro.rs`：使用 `#[pawbun_tool]` 宏定义工具，演示所有可用属性
- `examples/custom_input.rs`：宏生成的工具接收自定义输入结构体

### 5.3 Cookbook

新增 `docs/cookbook.md`：

```markdown
# Pawbun Cookbook

## 如何添加自定义工具
...

## 如何配置 MCP 服务器
...

## 如何安全加载文件
...

## 如何桥接外部 API（以 OpenAI 为例）
...

## 如何运行基准测试
...
```

---

## 6. 兼容性矩阵

### 6.1 Feature 组合

Workspace 层面的 feature 组合验证脚本 `scripts/check-features.sh`：

```bash
#!/usr/bin/env bash
set -euo pipefail

FEATURES=(
  ""
  "http"
  "tokio"
  "csv"
  "jsonpath"
  "schemars"
  "tracing"
  "macros"
  "http,tokio"
  "http,tokio,csv,jsonpath,schemars,tracing,macros"
)

for feat in "${FEATURES[@]}"; do
  if [ -z "$feat" ]; then
    echo "=== checking: no default features ==="
    cargo check --workspace --no-default-features
  else
    echo "=== checking: $feat ==="
    cargo check --workspace --no-default-features --features "$feat"
  fi
done

echo "=== checking: all features ==="
cargo check --workspace --all-features

echo "All feature combinations passed!"
```

### 6.2 最小依赖集验证

```bash
# 确保 default-features = false 的消费者不会拉取不必要依赖
cargo tree -p pawbun-toolkit --no-default-features
# 预期：仅 serde + serde_json + thiserror + async-trait

cargo tree -p pawbun-mcp-server --no-default-features
# 预期：仅 pawbun-mcp-core + pawbun-toolkit + pawbun-files + serde + thiserror
```

---

## 7. 依赖精简

### 7.1 评估项

| 依赖 | 当前用途 | 评估方向 | 风险 |
|---|---|---|---|
| `image` (pawbun-files) | 图片尺寸提取 | 评估 `image-meta` crate（更轻量） | `image-meta` 功能可能不足 |
| `reqwest` (pawbun-toolkit) | HTTP 请求 (WebFetch/WebSearch) | 同步场景是否可用 `ureq` 替代 | `ureq` 无 async 支持，async 场景仍需 reqwest |
| `tokio` (pawbun-mcp-server) | SSE 传输 runtime | 确认最小 feature set | 当前 `rt`, `rt-multi-thread`, `sync`, `macros` 已较精简 |
| `tower-http` (pawbun-mcp-server) | CORS 中间件 | 保持，仅 `http` feature | ✅ 已在 0.2.0 正确 feature-gate |
| `csv` (pawbun-toolkit) | CSV 解析 | 保持，独立 feature | ✅ |
| `jsonpath-rust` (pawbun-toolkit) | JSONPath | 保持，独立 feature | ✅ |

### 7.2 决策原则

- **替换条件**：新依赖的编译时间减少 > 30% 且功能覆盖 ≥ 90%
- **降级条件**：`image-meta` 能提取 JPEG/PNG/WebP 的尺寸和 MIME 类型
- **保持条件**：不满足替换/降级条件，或引入 breaking change

---

## 8. 实施阶段概览（高层面）

> 详细实施计划见 `docs/plans/pawbun-performance-api-audit-plan.md`。

| Phase | 内容 | 优先级 | 产出 |
|---|---|---|---|
| 1 | 基准测试 | P0 | `benches/README.md` 性能报告 |
| 2 | pub API 审计 | P0 | 清理后的 docs.rs 预览 |
| 3 | 文档与示例 | P1 | 8 个示例 + `docs/cookbook.md` |
| 4 | 兼容性矩阵 | P1 | `scripts/check-features.sh` |
| 5 | 依赖精简 | P2 | ADR 或替换实施 |
| 6 | 验收 | — | 全量通过所有检查项 |

---

## 9. 验收标准

| 检查项 | 标准 |
|---|---|
| 编译 | `cargo check --workspace --all-features` 零错误 |
| Clippy | `cargo clippy --workspace --all-features -- -D warnings` 零警告 |
| 文档 | `cargo doc --workspace --all-features` 零警告（含 `#![deny(missing_docs)]`） |
| 测试 | `cargo test --workspace --all-features` 全绿 |
| 基准 | `cargo bench --workspace` 运行通过，报告写入 `benches/README.md` |
| 示例 | 每个 crate 2 个示例可 `cargo run --example <name>` |
| 兼容性 | `scripts/check-features.sh` 全绿 |
| API 审计 | 无过度暴露的 `pub` 项；docs.rs 预览干净 |
| Breaking Changes | 所有 breaking changes 已记录到 `CHANGELOG.md` 或版本发布说明 |

---

## 10. 相关文档

- [pawbun-toolkit-spec.md](pawbun-toolkit-spec.md) — 工具层核心设计
- [pawbun-mcp-server-spec.md](pawbun-mcp-server-spec.md) — MCP 服务器设计
- [pawbun-files-spec.md](pawbun-files-spec.md) — 文件处理层设计
- [pawbun-ecosystem-integration-spec.md](pawbun-ecosystem-integration-spec.md) — 0.2.0 生态集成设计
- [../VERSIONS.md](../VERSIONS.md) — 版本记录与路线图
