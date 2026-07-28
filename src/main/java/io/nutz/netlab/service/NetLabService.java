package io.nutz.netlab.service;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

import org.nutz.ioc.impl.PropertiesProxy;
import org.nutz.ioc.loader.annotation.Inject;
import org.nutz.ioc.loader.annotation.IocBean;
import org.nutz.log.Log;
import org.nutz.log.Logs;

import io.nutz.netlab.impl.AbstractPortEntity;
import io.nutz.netlab.impl.tcp.TcpPortEntity;
import io.nutz.netlab.impl.udp.UdpPortEntity;
import io.nutz.netlab.ws.NetLabWsEndpoint;

/**
 * NetLab的主服务类
 *
 */
@IocBean
public class NetLabService {

	// 日志
	private static final Log log = Logs.get();

	// 已创建的连接实例
	protected Map<Integer, AbstractPortEntity> entities = new ConcurrentHashMap<>();

	// 通往websocket
	@Inject
	protected NetLabWsEndpoint endpoint;

	// 配置信息
	@Inject
	protected PropertiesProxy conf;

	@Inject
	protected PortManager portManager;
	
	@Inject
	protected MonitorService monitor;

	/**
	 * 新建一个连接实例,必须传入一个唯一的id
	 */
	public AbstractPortEntity newPort(String selfId, String type, int forcePort) {
		// 弹出一个可用端口
		Integer port;
		monitor.port_req_total.labelValues(type).inc();
		if (forcePort > 0 && portManager.take(forcePort)) {
			port = Integer.valueOf(forcePort);
		}
		else {
			port = portManager.take();
		}

		if (port == null) {
			// 全部端口都用完了!!
			log.warn("all port used!");
			return null;
		}

		// ---------------------------------------
		// 创建连接实例
		AbstractPortEntity entity = null;
		switch (type) {
		case "tcp":
			entity = new TcpPortEntity(selfId, port, endpoint);
			break;
		case "ssl-tcp":
		case "tcp_ssl":
			entity = new TcpPortEntity(selfId, port, endpoint);
			((TcpPortEntity)entity).setUseSSL(true);
			((TcpPortEntity)entity).setSslKeystorePassword(conf.get("netlab.ssl.keystore.password", "123456"));
			break;
		case "udp":
			entity = new UdpPortEntity(selfId, port, endpoint);
			break;
		default:
			log.info("bad port bind type " + type);
			return null;
		}
		entity.setMonitor(monitor);
		// GO,启动监听
		if (entity.start()) {
		} else {
			portManager.recycle(port); // 返还port
			monitor.port_req_total.labelValues("error").inc();
			return null;
		}
		// 创建成功了, 记录之
		entities.put(port, entity);
		// 记录历史

		monitor.port_req_total.labelValues("ok").inc();
		monitor.port_used.labelValues(type).inc();
		return entity;
	}

	/**
	 * 关闭端口,释放资源
	 */
	public void closePort(Integer port) {
		if (port < 1)
			return;
		// 从entities移除记录,然后关闭服务器
		AbstractPortEntity entity = entities.remove(port);
		if (entity != null && entity.shutdown()) {
			// 回收port
			portManager.recycle(port);
			// 记录历史
			monitor.port_used.labelValues("used").dec();
		}
	}
}
