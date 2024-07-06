DIR:=$(strip $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST)))))
GREEN:=\033[1;37m
RESET:=\033[0m

BINS := dns-filter drop ether-mirror pass rate-limiter strip-ether-vlan-header target-port udp-tcp-classifier

define BUILD_TARGET
$(1):
	@echo -e "   ${GREEN} Building ${RESET}$(1)..."
	@cd $(DIR) && cargo build --bin $(1) --target bpfel-unknown-none --release -Z build-std=core
	@-cp $(DIR)/target/bpfel-unknown-none/release/$(1) $(DIR)/../rootfs/$(1)
	@echo ""
endef

$(foreach bin,$(BINS),$(eval $(call BUILD_TARGET,$(bin))))

all: $(BINS)