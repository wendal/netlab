# NetLab 设计

## 使用用例

### 浏览器流程

1. 通过网页端访问服务器URL
2. 展示页面, 并通过 WebSocket 连接到后端 (`GET /ws/netlab`)
3. 服务器端新建一个监听实例, 分配一个独立的端口, 并分配 ID
4. 网页端显示服务器返回的响应中的端口号

### TCP/UDP 客户端流程

1. 客户端按 IP 和端口连接到服务器
2. 服务器记录客户端 IP 和端口, 分配 UUID, 并通知到网页端

### 数据传递

1. 网页端通过 WebSocket 发送数据到服务器, 可以指定单个客户端或全部客户端
2. 设备端通过 TCP/UDP 连接收发数据, 数据发送到服务器后, 分发到网页端及其他客户端

## 架构设计

Rust 后端 (`netlab-server/`) 采用三层 Clean Architecture:

```
┌─────────────────────────────────────────────────┐
│  infrastructure (axum, tokio, rustls, prometheus)│
├─────────────────────────────────────────────────┤
│  application (port_pool, port_service, ws_dispatch)│
├─────────────────────────────────────────────────┤
│  domain (port, client, errors, port_entity)      │
└─────────────────────────────────────────────────┘
```

### domain 层

纯类型定义, 无任何 IO 操作:

| 模块 | 职责 |
|------|------|
| `port.rs` | PortNumber, PortRange, PortState |
| `client.rs` | ClientId (UUID), ClientEndpoint, ClientStat |
| `errors.rs` | AppError 统一错误枚举 |
| `port_entity.rs` | PortEntity trait, PortType, WsEvent |
| `hex.rs` | HEX 编解码工具 |

### application 层

用例编排, 不依赖具体传输实现:

| 模块 | 职责 |
|------|------|
| `port_pool.rs` | 并发端口池 (RandomPortPool), 跟踪空闲/占用 |
| `port_service.rs` | 端口生命周期编排, 会话管理, EntityFactory trait |
| `ws_dispatch.rs` | WebSocket 消息路由 (newp/sendc/closec/config) |
| `metrics.rs` | Prometheus 指标发射辅助函数 |

### infrastructure 层

具体技术实现:

| 模块 | 职责 |
|------|------|
| `http/router.rs` | axum 路由组装 (WS + metrics + static) |
| `http/static_files.rs` | 静态文件服务 (tower-http ServeDir) |
| `ws/endpoint.rs` | WebSocket HTTP 升级入口 |
| `ws/handler.rs` | WS 会话生命周期 (读循环 + 写循环 + 事件扇入) |
| `tcp/entity.rs` | TcpPortEntity: TCP/SSL-TCP accept loop + 连接驱动 |
| `tcp/tls.rs` | TlsMaterial: PEM 加载 / 自签名证书 |
| `tcp/protocol.rs` | TcpProtocol trait (DumpProtocol 默认实现) |
| `udp/entity.rs` | UdpPortEntity: UDP 数据报收发 |
| `metrics/exporter.rs` | Prometheus recorder 安装 + HTTP handler |

## 关键设计决策

1. **纯内存状态**: 无数据库/Redis 依赖, 所有端口和会话状态存于 HashMap + DashMap
2. **会话隔离**: 每个 WebSocket 连接分配独立 UUID, 端口和事件按会话隔离
3. **单写者模型**: WS 会话内部通过 mpsc channel 汇聚所有输出, 保证单一写入者
4. **优雅降级**: TLS 证书缺失时自动生成自签名证书; 静态目录缺失时返回 404 而非 panic
5. **端口固定令牌**: 指定端口号需提供共享密钥 (`LuatOS-NetLab`), 否则降级为随机分配

## 数据流

```
浏览器 ──WS──> ws/handler ──> ws_dispatch ──> PortService
                                                  │
                                          EntityFactory.create()
                                                  │
                                    ┌─────────────┼─────────────┐
                                    ▼             ▼             ▼
                              TcpPortEntity  UdpPortEntity  (SslTcp)
                                    │             │
                              WsEvent channel ──> ws/handler ──WS──> 浏览器
```
# NetLab设计

## 使用用例

### 浏览器流程

1. 通过网页端访问服务器URL
2. 展示页面, 并通过websocket连接到后端
3. 服务器端新建一个监听实例, 分配一个独立的端口, 并分配ID
4. 网页端显示服务器返回的响应中的端口号

### TCP/UDP客户端流程

1. 客户端按IP和端口连接到服务器
2. 服务器记录客户端IP和端口, 分配id, 并通知到网页端

### 数据传递

1. 网页端通过websocket发送数据到服务器,  可以指定单个客户端或全部客户端
2. 设备端,通过TCP连接收发数据, 数据发送到服务器后, 分发到网页端及其他客户端

