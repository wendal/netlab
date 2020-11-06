<template>
  <div class="wstool"
    :class="TopClass">
    <header>
      <h1>
        {{i18n['wst-title']}}
        <em v-if="myClientPort>0">112.125.89.8:{{myClientPort}}</em>
        <u v-if="myLastHB">
          <i class="fas fa-heartbeat"></i>
          {{myLastHB}}
        </u>
      </h1>
      <aside>
        <a 
          target="_blank" 
          href="https://github.com/chenxuuu/llcom"
          data-balloon-pos="down-right"
          :aria-label="i18n['wst-llcom-tip']">
          <i class="fas fa-tools"></i>{{i18n['wst-llcom-text']}}</a>
        <i :class="ConnectIcon"></i>
        <template v-if="isClosed">
          <button @click="OnClickConnect('tcp')">{{i18n['wst-connect-tcp']}}</button>
          <button @click="OnClickConnect('udp')">{{i18n['wst-connect-udp']}}</button>
        </template>
        <button v-else @click="OnClickClosed">{{i18n['wst-connect-close']}}</button>
        <a 
          target="_blank" 
          href="https://gitee.com/openLuat/LuatOS/issues"
          data-balloon-pos="down-right"
          :aria-label="i18n['wst-bug-tip']">
          <i class="fas fa-bug"></i>{{i18n['wst-bug-text']}}</a>
      </aside>
    </header>
    <pre class="as-error" v-if="isError">{{myErrMsg}}</pre>
    <section>
      <nav>
        <ul v-if="hasClients">
          <li v-for="cl in TheClientList"
            :key="cl.clientId"
            :class="cl.className"
            @click.left="myCurrentClientId=cl.clientId">
            <div class="as-info">
              <i 
                v-if="cl.current"
                  class="fas fa-caret-right current-dot"></i>
              <span
                v-if="cl.connected && cl.current"
                  data-balloon-pos="right"
                  :aria-label="i18n['wst-client-close-tip']"
                  @click.left="OnCloseClient(cl)"
                  ><i class="fas fa-wifi"></i></span>
              <i
                v-else-if="cl.connected"
                  class="fas fa-wifi"></i>
              <i
                v-else
                  class="fas fa-plug"></i>
              {{cl.addrHost}}
            </div>
            <div class="as-tip">
              <code class="as-client-id">{{cl.clientId}}</code>
              <code class="as-addr-port">{{cl.addrPort}}</code>
            </div>
          </li>
        </ul>
        <blockquote v-else>
          {{i18n['wst-client-empty']}}
        </blockquote>
      </nav>
      <main>
        <header>
          <div
            class="as-hex" 
            data-balloon-pos="up-left"
            :aria-label="UseHexTip"
            @click.left="myUseHex=!myUseHex">HEX</div>
          <input :placeholder="i18n['wst-send-tip']" spellcheck="fase"
            ref="input"
            :readonly="!isCanSendData"
            @change="OnSendMsg"/>
          <div
            class="as-nl"
            data-balloon-pos="up-right"
            :aria-label="UseNLTip"
            @click.left="myUseNL=!myUseNL">NL</div>
        </header>
        <blockquote ref="log">
          <p v-for="line of CurrentClientData"
            :key="line.ams"
            :class="`as-type-${line.type}`">
            <em>[{{line.time}}]</em>
            <!--
            Icon
            -->
            <i v-if="'IN' == line.type" class="fas fa-satellite-dish"></i>
            <i v-else class="far fa-paper-plane"></i>
            <u v-if="line.hex">HEX</u>
            <span class="as-dis">{{line.strDisplay}}</span>
            <span 
              v-if="myShowLineHex"
                class="as-hex">{{line.hexCode}}</span>
          </p>
        </blockquote>
        <footer>
          <div 
            class="log-order" 
            :class="{'is-desc': 'DESC' == myLogOrder}"
            data-balloon-pos="up"
            :aria-label="LogOrderTip"
            @click.left="myLogOrder='ASC'==myLogOrder?'DESC':'ASC'">
            <i :class="LogOrderIcon"></i>
          </div>
          <div 
            class="echo-back" 
            :class="{'is-on':myEchoBack}"
            data-balloon-pos="left"
            data-balloon-length="medium"
            :aria-label="EchoBackTip"
            @click.left="myEchoBack=!myEchoBack">
            <i class="fas fa-retweet"></i>
          </div>
          <div 
            class="show-hex" 
            :class="{'is-on':myShowLineHex}"
            data-balloon-pos="right"
            :aria-label="ShowLineHexTip"
            @click.left="myShowLineHex=!myShowLineHex">HEX</div>
        </footer>
      </main>
    </section>
  </div>
</template>

<script>
import WST  from "../support/util.js"
import i18n from "../i18n/current.js"
import _ from 'lodash'

export default {
  name : 'WstMain',
  data : ()=>({
    // 多国语言
    i18n,
    // 心跳的句柄
    HBI : undefined,
    myLastHB : undefined,
    // CONNECTED | CLOSED | ERROR
    myStat : "CLOSED",
    myUseHex : false,
    myUseNL : false,
    myEchoBack : false,
    myLogOrder : "ASC",
    myShowLineHex : true,
    myClientPort : undefined,
    myErrMsg : undefined,
    myClients : {
      // "a7305652" : {
      //   clientId: "a7305652",
      //   addr : "/119.130.132.35:48905",
      //   connected: true
      // },
      // "fake0" : {
      //   clientId: "fake0",
      //   addr : "/119.130.132.35:48905",
      //   connected: false
      // },
      // "fake1" : {
      //   clientId: "fake1",
      //   addr : "/119.130.132.35:48905",
      //   connected: true
      // }
    },
    myCurrentClientId : undefined,
    myDataSet : {
      // "fake1" : [{
      //   type : "IN",
      //   ams  : Date.now(),
      //   time : WST.formatDate(new Date()),
      //   raw  : "abc",
      //   hex  : false, 
      //   str  : "hello", 
      //   strDisplay : "hello", 
      //   hexCode : "A0BBCCDEFA"
      // }]
    }
  }),
  props: {
    wsHost : {
      type : String,
      default : "//netlab.vue2.cn/ws/netlab"
    }
  },
  computed: {
    TopClass() {
      return {
        "use-hex" : this.myUseHex,
        "use-nl"  : this.myUseNL
      }
    },
    ConnectIcon() {
      return ({
        CONNECTED : "fas fa-signal",
        CLOSED : "fab fa-deezer",
        ERROR : "fas fa-exclamation-triangle",
      })[this.myStat]
    },
    isConnected() { return "CONNECTED" == this.myStat },
    isClosed() { return "CLOSED" == this.myStat },
    isError() { return "ERROR" == this.myStat },
    hasClients() {return !_.isEmpty(this.TheClientList)},
    isCanSendData() {
      if(!this.myCurrentClientId) {
        return false
      }
      return this.isConnected
    },
    UseHexTip() {
      return this.myUseHex
        ? this.i18n['wst-use-hex-on-tip']
        : this.i18n['wst-use-hex-off-tip']
    },
    UseNLTip() {
      return this.myUseNL
        ? this.i18n['wst-use-nl-on-tip']
        : this.i18n['wst-use-nl-off-tip']
    },
    EchoBackTip() {
      return this.myEchoBack
        ? this.i18n['wst-use-eb-on-tip']
        : this.i18n['wst-use-eb-off-tip']
    },
    ShowLineHexTip() {
      return this.myShowLineHex
        ? this.i18n['wst-show-hex-on-tip']
        : this.i18n['wst-show-hex-off-tip']
    },
    LogOrderIcon() {
      if('ASC' == this.myLogOrder) {
        return "fas fa-sort-amount-down-alt"
      }
      return "fas fa-sort-amount-up"
    },
    LogOrderTip() {
      if('ASC' == this.myLogOrder) {
        return this.i18n['wst-log-asc-tip']
      }
      return this.i18n['wst-log-desc-tip']
    },
    TheClientList() {
      let list = []
      _.forEach(this.myClients, it => {
        let li = _.cloneDeep(it)
        let m = /\/?([^:]+)(:(\d+))?/.exec(li.addr)
        if(m) {
          li.addrHost = _.trim(m[1])
          li.addrPort = _.trim(m[3])
        }
        li.data = _.get(this.myDataSet, it.clientId) || []
        li.current = this.myCurrentClientId == it.clientId
        li.className = {
          "is-current" : li.current,
          "is-connected" : li.connected
        }
        list.splice(0, 0, li)
      })
      return list
    },
    CurrentClientData() {
      return _.get(this.myDataSet, this.myCurrentClientId) || []
    }
  },
  methods : {
    OnCloseClient({clientId}) {
      this.send({
        action : "closec",
        client : clientId
      })
    },
    OnSendMsg() {
      if(!this.isCanSendData) {
        return
      }
      let str = _.trim(this.$refs.input.value)
      if(!str)
        return
      console.log(str)
      if(this.myUseNL) {
        if(this.myUseHex) {
          str += "0D0A"
        } else {
          str += "\r\n"
        }
      }
      let msg = {
        "action" : "sendc",
        "data"   : str,
        "hex"    : this.myUseHex,
        "client" : this.myCurrentClientId
      }
      this.send(msg)
      this.pushToCurrentData({
        type : "OUT",
        client : this.myCurrentClientId,
        data: str,
        hex: this.myUseHex
      })
      this.$refs.input.value = null
    },
    OnClickClosed() {
      console.log("hahah")
      if(this.isConnected) {
        this.$WEBS.close()
      }
    },
    send(obj) {
      if(!this.isConnected) {
        console.log("未连接，不能发消息！")
        return
      }
      let data = obj
      if(!_.isString(obj)) {
        data = JSON.stringify(obj)
      }
      console.log(">> send: ", data)
      this.$WEBS.send(data)
    },
    OnClickConnect(type="tcp") {
      // 如果之前已经建立了连接
      if(!this.isCanMakeNewWebSocketObj()) {
        return
      }
      // 建立新连接
      this.$WEBS = WST.connect({
        host: this.wsHost,
        onopen : re => {
          console.log("ws:open:", re)
          this.myStat = "CONNECTED"
          // 首条消息，申请新端口
          this.send({action: "newp", type})
          // 启动心跳
          this.startHeartBeat()
        },
        onmessage : ({data}={}) => {
          let reo = JSON.parse(data)
          console.log("ws:msg:", reo)
          this.dispatchAction(reo)
        },
        onclose : re => {
          console.warn("ws:close:", re)
          this.$WEBS = undefined
          this.myStat = "CLOSED"
          this.myClientPort = undefined
          this.stopHeartBeat()
        },
        onerror : re => {
          console.error("ws:error:", re)
          this.myStat = "ERROR"
        }
      })
    },
    dispatchAction(at={}) {
      let {action} = at
      let fn = ({
        port : ({port})=>{
          this.myClientPort = port
        },
        connected : ({client, addr})=>{
          _.set(this.myClients, client, {
            clientId  : client, 
            addr,
            connected : true
          })
          this.myCurrentClientId = client
        },
        closed : ({client})=>{
          _.set(this.myClients, `${client}.connected`, false)
        },
        data : ({client, data, hex})=>{
          this.pushToCurrentData({
            type : "IN",
            client, data, hex
          })

          // 自动回显
          if(this.myEchoBack) {
            let msg = {
              "action" : "sendc",
              "data"   : data,
              "hex"    : hex,
              "client" : client
            }
            this.send(msg)
            this.pushToCurrentData({
              type : "OUT",
              client, data, hex
            })
          }
        },
        error : ({msg})=> {
          this.myErrMsg = msg
        }
      })[action]

      if(_.isFunction(fn)) {
        fn.apply(this, [at])
      }
    },
    pushToCurrentData({type="IN", client, data, hex}) {
      console.log(data)
      let list = _.get(this.myDataSet, client)
      if(!_.isArray(list)) {
        list = []
        this.myDataSet[client] = list
      }
      let str, hexCode;
      // 输入的 data 是 hex，试图转一下
      if(hex) {
        hexCode = data
        try {
          str = WST.decodeUtf8(data)
          hex = false
        } catch (E) {
          console.warn("Fail to decodeUtf8", data)
        }
      }
      // 那么输入的就是普通字符串咯
      else {
        str = data
        hexCode = WST.encodeUtf8(data)
      }

      // Str display
      let regex = /((.+)|(\r?\n))/g
      let strDisplay = ""
      if(str) {
        let m = regex.exec(str)
        while(m) {
          let s = m[2]
          let nl = m[3]
          if(s) {
            strDisplay += s
          }
          else if(nl) {
            strDisplay += "⬅️\n"
          }
          m = regex.exec(str)
        }
      }

      let now = new Date()
      let logIt = {
        type,
        ams  : now.getTime(),
        time : WST.formatDate(now),
        raw  : data,
        hex, str, hexCode,
        strDisplay
      }

      // 插入
      if('ASC' == this.myLogOrder) {
        list.push(logIt)
      } else {
        list.splice(0, 0, logIt)
      }

      // 正序的话，需要自动滚屏
      if('ASC' == this.myLogOrder) {
        this.$nextTick(()=>{
          this.scrollLogToBottom()
        })
      }
    },
    sortLogData() {
      _.forEach(this.myDataSet, (list, k)=> {
        if(_.isEmpty(list)) {
          return
        }
        // 正序
        if('ASC' == this.myLogOrder) {
          list.sort((v1, v2) => {
            return v1.ams - v2.ams
          })
        }
        // 倒序 
        else {
          list.sort((v1, v2) => {
            return v2.ams - v1.ams
          })
        }
      })
    },
    scrollLogToBottom() {
      let $log = this.$refs.log
      if(_.isElement($log)) {
        $log.scrollTop = $log.scrollHeight
      }
    },
    isCanMakeNewWebSocketObj() {
      if(!this.$WEBS)
        return true
      let stat = this.$WEBS.readyState 
      // 0 (WebSocket.CONNECTING)
      if( 0 == stat) {
        alert("正在建立连接中，请稍候 ...")
        return false
      }
      if( 1 == stat) {
        alert("连接已建立，不能重复连接！")
        return false
      }
      if( 2 == stat) {
        alert("正在关闭连接中，请稍候 ...")
        return false
      }
      if( 3 == stat) {
        this.$WEBS = undefined
      }
      return true
    },
    saveSettings() {
      let json = JSON.stringify({
        useHex : this.myUseHex,
        useNL  : this.myUseNL,
        showLineHex : this.myShowLineHex,
        echoBack : this.myEchoBack,
        logOrder : this.myLogOrder
      })
      localStorage.setItem("LUATOS_WST_SETTINGS", json)
    },
    startHeartBeat() {
      this.HBI = window.setInterval(()=>{
        this.myLastHB = WST.formatDate(new Date())
        this.send({})
      }, 30000)
    },
    stopHeartBeat() {
      if(this.HBI) {
        window.clearInterval(this.HBI)
        this.HBI = null
      }
    }
  },
  watch : {
    "myUseHex" : "saveSettings",
    "myUseNL" : "saveSettings",
    "myShowLineHex" : "saveSettings",
    "myEchoBack" : "saveSettings",
    "myLogOrder" : function(){
      this.saveSettings()
      this.sortLogData()
    },
  },
  mounted() {
    let json = localStorage.getItem("LUATOS_WST_SETTINGS") || "{}"
    let settings = JSON.parse(json)
    _.defaults(settings, {
      useHex : false,
      useNL  : false,
      showLineHex : true,
      echoBack : false,
      logOrder : "ASC"
    })
    this.myUseHex = settings.useHex
    this.myUseNL = settings.useNL
    this.myShowLineHex = settings.showLineHex
    this.myEchoBack = settings.echoBack
    this.myLogOrder = settings.logOrder
  },
  beforeDestroied() {
    this.OnClickClosed()
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped  lang="scss" src="../css/main.module.scss"></style>
