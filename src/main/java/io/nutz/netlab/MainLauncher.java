package io.nutz.netlab;

import org.nutz.boot.NbApp;
import org.nutz.ioc.impl.PropertiesProxy;
import org.nutz.ioc.loader.annotation.Inject;
import org.nutz.ioc.loader.annotation.IocBean;
import org.nutz.mvc.annotation.At;
import org.nutz.mvc.annotation.Ok;

import io.nutz.netlab.service.MonitorService;

@IocBean(create = "init", depose = "depose")
public class MainLauncher {

	@Inject
	protected PropertiesProxy conf;
	
	@Inject
	protected MonitorService monitor;

	@At("/")
	@Ok("->:/index.html")
	public void index() {
	}

	public void init() {
		// nop
	}

	public void depose() {
		// nop
	}

	public static void main(String[] args) throws Exception {
		new NbApp().setArgs(args).setPrintProcDoc(true).run();
	}

}
