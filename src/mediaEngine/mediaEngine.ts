export default class MediaEngine {
    ymirAddr: string
    peerConnection: RTCPeerConnection
    dataChannel!: RTCDataChannel
    remoteStream: MediaStream

    public async negotiate(): Promise<void> {
        const offer: RTCSessionDescriptionInit = await this.getOffer()
        this.peerConnection.setRemoteDescription(offer)

        const answer: RTCSessionDescriptionInit = await this.peerConnection.createAnswer()
        this.peerConnection.setLocalDescription(answer)

        this.sendAnswer(answer)

        this.peerConnection.addEventListener("iceconnectionstatechange", () => {
            if (this.peerConnection.iceConnectionState == "checking") {
                this.getCandidates()
            }
        })

        this.peerConnection.addEventListener("connectionstatechange", () => {
            if (this.peerConnection.connectionState == "connected") {
                console.log("Connected!")
            }
        })

        this.peerConnection.addEventListener("track", (event) => {
            this.remoteStream.addTrack(event.track)
        })

        this.peerConnection.addEventListener("icecandidate", (event) => {
            if (event.candidate != null) {
                this.sendIceCandidate(event.candidate)
            }
        })

        this.peerConnection.addEventListener("datachannel", (event) => { //TODO: make sure this doesn't fire when constructing the class
            this.dataChannel = event.channel
        })
    }

    private async getOffer(): Promise<RTCSessionDescriptionInit> {
        return fetch(this.ymirAddr + "/offer", {
            method: "GET",
        }).then((response) => {
            return response.json()
        })
    }

    private async sendAnswer(offer: RTCSessionDescriptionInit): Promise<void> {
        fetch(this.ymirAddr + "/offer", {
            method: "POST",
            body: JSON.stringify(offer)
        })
    }

    private async sendIceCandidate(candidate: RTCIceCandidate): Promise<void> {
        fetch(this.ymirAddr + "/candidate", {
            method: "POST",
            body: JSON.stringify(candidate)
        })
    }

    private async getCandidates(): Promise<void> {
        fetch(this.ymirAddr + "/candidate", {
            method: "GET",
        }).then(async response => {
            if (response.status > 400) {
                // error
            }
            const candidate: RTCIceCandidate = await response.json()
            this.peerConnection.addIceCandidate(candidate)
            if (response.status == 100) {
                this.getCandidates() // recurse until the server send out all candidates
            }
        })
    }

    constructor(address: string) {
        this.ymirAddr = address
        this.peerConnection = new RTCPeerConnection({ iceServers: [{ "urls": "stun:stun.l.google.com:19302" }] })
        this.remoteStream = new MediaStream()
    }
}
