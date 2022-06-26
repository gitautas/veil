package media

import (
	"github.com/pion/interceptor"
	"github.com/pion/webrtc/v3"
	"fmt"
)

const (
	H264ClockRate = 90000
	H264PayloadType = 125
	OpusClockRate = 48000
	OpusPayloadType = 111
)

type VeilEngine struct {
	localCandidates    []*webrtc.ICECandidate
	PeerConnection     *webrtc.PeerConnection
	videoSender        *webrtc.RTPSender
	audioSender        *webrtc.RTPSender
	gamepadChan        *webrtc.DataChannel
}

func NewVeilEngine(config webrtc.Configuration) (*VeilEngine, error) {
	mediaEngine := &webrtc.MediaEngine{}

	err := mediaEngine.RegisterCodec(webrtc.RTPCodecParameters{
		RTPCodecCapability: webrtc.RTPCodecCapability{
			MimeType:     webrtc.MimeTypeH264,
			ClockRate:    90000,
			SDPFmtpLine:  "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=4d001f",
			RTCPFeedback: nil,
		},
		PayloadType:        H264PayloadType,
	}, webrtc.RTPCodecTypeVideo)
	err = mediaEngine.RegisterCodec(webrtc.RTPCodecParameters{
		RTPCodecCapability: webrtc.RTPCodecCapability{MimeType: webrtc.MimeTypeOpus, ClockRate: 48000, Channels: 2, RTCPFeedback: nil},
		PayloadType:        OpusPayloadType,
	}, webrtc.RTPCodecTypeAudio)

	if err != nil {
		return nil, err
	}

	interceptor := &interceptor.Registry{}
	if err := webrtc.RegisterDefaultInterceptors(mediaEngine, interceptor); err != nil {
		return nil, err
	}

	api := webrtc.NewAPI(webrtc.WithMediaEngine(mediaEngine), webrtc.WithInterceptorRegistry(interceptor))

	peerConnection, err := api.NewPeerConnection(config)
	if err != nil {
		return nil, err
	}

	gamepadChan, err := peerConnection.CreateDataChannel("gamepad", nil)
	if err != nil {
		return nil, err
	}

	videoTrack, err := webrtc.NewTrackLocalStaticSample(webrtc.RTPCodecCapability{
		MimeType:  webrtc.MimeTypeH264,
		ClockRate: H264ClockRate,
		SDPFmtpLine: "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42001f",
	}, "video", "veil")
	if err != nil {
		return nil, err
	}

	audioTrack, err := webrtc.NewTrackLocalStaticSample(webrtc.RTPCodecCapability{
		MimeType:  webrtc.MimeTypeOpus,
		ClockRate: OpusClockRate,
	}, "audio", "veil")
	if err != nil {
		return nil, err
	}

	videoSender, err := peerConnection.AddTrack(videoTrack)
	if err != nil {
		return nil, err
	}

	audioSender, err := peerConnection.AddTrack(audioTrack)
	if err != nil {
		return nil, err
	}

	localCandidates := []*webrtc.ICECandidate{}

	go func() {
		rtcpVideo := make([]byte, 1500)
		rtcpAudio := make([]byte, 1500)
		for {
			if _, _, rtcpErr := videoSender.Read(rtcpVideo); rtcpErr != nil {
				return
			}
			if _, _, rtcpErr := audioSender.Read(rtcpAudio); rtcpErr != nil {
				return
			}
		}
	}()

	ve := &VeilEngine{
		localCandidates:    localCandidates,
		PeerConnection:     peerConnection,
		videoSender:        videoSender,
		audioSender:        audioSender,
		gamepadChan:        gamepadChan,
	}

	ve.PeerConnection.OnConnectionStateChange(func(state webrtc.PeerConnectionState) {
		fmt.Println("Connection state changed: " + state.String())
		if state == webrtc.PeerConnectionStateConnected {
			Sigrun(videoTrack)
		}
	})


	ve.PeerConnection.OnICECandidate(func(candidate *webrtc.ICECandidate) {
		fmt.Println("Found a candidate")
		ve.localCandidates = append(ve.localCandidates, candidate)
	})

	return ve, nil
}

func (ve *VeilEngine) GenerateOffer() (webrtc.SessionDescription, error) {
	offer, err := ve.PeerConnection.CreateOffer(&webrtc.OfferOptions{
		OfferAnswerOptions: webrtc.OfferAnswerOptions{
			VoiceActivityDetection: false,
		},
		ICERestart: false,
	})
	if err != nil {
		return webrtc.SessionDescription{}, err
	}

	err = ve.PeerConnection.SetLocalDescription(offer)
	if err != nil {
		return webrtc.SessionDescription{}, err
	}
	fmt.Println("I HAVE ADDED THE FUCKING LOCAL DESCRIPTION")
	return offer, err
}

func (ve *VeilEngine) SetAnswer(answer webrtc.SessionDescription) error {
	err := ve.PeerConnection.SetRemoteDescription(answer)
	if err != nil {
		return err
	}
	fmt.Println("I HAVE ADDED THE FUCKING REMOTE DESCRIPTION")
	return nil
}

func (ve *VeilEngine) GetCandidate() (*webrtc.ICECandidate, error) {
	if ve.CandidatesLen() > 0 {
		candidate := ve.localCandidates[0]
		ve.localCandidates = ve.localCandidates[1:]
		return candidate, nil
	}
	return nil, nil
}

func (ve *VeilEngine) SendCandidate(candidate webrtc.ICECandidateInit) error {
	fmt.Println("Got candidate")
	err := ve.PeerConnection.AddICECandidate(candidate)
	if err != nil {
		fmt.Println("Couldn't add candidate")
		return err
	}
	fmt.Println("Added candidate")
	fmt.Printf("%+v\n", candidate)
	return nil
}

func (ve *VeilEngine) CandidatesLen() int {
	return len(ve.localCandidates)
}

// Create a new RTCPeerConnection
// var gamepad uinput.Gamepad

// dataChannel.OnMessage(func(msg webrtc.DataChannelMessage) {

// 	if gamepad == nil {
// 		var vendor uint16
// 		var product uint16

// 		split := strings.Split(string(msg.Data), " ")
// 		for i, v := range split {
// 			if v == "Vendor:" {
// 				str := strings.Replace(split[i+1], ")", "", -1)
// 				intVendor, err := strconv.ParseUint(str, 16, 16)
// 				if err != nil {
// 					panic(err)
// 				}
// 				vendor = uint16(intVendor)
// 				continue
// 			}

// 			if v == "Product:" {
// 				str := strings.Replace(split[i+1], ")", "", -1)
// 				intProduct, err := strconv.ParseUint(str, 16, 16)
// 				if err != nil {
// 					panic(err)
// 				}

// 				product = uint16(intProduct)
// 				continue
// 			}
// 		}

// 		gamepad, err = uinput.CreateGamepad("/dev/uinput", []byte("Veil Gamepad"), vendor, product)
// 		if err != nil {
// 			panic(err)
// 		}
// 		return
// 	}

// 	split := strings.Split(string(msg.Data), ":")
// 	code, _ := strconv.Atoi(split[0])

// 	if split[1] == "1" {
// 		gamepad.KeyDown(code)
// 	} else if split[1] == "0" {
// 		gamepad.KeyUp(code)
// 	}

// 	fmt.Println(split)

// 	// Gamepad.SendKey(split[0], split[1]) // Do this
// })

// This stuff is how I should write packets to the client
// if _, err = videoTrack.Write(inboundRTPPacket[:n]); err != nil {
// 	if errors.Is(err, io.ErrClosedPipe) {
// 		// The peerConnection has been closed.
// 		return
// 	}
// }
