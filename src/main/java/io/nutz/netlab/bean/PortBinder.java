package io.nutz.netlab.bean;

public interface PortBinder {

	String getId();

	boolean send(String clientId, byte[] data);

	void closeClient(String clientId);

	boolean shutdown();

	boolean start();
}
