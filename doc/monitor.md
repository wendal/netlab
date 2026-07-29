# NetLab 监控指标

## 概述

NetLab 使用 Prometheus 进行指标采集和监控. 指标通过 `metrics` + `metrics-exporter-prometheus` crate 实现, 无需外部依赖.

## Scrape 端点

指标通过两个独立的 HTTP 端点暴露:

| 端点 | 地址 | 说明 |
|------|------|------|
| 主端口内联 | `http://<host>:9073/metrics` | 与 WS/静态文件共享同一端口 |
| 独立 sidecar | `http://0.0.0.0:9400/metrics` | 独立监听, 供 Prometheus 抓取 |

两个端点返回相同的数据, 格式为 Prometheus text exposition format (`text/plain; version=0.0.4`).

## 指标列表

### 端口相关

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `port_req_total` | Counter | `type` | 端口分配请求累计次数 |
| `port_used` | Gauge | `type` | 当前已分配的端口数量 |

### 数据流量

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `data_total` | Counter | `type` | 数据流量累计字节数 (应用层) |
| `netlab_bytes_total` | Counter | `port_type`, `dir` | 传输层字节数, dir=rx/tx |

### 客户端连接

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `client` | Gauge | `type` | 客户端连接累计计数 |
| `connected_client` | Gauge | `type` | 当前在线客户端数量 |
| `netlab_clients_open` | Gauge | `port_type` | 当前打开的连接数 (传输层) |

### 标签值

`type` / `port_type` 标签的取值:

| 值 | 含义 |
|----|------|
| `tcp` | 普通 TCP 端口 |
| `udp` | UDP 端口 |
| `ssl-tcp` | TLS 加密 TCP 端口 |

## Prometheus 配置示例

```yaml
scrape_configs:
  - job_name: "netlab"
    static_configs:
      - targets: ["localhost:9400"]
    scrape_interval: 15s
```

## 配置

在 `config/application.toml` 中:

```toml
[metrics]
enabled = true
port    = 9400
```

设置 `enabled = false` 可禁用独立 sidecar 监听 (主端口的 `/metrics` 路由始终可用).
# 需要统计的数据

## 数量指标

1. 新分配的端口号数量(bop)
2. 回收的端口号数量  (bip)
3. 发送的数据量(tx:${udp,tcp})
4. 接收的数据量(rx:${udp,tcp})
5. 建立连接的客户端数量(newc:${udp,tcp})
5. 断开连接的客户端数量(clsc:${udp,tcp})

## 时间指标

1. 按小时计算
2. 按天计算
3. 按月统计

## redis键的设计

均使用最简单的key-value存储

```
netlab:2020:10:bop:udp=$value
netlab:2020:10:bip:udp=$value
netlab:2020:10:tx:udp=value
netlab:2020:10:rx:udp=$value
```