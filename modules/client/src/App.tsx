import { Video } from './components/Video';

const stream = new MediaStream();

export function App() {
  return (
    <Video stream={stream} />
  );
}
