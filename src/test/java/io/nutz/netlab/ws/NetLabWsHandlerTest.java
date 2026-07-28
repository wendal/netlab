package io.nutz.netlab.ws;

import static org.mockito.Mockito.*;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.mockito.Mock;
import org.mockito.junit.jupiter.MockitoExtension;
import org.nutz.lang.util.NutMap;
import org.nutz.plugins.mvc.websocket.AbstractWsEndpoint;

import io.nutz.netlab.impl.AbstractPortEntity;
import io.nutz.netlab.service.NetLabService;

@ExtendWith(MockitoExtension.class)
class NetLabWsHandlerTest {

    private NetLabWsHandler handler;

    @Mock
    private NetLabService netLabService;

    @Mock
    private AbstractWsEndpoint endpoint;

    @Mock
    private AbstractPortEntity entity;

    private static final String SESSION_ID = "test-session";

    @BeforeEach
    void setUp() {
        handler = new NetLabWsHandler(SESSION_ID, netLabService);
        handler.endpoint = endpoint;
    }

    @Test
    void testNewpSuccess() {
        when(netLabService.newPort(eq(SESSION_ID), eq("tcp"), eq(-1))).thenReturn(entity);
        when(entity.getPort()).thenReturn(21000);

        NutMap req = new NutMap();
        req.put("action", "newp");
        req.put("type", "tcp");

        handler.newp(req);

        verify(endpoint).sendJsonSync(eq(SESSION_ID), argThat(msg ->
                "port".equals(((NutMap) msg).getString("action")) &&
                ((NutMap) msg).getInt("port") == 21000));
    }

    @Test
    void testNewpFail() {
        when(netLabService.newPort(eq(SESSION_ID), eq("tcp"), eq(-1))).thenReturn(null);

        NutMap req = new NutMap();
        req.put("action", "newp");
        req.put("type", "tcp");

        handler.newp(req);

        verify(endpoint).sendJsonSync(eq(SESSION_ID), argThat(msg ->
                "error".equals(((NutMap) msg).getString("action"))));
    }

    @Test
    void testNewpDuplicate() {
        // 已经有entity了,不应该再创建
        handler.entity = entity;

        NutMap req = new NutMap();
        req.put("action", "newp");
        req.put("type", "tcp");

        handler.newp(req);

        verify(netLabService, never()).newPort(anyString(), anyString(), anyInt());
    }

    @Test
    void testNewpWithToken() {
        when(netLabService.newPort(eq(SESSION_ID), eq("tcp"), eq(21005))).thenReturn(entity);
        when(entity.getPort()).thenReturn(21005);

        NutMap req = new NutMap();
        req.put("action", "newp");
        req.put("type", "tcp");
        req.put("token", "LuatOS-NetLab");
        req.put("port", 21005);

        handler.newp(req);

        verify(netLabService).newPort(SESSION_ID, "tcp", 21005);
    }

    @Test
    void testNewpWithInvalidToken() {
        when(netLabService.newPort(eq(SESSION_ID), eq("tcp"), eq(-1))).thenReturn(entity);
        when(entity.getPort()).thenReturn(21000);

        NutMap req = new NutMap();
        req.put("action", "newp");
        req.put("type", "tcp");
        req.put("token", "wrong-token");
        req.put("port", 21005);

        handler.newp(req);

        // token错误时port应该被置为-1
        verify(netLabService).newPort(SESSION_ID, "tcp", -1);
    }

    @Test
    void testSendcWithoutEntity() {
        // entity为null时不应该报错
        NutMap req = new NutMap();
        req.put("action", "sendc");
        req.put("client", "client-1");
        req.put("data", "hello");

        handler.sendc(req);
        // 不应该有任何交互
        verifyNoInteractions(entity);
    }

    @Test
    void testSendcTextData() {
        handler.entity = entity;

        NutMap req = new NutMap();
        req.put("action", "sendc");
        req.put("client", "client-1");
        req.put("data", "hello");

        handler.sendc(req);

        verify(entity).send(eq("client-1"), eq("hello".getBytes(java.nio.charset.StandardCharsets.UTF_8)));
    }

    @Test
    void testSendcHexData() {
        handler.entity = entity;

        NutMap req = new NutMap();
        req.put("action", "sendc");
        req.put("client", "client-1");
        req.put("data", "48454C4C4F");
        req.put("hex", true);

        handler.sendc(req);

        verify(entity).send(eq("client-1"), eq(new byte[]{0x48, 0x45, 0x4C, 0x4C, 0x4F}));
    }

    @Test
    void testSendcInvalidHex() {
        handler.entity = entity;

        NutMap req = new NutMap();
        req.put("action", "sendc");
        req.put("client", "client-1");
        req.put("data", "GGHH");
        req.put("hex", true);

        handler.sendc(req);

        verify(endpoint).sendJsonSync(eq(SESSION_ID), argThat(msg ->
                "error".equals(((NutMap) msg).getString("action"))));
        verify(entity, never()).send(anyString(), any(byte[].class));
    }

    @Test
    void testSendcEmptyData() {
        handler.entity = entity;

        NutMap req = new NutMap();
        req.put("action", "sendc");
        req.put("client", "client-1");
        req.put("data", "");

        handler.sendc(req);

        verify(entity, never()).send(anyString(), any(byte[].class));
    }

    @Test
    void testSendcBlankClient() {
        handler.entity = entity;

        NutMap req = new NutMap();
        req.put("action", "sendc");
        req.put("client", "");
        req.put("data", "hello");

        handler.sendc(req);

        verify(entity, never()).send(anyString(), any(byte[].class));
    }

    @Test
    void testClosecWithoutEntity() {
        // entity为null时不应该报错
        NutMap req = new NutMap();
        req.put("action", "closec");
        req.put("client", "client-1");

        handler.closec(req);
        verifyNoInteractions(entity);
    }

    @Test
    void testClosecWithClient() {
        handler.entity = entity;

        NutMap req = new NutMap();
        req.put("action", "closec");
        req.put("client", "client-1");

        handler.closec(req);

        verify(entity).closeClient("client-1");
    }

    @Test
    void testClosecBlankClient() {
        handler.entity = entity;

        NutMap req = new NutMap();
        req.put("action", "closec");
        req.put("client", "");

        handler.closec(req);

        verify(entity, never()).closeClient(anyString());
    }

    @Test
    void testConfigWithoutEntity() {
        // entity为null时不应该报错
        NutMap req = new NutMap();
        req.put("broadcast", true);

        handler.config(req);
        verifyNoInteractions(entity);
    }

    @Test
    void testConfigBroadcast() {
        handler.entity = entity;

        NutMap req = new NutMap();
        req.put("broadcast", true);

        handler.config(req);

        verify(entity).setBroadcast(true);
    }

    @Test
    void testConfigNoBroadcastKey() {
        handler.entity = entity;

        NutMap req = new NutMap();

        handler.config(req);

        verify(entity, never()).setBroadcast(anyBoolean());
    }
}
