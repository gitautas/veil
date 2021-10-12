

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

class SimpleGP {
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
        str = `Button ${i}: ${v}`
      }
    })
    return str
  }

  compareAxes (old) {
    let str
    this.axes.map((v, i) => {
      if (Math.abs(v) > 0.05 && v != old.axes[i]) {
        str = `Axis ${i}: ${v}`
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

