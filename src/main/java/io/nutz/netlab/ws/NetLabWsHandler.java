package io.nutz.netlab.ws;

import java.util.Map;

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

	// 获取新端口
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

	// 发送数据到单个或全部客户端
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
				if (!netLabService.writeClient(se, buff)) {
					endpoint.sendJsonSync(id, new NutMap("action", "error").setv("msg", "write error"));
				}
			}
		}
		for (AioSession<byte[]> client : entity.clients.values()) {
			if (!netLabService.writeClient(client, buff)) {
				endpoint.sendJsonSync(id, new NutMap("action", "error").setv("msg", "write error"));
			}
		}
	}

	// 关掉指定客户端
	public void closec(NutMap req) {
		String clientId = req.getString("client");
		AioSession<byte[]> se = entity.clients.remove(clientId);
		if (se != null) {
			se.close();
		}
	}
	
	// 配置,当前仅支持broadcast, 是否进行广播
	public void config(NutMap req) {
		if (req.containsKey("broadcast")) {
			entity.broadcast = req.getBoolean("broadcast", false);
		}
	}
	
	// 查询状态
	public void stat(NutMap req) {
		NutMap re = new NutMap();
		for (Map.Entry<String, AioSession<byte[]>> en : entity.clients.entrySet()) {
			re.put(en.getKey(), en.getValue().stat);
		}
		endpoint.sendJsonSync(id, new NutMap("action", "stat").setv("data", re));
	}
}
