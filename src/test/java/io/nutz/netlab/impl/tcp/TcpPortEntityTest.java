package io.nutz.netlab.impl.tcp;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.io.IOException;
import java.io.OutputStream;
import java.net.Socket;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import org.nutz.plugins.mvc.websocket.AbstractWsEndpoint;

import io.nutz.netlab.service.MonitorService;

@ExtendWith(MockitoExtension.class)
class TcpPortEntityTest {

    private TcpPortEntity entity;

    @Mock
    private AbstractWsEndpoint endpoint;

    private MonitorService monitor;

    private static final int TEST_PORT = 21099;

    @BeforeEach
    void setUp() {
        monitor = new MonitorService();
        monitor.port_req_total = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        monitor.port_used = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);
        monitor.data = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        monitor.client = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);

        entity = new TcpPortEntity("test-id", TEST_PORT, endpoint);
        entity.setMonitor(monitor);
    }

    @AfterEach
    void tearDown() {
        if (entity != null) {
            entity.shutdown();
        }
    }

    @Test
    void testStartAndShutdown() {
        assertTrue(entity.start());
        assertNotNull(entity.getServer());
        assertTrue(entity.shutdown());
        // 二次shutdown应该返回false
        assertFalse(entity.shutdown());
    }

    @Test
    void testClientConnection() throws Exception {
        assertTrue(entity.start());

        // 模拟客户端连接
        try (Socket socket = new Socket("127.0.0.1", TEST_PORT)) {
            // 等待连接建立
            Thread.sleep(200);
            assertEquals(1, entity.getClients().size());
        }

        // 连接关闭后客户端应该被移除
        Thread.sleep(200);
        assertEquals(0, entity.getClients().size());
    }

    @Test
    void testSendToClient() throws Exception {
        assertTrue(entity.start());

        try (Socket socket = new Socket("127.0.0.1", TEST_PORT)) {
            Thread.sleep(200);
            assertEquals(1, entity.getClients().size());

            String clientId = entity.getClients().keySet().iterator().next();
            byte[] data = "Hello TCP".getBytes();
            assertTrue(entity.send(clientId, data));

            // 验证客户端收到数据
            byte[] buf = new byte[1024];
            int len = socket.getInputStream().read(buf);
            assertEquals(data.length, len);
            assertArrayEquals(data, java.util.Arrays.copyOf(buf, len));
        }
    }

    @Test
    void testSendToNonExistentClient() {
        assertTrue(entity.start());
        assertFalse(entity.send("non-existent", "data".getBytes()));
    }

    @Test
    void testCloseClient() throws Exception {
        assertTrue(entity.start());

        try (Socket socket = new Socket("127.0.0.1", TEST_PORT)) {
            Thread.sleep(200);
            assertEquals(1, entity.getClients().size());

            String clientId = entity.getClients().keySet().iterator().next();
            entity.closeClient(clientId);
            assertEquals(0, entity.getClients().size());
        }
    }

    @Test
    void testLargeDataSend() throws Exception {
        assertTrue(entity.start());

        try (Socket socket = new Socket("127.0.0.1", TEST_PORT)) {
            Thread.sleep(200);
            String clientId = entity.getClients().keySet().iterator().next();

            // 发送1MB数据
            byte[] largeData = new byte[1024 * 1024];
            java.util.Arrays.fill(largeData, (byte) 'A');
            assertTrue(entity.send(clientId, largeData));

            // 验证客户端能收到完整数据
            int totalRead = 0;
            byte[] buf = new byte[8192];
            socket.setSoTimeout(5000);
            while (totalRead < largeData.length) {
                int len = socket.getInputStream().read(buf);
                if (len == -1) break;
                totalRead += len;
            }
            assertEquals(largeData.length, totalRead);
        }
    }

    @Test
    void testShutdownClosesAllClients() throws Exception {
        assertTrue(entity.start());

        Socket s1 = new Socket("127.0.0.1", TEST_PORT);
        Socket s2 = new Socket("127.0.0.1", TEST_PORT);
        Thread.sleep(200);
        assertEquals(2, entity.getClients().size());

        entity.shutdown();
        assertEquals(0, entity.getClients().size());

        s1.close();
        s2.close();
    }
}
