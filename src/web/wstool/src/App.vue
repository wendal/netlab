<template>
  <WstMain/>
</template>

<script>
import style from './css/app.module.scss'
import WstMain from './components/WstMain.vue'

export default {
  name: 'App',
  components: {
    WstMain
  }
}
</script>

<style>
#app {
  padding: 0px; width:100%; height:100%
}
::-webkit-scrollbar {
  width: 8px; height: 8px;
}
::-webkit-scrollbar-track {
  background: #000;
}
::-webkit-scrollbar-thumb {
  background: #464;
}
::-webkit-scrollbar-thumb:hover {
  background: #0F0;
}
* {
  scrollbar-width: thin;
  scrollbar-color: #000 #494;
}
</style>
