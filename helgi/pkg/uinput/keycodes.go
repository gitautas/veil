package uinput

// the constants that are defined here relate 1:1 to the constants defined in input.h and represent actual
// key codes that can be triggered as key events
const (
	ABS_X  = 0x00
	ABS_Y  = 0x01
	ABS_RX = 0x03
	ABS_RY = 0x04

	BTN_SOUTH = 0x130
	BTN_EAST  = 0x131
	BTN_WEST  = 0x134
	BTN_NORTH = 0x133

	BTN_TL  = 0x136
	BTN_TR  = 0x137
	BTN_TL2 = 0x138
	BTN_TR2 = 0x139

	BTN_SELECT = 0x13a
	BTN_START  = 0x13b

	BTN_THUMBL = 0x13d
	BTN_THUMBR = 0x13e

	BTN_DPAD_UP    = 0x220
	BTN_DPAD_DOWN  = 0x221
	BTN_DPAD_LEFT  = 0x222
	BTN_DPAD_RIGHT = 0x223

	BTN_MODE = 0x13c

	keyMax = 248 // highest key currently defined in this keyboard api
)
