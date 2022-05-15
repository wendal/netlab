package io.nutz.netlab.impl.mqtt;

import java.net.StandardSocketOptions;

import org.nutz.log.Log;
import org.nutz.log.Logs;
import org.nutz.plugins.mvc.websocket.AbstractWsEndpoint;
import org.smartboot.socket.extension.plugins.SocketOptionPlugin;
import org.smartboot.socket.transport.AioQuickServer;

import io.nutz.netlab.impl.AbstractPortEntity;
import io.nutz.netlab.impl.tcp.SimpleTcpDumpProtocol;

public class MqttPortEntity extends AbstractPortEntity {
	
	private static Log log = Logs.get();

	protected AioQuickServer server;

	public MqttPortEntity(String id, int port, AbstractWsEndpoint endpoint) {
		super(id, port, endpoint);
	}

	@Override
	public boolean send(String clientId, byte[] data) {
		// TODO Auto-generated method stub
		return false;
	}

	@Override
	public void closeClient(String clientId) {
		// TODO Auto-generated method stub
		
	}

	@Override
	public boolean shutdown() {
		// TODO Auto-generated method stub
		return false;
	}

	@Override
	public boolean start() {
		SimpleTcpDumpProtocol protocol = new SimpleTcpDumpProtocol();
		MqttMessageProcessor processor = new MqttMessageProcessor();
		processor.entity = this;
		processor.endpoint = endpoint;
		processor.monitor = monitor;

		SocketOptionPlugin<byte[]> so = new SocketOptionPlugin<>();
		so.setOption(StandardSocketOptions.SO_SNDBUF, (Integer)64*1024);
		so.setOption(StandardSocketOptions.SO_KEEPALIVE, true);
		processor.addPlugin(so);
		
		this.server = new AioQuickServer(port, protocol, processor);
		
		server.setBannerEnabled(false); // 禁止打印bannder,不然好多日志
		//server.setOption(StandardSocketOptions.SO_SNDBUF, (Integer)16*1024);
		//server.setOption(StandardSocketOptions.SO_KEEPALIVE, true);

		try {
			server.start();
			return true;
		} catch (Throwable e) {
			// FUCK, 监听失败
			log.info("创建mqtt监听失败 port=" + port, e);
			return false;
		}
	}

}
