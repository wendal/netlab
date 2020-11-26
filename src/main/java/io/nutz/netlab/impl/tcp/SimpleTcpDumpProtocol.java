package io.nutz.netlab.impl.tcp;

import java.nio.ByteBuffer;

import org.smartboot.socket.Protocol;
import org.smartboot.socket.transport.AioSession;

/**
 * 这个Protocol实现的逻辑很简单, 来什么都直接读取出来
 */
public class SimpleTcpDumpProtocol implements Protocol<byte[]> {

	@Override
	public byte[] decode(ByteBuffer buffer, AioSession session) {
		byte[] tmp = new byte[buffer.remaining()];
		buffer.get(tmp);
		return tmp;
	}

}
