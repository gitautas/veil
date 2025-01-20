import React from 'react';
import { VeilSessionDescription } from '@veil/sdk';

interface VideoProps {
  stream: MediaStream;
}

/**
 * A full screen video component that displays a MediaStream
 */
export const Video = ({ stream }: VideoProps) => {
  const videoRef = React.useRef<HTMLVideoElement>(null);
  const desc: VeilSessionDescription = {
    command: ["/usr/games/gamescope", "--backend", "headless", "--", "vkcube"],
    display: {
      resolution: {
        width: 1920,
        height: 1080,
      },
      refreshRate: 60,
    },
  };

  console.log(desc);

  React.useEffect(() => {
    if (videoRef.current && stream) {
      videoRef.current.srcObject = stream;
    }
  }, [stream]);

  return (
    <video
      ref={videoRef}
      autoPlay
      playsInline
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        width: '100%',
        height: '100%',
        objectFit: 'cover'
      }}
    />
  );
};
