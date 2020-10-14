package io.nutz.netlab.service;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

import org.nutz.ioc.impl.PropertiesProxy;
import org.nutz.ioc.loader.annotation.Inject;
import org.nutz.ioc.loader.annotation.IocBean;
import org.nutz.lang.util.NutMap;
import org.nutz.log.Log;
import org.nutz.log.Logs;
import org.smartboot.socket.MessageProcessor;
import org.smartboot.socket.StateMachineEnum;
import org.smartboot.socket.transport.AioQuickServer;
import org.smartboot.socket.transport.AioSession;

import com.alibaba.druid.util.HexBin;

import io.nutz.netlab.bean.NetLabPortEntity;
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
	protected Map<Integer, NetLabPortEntity> entites = new HashMap<>();

	// 通往websocket
	@Inject
	protected NetLabWsEndpoint endpoint;

	// 配置信息
	@Inject
	protected PropertiesProxy conf;

	@Inject
	protected PortManager portManager;

	/**
	 * 新建一个连接实例,必须传入一个唯一的id
	 */
	public NetLabPortEntity newPort(String selfId) {
		// 弹出一个可用端口
		Integer port = portManager.take();
		if (port == null) {
			// 全部端口都用完了!!
			log.warn("all port used!");
			return null;
		}

		// ---------------------------------------
		// 创建连接实例
		NetLabPortEntity entity = new NetLabPortEntity();
		entity.id = selfId;
		SimpleDumpProtocol protocol = new SimpleDumpProtocol();
		NetLabMessageProcessor processor = new NetLabMessageProcessor();
		processor.entity = entity;

		// 把Socket监听准备好
		AioQuickServer<byte[]> server = new AioQuickServer<>(port, protocol, processor);
		entity.server = server;
		entity.port = port;
		server.setBannerEnabled(false); // 禁止打印bannder,不然好多日志
		server.setBossShareToWorkerThreadNum(2);
		server.setBossThreadNum(4);
		server.setWorkerThreadNum(2);
		try {
			// GO,启动监听
			server.start();
		} catch (IOException e) {
			// FUCK, 监听失败
			log.info("创建Socket监听失败 port=" + port, e);
			portManager.recycle(port); // 返还port
			return null;
		}
		// 创建成功了, 记录之
		entites.put(port, entity);
		return entity;
	}

	/**
	 * 关闭端口,释放资源
	 */
	public void closePort(Integer port) {
		// 从entities移除记录,然后关闭服务器
		NetLabPortEntity entity = entites.remove(port);
		if (entity != null && entity.server != null) {
			entity.server.shutdown();
			entity.server = null;
			// 回收port
			portManager.recycle(port);
		}
	}

	public class NetLabMessageProcessor implements MessageProcessor<byte[]> {

		public NetLabPortEntity entity;

		@Override
		public void stateEvent(AioSession<byte[]> session, StateMachineEnum stateMachineEnum, Throwable throwable) {
			NutMap re = new NutMap();
			re.put("client", session.getSessionID());
			switch (stateMachineEnum) {
			case NEW_SESSION:
				entity.clients.put(session.getSessionID(), session);
				// 通知网页端
				re.put("action", "connected");
				try {
					re.put("addr", session.getRemoteAddress().toString());
				} catch (IOException e) {
					// 不太可能出错吧
				}
				endpoint.sendJsonSync(entity.id, re);
				break;
			case SESSION_CLOSED:
				entity.clients.remove(session.getSessionID());
				// 通知网页端
				re.put("action", "closed");
				endpoint.sendJsonSync(entity.id, re);
				break;
			default:
				break;
			}
		}

		@Override
		public void process(AioSession<byte[]> session, byte[] msg) {

			// 这里是TCP客户端上传的数据, 将要发到网页端.
			NutMap re = new NutMap();
			re.put("action", "data");
			re.put("client", session.getSessionID());
			re.put("data", HexBin.encode(msg));
			re.put("hex", true);

			// 通过websocket发送出去
			endpoint.sendJsonSync(entity.id, re);

			// 更新统计信息
			session.stat.addRx(msg.length);

			// 广播到其他客户端
			if (entity.broadcast) {
				for (Map.Entry<String, AioSession<byte[]>> client : entity.clients.entrySet()) {
					if (client.getKey().equals(session.getSessionID()))
						continue; // 不要广播给自己
					writeClient(client.getValue(), msg);
				}
			}
		}

	}

	public boolean writeClient(AioSession<byte[]> session, byte[] msg) {
		try {
			// 把数据发出去
			session.writeBuffer().write(msg);
			// 更新统计信息
			session.stat.addTx(msg.length);
			return true;
		} catch (Throwable e) {
			log.debug("发送数据到客户端失败了", e);
			return false;
		}
	}
}
