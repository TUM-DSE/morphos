DIR:=$(strip $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST)))))
GRAY_BOLD:=\033[1;37m
RESET:=\033[0m

BINS := dns-filter drop ether-mirror pass rate-limiter strip-ether-vlan-header target-port udp-tcp-classifier

VERIFY ?= 0

define BUILD_TARGET
$(1):
	@echo -e "${GRAY_BOLD}Building ${RESET}$(1)..."
	@cd $(DIR) && cargo build --bin $(1) --target bpfel-unknown-none --release -Z build-std=core
	@-cp $(DIR)/target/bpfel-unknown-none/release/$(1) $(DIR)/../rootfs/$(1)
	@echo ""
	@if [ "$(VERIFY)" = "1" ]; then \
		echo -e "${GRAY_BOLD}Verification Results:${RESET}"; \
	 	$(DIR)/../verifier/build/ubpf_verifier -f $(DIR)/../rootfs/$(1) -k $(DIR)/../verifier/keys/ec_private_key.pem -o $(DIR)/../rootfs/$(1).sig || true; \
	 fi
endef

$(foreach bin,$(BINS),$(eval $(call BUILD_TARGET,$(bin))))

all: $(BINS)
