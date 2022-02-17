package ymir

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"

	generated "ymir/pkg/pb"

	"github.com/gin-gonic/gin"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/types/known/emptypb"
)

type Ymir struct {
	port int
    helgiAddr string
	rpcClient generated.WebRTCSignallingClient
	ginEngine *gin.Engine
}

func NewYmir(port int, helgiAddr string) (*Ymir, error){
	conn, err := grpc.Dial(helgiAddr, grpc.WithInsecure())
	if err != nil {
		return nil, err
	}

	helgi := generated.NewWebRTCSignallingClient(conn)

	engine := gin.Default()
	engine.GET("/offer", func(c *gin.Context) {
		offer, err := helgi.GetOffer(context.TODO(), &emptypb.Empty{})
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

		_, err = helgi.SendAnswer(context.TODO(), answer)
		if err != nil {
			c.JSON(http.StatusInternalServerError, err)
			fmt.Println(err)
			return
		}

		c.JSON(http.StatusOK, nil)
	})

	engine.GET("/candidate", func(c *gin.Context) {
		candidateClient, err := helgi.GetCandidate(context.TODO(), &emptypb.Empty{})
		if err != nil {
			c.JSON(http.StatusInternalServerError, err)
			fmt.Println(err)
			return
		}

		go func() {
			for {
				candidate, err := candidateClient.Recv()
				if err == io.EOF {
					c.JSON(http.StatusOK, nil)
				}
				if err != nil {
					c.JSON(http.StatusInternalServerError, err)
					fmt.Println(err)
					return
				}
				c.JSON(http.StatusContinue, candidate)
			}
		}()
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
		_, err = helgi.SendCandidate(context.TODO(), candidate)

	})

	return &Ymir{
		port: port,
		helgiAddr: helgiAddr,
		rpcClient: helgi,
		ginEngine: engine,
	}, nil
}

func (y *Ymir) Serve() {
	y.ginEngine.Run("0.0.0.0:" + strconv.Itoa(y.port))
}
