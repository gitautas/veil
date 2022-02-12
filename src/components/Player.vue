<template>
  <video autoplay muted>
    <source src="video.mp4">
  </video>
</template>

<script lang="ts">
import { Options, Vue } from 'vue-class-component'
import MediaEngine from '../mediaEngine/mediaEngine'
import VeilGamepad from '../gamepad/gamepad'

let mediaEngine = new MediaEngine("0")
console.log(mediaEngine)
 // mediaEngine.negotiate()

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
