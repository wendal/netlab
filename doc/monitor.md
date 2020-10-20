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