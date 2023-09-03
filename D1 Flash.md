Wrapper around `espflash` to flash binaries onto a Wemos D1 mini lite (ESP8266) via a Raspberry Pi.

The D1 requires grounding `GPIO D3` and resetting in order to flash. Afterwards, `D3` must be released, such that the device can be rebooted into the program.

This wrapper sets up GPIO pins on the Raspberry Pi to enable flashing of the D1, before passing arguments onto `espflash`, in order to execute the flashing procedure. 