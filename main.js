

const url = "http://127.0.0.1:8080/"

const remoteStream = new MediaStream();
const remoteVideo = document.querySelector('#video');
remoteVideo.srcObject = remoteStream;

async function getOffer() {
  let response = await fetch(url + "offer", {
    method: "GET",
  })

  let sdp = await response.text()

  let offer = new RTCSessionDescription({
    sdp: sdp,
    type: "offer"
  })
  return offer
}

async function sendAnswer(answer) {
  fetch(url + "answer", {
    method: "POST",
    body: answer
  })
}

async function getICECandidate(peerConnection) {
  let response = await fetch(url + "candidate", {
    method: "GET",
  })

  let candidate = await response.json()

  peerConnection.addIceCandidate(candidate)
  if (peerConnection.ICEGatheringState != "complete")
    getICECandidate(peerConnection)
}

async function sendCandidate(candidate) {
  fetch(url + "candidate", {
    method: "POST",
    body: JSON.stringify(candidate)
  })
}

const configuration = { 'iceServers': [{ 'urls': 'stun:stun.l.google.com:19302' }] }

const peerConnection = new RTCPeerConnection(configuration)


peerConnection.addEventListener('track', async (event) => {
  remoteStream.addTrack(event.track, remoteStream);
  console.log("Added remote track")
});


// Listen for local ICE candidates on the local RTCPeerConnection
peerConnection.addEventListener('icecandidate', event => {
  if (event.candidate) {
    sendCandidate(event.candidate)
    if (peerConnection.iceConnectionState != "completed") {
      getICECandidate(peerConnection)
    }
  }
});

let dataChannel

peerConnection.addEventListener('connectionstatechange', state => {
  console.log(`Connection state changed to ${peerConnection.connectionState}`)
});

peerConnection.addEventListener('datachannel', (ev) => {
  console.log("Data channel open!")
  console.log(ev.channel)
  dataChannel = ev.channel
  dataChannel.addEventListener("open", () => {
    console.log("Data channel open!")
  });
})

getOffer().then((offer) => {
  console.log(offer.sdp)
  peerConnection.setRemoteDescription(offer).then(
    peerConnection.createAnswer().then((answer) => {
      peerConnection.setLocalDescription(answer)
      console.log(answer.sdp)
      sendAnswer(answer.sdp)
      console.log(peerConnection)
    })
  )
})



// Gamepad stuff
//
// This is all hacked together to achieve MVP as soon as possible, but basically the entire
// comparison logic is flawed and WILL drop inputs, this is only hacked together for testing
// also this doesn't check that the gamepad has standard mapping and just assumes that as fact.
// This will be rectified in the full rewrite after MVP state is achieved.



class SimpleGP { // Can't wait to redo this shit
  buttons = []
  axes = []

  constructor (buttons, axes) {
    this.buttons = buttons.map(e => e.pressed)
    this.axes = axes.map(e => e.toFixed(2))
  }

  compareButtons (old) {
    let str
    this.buttons.map((v, i) => {
      if (v != old.buttons[i]) {
        str = INPUTH[i] + ":" + (+ v)
      }
    })
    return str
  }

  compareAxes (old) {
    let str
    this.axes.map((v, i) => {
      if (Math.abs(v) > 0.1 && v != old.axes[i]) {
        str = AXES[i] + ":" + (+ v)
      }
    })
    return str
  }

  compare (old) {
    let data = this.compareAxes(old)
    if (!data) {
      return this.compareButtons(old)
    }
    return data
  }
}

const POLL_RATE = 250 // Hz
const pollInterval = 1 / POLL_RATE * 1000


window.addEventListener("gamepadconnected", (e) => {
  console.log(`Gamepad ${e.gamepad.id} connected!`)
  dataChannel.send(e.gamepad.id)

  let ogp = navigator.getGamepads()[e.gamepad.index]
  let oldGP = new SimpleGP(ogp.buttons, ogp.axes)

  setInterval(() => {
    let gamep = navigator.getGamepads()[e.gamepad.index]
    var GP = new SimpleGP(gamep.buttons, gamep.axes)

    data = GP.compare(oldGP)
    if (data) {
      console.log(data)
      dataChannel.send(data)
    }


    oldGP = GP // This

  }, pollInterval)
})


const AXES = {
  0: 0x00, //ABS_X
  1: 0x01, //ABS_Y
  2: 0x03, //ABS_RX
  3: 0x04, //ABS_RY
}


const INPUTH = {
  // Face buttons
  0: 0x130, //BTN_SOUTH
  1: 0x131, //BTN_EAST
  2: 0x134, //BTN_WEST
  3: 0x133, //BTN_NORTH

  4: 0x136, //BTN_TL
  5: 0x137, //BTN_TR
  6: 0x138, //BTN_TL2
  7: 0x139, //BTN_TR2

  8: 0x13a, //BTN_SELECT
  9: 0x13b, //BTN_START

  10: 0x13d, //BTN_THUMBL
  11: 0x13e, //BTN_THUMBR

  12: 0x220, //BTN_DPAD_UP
  13: 0x221, //BTN_DPAD_DOWN
  14: 0x222, //BTN_DPAD_LEFT
  15: 0x223, //BTN_DPAD_RIGHT

  16: 0x13c, //BTN_MODE TODO: double check if that's what it actually evals to, doing this on a mac
}
