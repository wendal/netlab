package io.nutz.netlab.impl.udp;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import org.nutz.plugins.mvc.websocket.AbstractWsEndpoint;

import io.nutz.netlab.service.MonitorService;

@ExtendWith(MockitoExtension.class)
class UdpPortEntityTest {

    private UdpPortEntity entity;

    @Mock
    private AbstractWsEndpoint endpoint;

    private MonitorService monitor;

    private static final int TEST_PORT = 21098;

    @BeforeEach
    void setUp() {
        monitor = new MonitorService();
        monitor.port_req_total = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        monitor.port_used = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);
        monitor.data = mock(io.prometheus.metrics.core.metrics.Counter.class, RETURNS_DEEP_STUBS);
        monitor.client = mock(io.prometheus.metrics.core.metrics.Gauge.class, RETURNS_DEEP_STUBS);

        entity = new UdpPortEntity("test-id", TEST_PORT, endpoint);
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
        assertTrue(entity.shutdown());
        // 二次shutdown应该返回false
        assertFalse(entity.shutdown());
    }

    @Test
    void testReceiveData() throws Exception {
        assertTrue(entity.start());
        Thread.sleep(100);

        // 发送UDP数据到服务器
        try (DatagramSocket clientSocket = new DatagramSocket()) {
            byte[] data = "Hello UDP".getBytes();
            DatagramPacket packet = new DatagramPacket(data, data.length,
                    InetAddress.getByName("127.0.0.1"), TEST_PORT);
            clientSocket.send(packet);

            // 等待服务器处理
            Thread.sleep(300);

            // 应该有一个客户端注册
            assertEquals(1, entity.clients.size());
        }
    }

    @Test
    void testSendToClient() throws Exception {
        assertTrue(entity.start());
        Thread.sleep(100);

        try (DatagramSocket clientSocket = new DatagramSocket()) {
            clientSocket.setSoTimeout(3000);

            // 先发数据让服务器注册客户端
            byte[] data = "register".getBytes();
            DatagramPacket packet = new DatagramPacket(data, data.length,
                    InetAddress.getByName("127.0.0.1"), TEST_PORT);
            clientSocket.send(packet);
            Thread.sleep(300);

            // 获取客户端id
            assertEquals(1, entity.clients.size());
            String clientId = entity.clients.keySet().iterator().next();

            // 服务器向客户端发送数据
            byte[] responseData = "Response from server".getBytes();
            assertTrue(entity.send(clientId, responseData));

            // 客户端接收数据
            byte[] buf = new byte[1024];
            DatagramPacket receivePacket = new DatagramPacket(buf, buf.length);
            clientSocket.receive(receivePacket);
            assertEquals(responseData.length, receivePacket.getLength());
            assertArrayEquals(responseData,
                    java.util.Arrays.copyOf(receivePacket.getData(), receivePacket.getLength()));
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
        Thread.sleep(100);

        try (DatagramSocket clientSocket = new DatagramSocket()) {
            byte[] data = "register".getBytes();
            DatagramPacket packet = new DatagramPacket(data, data.length,
                    InetAddress.getByName("127.0.0.1"), TEST_PORT);
            clientSocket.send(packet);
            Thread.sleep(300);

            assertEquals(1, entity.clients.size());
            String clientId = entity.clients.keySet().iterator().next();

            entity.closeClient(clientId);
            assertEquals(0, entity.clients.size());
        }
    }

    @Test
    void testMultipleClients() throws Exception {
        assertTrue(entity.start());
        Thread.sleep(100);

        try (DatagramSocket client1 = new DatagramSocket();
             DatagramSocket client2 = new DatagramSocket()) {

            byte[] data = "hello".getBytes();
            client1.send(new DatagramPacket(data, data.length,
                    InetAddress.getByName("127.0.0.1"), TEST_PORT));
            client2.send(new DatagramPacket(data, data.length,
                    InetAddress.getByName("127.0.0.1"), TEST_PORT));

            Thread.sleep(300);
            assertEquals(2, entity.clients.size());
        }
    }

    @Test
    void testLargeUdpPacket() throws Exception {
        assertTrue(entity.start());
        Thread.sleep(100);

        try (DatagramSocket clientSocket = new DatagramSocket()) {
            clientSocket.setSoTimeout(3000);

            // 发送一个较大的UDP包(接近65535)
            byte[] data = new byte[1400]; // 使用安全大小,避免MTU问题
            java.util.Arrays.fill(data, (byte) 'X');
            DatagramPacket packet = new DatagramPacket(data, data.length,
                    InetAddress.getByName("127.0.0.1"), TEST_PORT);
            clientSocket.send(packet);

            Thread.sleep(300);
            assertEquals(1, entity.clients.size());
        }
    }
}
