package io.nutz.netlab.impl.udp;

import java.io.IOException;
import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.channels.ClosedSelectorException;
import java.nio.channels.DatagramChannel;
import java.nio.channels.SelectionKey;
import java.nio.channels.Selector;
import java.util.Iterator;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

import org.nutz.lang.Strings;
import org.nutz.lang.util.NutMap;
import org.nutz.log.Log;
import org.nutz.log.Logs;
import org.nutz.plugins.mvc.websocket.AbstractWsEndpoint;

import io.nutz.netlab.HexBin;
import io.nutz.netlab.impl.AbstractPortEntity;

public class UdpPortEntity extends AbstractPortEntity implements Runnable {
	// 日志
	private static final Log log = Logs.get();

	protected DatagramChannel dc;

	protected Map<String, UdpClientCtx> clients = new ConcurrentHashMap<>();

	protected Thread thread;

	protected Selector selector;

	public UdpPortEntity(String id, int port, AbstractWsEndpoint endpoint) {
		super(id, port, endpoint);
	}

	@Override
	public boolean send(String clientId, byte[] data) {
		UdpClientCtx client = clients.get(clientId);
		if (client == null) {
			return false;
		}
		try {
			dc.send(ByteBuffer.wrap(data), client.addr);
			client.stat.addTx(data.length);
			monitor.data.labelValues("udp").inc(data.length);
			return true;
		} catch (IOException e) {
			log.debug("发送数据到客户端失败", e);
		}
		return false;
	}

	@Override
	public void closeClient(String clientId) {
		// udp没有真正的连接, 不存在的关闭客户端一说
		if (!Strings.isBlank(clientId))
			clients.remove(clientId);
	}

	@Override
	public boolean shutdown() {
		if (dc != null) {
			try {
				if (selector != null) {
					selector.close();
					selector = null;
				}
			}
			catch (Throwable e) {
			}
			try {
				dc.close();
			} catch (IOException e) {
				log.debug("udp channel close error", e);
			}
			dc = null;
			monitor.client.labelValues("udp").dec(clients.size());
			return true;
		}
		return false;
	}

	@Override
	public void run() {
		log.info("开始监听UDP端口 " + port);
		byte[] buff = new byte[65535];
		ByteBuffer buf = ByteBuffer.wrap(buff);
		try {
			while (selector.select() > 0) {
				Iterator<SelectionKey> it = selector.selectedKeys().iterator();
				while (it.hasNext()) {
					SelectionKey sk = it.next();
					if (sk.isReadable()) {
						buf.clear();
						InetSocketAddress addr = (InetSocketAddress) dc.receive(buf);
						// 查找Client
						String ip = addr.getAddress().getHostAddress();
						int port = addr.getPort();
						String key = Integer.toHexString((ip + ":" + port).hashCode());
						UdpClientCtx client = clients.get(key);
						if (client == null) {
							// 登记Client
							NutMap re = new NutMap();
							re.put("client", key);
							client = new UdpClientCtx();
							client.addr = addr;
							client.ip = ip;
							client.port = port;
							clients.put(key, client);
							re.put("action", "connected");
							re.put("addr", ip + ":" + port);
							endpoint.sendJsonSync(this.id, re);
							re.remove("addr");
							monitor.client.labelValues("udp").inc();
						}
						// 通知网页端
						NutMap re = new NutMap();
						re.put("action", "data");
						re.put("client", key);
						buf.flip();
						client.stat.addTx(buf.limit());
						re.put("data", HexBin.encode(buff, true, buf.limit()));
						re.put("hex", true);
						monitor.data.labelValues("udp").inc(buf.limit());
						endpoint.sendJsonSync(this.id, re);
					}

					it.remove();
				}
			}
		} catch (ClosedSelectorException e) {
			// OK OK
		} catch (IOException e) {
			log.info("something happen?!!");
		}
		log.info("停止监听UDP端口 " + port);
	}

	@Override
	public boolean start() {
		try {
			dc = DatagramChannel.open();
			dc.configureBlocking(false);
			dc.socket().bind(new InetSocketAddress("::", port));
			selector = Selector.open();
			dc.register(selector, SelectionKey.OP_READ);
			thread = new Thread(this);
			thread.start();
			return true;
		} catch (Throwable e) {
			log.debug("udp port bind fail", e);
		}
		return false;
	}

}
