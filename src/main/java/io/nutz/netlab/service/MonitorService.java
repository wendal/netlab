package io.nutz.netlab.service;

import java.io.IOException;

import org.nutz.ioc.impl.PropertiesProxy;
import org.nutz.ioc.loader.annotation.Inject;
import org.nutz.ioc.loader.annotation.IocBean;
import org.nutz.log.Log;
import org.nutz.log.Logs;

import io.prometheus.metrics.core.metrics.Counter;
import io.prometheus.metrics.core.metrics.Gauge;
import io.prometheus.metrics.exporter.httpserver.HTTPServer;
import io.prometheus.metrics.instrumentation.jvm.JvmMetrics;

@IocBean(create = "init", depose = "depose")
public class MonitorService {

	protected static final Log log = Logs.get();
	
	public Counter port_req_total;
	
	public Gauge port_used;
	
	public Counter data;
	
	public Gauge client;
	
	HTTPServer httpServer;
	
	@Inject
	protected PropertiesProxy conf;

	public void init() throws IOException {
		port_req_total = Counter.builder().name("port_req_total")
							   .labelNames("type")
							   .register();
		port_used = Gauge.builder().name("port_used")
				   .labelNames("type")
				   .register();

		data = Counter.builder().name("data_total")
							   .labelNames("type")
							   .register();

		port_used = Gauge.builder().name("connected_client")
				   .labelNames("type")
				   .register();
		

		client = Gauge.builder().name("client")
				   .labelNames("type")
				   .register();

		JvmMetrics.builder().register();
		httpServer = HTTPServer.builder()
                .port(conf.getInt("prometheus.http.server.port", 9400))
                .buildAndStart();
	}
	
	public void depose() {
		if (httpServer != null) {
			httpServer.close();
		}
	}
}
