package io.nutz.netlab.service;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import org.nutz.ioc.impl.PropertiesProxy;

import io.nutz.netlab.impl.AbstractPortEntity;
import io.nutz.netlab.ws.NetLabWsEndpoint;

@ExtendWith(MockitoExtension.class)
class NetLabServiceTest {

    private NetLabService service;

    @Mock
    private NetLabWsEndpoint endpoint;

    @Mock
    private MonitorService monitor;

    private PortManager portManager;

    @BeforeEach
    void setUp() {
        service = new NetLabService();
        service.endpoint = endpoint;
        service.monitor = monitor;

        portManager = new PortManager();
        PropertiesProxy conf = new PropertiesProxy();
        conf.put("netlab.port.start", "21000");
        conf.put("netlab.port.end", "21010");
        portManager.conf = conf;
        portManager.init();
        service.portManager = portManager;

        PropertiesProxy serviceConf = new PropertiesProxy();
        service.conf = serviceConf;
    }

    @Test
    void testNewPortTcp() {
        when(monitor.port_req_total).thenReturn(mock(io.prometheus.metrics.core.metrics.Counter.class));
        when(monitor.port_used).thenReturn(mock(io.prometheus.metrics.core.metrics.Gauge.class));

        // 由于Prometheus Counter/Gauge的labelValues返回链式调用,需要更深层mock
        // 这里简化测试: 验证端口分配逻辑
        io.prometheus.metrics.core.metrics.Counter counter = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        io.prometheus.metrics.core.metrics.Gauge gauge = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);
        monitor.port_req_total = counter;
        monitor.port_used = gauge;
        monitor.data = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        monitor.client = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);

        AbstractPortEntity entity = service.newPort("test-session", "tcp", -1);
        assertNotNull(entity);
        assertTrue(entity.getPort() >= 21000 && entity.getPort() < 21010);
        assertEquals("test-session", entity.getId());

        // 清理
        entity.shutdown();
    }

    @Test
    void testNewPortUdp() {
        io.prometheus.metrics.core.metrics.Counter counter = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        io.prometheus.metrics.core.metrics.Gauge gauge = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);
        monitor.port_req_total = counter;
        monitor.port_used = gauge;
        monitor.data = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        monitor.client = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);

        AbstractPortEntity entity = service.newPort("test-session", "udp", -1);
        assertNotNull(entity);
        assertTrue(entity.getPort() >= 21000 && entity.getPort() < 21010);

        // 清理
        entity.shutdown();
    }

    @Test
    void testNewPortInvalidType() {
        io.prometheus.metrics.core.metrics.Counter counter = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        monitor.port_req_total = counter;

        AbstractPortEntity entity = service.newPort("test-session", "invalid", -1);
        assertNull(entity);
    }

    @Test
    void testNewPortWithForcePort() {
        io.prometheus.metrics.core.metrics.Counter counter = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        io.prometheus.metrics.core.metrics.Gauge gauge = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);
        monitor.port_req_total = counter;
        monitor.port_used = gauge;
        monitor.data = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        monitor.client = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);

        AbstractPortEntity entity = service.newPort("test-session", "tcp", 21005);
        assertNotNull(entity);
        assertEquals(21005, entity.getPort());

        // 清理
        entity.shutdown();
    }

    @Test
    void testClosePort() {
        io.prometheus.metrics.core.metrics.Counter counter = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        io.prometheus.metrics.core.metrics.Gauge gauge = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);
        monitor.port_req_total = counter;
        monitor.port_used = gauge;
        monitor.data = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        monitor.client = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);

        AbstractPortEntity entity = service.newPort("test-session", "tcp", -1);
        assertNotNull(entity);
        int port = entity.getPort();

        service.closePort(port);

        // 端口应该被回收,可以再次分配
        assertEquals(0, portManager.getUsed());
    }

    @Test
    void testClosePortInvalid() {
        // 关闭不存在的端口不应该报错
        service.closePort(0);
        service.closePort(-1);
        service.closePort(99999);
    }

    @Test
    void testPortExhaustion() {
        io.prometheus.metrics.core.metrics.Counter counter = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        io.prometheus.metrics.core.metrics.Gauge gauge = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);
        monitor.port_req_total = counter;
        monitor.port_used = gauge;
        monitor.data = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        monitor.client = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);

        // 取完所有10个端口
        for (int i = 0; i < 10; i++) {
            AbstractPortEntity entity = service.newPort("session-" + i, "tcp", -1);
            assertNotNull(entity, "第" + (i + 1) + "个端口分配失败");
        }

        // 第11个应该失败
        AbstractPortEntity entity = service.newPort("session-overflow", "tcp", -1);
        assertNull(entity);
    }
}
