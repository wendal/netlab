package io.nutz.netlab.impl.mqtt;

import org.nutz.plugins.mvc.websocket.AbstractWsEndpoint;
import org.smartboot.socket.StateMachineEnum;
import org.smartboot.socket.extension.processor.AbstractMessageProcessor;
import org.smartboot.socket.transport.AioSession;

import io.nutz.netlab.service.MonitorService;

public class MqttMessageProcessor extends AbstractMessageProcessor<byte[]> {

	public MqttPortEntity entity;
	public AbstractWsEndpoint endpoint;
	public MonitorService monitor;

	@Override
	public void process0(AioSession session, byte[] msg) {
		// TODO Auto-generated method stub
		
	}

	@Override
	public void stateEvent0(AioSession session, StateMachineEnum stateMachineEnum, Throwable throwable) {
		// TODO Auto-generated method stub
		
	}

}
