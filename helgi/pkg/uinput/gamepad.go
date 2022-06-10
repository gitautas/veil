package uinput

import (
	"fmt"
	"io"
	"os"
)

type Gamepad interface {
	KeyDown(key int) error
	KeyUp(key int) error

	io.Closer
}

type vGamepad struct {
	name       []byte
	deviceFile *os.File
}

func CreateGamepad(path string, name []byte, vendor uint16, product uint16) (Gamepad, error) {
	err := validateDevicePath(path)
	if err != nil {
		return nil, err
	}
	err = validateUinputName(name)
	if err != nil {
		return nil, err
	}

	fd, err := createVGamepadDevice(path, name, vendor, product)
	if err != nil {
		return nil, err
	}

	return vGamepad{name: name, deviceFile: fd}, nil
}

func (vk vGamepad) KeyDown(key int) error {
	return sendBtnEvent(vk.deviceFile, []int{key}, btnStatePressed)
}

func (vk vGamepad) KeyUp(key int) error {
	return sendBtnEvent(vk.deviceFile, []int{key}, btnStateReleased)
}

func (vk vGamepad) Close() error {
	return closeDevice(vk.deviceFile)
}

func createVGamepadDevice(path string, name []byte, vendor uint16, product uint16) (fd *os.File, err error) {

	codes := []uint16{
		0x130,
		0x131,
		0x134,
		0x133,

		0x136,
		0x137,
		0x138,
		0x139,

		0x13a,
		0x13b,

		0x13d,
		0x13e,

		0x220,
		0x221,
		0x222,
		0x223,

		0x13c,
	}

	deviceFile, err := createDeviceFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to create virtual gamepad device: %v", err)
	}

	err = registerDevice(deviceFile, uintptr(evKey))
	if err != nil {
		deviceFile.Close()
		return nil, fmt.Errorf("failed to register virtual gamepad device: %v", err)
	}

	// register key events
	for i := 0; i < len(codes); i++ {
		err = ioctl(deviceFile, uiSetKeyBit, uintptr(codes[i]))
		if err != nil {
			deviceFile.Close()
			return nil, fmt.Errorf("failed to register key number %d: %v", i, err)
		}
	}

	return createUsbDevice(deviceFile,
		uinputUserDev{
			Name: toUinputName(name),
			ID: inputID{
				Bustype: busUsb,
				Vendor:  vendor,
				Product: product,
				Version: 1}})
}
