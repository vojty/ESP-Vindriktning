bin:
	cargo espflash save-image \
		--chip esp32 \
		--release \
		--target xtensa-esp32-espidf \
		./target/firmware.bin

flash:
	cargo espflash flash \
		--release \
		--use-stub \
		--baud 460800 \
		--monitor \
		--flash-freq 40M \
		--target xtensa-esp32-espidf

monitor:
	cargo espmonitor --no-reset