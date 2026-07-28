package io.nutz.netlab;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class HexBinTest {

    @Test
    void testEncodeBasic() {
        byte[] data = {(byte) 0xAB, (byte) 0xCD, (byte) 0xEF};
        assertEquals("ABCDEF", HexBin.encode(data));
    }

    @Test
    void testEncodeUpperCase() {
        byte[] data = {(byte) 0x01, (byte) 0x23, (byte) 0x45, (byte) 0x67,
                       (byte) 0x89, (byte) 0xAB, (byte) 0xCD, (byte) 0xEF};
        assertEquals("0123456789ABCDEF", HexBin.encode(data, true, data.length));
    }

    @Test
    void testEncodeLowerCase() {
        byte[] data = {(byte) 0xAB, (byte) 0xCD};
        assertEquals("abcd", HexBin.encode(data, false, data.length));
    }

    @Test
    void testEncodeNull() {
        assertNull(HexBin.encode(null));
    }

    @Test
    void testEncodeEmpty() {
        assertEquals("", HexBin.encode(new byte[0]));
    }

    @Test
    void testEncodePartialLength() {
        byte[] data = {(byte) 0xAA, (byte) 0xBB, (byte) 0xCC};
        assertEquals("AABB", HexBin.encode(data, true, 2));
    }

    @Test
    void testDecodeBasic() {
        byte[] result = HexBin.decode("ABCDEF");
        assertNotNull(result);
        assertArrayEquals(new byte[]{(byte) 0xAB, (byte) 0xCD, (byte) 0xEF}, result);
    }

    @Test
    void testDecodeLowerCase() {
        byte[] result = HexBin.decode("abcdef");
        assertNotNull(result);
        assertArrayEquals(new byte[]{(byte) 0xAB, (byte) 0xCD, (byte) 0xEF}, result);
    }

    @Test
    void testDecodeMixedCase() {
        byte[] result = HexBin.decode("AbCdEf");
        assertNotNull(result);
        assertArrayEquals(new byte[]{(byte) 0xAB, (byte) 0xCD, (byte) 0xEF}, result);
    }

    @Test
    void testDecodeNull() {
        assertNull(HexBin.decode(null));
    }

    @Test
    void testDecodeOddLength() {
        assertNull(HexBin.decode("ABC"));
    }

    @Test
    void testDecodeInvalidChars() {
        assertNull(HexBin.decode("GGHH"));
    }

    @Test
    void testDecodeEmpty() {
        byte[] result = HexBin.decode("");
        assertNotNull(result);
        assertEquals(0, result.length);
    }

    @Test
    void testRoundTrip() {
        byte[] original = new byte[256];
        for (int i = 0; i < 256; i++) {
            original[i] = (byte) i;
        }
        String encoded = HexBin.encode(original);
        byte[] decoded = HexBin.decode(encoded);
        assertArrayEquals(original, decoded);
    }

    @Test
    void testRoundTripLargeData() {
        // 模拟1MB数据
        byte[] original = new byte[1024 * 1024];
        for (int i = 0; i < original.length; i++) {
            original[i] = (byte) (i % 256);
        }
        String encoded = HexBin.encode(original);
        assertEquals(2 * 1024 * 1024, encoded.length());
        byte[] decoded = HexBin.decode(encoded);
        assertArrayEquals(original, decoded);
    }
}
