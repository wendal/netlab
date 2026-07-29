# NetLab

给你一个随机的外网端口, 用于测试 TCP/UDP 连接.

测试地址: https://netlab.luatos.com

## 功能特性

- 分配可公网访问的临时端口 (TCP / UDP / SSL-TCP)
- 通过 WebSocket 实时收发数据、管理客户端连接
- 内置 Prometheus 指标导出
- TLS 支持 (Let's Encrypt PEM 或自动签名证书)
- 纯内存状态, 无外部依赖 (无数据库/Redis)

TODO:
1. 支持 MQTT
2. 支持 QUIC

## 架构概览

Rust 后端采用三层 Clean Architecture:

```
netlab-server/src/
├── domain/           # 纯类型, 无 IO (port, client, errors, port_entity)
├── application/      # 用例层 (port_pool, port_service, ws_dispatch, metrics)
└── infrastructure/   # 实现层 (axum http/ws, tokio tcp/udp, rustls tls, prometheus)
```

技术栈: tokio + axum + rustls + metrics-exporter-prometheus

## 目录结构

```
luatos-netlab/
├── netlab-server/    # Rust 后端 (主服务)
│   ├── src/          # 源码
│   ├── tests/        # 集成测试
│   ├── config/       # 默认配置 (application.toml)
│   └── cert/         # TLS 证书
├── web/              # 前端源码 (Vue.js WebSocket 工具页)
├── doc/              # 文档
└── .docker/          # Dockerfile
```

## 环境要求

- Rust 1.75+ (edition 2021)
- Node.js 14+ (仅前端编译需要)

## 快速启动

```bash
cd netlab-server
cargo run
```

服务启动后:
- HTTP + WebSocket: `http://127.0.0.1:9073`
- Prometheus 指标: `http://127.0.0.1:9400/metrics`

## 配置

配置文件位于 `netlab-server/config/application.toml`:

```toml
[server]
host = "127.0.0.1"
port = 9073

[port]
start = 21000
end   = 29000

[ssl]
cert_path = "cert/fullchain.pem"
key_path  = "cert/privkey.pem"

[metrics]
enabled = true
port    = 9400

[app]
static_dir = "./static"
```

支持环境变量覆盖, 前缀 `NETLAB_`, 双下划线分隔层级:

```bash
NETLAB_SERVER__PORT=8080 NETLAB_PORT__START=30000 cargo run
```

## 构建与测试

```bash
cd netlab-server

# 构建
cargo build --release

# 运行测试
cargo test

# 代码检查
cargo clippy
```

## 前端编译

前端源码位于 `web/` 目录:

```bash
cd web
npm install
npm run build
```

编译产物需放置到 `netlab-server/static/` 目录 (或通过 `app.static_dir` 配置指向).

**注意**: 自行部署需修改 `web/src/components/WstMain.vue` 中的后端地址和公网 IP 显示.

## 部署

详见 [doc/deploy.md](doc/deploy.md)

## 相关文档

- [设计文档](doc/design.md)
- [WebSocket API](doc/websocket_api.md)
- [监控指标](doc/monitor.md)
- [部署指南](doc/deploy.md)

## 贡献

1. 发现 bug? 欢迎 pull request
2. 有开发需求? 欢迎 pull request
# NetLab

## 功能简介

一句话, 给你一个随机的外网端口, 用于测试TCP/UDP链接

测试地址: https://netlab.luatos.com

TODO:
1. 支持 mqtt
2. 支持 quic

计划完成时间? 猴年马月

## 基本原理

不知道 ^_^, 自己翻源码吧.

## 我想XXX

1. 发现bug? 欢迎pull request
2. 有开发需求? 欢迎 pull request
3. 不会部署?源码讲解? 自助吧

## 源码说明

一个maven工程, eclipse/idea均可按maven项目导入

MainLauncher是入口,启动即可

**注意**

自行部署要公网IP和域名, 且修改如下文件中的后端地址

文件路径: src\web\wstool\src\components

```js
default: "//netlab.luatos.com/ws/netlab",
```

修改成你自己的域名, 要**公网可访问的**!!

然后还得修改页面显示

```html
<em v-if="myClientPort > 0">112.125.89.8:{{ myClientPort }}</em>
```

把`112.125.89.8` 改成你的公网ip

最后重新编译前端代码, src/web 目录有编译脚本


## 环境要求

* 必须JDK8+
* eclipse或idea等IDE开发工具,可选

## 配置信息位置

数据库配置信息,jetty端口等配置信息,均位于src/main/resources/application.properties

## 命令下启动

仅供测试用,使用mvn命令即可

```
// for windows
set MAVEN_OPTS="-Dfile.encoding=UTF-8"
mvn compile nutzboot:run

// for *uix
export MAVEN_OPTS="-Dfile.encoding=UTF-8"
mvn compile nutzboot:run
```

## 项目打包

```
mvn clean package nutzboot:shade
```

请注意,当前需要package + nutzboot:shade, 单独执行package或者nutzboot:shade是不行的


### 跳过测试
```
mvn clean package nutzboot:shade -Dmaven.test.skip=true
```

## 相关资源

* 论坛: https://nutz.cn
* 官网: https://nutz.io
* 一键生成NB的项目: https://get.nutz.io
