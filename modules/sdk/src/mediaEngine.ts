import { EventEmitter } from "events";

export interface MediaEngineConfig {
  peerConnection: RTCConfiguration;
}

export class MediaEngine extends EventEmitter {
  private peerConnection: RTCPeerConnection;

  constructor(config: MediaEngineConfig) {
    super();
    this.peerConnection = new RTCPeerConnection(config.peerConnection);
  }

  public async createAnswer(
    offer: RTCSessionDescriptionInit,
  ): Promise<RTCSessionDescriptionInit> {
    await this.peerConnection.setRemoteDescription(offer);

    const answer = await this.peerConnection.createAnswer();
    await this.peerConnection.setLocalDescription(answer);

    return answer;
  }
}
