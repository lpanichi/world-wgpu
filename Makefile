SIM_EXAMPLES := vernal_equinox summer_solstice autumnal_equinox \
                winter_solstice moon_phases earth_texture orbit_elements \
                frame_validation shapes

.PHONY: all simulation build release check run clean $(SIM_EXAMPLES)

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