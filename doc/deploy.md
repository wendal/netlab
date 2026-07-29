# NetLab 部署指南

## 环境要求

- Linux 服务器 (推荐 Debian 12 / Ubuntu 22.04+)
- 公网 IP
- 域名 (用于 TLS 证书和前端访问)
- Rust 1.75+ (仅从源码构建时需要)
- 开放端口: 9073 (HTTP/WS), 9400 (Prometheus), 21000-29000 (分配的 TCP/UDP 端口)

## 从源码构建

```bash
cd netlab-server
cargo build --release
```

产物位于 `target/release/netlab-server`.

## 配置

### 配置文件

默认配置编译进二进制文件. 运行时按以下优先级加载覆盖:

1. `config/application.toml` (相对于工作目录)
2. `./application.toml`
3. 环境变量 (最高优先级)

### 配置字段说明

```toml
[server]
host = "0.0.0.0"       # 监听地址, 生产环境建议 0.0.0.0
port = 9073            # HTTP + WebSocket 端口

[port]
start = 21000          # 可分配端口范围起始 (含)
end   = 29000          # 可分配端口范围结束 (不含)

[ssl]
cert_path = "cert/fullchain.pem"   # PEM 证书链 (leaf + intermediates)
key_path  = "cert/privkey.pem"     # PEM 私钥 (PKCS#8 或 RSA)
# key_password = "secret"          # 加密私钥的密码 (可选)

[metrics]
enabled = true         # 是否启动独立 Prometheus 监听
port    = 9400         # Prometheus sidecar 端口

[app]
static_dir = "./static"  # 前端静态文件目录
```

### 环境变量覆盖

前缀 `NETLAB_`, 双下划线 `__` 分隔层级:

```bash
export NETLAB_SERVER__HOST=0.0.0.0
export NETLAB_SERVER__PORT=8080
export NETLAB_PORT__START=30000
export NETLAB_PORT__END=39000
export NETLAB_METRICS__ENABLED=true
export NETLAB_METRICS__PORT=9400
```

## TLS 证书配置

### 使用 Let's Encrypt (推荐)

通过 caddy 或 certbot 获取证书后, 将文件放到 `cert/` 目录:

```
cert/
├── fullchain.pem   # 证书链 (leaf + intermediates)
└── privkey.pem     # 私钥 (未加密 PKCS#8)
```

caddy 自动续签示例:

```bash
caddy run --config /etc/caddy/Caddyfile
```

Caddyfile:
```
netlab.example.com {
    tls {
        issuer acme {
            dir https://acme-v02.api.letsencrypt.org/directory
        }
    }
    # caddy 仅管理证书, netlab 自行处理 TLS
}
```

### 自签名证书 (开发环境)

若 `cert_path` / `key_path` 未配置或文件不存在, 服务会自动生成自签名证书 (仅适用于开发/测试).

### 加密私钥

当前不支持加密 PEM 私钥. 请先转换:

```bash
openssl pkey -in encrypted_key.pem -out privkey.pem
```

## 防火墙规则

```bash
# HTTP + WebSocket
ufw allow 9073/tcp

# Prometheus (仅内网)
ufw allow from 10.0.0.0/8 to any port 9400 proto tcp

# 分配的端口范围
ufw allow 21000:29000/tcp
ufw allow 21000:29000/udp
```

## systemd 服务

创建 `/etc/systemd/system/netlab.service`:

```ini
[Unit]
Description=NetLab TCP/UDP Port Allocator
After=network.target

[Service]
Type=simple
User=netlab
Group=netlab
WorkingDirectory=/opt/netlab
ExecStart=/opt/netlab/netlab-server
Restart=on-failure
RestartSec=5
LimitNOFILE=65536

# 安全加固
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/opt/netlab

[Install]
WantedBy=multi-user.target
```

部署步骤:

```bash
# 创建用户
useradd -r -s /usr/sbin/nologin netlab

# 部署文件
mkdir -p /opt/netlab
cp target/release/netlab-server /opt/netlab/
cp -r netlab-server/config /opt/netlab/
cp -r netlab-server/cert /opt/netlab/
cp -r static /opt/netlab/

# 设置权限
chown -R netlab:netlab /opt/netlab

# 启动
systemctl daemon-reload
systemctl enable --now netlab
```

## Docker 部署

```bash
# 构建镜像
docker build -f .docker/Dockerfile -t netlab-server .

# 运行
docker run -d \
  --name netlab \
  -p 9073:9073 \
  -p 9400:9400 \
  -p 21000-29000:21000-29000/tcp \
  -p 21000-29000:21000-29000/udp \
  -v /etc/netlab/cert:/etc/netlab/cert \
  netlab-server
```

## Prometheus 监控接入

在 Prometheus 配置中添加:

```yaml
scrape_configs:
  - job_name: "netlab"
    static_configs:
      - targets: ["<server-ip>:9400"]
    scrape_interval: 15s
```

详见 [monitor.md](monitor.md) 了解可用指标.

## 前端编译与部署

```bash
cd web
npm install
npm run build
```

将编译产物复制到服务器的 `static/` 目录:

```bash
cp -r web/dist/* /opt/netlab/static/
```

**注意**: 编译前需修改 `web/src/components/WstMain.vue` 中的:
- WebSocket 后端地址 (默认为 `//netlab.luatos.com/ws/netlab`)
- 公网 IP 显示

## 健康检查

```bash
# 检查 HTTP 服务
curl http://localhost:9073/metrics

# 检查 WebSocket (需要 wscat 工具)
wscat -c ws://localhost:9073/ws/netlab
```
