package io.nutz.netlab.impl;

import org.nutz.plugins.mvc.websocket.AbstractWsEndpoint;

import io.nutz.netlab.bean.PortBinder;
import io.nutz.netlab.service.MonitorService;

/**
 * 持有端口转发所需要的全部参数
 */
public abstract class AbstractPortEntity implements PortBinder {

	// 自身的id, 同时也是websocket的session id
	protected String id;
	// 所属端口, 方便调试
	protected int port;
	// 通往websocket
	public AbstractWsEndpoint endpoint;

	// 是否广播到其他客户端, 默认是禁止的
	protected boolean broadcast;
	
	protected MonitorService monitor;

	public AbstractPortEntity(String id, int port, AbstractWsEndpoint endpoint) {
		this.id = id;
		this.port = port;
		this.endpoint = endpoint;
	}

	public String getId() {
		return id;
	}

	public int getPort() {
		return port;
	}

	public void setBroadcast(boolean broadcast) {
		this.broadcast = broadcast;
	}

	public boolean isBroadcast() {
		return broadcast;
	}
	
	public void setMonitor(MonitorService monitor) {
		this.monitor = monitor;
	}
}
