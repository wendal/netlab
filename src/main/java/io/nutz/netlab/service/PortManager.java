package io.nutz.netlab.service;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Queue;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.atomic.AtomicInteger;

import org.nutz.ioc.impl.PropertiesProxy;
import org.nutz.ioc.loader.annotation.Inject;
import org.nutz.ioc.loader.annotation.IocBean;
import org.nutz.log.Log;
import org.nutz.log.Logs;

/**
 * 端口管理器
 * 
 * @author wendal
 *
 */
@IocBean(create = "init")
public class PortManager {

	private static final Log log = Logs.get();

	@Inject
	protected PropertiesProxy conf;

	// 保存可用端口
	protected Queue<Integer> avaiPorts;

	protected int startPort;

	protected int endPort;

	protected AtomicInteger used = new AtomicInteger();

	/**
	 * 出借一个端口,如果没有可用的端口, 会返回null
	 */
	public Integer take() {
		Integer port = avaiPorts.poll();
		if (port != null)
			used.incrementAndGet();
		log.debug("借出端口 " + port);
		return port;
	}

	/**
	 * 返还一个端口,必须在范围内
	 */
	public void recycle(Integer port) {
		if (port == null || port < startPort || port > endPort)
			return;
		avaiPorts.add(port);
		used.decrementAndGet();
		log.debug("回收端口 " + port);
	}

	public void init() {
		startPort = conf.getInt("netlab.port.start", 21000);
		endPort = conf.getInt("netlab.port.end", 29000);
		avaiPorts = new LinkedBlockingQueue<>(endPort - startPort);
		// 添加所有可用端口
		java.util.List<Integer> list = new ArrayList<Integer>(endPort - startPort);
		for (int i = startPort; i < endPort; i++) {
			list.add(Integer.valueOf(i));
		}
		// 将端口随机化
		Collections.shuffle(list);
		avaiPorts.addAll(list);
	}
}
