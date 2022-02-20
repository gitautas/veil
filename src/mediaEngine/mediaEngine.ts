export default class MediaEngine {
  ymirAddr: string;
  peerConnection: RTCPeerConnection;
  dataChannel!: RTCDataChannel;
  remoteStream: MediaStream;

  public async negotiate(): Promise<void> {
    const offer: RTCSessionDescriptionInit = await this.getOffer();
    console.log(offer.sdp);
    await this.peerConnection.setRemoteDescription(offer);

    const answer: RTCSessionDescriptionInit =
      await this.peerConnection.createAnswer();
    console.log(answer.sdp);
    await this.peerConnection.setLocalDescription(answer);

    await this.sendAnswer(answer);
  }

  private async getOffer(): Promise<RTCSessionDescriptionInit> {
    return fetch(this.ymirAddr + "/offer", {
      method: "GET",
    }).then((response) => {
      return response.json();
    });
  }

  private async sendAnswer(offer: RTCSessionDescriptionInit): Promise<void> {
    fetch(this.ymirAddr + "/answer", {
      method: "POST",
      body: JSON.stringify(offer),
    });
  }

  private async sendIceCandidate(candidate: RTCIceCandidate): Promise<void> {
    fetch(this.ymirAddr + "/candidate", {
      method: "POST",
      body: JSON.stringify(candidate),
    });
  }

  private async getCandidates(): Promise<void> {
    fetch(this.ymirAddr + "/candidate", {
      method: "GET",
    }).then(async (response) => {
      if (response.status > 400) {
        // error
      }
      const candidate: RTCIceCandidate = await response.json();
      this.peerConnection.addIceCandidate(candidate);
      if (response.status == 100) {
        this.getCandidates(); // recurse until the server send out all candidates
      }
    });
  }

  constructor(address: string) {
    this.ymirAddr = address;
    this.remoteStream = new MediaStream();
    const peerConnection = new RTCPeerConnection({
      iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
    });

    peerConnection.addEventListener("iceconnectionstatechange", (state) => {
      console.log(`ICE connection state changed to ${state}`);
      if (this.peerConnection.iceConnectionState == "checking") {
        this.getCandidates();
      }
    });

    peerConnection.addEventListener("connectionstatechange", (state) => {
      console.log(`Connection state changed to ${state}`);
    });

    peerConnection.addEventListener("track", (event) => {
      this.remoteStream.addTrack(event.track);
    });

    peerConnection.addEventListener("icecandidate", (event) => {
      if (event.candidate != null) {
        this.sendIceCandidate(event.candidate);
      }
    });

    peerConnection.addEventListener("datachannel", (event) => {
      //TODO: make sure this doesn't fire when constructing the class
      console.log("reee");
      this.dataChannel = event.channel;
    });

    this.peerConnection = peerConnection;
  }
}
