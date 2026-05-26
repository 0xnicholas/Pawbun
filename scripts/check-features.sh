#!/usr/bin/env bash
set -euo pipefail

CRATES=(
  "pawbun-toolkit"
  "pawbun-files"
  "pawbun-mcp-core"
  "pawbun-mcp-server"
)

# 对每个 crate 检查最小依赖集（no-default-features）
for crate in "${CRATES[@]}"; do
  echo "=== $crate: no default features ==="
  cargo check -p "$crate" --no-default-features
done

# 对 pawbun-toolkit 检查关键 feature 组合
echo "=== pawbun-toolkit: key feature combinations ==="
cargo check -p pawbun-toolkit --no-default-features --features http
cargo check -p pawbun-toolkit --no-default-features --features tokio
cargo check -p pawbun-toolkit --no-default-features --features csv
cargo check -p pawbun-toolkit --no-default-features --features jsonpath
cargo check -p pawbun-toolkit --no-default-features --features schemars
cargo check -p pawbun-toolkit --no-default-features --features tracing
cargo check -p pawbun-toolkit --no-default-features --features macros
cargo check -p pawbun-toolkit --no-default-features --features "http,tokio,csv,jsonpath,schemars,tracing,macros"

# 对 pawbun-files 检查关键 feature 组合
echo "=== pawbun-files: key feature combinations ==="
cargo check -p pawbun-files --no-default-features --features url-source
cargo check -p pawbun-files --no-default-features --features image-meta
cargo check -p pawbun-files --no-default-features --features "url-source,image-meta,tracing,tokio"

# workspace 全 feature 验证
echo "=== workspace: all features ==="
cargo check --workspace --all-features

echo "All feature combinations passed!"
