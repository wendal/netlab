package io.nutz.netlab.ws;

import java.nio.charset.StandardCharsets;

import org.nutz.lang.Strings;
import org.nutz.lang.util.NutMap;
import org.nutz.plugins.mvc.websocket.handler.SimpleWsHandler;

import io.nutz.netlab.HexBin;

import io.nutz.netlab.impl.AbstractPortEntity;
import io.nutz.netlab.service.NetLabService;

public class NetLabWsHandler extends SimpleWsHandler {

	protected AbstractPortEntity entity;

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
		String token = req.getString("token");
		int port = req.getInt("port");
		if (!"LuatOS-NetLab".equals(token)) {
			port = -1;
		}
		entity = netLabService.newPort(id, req.getString("type", "tcp"), port);
		if (entity == null) {
			endpoint.sendJsonSync(id, new NutMap("action", "error").setv("msg", "alloc port fail"));
		} else {
			endpoint.sendJsonSync(id, new NutMap("action", "port").setv("port", entity.getPort()));
		}
	}

	// 发送数据到单个或全部客户端
	public void sendc(NutMap req) {
		if (entity == null) {
			return; // 没建立端口呢,何来发送
		}
		String clientId = req.getString("client");
		if (Strings.isBlank(clientId)) {
			return;
		}
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
			buff = data.getBytes(StandardCharsets.UTF_8);
		}
		entity.send(clientId, buff);
	}

	// 关掉指定客户端
	public void closec(NutMap req) {
		if (entity == null) {
			return;
		}
		String clientId = req.getString("client");
		if (!Strings.isBlank(clientId))
			entity.closeClient(clientId);
	}

	// 配置,当前仅支持broadcast, 是否进行广播
	public void config(NutMap req) {
		if (entity == null) {
			return;
		}
		if (req.containsKey("broadcast")) {
			entity.setBroadcast(req.getBoolean("broadcast", false));
		}
	}
}
