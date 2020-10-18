<template>
  <WstMain/>
</template>

<script>
import WstMain from './components/WstMain.vue'

export default {
  name: 'App',
  components: {
    WstMain
  }
}
</script>

<style>
body {
  margin:0; padding:0; font-size: 14px;
}
button {
  font-size: inherit; padding: .5em 1em;
}
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: #2c3e50;
  margin: 0;
  padding: 20px;
}
</style>
