package media

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"syscall"

	"github.com/pion/rtp"
	"github.com/pion/rtp/codecs"
	"github.com/pion/webrtc/v3"
)

func Sigrun(videoTrack *webrtc.TrackLocalStaticRTP) (error) {
	pipePath := "../video_pipe"
	err := syscall.Mkfifo(pipePath, 0666)
	if err != nil {
		return err
	}

	sigrun := exec.Command("../sigrun/sigrun", pipePath)
	err = sigrun.Start()
	if err != nil {
		return err
	}

	fpipe, err := os.OpenFile(pipePath, os.O_RDONLY, os.ModeNamedPipe)
	if err != nil {
		return err
	}

	reader := bufio.NewReader(fpipe)
	sequencer := rtp.NewFixedSequencer(0) // This is fine since we don't encrypt our media stream.

	packetizer := rtp.NewPacketizer(1200, H264PayloadType, 0, &codecs.H264Payloader{}, sequencer, H264ClockRate)
	go func() {
		for {
			line, _, _ := reader.ReadLine()
			packet := packetizer.Packetize(line, 2000)[0]
			videoTrack.WriteRTP(packet)
			fmt.Println(packet)
		}
	}()

	return nil
}
