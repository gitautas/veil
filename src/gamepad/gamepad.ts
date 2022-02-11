// import { VIP, EventType } from "./VIP"

export default class VeilGamepad {
    gamepad: Gamepad
    pollInterval: number
    deadZone: number
    dataChannel: RTCDataChannel
    oldGamepad: Gamepad

    private compareButtons() {
        if (this.gamepad.buttons != this.oldGamepad.buttons) {
            console.log(this.oldGamepad.buttons)
        }
        this.gamepad.buttons.map((v, i) => {
            if (v != this.oldGamepad.buttons[i]) {
                console.log(v)
            }
        })
    }

    private compareAxes() {
        if (this.gamepad.axes != this.oldGamepad.axes) {
            console.log(this.oldGamepad.axes)
        }
        this.gamepad.axes.map((v, i) => {
            if (Math.abs(v) > this.deadZone && v != this.oldGamepad.axes[i]) {
                console.log(v)
            }
        })
    }

    public async poll(): Promise<void> {
        console.log("starting to poll")
        setInterval(() => {
            const gp: Array<Gamepad | null> = navigator.getGamepads()
            if (gp[0] != null) {
                this.gamepad = gp[0] // This works
            }

            this.compareButtons()
            this.compareAxes()

            this.oldGamepad = this.gamepad // This works too
        }, this.pollInterval)
    }

    constructor(gamepad: Gamepad, dataChannel: RTCDataChannel, pollRateHz = 2, deadZone = 0.05) {
        this.gamepad = gamepad
        this.oldGamepad = gamepad
        this.pollInterval = 1 / pollRateHz * 1000 // the interval of time between polls
        this.deadZone = deadZone
        this.dataChannel = dataChannel
    }
}
