package io.nutz.netlab.module;

import org.nutz.integration.jedis.RedisService;
import org.nutz.ioc.loader.annotation.Inject;
import org.nutz.ioc.loader.annotation.IocBean;
import org.nutz.lang.Strings;
import org.nutz.lang.util.NutMap;
import org.nutz.mvc.annotation.At;
import org.nutz.mvc.annotation.Fail;
import org.nutz.mvc.annotation.Ok;

import io.nutz.netlab.service.MonitorService;
import io.nutz.netlab.service.PortManager;

@Ok("json:full")
@Fail("http:500")
@At("/monitor")
@IocBean
public class MonitorModule {

	@Inject
	protected MonitorService monitor;
	
	@Inject(typeFirst = true)
	protected RedisService redis;
	
	@Inject
	protected PortManager portManager;
	
	protected static String[] keys = {
			"bop", "bip",
			"tx:udp", "rx:udp",
			"tx:tcp", "rx:tcp",
			"newc:tcp","clsc:tcp",
			"newc:udp",
	};
	
	@At({"/time/?"})
	public NutMap day(String timeStr) {
		NutMap re = new NutMap();
		re.put("ok", true);

		NutMap data = new NutMap();
		for (String key : keys) {
			String value = redis.get("netlab:" + timeStr.replace('-', ':') + ":" + key);
			if (!Strings.isBlank(value))
				data.put(key.replace(':', '_'), Long.parseLong(value));
		}
		re.put("data", data);
		
		re.put("now", new NutMap("used", portManager.getUsed()));
		
		return re;
	}
}
