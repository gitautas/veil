/* eslint-disable @typescript-eslint/no-explicit-any */
// The above is due to peerConnection using weird Event types that aren't actually Events.
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
    this.getCandidates();

    console.log(this.peerConnection);
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
    console.log("GET /candidate");
    fetch(this.ymirAddr + "/candidate", {
      method: "GET",
    }).then(async (response) => {
      console.log(response);
      if (response.status == 102) {
        this.getCandidates();
      }

      if (response.status == 100) {
        const candidate: RTCIceCandidate = await response.json();
        this.peerConnection.addIceCandidate(candidate);
        this.getCandidates();
      }
    });
  }

  constructor(address: string) {
    this.ymirAddr = address;
    this.remoteStream = new MediaStream();
    this.peerConnection = new RTCPeerConnection({
      iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
    });
    if (this.peerConnection) {
      this.peerConnection.addEventListener(
        "connectionstatechange",
        (event: Event) => {
          console.log(`Connection state changed to ${event}`);
        }
      );

      this.peerConnection.addEventListener(
        "onconnectionstatechange",
        (event: Event) => {
          console.log(`Connection state changed to ${event}`);
        }
      );

      this.peerConnection.addEventListener(
        "oniceconnectionstatechange",
        (event: any) => {
          console.log(`ICE connection state changed to ${event.state}`);
          if (this.peerConnection.iceConnectionState == "checking") {
            this.getCandidates();
          }
        }
      );

      this.peerConnection.addEventListener("track", (event: RTCTrackEvent) => {
        this.remoteStream.addTrack(event.track);
      });

      this.peerConnection.addEventListener(
        "icecandidate",
        (event: RTCPeerConnectionIceEvent) => {
          if (event.candidate != null) {
            this.sendIceCandidate(event.candidate);
          }
        }
      );

      this.peerConnection.addEventListener(
        "datachannel",
        (event: RTCDataChannelEvent) => {
          this.dataChannel = event.channel;
        }
      );
    }
  }
}
