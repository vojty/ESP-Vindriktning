build:
	cargo build --release

flash:
	cargo espflash flash \
		--release \
		--use-stub \
		--baud 460800 \
		--monitor \
		--flash-freq 40M \
		--target xtensa-esp32-espidf \
		--port /dev/tty.usbserial-110

monitor:
	cargo espmonitor --no-reset /dev/tty.usbserial-110