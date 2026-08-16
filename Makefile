EXAMPLES := simulation vernal_equinox summer_solstice autumnal_equinox \
            winter_solstice moon_phases earth_texture orbit_elements \
            frame_validation shapes

.PHONY: all build check run $(EXAMPLES) clean

all: run

build:
	cargo build --workspace

check:
	cargo clippy --workspace --all-targets

run: $(EXAMPLES)

$(EXAMPLES):
	cargo run -p $@

clean:
	cargo clean