package media

import (
	"os"
	"os/exec"
	"syscall"
	"time"

	"github.com/pion/webrtc/v3"
	"github.com/pion/webrtc/v3/pkg/media"
	"github.com/pion/webrtc/v3/pkg/media/h264reader"
)

func Sigrun(videoTrack *webrtc.TrackLocalStaticSample) (error) {
	const h264FrameDuration = time.Second / 100

	pipePath := "../video_pipe"

	_ = syscall.Mkfifo(pipePath, 0666)

	sigrun := exec.Command("../sigrun/sigrun", pipePath)
	// sigrun := exec.Command("../NvFBCToGLEnc", "-o", pipePath)

	err := sigrun.Start()
	if err != nil {
		panic(err)
	}

	fpipe, err := os.OpenFile(pipePath, os.O_RDONLY, os.ModeNamedPipe)
	if err != nil {
		panic(err)
	}

	h264, err := h264reader.NewReader(fpipe)
	if err != nil {
		panic(err)
	}

	go func() {
		ticker := time.NewTicker(h264FrameDuration)

		for ; true; <-ticker.C {
			nal, err := h264.NextNAL()
			if err != nil {
				panic(err)
			}

			err = videoTrack.WriteSample(
				media.Sample{
					Data:               nal.Data,
					Timestamp:          time.Now(),
					Duration:           h264FrameDuration,
				},)

			if err != nil {
				panic(err)
			}
		}
	}()

	return nil
}
