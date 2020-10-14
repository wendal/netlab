package io.nutz.netlab.ws;

import javax.websocket.CloseReason;
import javax.websocket.EndpointConfig;
import javax.websocket.Session;
import javax.websocket.server.ServerEndpoint;

import org.nutz.ioc.Ioc;
import org.nutz.ioc.loader.annotation.Inject;
import org.nutz.ioc.loader.annotation.IocBean;
import org.nutz.plugins.mvc.websocket.AbstractWsEndpoint;
import org.nutz.plugins.mvc.websocket.NutWsConfigurator;
import org.nutz.plugins.mvc.websocket.WsHandler;

import io.nutz.netlab.service.NetLabService;

/**
 * 	页面websocket入口
 *
 */
@ServerEndpoint(value = "/ws/netlab", configurator = NutWsConfigurator.class)
@IocBean
public class NetLabWsEndpoint extends AbstractWsEndpoint {

	@Inject
	protected Ioc ioc;

	protected NetLabService netLabService;

	public WsHandler createHandler(Session session, EndpointConfig config) {
		if (netLabService == null)
			netLabService = ioc.get(NetLabService.class);
		return new NetLabWsHandler(session.getId(), netLabService);
	}

	@Override
	public void onClose(Session session, CloseReason closeReason) {
		NetLabWsHandler handler = (NetLabWsHandler) session.getMessageHandlers().iterator().next();
		if (handler != null && handler.entity != null) {
			netLabService.closePort(handler.entity.port);
		}
	}
}
