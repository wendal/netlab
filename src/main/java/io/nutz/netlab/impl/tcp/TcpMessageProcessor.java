package io.nutz.netlab.impl.tcp;

import java.io.IOException;
import java.net.InetSocketAddress;

import org.nutz.lang.util.NutMap;
import org.nutz.plugins.mvc.websocket.AbstractWsEndpoint;
import org.smartboot.socket.MessageProcessor;
import org.smartboot.socket.StateMachineEnum;
import org.smartboot.socket.transport.AioSession;

import com.alibaba.druid.util.HexBin;

public class TcpMessageProcessor implements MessageProcessor<byte[]> {

	public TcpPortEntity entity;
	public AbstractWsEndpoint endpoint;

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
				InetSocketAddress addr = session.getRemoteAddress();
				re.put("addr", addr.getAddress().getHostAddress() + ":" + addr.getPort());
			} catch (IOException e) {
				// 不太可能出错吧
			}
			endpoint.sendJsonSync(entity.getId(), re);
			break;
		case SESSION_CLOSED:
			entity.clients.remove(session.getSessionID());
			// 通知网页端
			re.put("action", "closed");
			endpoint.sendJsonSync(entity.getId(), re);
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
		endpoint.sendJsonSync(entity.getId(), re);

		// 更新统计信息
		session.stat.addRx(msg.length);

		// 广播到其他客户端
		if (entity.isBroadcast()) {
			String myid = session.getSessionID();
			for (String cid : entity.clients.keySet()) {
				if (!cid.endsWith(myid))
					entity.send(cid, msg);
			}
		}
	}

}