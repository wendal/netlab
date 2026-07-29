# AGENTS.md — AI Agent 开发指引

## 项目概述

NetLab 是一个 TCP/UDP 端口分配器服务. 通过 WebSocket 向浏览器分配临时公网端口, 用于测试嵌入式设备的 TCP/UDP 连接.

后端: Rust (tokio + axum), 位于 `netlab-server/`
前端: Vue.js WebSocket 工具页, 位于 `web/`

## 架构分层规则

三层 Clean Architecture, 依赖方向严格单向: infrastructure -> application -> domain

### domain (`netlab-server/src/domain/`)

- 纯类型定义, 禁止任何 IO 操作
- 禁止引入 tokio, std::net, axum 等 IO crate
- 核心类型: `PortNumber`, `PortRange`, `ClientId`, `AppError`, `PortEntity` trait, `WsEvent`

### application (`netlab-server/src/application/`)

- 用例编排层, 通过 trait 抽象具体传输
- 禁止直接引入 tokio::net, rustls 等具体实现
- 核心: `PortPool` trait, `PortService`, `EntityFactory` trait, `ws_dispatch::handle`

### infrastructure (`netlab-server/src/infrastructure/`)

- 具体技术实现: axum HTTP/WS, tokio TCP/UDP, rustls TLS, Prometheus
- 实现 domain 和 application 定义的 trait
- 子模块: `http/`, `ws/`, `tcp/`, `udp/`, `metrics/`

## 关键文件路径

| 文件 | 职责 |
|------|------|
| `netlab-server/src/main.rs` | 入口, 调用 bootstrap::run() |
| `netlab-server/src/bootstrap.rs` | 启动编排: 配置加载, TLS, 端口池, 路由, 监听 |
| `netlab-server/src/config.rs` | 配置加载 (TOML + 环境变量) |
| `netlab-server/src/application/port_service.rs` | 端口生命周期核心逻辑 |
| `netlab-server/src/application/ws_dispatch.rs` | WebSocket 消息路由 |
| `netlab-server/src/application/port_pool.rs` | 并发端口池 |
| `netlab-server/src/infrastructure/tcp/entity.rs` | TCP/SSL-TCP 实体 |
| `netlab-server/src/infrastructure/udp/entity.rs` | UDP 实体 |
| `netlab-server/src/infrastructure/ws/handler.rs` | WS 会话驱动 |
| `netlab-server/src/infrastructure/http/router.rs` | axum 路由组装 |
| `netlab-server/config/application.toml` | 默认配置 |

## 代码规范

- 命名: snake_case 函数/变量, PascalCase 类型, SCREAMING_SNAKE 常量
- 错误处理: domain 层用 `AppError` (thiserror), bootstrap 用 `anyhow::Result`
- 异步: 所有 IO 操作使用 tokio async/await
- 共享状态: `Arc<T>` + `parking_lot::Mutex` 或 `DashMap`
- 日志: 使用 `tracing` 宏 (info!, warn!, error!, debug!)
- 文档注释: 公开 API 必须有 `///` 文档注释

## 构建命令

```bash
cd netlab-server

# 开发构建
cargo build

# 发布构建
cargo build --release

# 运行全部测试
cargo test

# 运行特定测试
cargo test port_pool
cargo test ws_dispatch

# 代码检查
cargo clippy -- -D warnings

# 格式化
cargo fmt
```

## 测试策略

- 单元测试: 各模块内 `#[cfg(test)] mod tests`, 使用 mockall 创建 mock
- 集成测试: `netlab-server/tests/` 目录
  - `end_to_end.rs` — 完整 WS 流程 (newp -> connect -> data -> close)
  - `tcp_port.rs` — TCP 实体测试
  - `udp_port.rs` — UDP 实体测试
- Mock 模式: `EntityFactory` trait + `PortPool` trait 通过 mockall mock
- 测试运行时: 使用 `tokio::runtime::Builder::new_current_thread()` 的 `block_on`

## WebSocket 协议要点

- 入口: `GET /ws/netlab` (axum WebSocketUpgrade)
- 消息格式: JSON, 必须含 `action` 字段 (心跳除外, 心跳为 `{}`)
- 支持的 action: `newp`, `sendc`, `closec`, `config`
- 服务器主动推送: `connected`, `data`, `closed`, `error`
- 端口类型: `tcp`, `udp`, `tcp_ssl` / `ssl-tcp`
- 数据编码: hex 字符串 (hex=true) 或 UTF-8 明文 (hex=false)
- 客户端 ID: UUID v4 字符串

## 配置加载优先级

1. 编译时嵌入的 `config/application.toml` (include_str!)
2. 运行时 `config/application.toml` 或 `./application.toml`
3. 环境变量 `NETLAB_<SECTION>__<KEY>` (最高优先级)

## 常见开发任务入口点

| 任务 | 入口 |
|------|------|
| 添加新的端口类型 | `domain/port_entity.rs` PortType + `bootstrap.rs` DefaultEntityFactory |
| 添加新的 WS action | `application/ws_dispatch.rs` handle() match 分支 |
| 修改端口分配策略 | `application/port_pool.rs` PortPool trait |
| 添加新的 Prometheus 指标 | `application/metrics.rs` + 调用点 |
| 修改 WS 事件格式 | `infrastructure/ws/handler.rs` event_to_json() |
| 修改 HTTP 路由 | `infrastructure/http/router.rs` build_router() |
| 修改配置项 | `config.rs` NetlabConfig struct + `config/application.toml` |
