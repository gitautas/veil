

const url = "http://127.0.0.1:8080/"

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

peerConnection.addEventListener('connectionstatechange', state => {
  console.log(`Connection state changed to ${peerConnection.connectionState}`)
});

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

const remoteStream = new MediaStream();
const remoteVideo = document.querySelector('#video');
remoteVideo.srcObject = remoteStream;
console.log("Added remote source")

// Gamepad stuff

class SimpleGP {
  buttons = []
  axes = []

  constructor (buttons, axes) {
    this.buttons = buttons.map(e => e.pressed)
    this.axes = axes.map(e => e.toFixed(2))
  }

  compareButtons (old) {
    this.buttons.map((v, i) => {
      if (v != old.buttons[i]) {
        console.log(`Button ${i}: ${v}`)
      }
    })
  }

  compareAxes (old) {
    this.axes.map((v, i) => {
      if (Math.abs(v) > 0.1 && v != old.axes[i]) {
        console.log(`Axis ${i}: ${v}`)
      }
    })
  }

  compare (old) {
    this.compareAxes(old)
    this.compareButtons(old)
  }
}

const POLL_RATE = 250 // Hz
const pollInterval = 1 / POLL_RATE * 1000

window.addEventListener("gamepadconnected", (e) => {
  console.log(`Gamepad ${e.gamepad.id} connected!`)

  let ogp = navigator.getGamepads()[e.gamepad.index]
  let oldGP = new SimpleGP(ogp.buttons, ogp.axes)

  setInterval(() => {
    let gamep = navigator.getGamepads()[e.gamepad.index]
    var GP = new SimpleGP(gamep.buttons, gamep.axes)

    if (JSON.stringify(oldGP) != JSON.stringify(GP)) {
      GP.compare(oldGP)
    }

    oldGP = GP // This

  }, pollInterval * 10)
})
