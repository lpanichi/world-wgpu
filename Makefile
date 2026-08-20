SIM_EXAMPLES := vernal_equinox summer_solstice autumnal_equinox \
                winter_solstice moon_phases earth_texture orbit_elements \
                frame_validation shapes

SCREENSHOT_DIR := docs/screenshots
SCREENSHOTS := $(addprefix $(SCREENSHOT_DIR)/,$(addsuffix .png,$(SIM_EXAMPLES) simulation))

.PHONY: all simulation build release check run clean screenshots $(SIM_EXAMPLES)

# Default target: run the main simulation example
simulation:
	cargo run -p simulation

all: release

release:
	cargo build --workspace --release

build:
	cargo build --workspace

check:
	cargo clippy --workspace --all-targets

run: simulation $(SIM_EXAMPLES)

$(SIM_EXAMPLES):
	cargo run -p $@

clean:
	cargo clean

# Regenerate the README screenshots on demand. Each example renders for a short
# settle period, captures its framebuffer to docs/screenshots/<name>.png and
# exits. Use `make -B screenshots` to force a full regeneration.
screenshots: $(SCREENSHOTS)

$(SCREENSHOT_DIR)/%.png: examples/%/src/main.rs examples/%/Cargo.toml gui/src/screenshot.rs
	cargo run -q -p $(notdir $*) -- --screenshot=$@