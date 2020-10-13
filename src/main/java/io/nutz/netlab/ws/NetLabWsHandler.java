package io.nutz.netlab.ws;

import java.io.IOException;

import org.nutz.lang.util.NutMap;
import org.nutz.plugins.mvc.websocket.handler.SimpleWsHandler;
import org.smartboot.socket.transport.AioSession;

import com.alibaba.druid.util.HexBin;

import io.nutz.netlab.bean.NetLabPortEntity;
import io.nutz.netlab.service.NetLabService;

public class NetLabWsHandler extends SimpleWsHandler {

	protected NetLabPortEntity entity;

	protected NetLabService netLabService;

	protected String id;

	public NetLabWsHandler(String id, NetLabService netLabService) {
		this.id = id;
		this.netLabService = netLabService;
	}

	public void newp(NutMap req) {
		if (entity != null) {
			return;
		}
		entity = netLabService.newPort(id);
		if (entity == null) {
			endpoint.sendJsonSync(id, new NutMap("action", "error").setv("msg", "alloc port fail"));
		} else {
			endpoint.sendJsonSync(id, new NutMap("action", "port").setv("port", entity.port));
		}
	}

	public void sendc(NutMap req) {
		if (entity == null) {
			return; // 没建立端口呢,何来发送
		}
		String clientId = req.getString("client");
		boolean hex = req.getBoolean("hex", false);
		String data = req.getString("data");
		byte[] buff = null;
		if (data == null || data.length() == 0) {
			return;
		}
		if (hex) {
			buff = HexBin.decode(data);
			if (buff == null) {
				endpoint.sendJsonSync(id, new NutMap("action", "error").setv("msg", "bad hex string"));
			}
		} else {
			buff = data.getBytes();
		}
		if (clientId != null) {
			AioSession<byte[]> se = entity.clients.get(clientId);
			if (se == null) {
				endpoint.sendJsonSync(id, new NutMap("action", "error").setv("msg", "client is closed"));
			} else {
				try {
					se.writeBuffer().write(buff);
				} catch (IOException e) {
					endpoint.sendJsonSync(id, new NutMap("action", "error").setv("msg", "write error"));
				}
			}
		}
		for (AioSession<byte[]> client : entity.clients.values()) {
			try {
				client.writeBuffer().writeAndFlush(buff);
			} catch (IOException e) {
				endpoint.sendJsonSync(id, new NutMap("action", "error").setv("msg", "write error"));
			}
		}
	}

	public void closec(NutMap req) {
		String clientId = req.getString("client");
		AioSession<byte[]> se = entity.clients.remove(clientId);
		if (se != null) {
			se.close();
		}
	}
	
	public void config(NutMap req) {
		if (req.containsKey("broadcast")) {
			entity.broadcast = req.getBoolean("broadcast", false);
		}
	}
}
