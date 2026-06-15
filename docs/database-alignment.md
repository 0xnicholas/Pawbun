# Pawbun — 数据库对齐方案

> 参见主设计文档：[pandaria/docs/database-design.md](../../pandaria/docs/database-design.md)

## 定位

Pawbun 是 Pandaria 生态的 **Rust library**，提供 `Tool` trait、`ToolKit` 注册中心、MCP 协议和多模态文件处理能力。

## 是否需要数据库：不

Pawbun 是编译期依赖（Cargo dependency），被 Pandaria 和 Tavern 引用为库代码。它不作为一个独立服务运行，没有需要持久化的状态。

| 为什么不需要 | 说明 |
|-------------|------|
| 工具注册 | 编译期通过 `ToolKit::register()` 完成，内存数据结构 |
| 文件处理 | `pawbun-files` 加载文件后由调用方（Pandaria session）决定是否持久化 |
| MCP 连接 | 客户端/服务端连接是进程内的，不跨重启 |

## 如果未来需要持久化

如果未来 Pawbun 需要持久化（例如工具调用审计、MCP 服务器注册表），数据属于**调用方的数据库**：

| 场景 | 谁的表 | 示例 |
|------|--------|------|
| 工具调用审计 | Pandaria `session_token_usage` 或 Tavern `workflow_events` | 已有 |
| MCP 服务器注册 | Tavern `agent_definitions.skills` | `agent_definitions` 的 `skills JSONB` 中已包含工具定义 |
| 文件元数据缓存 | Pandaria `sessions.metadata` | 可在 metadata 中附加文件引用 |

原则：**Pawbun 不拥有数据库，它的数据由使用方在自己的表中管理。**

## 与统一方案的关系

```
Pandaria 引用 Pawbun ──→ Tool 执行结果写入 Pandaria 的 sessions.entries
Tavern 引用 Pawbun ──→ Tool 执行结果写入 Tavern 的 workflow_events.payload

Pawbun 不直接写任何数据库。
```

## 对齐动作

**无需任何变更。**

- [x] 确认 Pawbun 不需要自己的数据库
- [x] 确认 Pawbun 的数据（工具执行结果）已由使用方（Pandaria、Tavern）在其各自表中覆盖

## 参考

- Pawbun 作为 Cargo dependency 被 Pandaria `Cargo.toml` 引用
- Pawbun 作为 Cargo dependency 被 Tavern `Cargo.toml` 引用（通过 Tavern 的 tool calling 机制）
