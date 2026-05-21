use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pawbun_toolkit::{
    Tool, ToolError, ToolExecutor, ToolKit, ToolParameter, ToolRegistry, ToolResult,
};
use std::borrow::Cow;

#[derive(Debug)]
struct NoOpTool;

impl Tool for NoOpTool {
    fn name(&self) -> &str {
        "noop"
    }

    fn description(&self) -> &str {
        "A no-op tool for benchmarking."
    }

    fn parameters(&self) -> Cow<'static, [ToolParameter]> {
        Cow::Owned(vec![])
    }

    fn execute(&self, input: &str) -> Result<ToolResult, ToolError> {
        Ok(ToolResult {
            success: true,
            content: input.into(),
            metadata: None,
            elapsed_ms: None,
        })
    }
}

fn benchmark_registry_lookup(c: &mut Criterion) {
    let mut kit = ToolKit::new();
    for i in 0..100 {
        kit.register(Box::new(NamedNoOpTool(format!("tool_{}", i))));
    }

    c.bench_function("registry_get", |b| {
        b.iter(|| {
            let _ = kit.get(black_box("tool_50"));
        })
    });
}

fn benchmark_tool_execution(c: &mut Criterion) {
    let mut kit = ToolKit::new();
    kit.register(Box::new(NoOpTool));

    c.bench_function("tool_execute_overhead", |b| {
        b.iter(|| {
            let result = kit.execute(black_box("noop"), black_box("hello")).unwrap();
            black_box(result);
        })
    });
}

fn benchmark_register(c: &mut Criterion) {
    c.bench_function("tool_register", |b| {
        let mut kit = ToolKit::new();
        let mut counter = 0usize;
        b.iter(|| {
            kit.register(Box::new(NamedNoOpTool(format!("tool_{}", counter))));
            counter += 1;
            black_box(&kit);
        })
    });
}

#[derive(Debug)]
struct NamedNoOpTool(String);

impl Tool for NamedNoOpTool {
    fn name(&self) -> &str {
        &self.0
    }

    fn description(&self) -> &str {
        "Named no-op tool"
    }

    fn parameters(&self) -> Cow<'static, [ToolParameter]> {
        Cow::Owned(vec![])
    }

    fn execute(&self, input: &str) -> Result<ToolResult, ToolError> {
        Ok(ToolResult {
            success: true,
            content: input.into(),
            metadata: None,
            elapsed_ms: None,
        })
    }
}

criterion_group!(
    benches,
    benchmark_registry_lookup,
    benchmark_tool_execution,
    benchmark_register
);
criterion_main!(benches);
