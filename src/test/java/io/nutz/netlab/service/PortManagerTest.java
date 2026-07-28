package io.nutz.netlab.service;

import static org.junit.jupiter.api.Assertions.*;

import java.util.HashSet;
import java.util.Set;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.nutz.ioc.impl.PropertiesProxy;

class PortManagerTest {

    private PortManager portManager;

    @BeforeEach
    void setUp() {
        portManager = new PortManager();
        PropertiesProxy conf = new PropertiesProxy();
        conf.put("netlab.port.start", "21000");
        conf.put("netlab.port.end", "21010");
        portManager.conf = conf;
        portManager.init();
    }

    @Test
    void testTakeReturnsPortInRange() {
        Integer port = portManager.take();
        assertNotNull(port);
        assertTrue(port >= 21000 && port < 21010);
    }

    @Test
    void testTakeReturnsUniquePorts() {
        Set<Integer> ports = new HashSet<>();
        for (int i = 0; i < 10; i++) {
            Integer port = portManager.take();
            assertNotNull(port);
            assertTrue(ports.add(port), "端口重复: " + port);
        }
    }

    @Test
    void testTakeExhausted() {
        // 取完所有端口(21000-21009, 共10个)
        for (int i = 0; i < 10; i++) {
            assertNotNull(portManager.take());
        }
        // 第11个应该返回null
        assertNull(portManager.take());
    }

    @Test
    void testRecycle() {
        Integer port = portManager.take();
        assertNotNull(port);
        assertEquals(1, portManager.getUsed());

        portManager.recycle(port);
        assertEquals(0, portManager.getUsed());

        // 回收后可以再次借出
        Integer port2 = portManager.take();
        assertNotNull(port2);
    }

    @Test
    void testRecycleOutOfRange() {
        // 超出范围的端口不应该被回收
        portManager.recycle(9999);
        portManager.recycle(30000);
        portManager.recycle(null);
        assertEquals(0, portManager.getUsed());
    }

    @Test
    void testTakeSpecificPort() {
        assertTrue(portManager.take(21005));
        assertEquals(1, portManager.getUsed());

        // 再次取同一个端口应该失败
        assertFalse(portManager.take(21005));
    }

    @Test
    void testTakeSpecificPortOutOfRange() {
        assertFalse(portManager.take(9999));
        assertFalse(portManager.take(30000));
    }

    @Test
    void testUsedCounter() {
        assertEquals(0, portManager.getUsed());

        portManager.take();
        assertEquals(1, portManager.getUsed());

        portManager.take();
        assertEquals(2, portManager.getUsed());

        Integer p = portManager.take();
        assertEquals(3, portManager.getUsed());

        portManager.recycle(p);
        assertEquals(2, portManager.getUsed());
    }

    @Test
    void testPortRandomization() {
        // 多次初始化,端口顺序应该不同(概率性测试)
        PortManager pm2 = new PortManager();
        PropertiesProxy conf2 = new PropertiesProxy();
        conf2.put("netlab.port.start", "21000");
        conf2.put("netlab.port.end", "21100");
        pm2.conf = conf2;
        pm2.init();

        // 取前10个端口,不应该完全是顺序的
        boolean allSequential = true;
        Integer prev = pm2.take();
        for (int i = 1; i < 10; i++) {
            Integer curr = pm2.take();
            if (curr != prev + 1) {
                allSequential = false;
                break;
            }
            prev = curr;
        }
        // 100个端口随机化后前10个完全顺序的概率极低
        assertFalse(allSequential, "端口应该被随机化");
    }
}
