/*******************************************************************************
 * Copyright (c) 2017-2019, org.smartboot. All rights reserved.
 * project name: smart-socket
 * file name: AioSession.java
 * Date: 2019-12-31
 * Author: sandao (zhengjunweimail@163.com)
 *
 ******************************************************************************/

package org.smartboot.socket.transport;

import java.io.IOException;
import java.io.InputStream;
import java.net.InetSocketAddress;
import java.nio.channels.AsynchronousSocketChannel;

import io.nutz.netlab.bean.ClientStat;

/**
 * @author 涓夊垁
 * @version V1.0 , 2019/8/25
 */
public abstract class AioSession {


    /**
     * Session鐘舵��:宸插叧闂�
     */
    protected static final byte SESSION_STATUS_CLOSED = 1;
    /**
     * Session鐘舵��:鍏抽棴涓�
     */
    protected static final byte SESSION_STATUS_CLOSING = 2;
    /**
     * Session鐘舵��:姝ｅ父
     */
    protected static final byte SESSION_STATUS_ENABLED = 3;


    /**
     * 浼氳瘽褰撳墠鐘舵��
     *
     * @see AioSession#SESSION_STATUS_CLOSED
     * @see AioSession#SESSION_STATUS_CLOSING
     * @see AioSession#SESSION_STATUS_ENABLED
     */
    protected byte status = SESSION_STATUS_ENABLED;
    /**
     * 闄勪欢瀵硅薄
     */
    private Object attachment;

    /**
     * 鑾峰彇WriteBuffer鐢ㄤ互鏁版嵁杈撳嚭
     *
     * @return WriteBuffer
     */
    public abstract WriteBuffer writeBuffer();

    /**
     * 寮哄埗鍏抽棴褰撳墠AIOSession銆�
     * <p>鑻ユ鏃惰繕瀛樼暀寰呰緭鍑虹殑鏁版嵁锛屽垯浼氬鑷磋閮ㄥ垎鏁版嵁涓㈠け</p>
     */
    public final void close() {
        close(true);
    }

    /**
     * 鏄惁绔嬪嵆鍏抽棴浼氳瘽
     *
     * @param immediate true:绔嬪嵆鍏抽棴,false:鍝嶅簲娑堟伅鍙戦�佸畬鍚庡叧闂�
     */
    public abstract void close(boolean immediate);

    /**
     * 鑾峰彇褰撳墠Session鐨勫敮涓�鏍囪瘑
     *
     * @return sessionId
     */
    public String getSessionID() {
        return "aioSession-" + hashCode();
    }

    /**
     * 褰撳墠浼氳瘽鏄惁宸插け鏁�
     *
     * @return 鏄惁澶辨晥
     */
    public boolean isInvalid() {
        return status != SESSION_STATUS_ENABLED;
    }


    /**
     * 鑾峰彇闄勪欢瀵硅薄
     *
     * @param <A> 闄勪欢瀵硅薄绫诲瀷
     * @return 闄勪欢
     */
    public final <A> A getAttachment() {
        return (A) attachment;
    }

    /**
     * 瀛樻斁闄勪欢锛屾敮鎸佷换鎰忕被鍨�
     *
     * @param <A>        闄勪欢瀵硅薄绫诲瀷
     * @param attachment 闄勪欢瀵硅薄
     */
    public final <A> void setAttachment(A attachment) {
        this.attachment = attachment;
    }

    /**
     * 鑾峰彇褰撳墠浼氳瘽鐨勬湰鍦拌繛鎺ュ湴鍧�
     *
     * @return 鏈湴鍦板潃
     * @throws IOException IO寮傚父
     * @see AsynchronousSocketChannel#getLocalAddress()
     */
    public abstract InetSocketAddress getLocalAddress() throws IOException;

    /**
     * 鑾峰彇褰撳墠浼氳瘽鐨勮繙绋嬭繛鎺ュ湴鍧�
     *
     * @return 杩滅▼鍦板潃
     * @throws IOException IO寮傚父
     * @see AsynchronousSocketChannel#getRemoteAddress()
     */
    public abstract InetSocketAddress getRemoteAddress() throws IOException;

    /**
     * 鑾峰緱鏁版嵁杈撳叆娴佸璞°��
     * <p>
     * faster妯″紡涓嬭皟鐢ㄨ鏂规硶浼氳Е鍙慤nsupportedOperationException寮傚父銆�
     * </p>
     * <p>
     * MessageProcessor閲囩敤寮傛澶勭悊娑堟伅鐨勬柟寮忔椂锛岃皟鐢ㄨ鏂规硶鍙兘浼氬嚭鐜板紓甯搞��
     * </p>
     *
     * @return 杈撳叆娴�
     * @throws IOException IO寮傚父
     */
    public InputStream getInputStream() throws IOException {
        throw new UnsupportedOperationException();
    }

    /**
     * 鑾峰彇宸茬煡闀垮害鐨処nputStream
     *
     * @param length InputStream闀垮害
     * @return 杈撳叆娴�
     * @throws IOException IO寮傚父
     */
    public InputStream getInputStream(int length) throws IOException {
        throw new UnsupportedOperationException();
    }

    public ClientStat stat = new ClientStat();
}
