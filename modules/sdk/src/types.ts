export interface VeilSessionDescription {
  // Command to execute
  command: string[];

  // Virtual display parameters
  display: Display
}

interface Display {
  resolution: Resolution;
  refreshRate: number;
}

interface Resolution {
  width: number;
  height: number;
}
