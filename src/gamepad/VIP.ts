// 0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |            EV Type            |              IEC              |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                             Value                             |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

export enum EventType {
    ButtonDown,
    ButtonUp,
    Axis
}

export interface InputEvent {
    eventType: EventType

}

export class VIP {
    public static marshal(ev: EventType, ie: number, value = 0.00): Blob {
        const evBuffer: Int16Array = Int16Array.from([this.EventCode[ev]])
        if (ev == EventType.Axis) {
            const axisBuffer: Float32Array = Float32Array.from([value])
            const iecBuffer: Int16Array = Int16Array.from([this.AxisIEC[ie]])
            return new Blob([evBuffer, iecBuffer, axisBuffer])
        }

        const iecBuffer: Int16Array = Int16Array.from([this.ButtonIEC[ie]])
        return new Blob([evBuffer, iecBuffer, new Float32Array(new ArrayBuffer(8))])
    }

    private static EventCode: Record<EventType, number> = {
        // [EventType.] : 0x00, // EV_SYN
        [EventType.ButtonDown]: 0x01, // EV_KEY
        [EventType.ButtonUp]: 0x02, // EV_REL
        [EventType.Axis]: 0x03, // EV_ABS
        // [EventType.] : 0x04, // EV_MSC
        // [EventType.] : 0x05, // EV_SW
        // [EventType.] : 0x11, // EV_LED
        // [EventType.] : 0x12, // EV_SND
        // [EventType.] : 0x14, // EV_REP
        // [EventType.] : 0x15, // EV_FF
        // [EventType.] : 0x16, // EV_PWR
        // [EventType.] : 0x17, // EV_FF_STATUS
        // [EventType.] : 0x1f, // EV_MAX
        // [EventType.] : 0x20, // EV_CNT
    }

    // These are maps of the standard gamepad buttons to Linux IEC see -> https://www.w3.org/TR/gamepad/#dom-gamepadmappingtype
    private static AxisIEC: Record<number, number> = {
        0: 0x00, //ABS_X
        1: 0x01, //ABS_Y
        2: 0x03, //ABS_RX
        3: 0x04, //ABS_RY
    }

    private static ButtonIEC: Record<number, number> = {
        // Face buttons
        0: 0x130, //BTN_SOUTH
        1: 0x131, //BTN_EAST
        2: 0x134, //BTN_WEST
        3: 0x133, //BTN_NORTH

        4: 0x136, //BTN_TL
        5: 0x137, //BTN_TR
        6: 0x138, //BTN_TL2
        7: 0x139, //BTN_TR2

        8: 0x13a, //BTN_SELECT
        9: 0x13b, //BTN_START

        10: 0x13d, //BTN_THUMBL
        11: 0x13e, //BTN_THUMBR

        12: 0x220, //BTN_DPAD_UP
        13: 0x221, //BTN_DPAD_DOWN
        14: 0x222, //BTN_DPAD_LEFT
        15: 0x223, //BTN_DPAD_RIGHT

        16: 0x13c, //BTN_MODE TODO: double check if that's what it actually evals to, doing this on a mac
    }

}
