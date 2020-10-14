package io.nutz.netlab.bean;

import java.util.concurrent.atomic.AtomicLong;

public class ClientStat {

	// 增加几个统计属性
	public AtomicLong txByteCount = new AtomicLong();
	public AtomicLong txTimeCount = new AtomicLong();
	public long txLastTime = 0;
	public AtomicLong rxByteCount = new AtomicLong();
	public AtomicLong rxTimeCount = new AtomicLong();
	public long rxLastTime = 0;

	public void addTx(int size) {
		txByteCount.addAndGet(size);
		txTimeCount.incrementAndGet();
		txLastTime = System.currentTimeMillis();
	}

	public void addRx(int size) {
		rxByteCount.addAndGet(size);
		rxTimeCount.incrementAndGet();
		rxLastTime = System.currentTimeMillis();
	}
}
