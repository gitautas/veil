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

async function getICECandidate() {
  let response = await fetch(url + "candidate", {
    method: "GET",
  })

  let candidate = await response.json()

  return candidate
}

async function sendCandidate(candidate) {
  fetch(url + "candidate", {
    method: "POST",
    body: candidate
  })
}





const configuration = { 'iceServers': [{ 'urls': 'stun:stun.l.google.com:19302' }] }

const peerConnection = new RTCPeerConnection(configuration)

getOffer().then((offer) => {
  console.log(offer)
  peerConnection.setRemoteDescription(offer).then(
    peerConnection.createAnswer().then((answer) => {
      console.log(answer)
      sendAnswer(answer.sdp)
    })
  )
})

// Listen for local ICE candidates on the local RTCPeerConnection
peerConnection.addEventListener('icecandidate', event => {
  console.log(`Found new ice candidate`)
  if (event.candidate) {
    sendCandidate(event.candidate)
  }
});

peerConnection.addEventListener('connectionstatechange', () => {
  console.log(`Connection state changed to ${peerConnection.connectionState}`)
});

async () => {
  for (; ;) {
    if (peerConnection.iceGatheringState != "complete") {
      candidate = await getICECandidate()
      peerConnection.addIceCandidate(candidate)
    } else {
      break
    }
  }
}
