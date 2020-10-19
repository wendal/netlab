<template>
  <div class="wstool"
    :class="TopClass">
    <header>
      <h1>
        LuatOS 网络测试工具 【{{myStat}}】
        <em v-if="myClientPort>0">{{myClientPort}}</em>
      </h1>
      <aside>
        <button v-if="isClosed" @click="OnClickConnect">连接服务器</button>
        <button v-else @click="OnClickClosed">断开连接</button>
      </aside>
    </header>
    <pre class="as-error">{{myErrMsg}}</pre>
    <section>
      <nav>
        <ul>
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
          <p v-for="(line, index) of CurrentClientData"
            :key="index"
            :class="`as-type-${line.type}`">
            <em>[{{line.time}}]</em>
            <u>[{{line.type}}]</u>
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
    // CONNECTED | CLOSED | ERROR
    myStat : "CLOSED",
    myUseHex : false,
    myUseNL : false,
    myClientPort : undefined,
    myErrMsg : undefined,
    myClients : {
      "a7305652" : {
        clientId: "a7305652",
        addr : "/119.130.132.35:48905",
        connected: true
      }
    },
    myCurrentClientId : "a7305652",
    myDataSet : {
      "a7305652" : [{
        "type" : "IN",
        "time": "2020-10-19 16:59:46", 
        "raw": "30383637383134303436343336323535233238380D0A", 
        "hex": true, 
        "str": "0867814046436255#288\r\n" 
      }, { 
        "type" : "OUT",
        "time": "2020-10-19 16:59:47", 
        "raw": "30383637383134303436343336323535233238380D0A", 
        "hex": true, "str": "0867814046436255#288\r\n" 
      }, { 
        "type" : "OUT",
        "time": "2020-10-19 16:59:47", 
        "raw": "30383637383134303436343336323535233238380D0A", 
        "hex": true, "str": "0867814046436255#288\r\n"
      }]
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
    isConnected() { return "CONNECTED" == this.myStat },
    isClosed() { return "CLOSED" == this.myStat },
    isError() { return "ERROR" == this.myStat },
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
      let data = _.get(this.myDataSet, this.myCurrentClientId)
      return (data || []).reverse()
    }
  },
  methods : {
    OnSendMsg() {
      let str = _.trim(this.$refs.input.value)
      console.log(str)
      let msg = {
        "action" : "sendc",
        "data"   : str,
        "hex"    : false,
        "client" : this.myCurrentClientId
      }
      this.send(msg)
      this.pushToCurrentData({
        type : "OUT",
        client : this.myCurrentClientId,
        data:msg,
        hex:false
      })
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
      this.$WEBS.send(data)
    },
    OnClickConnect() {
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
          this.send({action: "newp"})
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
      let list = _.get(this.myDataSet, client) || []
      list.push({
        type,
        time : WST.formatDate(new Date()),
        raw  : data,
        hex,
        str : hex ? WST.decodeUtf8(data) : data
      })
      _.set(this.myDataSet, client, list)
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
    }
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped  lang="scss" src="../css/main.module.scss"></style>
