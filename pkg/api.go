package ymir

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"

	generated "ymir/pkg/pb"

	"github.com/gin-contrib/cors"
	"github.com/gin-gonic/gin"
	"github.com/pion/webrtc/v3"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/types/known/emptypb"
)

type Ymir struct {
	port            string
	helgiAddr       string
	rpcClient       generated.WebRTCSignallingClient
	helgiCandidates []*webrtc.ICECandidate
	iceEOF          bool
	ginEngine       *gin.Engine
}

func NewYmir(port string, helgiAddr string) (*Ymir, error) {
	return &Ymir{
		helgiCandidates: []*webrtc.ICECandidate{},
		port:            port,
		helgiAddr:       helgiAddr,
		rpcClient:       nil,
		ginEngine:       nil,
		iceEOF:          false,
	}, nil
}

func (y *Ymir) GetOffer(c *gin.Context) {
	offer, err := y.rpcClient.GetOffer(context.TODO(), new(emptypb.Empty))
	if err != nil {
		c.JSON(http.StatusInternalServerError, err)
		fmt.Println(err)
		return
	}

	c.JSON(http.StatusOK, offer)
}

func (y *Ymir) SendAnswer(c *gin.Context) {
	jAnswer, err := c.GetRawData()
	if err != nil {
		c.JSON(http.StatusInternalServerError, err)
		fmt.Println(err)
		return
	}

	var answer *generated.SDP
	err = json.Unmarshal(jAnswer, &answer)
	if err != nil {
		c.JSON(http.StatusInternalServerError, err)
		fmt.Println(err)
		return
	}

	_, err = y.rpcClient.SendAnswer(c.Request.Context(), answer)
	if err != nil {
		c.JSON(http.StatusInternalServerError, err)
		fmt.Println(err)
		return
	}

	candidateClient, err := y.rpcClient.GetCandidate(context.Background(), new(emptypb.Empty))
	if err != nil {
		c.JSON(http.StatusInternalServerError, err)
		fmt.Println(err)
		return
	}

	go y.collectCandidates(candidateClient)
}

func (y *Ymir) GetCandidates(c *gin.Context) {
	fmt.Println("GET /candidate")
	if y.iceEOF && len(y.helgiCandidates) == 0 {
		c.Status(http.StatusOK)
		return
	}
	if len(y.helgiCandidates) > 0 {
		candidate := y.helgiCandidates[0]
		y.helgiCandidates = y.helgiCandidates[1:]
		fmt.Println(candidate)
		c.JSON(http.StatusAccepted, candidate.ToJSON())
		return
	}
	c.Status(http.StatusNoContent)
	return
}

func (y *Ymir) SendCandidate(c *gin.Context) {
	jCandidate, err := c.GetRawData()
	if err != nil {
		c.JSON(http.StatusInternalServerError, err)
		fmt.Println(err)
		return
	}

	var candidate *generated.ICECandidate
	err = json.Unmarshal(jCandidate, &candidate)
	if err != nil {
		c.JSON(http.StatusInternalServerError, err)
		fmt.Println(err)
		return
	}
	_, err = y.rpcClient.SendCandidate(c.Request.Context(), candidate)
}

func (y *Ymir) Serve() error {
	conn, err := grpc.Dial(y.helgiAddr, grpc.WithInsecure())
	if err != nil {
		return err
	}

	helgi := generated.NewWebRTCSignallingClient(conn)
	y.rpcClient = helgi

	y.ginEngine = gin.Default()
	y.ginEngine.Use(cors.Default())

	y.ginEngine.GET("/offer", y.GetOffer)
	y.ginEngine.POST("/answer", y.SendAnswer)
	y.ginEngine.GET("/candidate", y.GetCandidates)
	y.ginEngine.POST("/candidate", y.SendCandidate)

	y.ginEngine.Run("0.0.0.0:" + y.port)

	return nil
}

func (y *Ymir) collectCandidates(candidateClient generated.WebRTCSignalling_GetCandidateClient) {
	fmt.Println("Starting candidate thread")
	for {
		candidate, err := candidateClient.Recv()
		if err == io.EOF {
			y.iceEOF = true
			return
		}
		if err != nil {
			fmt.Println(err)
			return
		}
		fmt.Println("Got a candidate")
		realCandidate := &webrtc.ICECandidate{
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
		y.helgiCandidates = append(y.helgiCandidates, realCandidate)
	}
}
