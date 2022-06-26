<template>
  <video autoplay muted />
</template>

<script setup lang="ts">
import MediaEngine from "../mediaEngine/mediaEngine";
// import VeilGamepad from "../gamepad/gamepad";

// const mediaEngine = new MediaEngine("http://192.168.1.105:4000");
const mediaEngine = new MediaEngine("http://0.0.0.0:3002");
mediaEngine.negotiate().catch((err) => {
  console.log("errored");
  console.log(err);
});
console.log(mediaEngine);

// window.addEventListener("gamepadconnected", (event: GamepadEvent) => {
//   console.log("Gamepad connected");
//   let gp = new VeilGamepad(event.gamepad, mediaEngine.dataChannel);
//   gp.poll();
// });
</script>

<style scoped lang="sass">
video
  width: 100vw
  height: 100vh
</style>
