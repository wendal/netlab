<template>
  <div class="wstool"
    :class="TopClass">
    <header>
      <h1>
        LuatOS 网络测试工具 【{{myStat}}】
        <em v-if="myClientPort>0">{{myClientPort}}</em>
        <u>{{myLastHB}}</u>
      </h1>
      <aside>
        <template v-if="isClosed">
          <button @click="OnClickConnect('tcp')">申请(TCP)</button>
          <button @click="OnClickConnect('udp')">申请(UDP)</button>
        </template>
        <button v-else @click="OnClickClosed">断开连接</button>
      </aside>
    </header>
    <pre class="as-error">{{myErrMsg}}</pre>
    <section>
      <nav>
        <ul v-if="hasClients">
          <li v-for="cl in TheClientList"
            :key="cl.clientId"
            :class="cl.className"
            @click.left="myCurrentClientId=cl.clientId">
            <div>
              <span v-if="cl.current">&gt;</span>
              <span v-if="cl.connected">[ON]</span>
              <span v-else>[OF]</span>
              {{cl.clientId}}
            </div>
            <em>{{cl.addr}}</em>
          </li>
        </ul>
        <blockquote v-else>
          未侦测到连接的设备
        </blockquote>
      </nav>
      <main>
        <header>
          <div class="as-hex" @click.left="myUseHex=!myUseHex">HEX</div>
          <input placeholder="发送消息" spellcheck="fase"
            ref="input"
            @change="OnSendMsg"/>
          <div class="as-nl" @click.left="myUseNL=!myUseNL">添加换行</div>
        </header>
        <blockquote>
          <p v-for="line of CurrentClientData"
            :key="line.ams"
            :class="`as-type-${line.type}`">
            <em>[{{line.time}}]</em>
            <u>[{{line.type}}]</u>
            <u v-if="line.hex">HEX</u>
            <span>{{line.str}}</span>
          </p>
        </blockquote>
      </main>
    </section>
  </div>
</template>

<script>
import WST from "../support/util.js"
import _ from 'lodash'

export default {
  name : 'WstMain',
  data : ()=>({
    // 心跳的句柄
    HBI : undefined,
    myLastHB : undefined,
    // CONNECTED | CLOSED | ERROR
    myStat : "CLOSED",
    myUseHex : false,
    myUseNL : false,
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
    myDataSet : {}
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
    isConnected() { return "CONNECTED" == this.myStat },
    isClosed() { return "CLOSED" == this.myStat },
    isError() { return "ERROR" == this.myStat },
    hasClients() {return !_.isEmpty(this.TheClientList)},
    TheClientList() {
      let list = []
      _.forEach(this.myClients, it => {
        let li = _.cloneDeep(it)
        li.data = _.get(this.myDataSet, it.clientId) || []
        li.current = this.myCurrentClientId == it.clientId
        li.className = {
          "is-current" : li.current,
          "is-connected" : li.connected
        }
        list.push(li)
      })
      return list
    },
    CurrentClientData() {
      return _.get(this.myDataSet, this.myCurrentClientId) || []
    }
  },
  methods : {
    OnSendMsg() {
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
        hex:false
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
          if(!this.myCurrentClientId) {
            this.myCurrentClientId = client
          }
        },
        closed : ({client})=>{
          _.set(this.myClients, `${client}.connected`, false)
        },
        data : ({client, data, hex})=>{
          this.pushToCurrentData({
            type : "IN",
            client, data, hex
          })
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
      let list = _.get(this.myDataSet, client)
      if(!_.isArray(list)) {
        list = []
        this.myDataSet[client] = list
      }
      let str = data
      if(hex) {
        try {
          str = WST.decodeUtf8(data)
          hex = false
        } catch (E) {
          console.warn("Fail to decodeUtf8", data)
        }
      }
      let now = new Date()
      list.splice(0, 0, {
        type,
        ams  : now.getTime(),
        time : WST.formatDate(now),
        raw  : data,
        hex, str
      })
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
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped  lang="scss" src="../css/main.module.scss"></style>
