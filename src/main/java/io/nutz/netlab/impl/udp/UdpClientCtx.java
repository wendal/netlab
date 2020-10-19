package io.nutz.netlab.impl.udp;

import java.net.SocketAddress;

import io.nutz.netlab.bean.ClientStat;

public class UdpClientCtx {

	public String id;
	public SocketAddress addr;
	public String ip;
	public int port;
	public ClientStat stat = new ClientStat();
}
