package io.nutz.netlab.module;

import org.nutz.ioc.loader.annotation.IocBean;
import org.nutz.mvc.annotation.At;
import org.nutz.mvc.annotation.Fail;
import org.nutz.mvc.annotation.Ok;

@Ok("json:full")
@Fail("http:500")
@At("/monitor")
@IocBean
public class MonitorModule {

}
