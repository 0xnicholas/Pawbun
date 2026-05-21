# Pawbun

Pawbun 是 Pandaria 生态的 Rust 库，用来支持 Pandaria 生态内的各种项目。

## 模块

### pawbun-toolkit
提供各种 Agent 使用的工具。与 [CrewAI Tools](https://github.com/crewAIInc/crewAI/tree/main/lib/crewai-tools) 类似。

### pawbun-files
File handling utilities for Pandaria multimodal inputs.

## 项目结构

```
Pawbun/
├── Cargo.toml                 # Workspace 配置
├── crates/
│   ├── pawbun-toolkit/        # Agent 工具集
│   └── pawbun-files/          # 多模态文件处理
```

## 快速开始

### pawbun-toolkit

```rust
use pawbun_toolkit::{ToolKit, ToolExecutor, FileReadTool, FileWriteTool};

let mut toolkit = ToolKit::new();
toolkit.register(Box::new(FileReadTool::default()));
toolkit.register(Box::new(FileWriteTool::default()));

// 读取文件
let result = toolkit.execute("file_read", r#"{"path": "README.md"}"#).unwrap();
println!("{}", result.content);

// 写入文件
let result = toolkit.execute(
    "file_write",
    r#"{"path": "output.txt", "content": "hello"}"#,
).unwrap();
println!("{}", result.content);
```

## 构建

```bash
# 检查整个 Workspace
cargo check --workspace

# 运行测试
cargo test --workspace

# 生成文档
cargo doc --workspace --no-deps
```

## 参考
[Pandaria](https://github.com/0xnicholas/pandaria)
