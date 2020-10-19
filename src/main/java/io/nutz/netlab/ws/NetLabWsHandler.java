package io.nutz.netlab.ws;

import org.nutz.lang.Strings;
import org.nutz.lang.util.NutMap;
import org.nutz.plugins.mvc.websocket.handler.SimpleWsHandler;

import com.alibaba.druid.util.HexBin;

import io.nutz.netlab.bean.AbstractPortEntity;
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
		entity = netLabService.newPort(id, req.getString("type", "tcp"));
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
		entity.send(clientId, buff);
	}

	// 关掉指定客户端
	public void closec(NutMap req) {
		String clientId = req.getString("client");
		if (!Strings.isBlank(clientId))
			entity.closeClient(clientId);
	}

	// 配置,当前仅支持broadcast, 是否进行广播
	public void config(NutMap req) {
		if (req.containsKey("broadcast")) {
			entity.setBroadcast(req.getBoolean("broadcast", false));
		}
	}
}
