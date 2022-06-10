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

//FIXME: This entire part is completely untested, this should be set up to run in parallel with sigrun and be robust enough not to randomly break
type AudioManager struct {}

func NewAudioManager(audioPipe string, audioTrack *webrtc.TrackLocalStaticRTP) (*AudioManager, error) {
	err := syscall.Mkfifo(audioPipe, 0666)
	if err != nil {
		return nil, err
	}

	sif := exec.Command("sif", audioPipe)
	err = sif.Start()
	if err != nil {
		return nil, err
	}

	fpipe, err := os.OpenFile(audioPipe, os.O_RDONLY, os.ModeNamedPipe)
	if err != nil {
		return nil, err
	}

	reader := bufio.NewReader(fpipe)
	sequencer := rtp.NewFixedSequencer(0) // This is fine since we don't encrypt our media stream.

	packetizer := rtp.NewPacketizer(1200, OpusPayloadType, 0, &codecs.OpusPayloader{}, sequencer, OpusClockRate)
	go func() {
		for {
			line, _, _ := reader.ReadLine()
			packet := packetizer.Packetize(line, 2000)[0]
			audioTrack.WriteRTP(packet)
			fmt.Println(packet)
		}
	}()

	return &AudioManager{}, nil
}
