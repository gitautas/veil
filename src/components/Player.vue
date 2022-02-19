<template>
  <video autoplay muted />
</template>

<script lang="ts">
import { Options, Vue } from 'vue-class-component'
import MediaEngine from '../mediaEngine/mediaEngine'
import VeilGamepad from '../gamepad/gamepad'

let mediaEngine = new MediaEngine("http://192.168.1.105:4000")
console.log(mediaEngine)
 mediaEngine.negotiate().catch((err) => {
     console.log(err)
     console.log(mediaEngine)
 });

 window.addEventListener("gamepadconnected", (event: any) => {
     console.log("Gamepad connected");
     let gp = new VeilGamepad(event.gamepad, mediaEngine.dataChannel)
     gp.poll()

 })

 @Options({
     props: {
  },
})
export default class Player extends Vue {
}
</script>

<style scoped lang="scss">
 video {
   width:100vw;
   height:100vh;
 }
</style>
