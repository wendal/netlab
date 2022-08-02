package io.nutz.netlab.impl.tcp;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.net.StandardSocketOptions;
import java.util.HashMap;
import java.util.concurrent.ConcurrentHashMap;

import org.nutz.lang.Streams;
import org.nutz.log.Log;
import org.nutz.log.Logs;
import org.nutz.plugins.mvc.websocket.AbstractWsEndpoint;
import org.smartboot.socket.extension.plugins.SocketOptionPlugin;
import org.smartboot.socket.extension.plugins.SslPlugin;
import org.smartboot.socket.extension.ssl.ClientAuth;
import org.smartboot.socket.transport.AioQuickServer;
import org.smartboot.socket.transport.AioSession;

import io.nutz.netlab.impl.AbstractPortEntity;

public class TcpPortEntity extends AbstractPortEntity {

	// 日志
	private static final Log log = Logs.get();
	
	protected boolean useSSL;

	// 对应port的TCP服务器
	protected AioQuickServer server;
	// 已连接的客户端
	public ConcurrentHashMap<String, AioSession> clients = new ConcurrentHashMap<>();
	
	public TcpPortEntity(String id, int port, AbstractWsEndpoint endpoint) {
		super(id, port, endpoint);
	}

	public boolean start() {
		SimpleTcpDumpProtocol protocol = new SimpleTcpDumpProtocol();
		TcpMessageProcessor processor = new TcpMessageProcessor();
		processor.entity = this;
		processor.endpoint = endpoint;
		processor.monitor = monitor;

		SocketOptionPlugin<byte[]> so = new SocketOptionPlugin<>();
		so.setOption(StandardSocketOptions.SO_SNDBUF, (Integer)64*1024);
		so.setOption(StandardSocketOptions.SO_KEEPALIVE, true);
		processor.addPlugin(so);

		if (this.useSSL) {
			InputStream ins = null;
			try {
				if (new File("keystore.jks").exists()) {
					ins = Streams.fileIn("keystore.jks");
				}
				else {
					ins = getClass().getClassLoader().getResourceAsStream("keystore.jks");
				}
				SslPlugin<byte[]> ssl = new SslPlugin<>();
				ssl.initForServer(ins, "123456", "123456", ClientAuth.NONE);
				processor.addPlugin(ssl);
			} catch (Throwable e) {
				log.debug("SSL init error", e);
			}
			finally {
				Streams.safeClose(ins);
			}
		}
		
		this.server = new AioQuickServer(port, protocol, processor);
		
		server.setBannerEnabled(false); // 禁止打印bannder,不然好多日志
		//server.setOption(StandardSocketOptions.SO_SNDBUF, (Integer)16*1024);
		//server.setOption(StandardSocketOptions.SO_KEEPALIVE, true);

		try {
			server.start();
			return true;
		} catch (Throwable e) {
			// FUCK, 监听失败
			log.info("创建Socket监听失败 port=" + port, e);
			return false;
		}
	}

	public ConcurrentHashMap<String, AioSession> getClients() {
		return clients;
	}

	public AioQuickServer getServer() {
		return server;
	}

	public void setServer(AioQuickServer server) {
		this.server = server;
	}

	@Override
	public boolean send(String clientId, byte[] data) {
		AioSession se = clients.get(clientId);
		if (se != null) {
			try {
				se.writeBuffer().writeAndFlush(data);
				// 更新统计信息
				se.stat.addTx(data.length);
				monitor.incr("tx:tcp", data.length);
				return true;
			} catch (IOException e) {
				log.debug("发送数据到客户端失败了", e);
			}
		}
		else {
			log.debug("客户端不存在" + clientId);
		}
		return false;
	}

	@Override
	public void closeClient(String clientId) {
		AioSession client = clients.remove(clientId);
		if (client != null)
			client.close();
	}

	protected Object lock = new Object();

	public boolean shutdown() {
		if (server != null) {
			synchronized (lock) {
				try {
					if (server != null) {
						server.shutdown();
						server = null;
					}
				} catch (Throwable e) {
					log.info("something happen", e);
				}
			}
			// 关闭客户端
			try {
				HashMap<String, AioSession> clients = new HashMap<String, AioSession>(this.clients);
				this.clients.clear();
				for (AioSession session : clients.values()) {
					try {
						session.close();
					} catch (Throwable e) {
						log.info("something happen", e);
					}
				}
			}
			catch (Throwable e) {
				log.info("something happen", e);
			}
			this.clients.clear();
			return true;
		}
		return false;
	}
	
	public void setUseSSL(boolean useSSL) {
		this.useSSL = useSSL;
	}
}
