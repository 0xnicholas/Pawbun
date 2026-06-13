# Pawbun Phase 2: CodeExecuteTool Subprocess — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the CodeExecuteTool placeholder with a subprocess-based `LocalCodeExecutor` that executes bash commands in a sandboxed workspace.

**Architecture:** New struct `LocalCodeExecutor` implements `Tool` + `AsyncTool`, using `std::process::Command` + `bash -c` with `wait_timeout`. The original `CodeExecuteTool` placeholder is preserved for backward compatibility.

**Tech Stack:** Rust stdlib (`std::process`), tokio (`spawn_blocking`), existing pawbun-toolkit path_utils.

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `crates/pawbun-toolkit/src/tools/local_code_executor.rs` | Create | `LocalCodeExecutor` struct + `Tool` + `AsyncTool` impl |
| `crates/pawbun-toolkit/src/tools/mod.rs` | Modify | Register module + re-export |
| `crates/pawbun-toolkit/src/lib.rs` | Modify | Re-export `LocalCodeExecutor` |

---

### Task 1: LocalCodeExecutor implementation

**Files:**
- Create: `crates/pawbun-toolkit/src/tools/local_code_executor.rs`

- [ ] **Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // We don't use TokioExecutor — call execute_async directly since it's async.
    // Tests use #[tokio::test] for the async runtime.

    fn make_executor() -> LocalCodeExecutor {
        let dir = std::env::temp_dir().join("pawbun_code_exec_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        LocalCodeExecutor::new(&dir).with_timeout(Duration::from_secs(5))
    }

    #[test]
    fn test_name_and_description() {
        let e = make_executor();
        assert_eq!(e.name(), "code_execute");
        assert!(e.description().contains("shell command"));
    }

    #[test]
    fn test_parameters_schema() {
        let e = make_executor();
        let params = e.parameters();
        assert_eq!(params.len(), 3);
        let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"command"));
        assert!(names.contains(&"work_dir"));
        assert!(names.contains(&"timeout_ms"));
    }

    #[tokio::test]
    async fn test_execute_simple_command() {
        let e = make_executor();
        let result = e.execute_async(r#"{"command": "echo hello"}"#).await.unwrap();
        assert!(result.success, "echo should succeed: {:?}", result);
        assert!(result.content.contains("hello"), "expected 'hello' in: {}", result.content);
    }

    #[tokio::test]
    async fn test_execute_failing_command() {
        let e = make_executor();
        let result = e.execute_async(r#"{"command": "ls /nonexistent_xyz"}"#).await.unwrap();
        assert!(!result.success, "ls of nonexistent should fail");
        assert!(result.content.contains("exit code"), "should show exit code");
    }

    #[tokio::test]
    async fn test_execute_timeout() {
        let e = LocalCodeExecutor::new(std::env::temp_dir().join("pawbun_timeout"))
            .with_timeout(Duration::from_millis(500));
        let _ = std::fs::create_dir_all(&e.work_dir);
        let result = e.execute_async(r#"{"command": "sleep 60"}"#).await;
        assert!(result.is_err(), "sleep 60 should timeout");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("timeout"), "expected timeout error, got: {err}");
    }

    #[tokio::test]
    async fn test_execute_allowed_commands() {
        let dir = std::env::temp_dir().join("pawbun_allowed");
        let _ = std::fs::create_dir_all(&dir);
        let e = LocalCodeExecutor::new(&dir)
            .with_allowed_commands(vec!["echo".into()])
            .with_timeout(Duration::from_secs(5));
        // echo is allowed
        let r = e.execute_async(r#"{"command": "echo ok"}"#).await.unwrap();
        assert!(r.success);
        // ls is NOT allowed
        let r = e.execute_async(r#"{"command": "ls"}"#).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn test_execute_missing_command() {
        let e = make_executor();
        let result = e.execute_async(r#"{}"#).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("missing 'command'"), "got: {err}");
    }

    #[tokio::test]
    async fn test_execute_work_dir_subdirectory() {
        let e = make_executor();
        std::fs::create_dir_all(e.work_dir.join("sub")).unwrap();
        let result = e.execute_async(
            r#"{"command": "pwd", "work_dir": "sub"}"#
        ).await.unwrap();
        assert!(result.success);
        assert!(result.content.contains("sub"), "pwd should show sub dir: {}", result.content);
    }

    #[tokio::test]
    async fn test_execute_work_dir_traversal() {
        let e = make_executor();
        let result = e.execute_async(
            r#"{"command": "pwd", "work_dir": "../"}"#
        ).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("path traversal") || err.contains("invalid path"),
            "expected traversal error, got: {err}");
    }

    #[tokio::test]
    async fn test_execute_invalid_json() {
        let e = make_executor();
        let result = e.execute_async("not json").await;
        assert!(result.is_err());
    }
}
```

Run: `cargo test -p pawbun-toolkit -- local_code_executor`

Expected: COMPILE ERROR — `LocalCodeExecutor` not defined.

- [ ] **Step 3: Implement LocalCodeExecutor**

```rust
use std::borrow::Cow;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use crate::tools::path_utils::resolve_sandbox_path;
use crate::{AsyncTool, Tool, ToolError, ToolParameter, ToolResult};

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

impl Tool for LocalCodeExecutor {
    fn name(&self) -> &str {
        "code_execute"
    }

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
                description: "Working directory relative to sandbox root".into(),
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

    fn as_async(&self) -> Option<&dyn AsyncTool> {
        Some(self)
    }
}

#[async_trait]
impl AsyncTool for LocalCodeExecutor {
    async fn execute_async(&self, input: &str) -> Result<ToolResult, ToolError> {
        let parsed: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| ToolError::invalid_input(format!("invalid JSON: {e}")))?;

        let command = parsed["command"]
            .as_str()
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

        // 2. 解析 work_dir（可选）—— 路径沙箱
        let work_dir = if let Some(sub) = parsed["work_dir"].as_str() {
            resolve_sandbox_path(Some(&self.work_dir), sub)?
        } else {
            self.work_dir.clone()
        };

        // 3. 解析超时
        let timeout_ms = parsed["timeout_ms"]
            .as_u64()
            .unwrap_or(self.timeout.as_millis() as u64);
        let timeout = Duration::from_millis(timeout_ms);

        // 4. spawn_blocking 包装阻塞的 wait_timeout
        let work_dir_clone = work_dir.clone();
        let cmd = command.to_string();

        tokio::task::spawn_blocking(move || {
            let start = std::time::Instant::now();

            let mut child = std::process::Command::new("bash")
                .arg("-c")
                .arg(&cmd)
                .current_dir(&work_dir_clone)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| {
                    ToolError::execution_failed(format!("spawn failed: {e}")).with_source(e)
                })?;

            let exit_status = match child.wait_timeout(timeout).map_err(|e| {
                ToolError::execution_failed(format!("wait failed: {e}")).with_source(e)
            })? {
                Some(status) => status,
                None => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ToolError::Timeout(timeout_ms));
                }
            };

            let output = child.wait_with_output().map_err(|e| {
                ToolError::execution_failed(format!("read output failed: {e}")).with_source(e)
            })?;

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
                    "work_dir": work_dir_clone.to_string_lossy(),
                })),
                elapsed_ms: Some(elapsed),
            })
        })
        .await
        .map_err(|e| {
            ToolError::execution_failed(format!("blocking task panicked: {e}"))
        })?
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p pawbun-toolkit -- local_code_executor -- --nocapture
```

Expected: 10 tests PASS.

- [ ] **Step 5: Register module**

In `crates/pawbun-toolkit/src/tools/mod.rs`, add:

```rust
pub mod local_code_executor;
pub use local_code_executor::LocalCodeExecutor;
```

- [ ] **Step 6: Re-export from lib.rs**

In `crates/pawbun-toolkit/src/lib.rs`, add to existing re-exports:

```rust
pub use tools::LocalCodeExecutor;
```

- [ ] **Step 7: Verify full workspace**

```bash
cargo test --workspace 2>&1 | tail -5
cargo clippy --workspace -- -D warnings 2>&1 | tail -5
```

Expected: all tests pass, zero clippy warnings.

- [ ] **Step 8: Commit**

```bash
git add crates/pawbun-toolkit/src/tools/local_code_executor.rs \
        crates/pawbun-toolkit/src/tools/mod.rs \
        crates/pawbun-toolkit/src/lib.rs
git commit -m "feat: add LocalCodeExecutor for subprocess-based code execution"
```

---

### Task 2: Update docs & update VERSIONS.md

**Files:**
- Modify: `VERSIONS.md`
- Modify: `docs/cookbook.md` (if exists)

- [ ] **Step 1: Update VERSIONS.md**

Add to the 0.3.0 section or create a new Phase 2 entry:

```markdown
### Phase 2: CodeExecuteTool subprocess implementation

- `LocalCodeExecutor` — subprocess bash execution via `std::process::Command`
  - Sandbox work_dir via `resolve_sandbox_path`
  - Configurable timeout with `wait_timeout` + `kill`
  - Optional command whitelist
  - Implements both `Tool` + `AsyncTool`
- `CodeExecuteTool` placeholder preserved for backward compatibility
```

- [ ] **Step 2: Commit**

```bash
git add VERSIONS.md
git commit -m "docs: document LocalCodeExecutor in VERSIONS.md"
```

---

### Task 3: Release & Pandaria integration

**Files in Pandaria repo:**
- Modify: `crates/agent-core/src/harness/builder.rs`

- [ ] **Step 1: Update builder.rs in Pandaria**

```rust
// In build_pawbun_tool_refs():
// Replace:
make(Box::new(CodeExecuteTool)),
// With:
make(Box::new(
    pawbun_toolkit::LocalCodeExecutor::new(workspace.to_path_buf())
        .with_timeout(std::time::Duration::from_secs(30)),
)),
```

- [ ] **Step 2: Run Pandaria tests**

```bash
cargo test -p agent-core -- pawbun
cargo test -p agent-core --test pawbun_integration
```

Expected: `test_code_execute_echo` passes with actual output, `test_code_execute_timeout` works.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat: switch to LocalCodeExecutor for subprocess code execution"
```

---

## Execution Order

```
Task 1 (LocalCodeExecutor) → Task 2 (docs) → Task 3 (Pandaria integration)
```

All tasks sequential.
