package io.nutz.netlab.service;

import java.util.Calendar;

import org.nutz.aop.interceptor.async.Async;
import org.nutz.integration.jedis.RedisService;
import org.nutz.ioc.impl.PropertiesProxy;
import org.nutz.ioc.loader.annotation.Inject;
import org.nutz.ioc.loader.annotation.IocBean;
import org.nutz.lang.util.NutMap;
import org.nutz.log.Log;
import org.nutz.log.Logs;

@IocBean(create = "init", depose = "depose")
public class MonitorService implements Runnable {

	private static final Log log = Logs.get();

	public static String TIME_MINITE;
	public static String TIME_HOUR;
	public static String TIME_DAY;

	@Inject
	protected PropertiesProxy conf;

	@Inject
	protected RedisService redisService;

	protected boolean enable;

	@Async
	public void incr(String key, long val) {
		if (enable) {
			redisService.incrBy(TIME_MINITE + ":" + key, val);
			redisService.incrBy(TIME_HOUR + ":" + key, val);
			redisService.incrBy(TIME_DAY + ":" + key, val);
		}
	}

	@Async
	public void set(String key, String val) {
		if (enable) {
			redisService.set(key, val);
		}
	}

	@Async
	public void log(NutMap params) {
	}

	public Thread thread;

	public void init() {
		enable = conf.getBoolean("netlab.monitor.enable", false);
		if (enable) {
			thread = new Thread(this, "monitor_time_updater");
			thread.start();
		} else {
			log.info("monitor is disabled");
		}
	}

	public void run() {
		while (true) {
			Calendar cal = Calendar.getInstance();
			TIME_DAY = String.format("%d%02d", cal.get(Calendar.YEAR), cal.get(Calendar.DAY_OF_MONTH));
			TIME_HOUR = TIME_DAY + String.format("%02d", cal.get(Calendar.HOUR_OF_DAY));
			TIME_MINITE = TIME_HOUR + String.format("%02d", cal.get(Calendar.MINUTE));

			cal.set(Calendar.MILLISECOND, 0);
			cal.set(Calendar.SECOND, 0);
			cal.add(Calendar.MINUTE, 1);

			log.debugf("updated time tags : %s", TIME_MINITE);

			long next = cal.getTimeInMillis();
			long now = System.currentTimeMillis();
			if (next > now) {
				synchronized (this) {
					try {
						this.wait(next - now + 1);
					} catch (InterruptedException e) {
						break;
					}
				}
			}
		}
	}

	public void depose() {
		if (thread != null && thread.isAlive()) {
			synchronized (thread) {
				thread.notify();
			}
		}
	}
}
