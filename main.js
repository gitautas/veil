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

// Listen for local ICE candidates on the local RTCPeerConnection
peerConnection.addEventListener('icecandidate', event => {
  if (event.candidate) {
    console.log("Found new ice candidate")
    console.log(event.candidate)
    sendCandidate(event.candidate)
    if (peerConnection.iceConnectionState != "completed") {
      getICECandidate(peerConnection)
    }
  }
});

peerConnection.addEventListener('connectionstatechange', state => {
  console.log(state)
  console.log(`Connection state changed to ${peerConnection.connectionState}`)
});

getOffer().then((offer) => {
  console.log(offer)
  peerConnection.setRemoteDescription(offer).then(
    peerConnection.createAnswer().then((answer) => {
      peerConnection.setLocalDescription(answer)
      console.log(answer)
      sendAnswer(answer.sdp)
      console.log(peerConnection)
    })
  )
})
