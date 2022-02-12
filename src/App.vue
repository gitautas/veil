<template>
  <Player/>
</template>

<script lang="ts">
import { Options, Vue } from 'vue-class-component';
import Player from './components/Player.vue';

@Options({
  components: {
    Player,
  },
})
export default class App extends Vue {}
</script>

<style lang="scss">
 body {
   margin: 0;
   overflow: hidden;
 }
 #app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: #2c3e50;
  margin: 0;
 }
</style>
