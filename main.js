const url = "http://127.0.0.1:8080/"

async function getOffer() {
  let response = await fetch(url + "offer", {
    method: "GET",
    mode: "cors"
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
    mode: "cors",
    body: answer
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
  if (event.candidate) {
    signalingChannel.send({ 'new-ice-candidate': event.candidate });
  }
});

peerConnection.addEventListener('connectionstatechange', event => {
  if (peerConnection.connectionState === 'connected') {
    console.log("HOLY SHIT WE'RE IN")
  }
});
