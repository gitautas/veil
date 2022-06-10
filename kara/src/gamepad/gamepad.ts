import { VIP, EventType } from "./VIP";

export default class VeilGamepad {
  gamepad: Gamepad;
  oldGamepad: Gamepad;
  pollInterval: number;
  deadZone: number;
  dataChannel: RTCDataChannel;

  private compareButtons() {
    this.gamepad.buttons.map((v, i) => {
      if (v.pressed != this.oldGamepad.buttons[i].pressed) {
        let data: Blob;
        if (v.pressed) {
          data = VIP.marshal(EventType.ButtonDown, i);
        } else {
          data = VIP.marshal(EventType.ButtonUp, i);
        }
        // this.dataChannel.send(data)
      }
    });
  }

  private compareAxes() {
    this.gamepad.axes.map((v, i) => {
      if (Math.abs(v) > this.deadZone && v != this.oldGamepad.axes[i]) {
        const data: Blob = VIP.marshal(EventType.Axis, i, v);
        // this.dataChannel.send(data)
      }
    });
  }

  public async poll(): Promise<void> {
    console.log("starting to poll");
    setInterval(() => {
      const gp: Array<Gamepad | null> = navigator.getGamepads();
      if (gp[0] != null) {
        this.gamepad = gp[0]; // This works
      }

      this.compareButtons();
      this.compareAxes();

      this.oldGamepad = this.gamepad; // This works too
    }, this.pollInterval);
  }

  constructor(
    gamepad: Gamepad,
    dataChannel: RTCDataChannel,
    pollRateHz = 133,
    deadZone = 0.1
  ) {
    this.gamepad = gamepad;
    this.oldGamepad = gamepad;
    this.pollInterval = (1 / pollRateHz) * 1000; // the interval of time between polls
    this.deadZone = deadZone;
    this.dataChannel = dataChannel;
  }
}
