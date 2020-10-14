package io.nutz.netlab.bean;

import java.util.concurrent.ConcurrentHashMap;

import org.smartboot.socket.transport.AioQuickServer;
import org.smartboot.socket.transport.AioSession;

/**
 * 持有端口转发所需要的全部参数
 */
public class NetLabPortEntity {

	// 自身的id, 同时也是websocket的session id
	public String id;
	// 对应port的TCP服务器
	public AioQuickServer<byte[]> server;
	// 所属端口, 方便调试
	public int port;
	// 已连接的客户端
	public ConcurrentHashMap<String, AioSession<byte[]>> clients = new ConcurrentHashMap<>();

	// 是否广播到其他客户端, 默认是禁止的
	public boolean broadcast;
}
