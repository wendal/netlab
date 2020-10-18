<template>
  <div class="wstool">
    <h1>LuatOS 网络测试工具</h1>
    <button @click="OnClickTest">连接服务器</button>
  </div>
</template>

<script>
import WST from "../support/util.js"

export default {
  name: 'WstMain',
  props: {
    msg: String
  },
  methods : {
    OnClickTest() {
      WST.abc();
    }
  }
}
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped>
h3 {
  margin: 40px 0 0;
}
ul {
  list-style-type: none;
  padding: 0;
}
li {
  display: inline-block;
  margin: 0 10px;
}
a {
  color: #42b983;
}
</style>
