# Bootloader built by esp-idf-sys from sdkconfig.defaults (with app rollback enabled), espflash's bundled one has it disabled
BOOTLOADER = $$(ls -t target/xtensa-esp32-espidf/release/build/esp-idf-sys-*/out/build/bootloader/bootloader.bin | head -1)

bin:
	cargo espflash save-image \
		--chip esp32 \
		--release \
		--target xtensa-esp32-espidf \
		./target/firmware.bin

# otadata is erased so the device boots the freshly flashed ota_0, not a previous OTA update in ota_1
flash:
	cargo build --release
	cargo espflash flash \
		--release \
		--baud 460800 \
		--monitor \
		--flash-freq 40mhz \
		--bootloader $(BOOTLOADER) \
		--erase-parts otadata \
		--target xtensa-esp32-espidf

monitor:
	cargo espflash monitor

ota:
	@test -n "$(DEVICE)" || (echo "Usage: make ota DEVICE=<device-ip>" && exit 1)
	$(MAKE) bin
	@# curl only shows the progress bar when the response isn't written to the terminal
	@curl --fail-with-body --progress-bar \
		-H "Content-Type: application/octet-stream" \
		--data-binary @target/firmware.bin \
		-o target/ota-response.txt \
		http://$(DEVICE)/ota; \
	status=$$?; cat target/ota-response.txt; echo; exit $$status

.PHONY: bin flash monitor ota
