/* eslint-disable @typescript-eslint/no-explicit-any */
// The above is due to peerConnection using weird Event types that aren't actually Events.
export default class MediaEngine {
  ymirAddr: string;
  peerConnection: RTCPeerConnection;
  dataChannel!: RTCDataChannel;
  player: HTMLVideoElement;

  public async negotiate(): Promise<void> {
    const offer: RTCSessionDescriptionInit = await this.getOffer();
    console.log(offer.sdp);
    await this.peerConnection.setRemoteDescription(offer).then(() => {
      console.log("I HAVE ADDED THE FUCKING REMOTE DESCRIPTION");
    });

    const answer: RTCSessionDescriptionInit =
      await this.peerConnection.createAnswer();
    console.log(answer.sdp);
    await this.peerConnection.setLocalDescription(answer).then(() => {
      console.log("I HAVE ADDED THE FUCKING LOCAL DESCRIPTION");
    });

    await this.sendAnswer(answer);
    this.getCandidates();
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
    console.log("Getting candidates");
    fetch(this.ymirAddr + "/candidate", {
      method: "GET",
    }).then(async (response: Response) => {
      console.log("Got response for GET candidate:" + response.status);
      switch (response.status) {
        case 204: {
          await new Promise((r) => setTimeout(r, 1000));
          await this.getCandidates();
          break;
        }
        case 202: {
          const candidate: RTCIceCandidate = await response.json();
          console.log(candidate);
          await this.peerConnection.addIceCandidate(candidate);
          console.log("Added candidate");
          await this.getCandidates();
          break;
        }
      }
    });
  }

  constructor(address: string, player: HTMLVideoElement) {
    this.ymirAddr = address;
    this.player = player;
    this.peerConnection = new RTCPeerConnection({
      iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
    });
    if (this.peerConnection) {
      this.peerConnection.addEventListener("connectionstatechange", () => {
        console.log(
          `Connection state changed to ${this.peerConnection.connectionState}`
        );
      });

      this.peerConnection.addEventListener("iceconnectionstatechange", () => {
        console.log(
          `ICE connection state changed to ${this.peerConnection.iceConnectionState}`
        );
      });

      this.peerConnection.addEventListener(
        "oniceconnectionstatechange",
        (event: any) => {
          console.log(`ICE connection state changed to ${event.state}`);
          if (this.peerConnection.iceConnectionState == "checking") {
            this.getCandidates();
          }
        }
      );

      this.peerConnection.addEventListener(
        "track",
        async (event: RTCTrackEvent) => {
          const player: HTMLVideoElement | null =
            document.querySelector("#mainPlayer");
          if (player == null) {
            console.log("Player doesn't exist yet :(");
            return;
          }

          console.log(player);
          const [remote] = event.streams;
          player.srcObject = remote;
          // this.remoteStream
          // this.remoteStream.addTrack(event.track);
        }
      );

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
