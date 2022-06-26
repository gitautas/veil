package models

import (
	"github.com/pion/webrtc/v3"
	"helgi/pkg/pb"
)

func SDPToProto(sdp *webrtc.SessionDescription) *generated.SDP {
	return &generated.SDP{
		Sdp:  sdp.SDP,
		Type: sdp.Type.String(),
	}
}

func ICECandidateToProto(candidate *webrtc.ICECandidate) *generated.ICECandidate {
	return &generated.ICECandidate{
		Foundation:     candidate.Foundation,
		Priority:       candidate.Priority,
		Address:        candidate.Address,
		Protocol:       int32(candidate.Protocol),
		Port:           uint32(candidate.Port),
		Type:           int32(candidate.Typ),
		Component:      uint32(candidate.Component),
		Relatedaddress: candidate.RelatedAddress,
		Relatedport:    uint32(candidate.RelatedPort),
		Tcptype:        candidate.TCPType,
	}
}

func PBToPion(candidate *generated.ICECandidate) *webrtc.ICECandidate {
	return &webrtc.ICECandidate{
		Foundation:     candidate.Foundation,
		Priority:       candidate.Priority,
		Address:        candidate.Address,
		Protocol:       webrtc.ICEProtocol(candidate.Protocol),
		Port:           uint16(candidate.Port),
		Typ:            webrtc.ICECandidateType(candidate.Type),
		Component:      uint16(candidate.Component),
		RelatedAddress: candidate.Relatedaddress,
		RelatedPort:    uint16(candidate.Relatedport),
		TCPType:        candidate.Tcptype,
	}
}
