package signalling

import (
	"context"
	"fmt"
	"helgi/pkg/media"
	"helgi/protobuf/pb"
	"net"

	"github.com/pion/webrtc/v3"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/types/known/emptypb"
)

type RpcServer struct {
	Server      *grpc.Server
	mediaEngine *media.VeilEngine
	generated.UnimplementedWebRTCSignallingServer
}

func NewRPCServer() (*RpcServer, error) {
	errHandler := func(ctx context.Context, req interface{}, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (interface{}, error) {
		resp, err := handler(ctx, req)
		if err != nil {
			fmt.Printf("method %q failed: %s", info.FullMethod, err)
		}
		return resp, err
	}
	srv := grpc.NewServer(grpc.UnaryInterceptor(errHandler))

	engine, err := media.NewVeilEngine(webrtc.Configuration{
		ICEServers: []webrtc.ICEServer{{URLs: []string{"stun:stun.l.google.com:19302"}}},
	})
	if err != nil {
		return nil, err
	}

	rpcServer := &RpcServer{
		Server:      srv,
		mediaEngine: engine,
	}

	var _ generated.WebRTCSignallingServer = rpcServer

	srv.RegisterService(&generated.WebRTCSignalling_ServiceDesc, rpcServer)

	return rpcServer, nil
}

func (rs *RpcServer) Serve(port string) error {
	rpcListener, err := net.Listen("tcp", "127.0.0.1:"+port)
	if err != nil {
		return err
	}

	err = rs.Server.Serve(rpcListener)
	if err != nil {
		return err
	}
	return nil
}

func (rs *RpcServer) GetOffer(ctx context.Context, _ *emptypb.Empty) (*generated.SDP, error) {
	offer, err := rs.mediaEngine.GenerateOffer()
	return &generated.SDP{
		Sdp:  offer.SDP,
		Type: offer.Type.String(),
	}, err
}

func (rs *RpcServer) SendAnswer(ctx context.Context, answer *generated.SDP) (*emptypb.Empty, error) {
	return new(emptypb.Empty), rs.mediaEngine.SetAnswer(webrtc.SessionDescription{
		Type: webrtc.NewSDPType(answer.Type),
		SDP:  answer.Sdp,
	})
}

func (rs *RpcServer) GetCandidate(_ *emptypb.Empty, stream generated.WebRTCSignalling_GetCandidateServer) error {
	for rs.mediaEngine.PeerConnection.ICEGatheringState() != webrtc.ICEGatheringStateComplete && rs.mediaEngine.CandidatesLen() != 0 {
		candidate, err := rs.mediaEngine.GetCandidate()
		if err != nil {
			break
		}

		if candidate == nil {
			continue
		}

		fmt.Println("Sending candidate")
		err = stream.Send(generated.ICECandidateToProto(candidate))
		fmt.Println("Sent candidate")
		if err != nil {
			fmt.Println(err)
		}
	}
	fmt.Println("EOF")
	return nil
}

func (rs *RpcServer) SendCandidate(ctx context.Context, candidate *generated.ICECandidate) (*emptypb.Empty, error) {
	return new(emptypb.Empty), rs.mediaEngine.SendCandidate(candidate.ToPion().ToJSON())
}
