package ymir

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"

	generated "ymir/pkg/pb"

	"github.com/gin-contrib/cors"
	"github.com/gin-gonic/gin"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/types/known/emptypb"
)

type Ymir struct {
	port            int
	helgiAddr       string
	rpcClient       generated.WebRTCSignallingClient
	helgiCandidates []*generated.ICECandidate
	helgiEOF        bool
	ginEngine       *gin.Engine
}

func NewYmir(port int, helgiAddr string) (*Ymir, error) {
	conn, err := grpc.Dial(helgiAddr, grpc.WithInsecure())
	if err != nil {
		return nil, err
	}

	helgi := generated.NewWebRTCSignallingClient(conn)
	candidates := []*generated.ICECandidate{}
	eof := false

	engine := gin.Default()
	engine.Use(cors.Default())
	engine.GET("/offer", func(c *gin.Context) {
		offer, err := helgi.GetOffer(context.TODO(), new(emptypb.Empty))
		if err != nil {
			c.JSON(http.StatusInternalServerError, err)
			fmt.Println(err)
			return
		}

		c.JSON(http.StatusOK, offer)
	})

	engine.POST("/answer", func(c *gin.Context) {
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

		_, err = helgi.SendAnswer(c.Request.Context(), answer)
		if err != nil {
			c.JSON(http.StatusInternalServerError, err)
			fmt.Println(err)
			return
		}

		candidateClient, err := helgi.GetCandidate(context.Background(), new(emptypb.Empty))
		if err != nil {
			c.JSON(http.StatusInternalServerError, err)
			fmt.Println(err)
			return
		}

		go func() {
			fmt.Println("Starting candidate thread")
			for {
				candidate, err := candidateClient.Recv()
				if err != nil {
					if err == io.EOF {
						eof = true
						return
					}

					fmt.Println(err)
					fmt.Println("Errored on candidate thread")
					return
				}
				fmt.Println("Got a candidate")
				candidates = append(candidates, candidate)
			}
		}()

		c.JSON(http.StatusOK, nil)
	})

	engine.GET("/candidate", func(c *gin.Context) {
		fmt.Println("GET /candidate")
		if eof && len(candidates) == 0 {
			c.Status(http.StatusOK)
			return
		}
		if len(candidates) > 0 {
			candidate := candidates[0]
			candidates = candidates[1:]
			fmt.Println(candidate)
			c.JSON(http.StatusAccepted, candidate)
			fmt.Println("Sent out the fucker")
			return
		}
		c.Status(http.StatusContinue)
	})

	engine.POST("/candidate", func(c *gin.Context) {
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
		_, err = helgi.SendCandidate(c.Request.Context(), candidate)
	})

	return &Ymir{
		helgiCandidates: candidates,
		port:            port,
		helgiAddr:       helgiAddr,
		rpcClient:       helgi,
		ginEngine:       engine,
	}, nil
}

func (y *Ymir) Serve() {
	y.ginEngine.Run("0.0.0.0:" + strconv.Itoa(y.port))
}
