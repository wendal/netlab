# NetLab WebSocket API

## 基本信息

* 入口 URI: `/ws/netlab`
* 通信空闲超时: 60 秒
* 默认端口号: 9073

## 基本数据结构

通信内容必须是合法的 JSON 字符串, 且必须是一个 map.

除用于维持连接所发的心跳信息外, 必须包含 `action` 属性.

```json
{
	"action" : "newp",
	"type"   : "tcp",
	"client" : "abcdef",
	"data"   : "AABBCCDD",
	"hex"    : true,
	"addr"   : "123.8.8.8:123",
	"port"   : 8080,
	"token"  : "LuatOS-NetLab"
}
```

字段说明:

| 字段 | 必选 | 说明 |
|------|------|------|
| action | 是 | 本消息所对应的动作类型 |
| type | 否 | 端口类型: `tcp`, `udp`, `tcp_ssl` (或 `ssl-tcp`) |
| client | 否 | 目标客户端 UUID |
| data | 否 | 本消息的主要数据字段 |
| hex | 否 | data 字段是否为 hex 字符串 |
| addr | 否 | 客户端远程 ip 及端口 (仅服务器下发) |
| port | 否 | 指定端口号 (需配合 token 使用) |
| token | 否 | 端口固定令牌, 值为 `LuatOS-NetLab` |

## API 列表

每个 API 对应一个 action 值, 除心跳消息外.

注意:
* 并非所有 API 都有响应值
* 对于未知 action 值, 服务器返回 error 消息
* 服务器会主动上报一些消息, 例如客户端连接成功、客户端上报数据

### 心跳消息

发送方向: 浏览器 --> 服务器

为维持 WebSocket 连接, 在空闲时 (通常是 30 秒), 需要发送一次心跳消息. 否则服务器会断开连接.

```json
{}
```

本消息不含任何字段, 仅 2 个字符, 服务器也不会做出响应.

### 申请新端口 (newp)

建立 WebSocket 后, 第一条消息应该就是本消息.

发送方向: 浏览器 --> 服务器

```json
{
	"action" : "newp",
	"type"   : "tcp"
}
```

`type` 支持的值:
* `tcp` — 普通 TCP 端口
* `udp` — UDP 端口
* `tcp_ssl` 或 `ssl-tcp` — TLS 加密的 TCP 端口

可选: 指定端口号 (需提供 token):

```json
{
	"action" : "newp",
	"type"   : "tcp",
	"port"   : 8080,
	"token"  : "LuatOS-NetLab"
}
```

若 token 不正确或未提供, 指定端口的请求会降级为随机分配.

若分配成功, 服务器会响应 `port` 消息, 否则返回 `error` 消息.

没有 port 消息前, 任何其他操作都没有意义.

### 响应新端口的申请 (port)

发送方向: 浏览器 <-- 服务器

```json
{
	"action" : "port",
	"port"   : 12345
}
```

### 新客户端已建立连接 (connected)

发送方向: 浏览器 <-- 服务器

```json
{
	"action" : "connected",
	"client" : "abcdef",
	"addr"   : "123.8.8.8:123"
}
```

### 发送数据到指定客户端 (sendc)

发送方向: 浏览器 --> 服务器

```json
{
	"action" : "sendc",
	"data"   : "ABCD",
	"hex"    : true,
	"client" : "abcdef"
}
```

注意: 当前实现中 `client` 字段为必填. 广播功能 (不指定 client 则发送到全部客户端) 尚未实现.

### 关掉指定客户端 (closec)

发送方向: 浏览器 --> 服务器

```json
{
	"action" : "closec",
	"client" : "abcdef"
}
```

### 客户端的连接被关闭 (closed)

发送方向: 浏览器 <-- 服务器

```json
{
	"action" : "closed",
	"client" : "abcdef"
}
```

### 收到客户端的数据 (data)

发送方向: 浏览器 <-- 服务器

```json
{
	"action" : "data",
	"client" : "abcdef",
	"data"   : "ABCD",
	"hex"    : true
}
```

### 会话配置 (config)

发送方向: 浏览器 --> 服务器

```json
{
	"action"    : "config",
	"broadcast" : true
}
```

当前为预留接口, 广播功能尚未实现.

### 统一错误信息 (error)

发送方向: 浏览器 <-- 服务器

```json
{
	"action" : "error",
	"msg"    : "error of what"
}
```
# NetLab的Websocket API

## 基本信息

* 入口URI: /ws/netlab
* 通信空闲超时: 60秒
* 开发环境默认端口号: 8386

## 基本数据结构

首先,通信的内容必须是合法的JSON字符串, 且必须是一个map

除用于维持连接所发的心跳信息外, 必须包含action属性.

```json
{
	"action" : "newp", 		//  [必] 本消息所对应的动作类型
	"client" : "abcdef", 		//  [选] 目标客户端id或来源客户端id
	"data"   : "AABBCCDD",	//  [选] 本消息的主要数据字段
	"hex"    : true,           //  [选]      data字段是否为hex字符串
	"addr"   : "123.8.8.8:123",// [选] 客户端远程ip及端口
}
```

## API 列表

每个API对应一个action值, 除心跳消息外

注意: 
* 并非所有API都有响应值
* 对于未知action值,应忽略之,不应该抛出异常
* 服务器会主动上报一些消息,例如客户端连接成功,客户端上报数据

### 心跳消息

发送方向: 浏览器 --> 服务器

为维持websocket连接,在空闲时(通常是30秒),需要发送一次心跳消息.否则服务器会断开连接

```json
{}
```

本消息不含任何字段,仅2个字符,服务器也不会做出响应

### 申请新端口(newp)

建立websocket后, 第一条消息应该就是本消息

发送方向: 浏览器 --> 服务器

```json
{
	"action" : "newp",
	"type"   : "tcp" // 端口类型,当前支持tcp和udp两种
}
```

若分配成功, 服务器会响应"port"消息, 否则返回"error"消息

没有port消息前, 任何其他操作都没有意义.

### 响应新端口的申请(port)

发送方向: 浏览器 <- 服务器

```json
{
	"action" : "port",
	"port"   : 12345, // 分配的端口号
}
```

### 新客户端已建立连接(connected)

发送方向: 浏览器 <- 服务器

```json
{
	"action" : "client",
	"client" : "abcdef", // 客户端的唯一id
	"addr"   : "123.8.8.8:123" // 客户端的远程ip及端口,不一定存在
}
```

### 发送数据到指定客户端(sendc)

发送方向: 浏览器 --> 服务器

```json
{
	"action" : "sendc",
	"data"   : "ABCD", //普通字符串或HEX字符串
	"hex"    : true,   // data字段是否为hex字符串
	"client" : "abcdef" // 客户端id,若不指定,则发送到全部客户端
}
```

### 关掉指定客户端(closec)

发送方向: 浏览器 --> 服务器

```json
{
	"action" : "closec",
	"client" : "abcdef" // 客户端id
}
```

### 客户端的连接被端口(closed)

发送方向: 浏览器 <- 服务器


```json
{
	"action" : "closed",
	"client" : "abcdef" // 客户端的唯一id
}
```

### 收到客户端的数据(data)

发送方向: 浏览器 <-- 服务器

```json
{
	"action" : "data",
	"client" : "abcdef", // 客户端id
	"data"   : "ABCD",   // 数据
	"hex"    : true      // 是否为hex字符串
}
```

### 统一正确回应(ok)

本回应当前仅作为预留

```json
{
	"action" : "ok"
}
```

### 统一错误信息(error)

发送方向: 浏览器 <- 服务器

```json
{
	"action" : "error",
	"msg" : "error of what", // 报错信息
}
```
