# Pawbun Phase 2: CodeExecuteTool Subprocess 实现

**日期**: 2026-06-13  
**状态**: 设计中  
**目标版本**: pawbun-toolkit 0.3.0  
**关联**: [Pandaria 内置工具集集成](https://github.com/0xnicholas/pandaria)

---

## 1. 问题陈述

`CodeExecuteTool` 当前是占位接口（unit struct，`execute()` 直接返回错误）。Agent 无法通过它执行 bash 命令或代码。需要提供基于 subprocess 的实际执行能力，同时保持合理的沙箱安全边界。

### 已有资产

- `examples/docker_code_executor.rs` — Docker 沙箱实现的适配器示例（完整可用）
- `Tool` / `AsyncTool` trait — 支持同步和异步执行路径
- `tools/path_utils.rs` — 路径沙箱校验（`resolve_sandbox_path`）

### 与 Docker 示例的定位关系

Docker 示例保留为**生产级安全方案**（完整隔离）。本次实现提供**轻量级 subprocess 方案**，适用于：
- 本地开发（无需 Docker daemon）
- 信任环境（CI/CD、内部工具）
- 资源受限场景（无法运行容器）

两种方案通过不同的 struct 实例共存，共享 `Tool` trait 和 `"code_execute"` 工具名。

---

## 2. 设计决策

### 2.1 执行引擎：`std::process::Command` + bash

```rust
std::process::Command::new("bash")
    .arg("-c")
    .arg(command)
    .current_dir(&self.work_dir)
    .output()
```

### 2.2 安全模型：三层边界

| 层级 | 措施 | 强制 |
|------|------|------|
| **path_utils** | `work_dir` 通过 `resolve_sandbox_path` 限定在 `base_dir` 内 | ✅ |
| **白名单** | `allowed_commands: Vec<String>`（空 = 允许全部） | 可选 |
| **超时** | `wait_timeout` → 超时则 `kill()` | ✅ 默认 30s |

### 2.3 API 设计：新建 struct，不修改占位

占位 `CodeExecuteTool` 保留不动。新建 `LocalCodeExecutor` struct：

```rust
pub struct LocalCodeExecutor {
    pub work_dir: PathBuf,          // 沙箱工作目录
    pub timeout: Duration,          // 执行超时
    pub allowed_commands: Vec<String>, // 命令白名单
}
```

`Tool::name()` 返回 `"code_execute"`，`as_async()` 返回 `Some(self)`（异步工具）。

**理由**：保持向后兼容。占位 `CodeExecuteTool` 是库的稳定 API，直接修改会 breaking。通过新 struct 共存，调用方自行选择。

### 2.4 参数重设计

当前占位参数：`code`、`language`、`timeout_ms`

新设计：`command`、`work_dir`（可选）、`timeout_ms`（可选）

```json
{
  "command": "ls -la && cat foo.txt",
  "work_dir": "subdir",
  "timeout_ms": 10000
}
```

**理由**：Pandaria 的 Agent 需要的是 bash 命令执行，不是多语言代码执行（后者由 Docker 版本覆盖）。参数名 `command`（而非 `code`）也更语义准确。

---

## 3. 实现规格

### 3.1 Struct 定义

```rust
/// 基于 subprocess 的本地代码执行器。
///
/// 通过 `bash -c` 在沙箱工作目录中执行 shell 命令。
/// 适用于本地开发和信任环境。生产环境请使用 DockerCodeExecutor。
#[derive(Debug)]
pub struct LocalCodeExecutor {
    /// 沙箱工作目录（所有命令在此执行）。
    pub work_dir: PathBuf,
    /// 执行超时（默认 30 秒）。
    pub timeout: Duration,
    /// 允许的命令白名单。空 vec 表示允许所有命令。
    pub allowed_commands: Vec<String>,
}

impl LocalCodeExecutor {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            work_dir: base_dir.into(),
            timeout: Duration::from_secs(30),
            allowed_commands: Vec::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_allowed_commands(mut self, cmds: Vec<String>) -> Self {
        self.allowed_commands = cmds;
        self
    }
}
```

### 3.2 Tool trait 实现

```rust
impl Tool for LocalCodeExecutor {
    fn name(&self) -> &str { "code_execute" }
    fn description(&self) -> &str {
        "Execute a shell command via bash in the sandboxed workspace."
    }
    fn parameters(&self) -> Cow<'static, [ToolParameter]> {
        Cow::Owned(vec![
            ToolParameter {
                name: "command".into(),
                description: "Shell command to execute via bash -c".into(),
                required: true,
                schema: json!({"type": "string"}),
            },
            ToolParameter {
                name: "work_dir".into(),
                description: "Working directory relative to sandbox".into(),
                required: false,
                schema: json!({"type": "string"}),
            },
            ToolParameter {
                name: "timeout_ms".into(),
                description: "Execution timeout in milliseconds (default 30000)".into(),
                required: false,
                schema: json!({"type": "integer"}),
            },
        ])
    }
    fn execute(&self, _input: &str) -> Result<ToolResult, ToolError> {
        Err(ToolError::execution_failed(
            "LocalCodeExecutor requires async execution. Use execute_async instead.",
        ))
    }
    fn as_async(&self) -> Option<&dyn AsyncTool> { Some(self) }
}
```

### 3.3 AsyncTool 实现

```rust
#[async_trait]
impl AsyncTool for LocalCodeExecutor {
    async fn execute_async(&self, input: &str) -> Result<ToolResult, ToolError> {
        let parsed: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| ToolError::invalid_input(format!("invalid JSON: {e}")))?;

        let command = parsed["command"].as_str()
            .ok_or_else(|| ToolError::invalid_input("missing 'command' field"))?;

        // 1. 白名单校验
        if !self.allowed_commands.is_empty() {
            let cmd_name = command.split_whitespace().next().unwrap_or("");
            if !self.allowed_commands.iter().any(|a| a == cmd_name) {
                return Err(ToolError::invalid_input(format!(
                    "command '{cmd_name}' not in allowed list"
                )));
            }
        }

        // 2. 解析 work_dir（可选）
        let work_dir = if let Some(sub) = parsed["work_dir"].as_str() {
            resolve_sandbox_path(Some(&self.work_dir), sub)?
        } else {
            self.work_dir.clone()
        };

        // 3. 解析超时
        let timeout_ms = parsed["timeout_ms"].as_u64()
            .unwrap_or(self.timeout.as_millis() as u64);
        let timeout = Duration::from_millis(timeout_ms);

        // 4. 执行
        let start = std::time::Instant::now();
        let mut child = std::process::Command::new("bash")
            .arg("-c")
            .arg(command)
            .current_dir(&work_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::execution_failed(format!("spawn failed: {e}"))
                .with_source(e))?;

        // 5. 等待 + 超时处理
        let exit_status = match child.wait_timeout(timeout)
            .map_err(|e| ToolError::execution_failed(format!("wait failed: {e}"))
                .with_source(e))?
        {
            Some(status) => status,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ToolError::Timeout(timeout_ms));
            }
        };

        let output = child.wait_with_output()
            .map_err(|e| ToolError::execution_failed(format!("read output failed: {e}"))
                .with_source(e))?;

        let elapsed = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let success = exit_status.success();

        let content = if success {
            stdout.trim().to_string()
        } else {
            format!(
                "exit code: {}\nstdout:\n{}\nstderr:\n{}",
                exit_status.code().unwrap_or(-1),
                stdout.trim(),
                stderr.trim()
            )
        };

        Ok(ToolResult {
            success,
            content,
            metadata: Some(json!({
                "exit_code": exit_status.code(),
                "elapsed_ms": elapsed,
                "work_dir": work_dir.to_string_lossy(),
            })),
            elapsed_ms: Some(elapsed),
        })
    }
}
```

### 3.4 并发安全性

`LocalCodeExecutor` 不包含可变状态（所有字段在构造后不可变）→ 天然 `Send + Sync`。`wait_timeout` 是阻塞调用，`execute_async` 在 Tokio 异步上下文中运行——对于长时间命令，调用方应在外层包装 `tokio::task::spawn_blocking`（由 Pandaria 的 `PawbunToolAdapter` 处理，因为 `as_async()` 返回 `Some(self)` 会走异步路径而非 spawn_blocking）。

**⚠️ 重要**：当前 `PawbunToolAdapter` 检测到 `as_async()` 返回 `Some` 后直接在 async 上下文中调用 `execute_async`。`LocalCodeExecutor` 的 `wait_timeout` 是阻塞的——它应该在 `spawn_blocking` 中运行。解决方式：适配器对异步工具也使用 `spawn_blocking` 包装，或 `LocalCodeExecutor` 自身用 `tokio::task::spawn_blocking` 包装阻塞部分。

**推荐**：`LocalCodeExecutor::execute_async` 内部使用 `tokio::task::spawn_blocking` 包装 `wait_timeout`，这样无论调用方如何处理都能正确工作。

---

## 4. Pandaria 侧适配

Pawbun Phase 2 完成后，Pandaria 的 `build_pawbun_tool_refs()` 需要更新：

```rust
// 原来（占位）
make(Box::new(CodeExecuteTool)),

// 改为（Phase 2 后）
make(Box::new(
    LocalCodeExecutor::new(workspace.to_path_buf())
        .with_timeout(Duration::from_secs(30))
)),
```

同时需要在 `agent-core/Cargo.toml` 中确保 pawbun-toolkit 依赖更新到包含 `LocalCodeExecutor` 的版本。

---

## 5. 测试策略

### 单元测试（pawbun-toolkit）

| 测试 | 验证点 |
|------|--------|
| `test_execute_simple_command` | `echo hello` → stdout "hello", success=true |
| `test_execute_command_with_stderr` | `ls /nonexistent` → exit code != 0, content 含 stderr |
| `test_execute_timeout` | `sleep 60` + timeout=500ms → `ToolError::Timeout` |
| `test_execute_allowed_commands` | 白名单 `["echo"]`，`ls` 被拒绝 |
| `test_execute_allowed_commands_empty` | 空白名单，任意命令通过 |
| `test_execute_missing_command_field` | 输入 `{}` → `ToolError::invalid_input` |
| `test_execute_work_dir` | 子目录中 `pwd` 返回正确路径 |
| `test_execute_work_dir_traversal` | `work_dir: "../"` → 被 `resolve_sandbox_path` 拒绝 |
| `test_execute_invalid_json` | 输入 `"not json"` → `ToolError::invalid_input` |

### 集成测试（Pandaria 侧，Phase 2 合并后）

| 测试 | 验证点 |
|------|--------|
| `test_code_execute_echo` | 通过 PawbunToolAdapter 执行 `echo hello` → 成功 |
| `test_code_execute_timeout` | 通过适配器执行 `sleep 60` → 超时错误 |

---

## 6. 风险

| 风险 | 缓解 |
|------|------|
| `wait_timeout` 阻塞 async 上下文 | `execute_async` 内部用 `spawn_blocking` 包装 |
| 无真正进程隔离（与 Docker 方案比） | 文档明确标注：适用于信任环境；生产用 DockerCodeExecutor |
| 命令注入（如 `ls; rm -rf /`） | bash -c 天然会执行完整命令字符串；调用方（Pandaria path_guard）负责文件系统拦截 |
