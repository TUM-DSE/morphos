DIR:=$(strip $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST)))))
GRAY_BOLD:=\033[1;37m
RESET:=\033[0m
TARGET_DIR:=$(DIR)/target/bpfel-unknown-none/release
EXAMPLES_DIR:=$(DIR)/../examples
BENCHMARK_DIR:=$(DIR)/../benchmark/bpfilters

BINS := dns-filter drop ether-mirror pass rate-limiter round-robin strip-ether-vlan-header target-port udp-tcp-classifier state-migration-v1 state-migration-v2 stringmatcher nat

VERIFY ?= 0

define BUILD_TARGET
$(1):
	@echo -e "${GRAY_BOLD}Building ${RESET}$(1)..."
	@cd $(DIR) && cargo build --bin $(1) --target bpfel-unknown-none --release -Z build-std=core
	@echo ""
	@if [ "$(VERIFY)" = "1" ]; then \
		echo -e "${GRAY_BOLD}Verification Results:${RESET}"; \
	 	$(DIR)/../verifier/build/ubpf_verifier -f $(TARGET_DIR)/$(1) -k $(DIR)/../verifier/keys/ec_private_key.pem -o $(TARGET_DIR)/$(1).sig || true; \
	 fi
endef

$(foreach bin,$(BINS),$(eval $(call BUILD_TARGET,$(bin))))

all: $(BINS)

sync:
	@-cp $(TARGET_DIR)/dns-filter $(EXAMPLES_DIR)/dns-filter/rootfs/dns-filter
	@-cp $(TARGET_DIR)/drop $(EXAMPLES_DIR)/drop/rootfs/drop
	@-cp $(TARGET_DIR)/drop.sig $(EXAMPLES_DIR)/drop/rootfs/drop.sig
	@-cp $(TARGET_DIR)/ether-mirror $(EXAMPLES_DIR)/ether-mirror/rootfs/ether-mirror
	@-cp $(TARGET_DIR)/ether-mirror.sig $(EXAMPLES_DIR)/ether-mirror/rootfs/ether-mirror.sig
	@-cp $(TARGET_DIR)/pass $(EXAMPLES_DIR)/pass/rootfs/pass
	@-cp $(TARGET_DIR)/pass.sig $(EXAMPLES_DIR)/pass/rootfs/pass.sig
	@-cp $(TARGET_DIR)/rate-limiter $(EXAMPLES_DIR)/rate-limiter/rootfs/rate-limiter
	@-cp $(TARGET_DIR)/rate-limiter.sig $(EXAMPLES_DIR)/rate-limiter/rootfs/rate-limiter.sig
	@-cp $(TARGET_DIR)/strip-ether-vlan-header $(EXAMPLES_DIR)/strip-ether-vlan-header/rootfs/strip-ether-vlan-header
	@-cp $(TARGET_DIR)/strip-ether-vlan-header.sig $(EXAMPLES_DIR)/strip-ether-vlan-header/rootfs/strip-ether-vlan-header.sig
	@-cp $(TARGET_DIR)/target-port $(EXAMPLES_DIR)/target-port/rootfs/target-port
	@-cp $(TARGET_DIR)/target-port.sig $(EXAMPLES_DIR)/target-port/rootfs/target-port.sig
	@-cp $(TARGET_DIR)/udp-tcp-classifier $(EXAMPLES_DIR)/udp-tcp-classifier/rootfs/udp-tcp-classifier
	@-cp $(TARGET_DIR)/udp-tcp-classifier.sig $(EXAMPLES_DIR)/udp-tcp-classifier/rootfs/udp-tcp-classifier.sig
	@-cp $(TARGET_DIR)/state-migration-v1 $(EXAMPLES_DIR)/state-migration/rootfs/state-migration-v1
	@-cp $(TARGET_DIR)/state-migration-v1.sig $(EXAMPLES_DIR)/state-migration/rootfs/state-migration-v1.sig
	@-cp $(TARGET_DIR)/state-migration-v2 $(EXAMPLES_DIR)/state-migration/rootfs/state-migration-v2
	@-cp $(TARGET_DIR)/state-migration-v2.sig $(EXAMPLES_DIR)/state-migration/rootfs/state-migration-v2.sig

	@-cp $(TARGET_DIR)/rate-limiter $(EXAMPLES_DIR)/chain/firewall/rate-limiter
	@-cp $(TARGET_DIR)/rate-limiter.sig $(EXAMPLES_DIR)/chain/firewall/rate-limiter.sig
	@-cp $(TARGET_DIR)/target-port $(EXAMPLES_DIR)/chain/firewall/target-port
	@-cp $(TARGET_DIR)/target-port.sig $(EXAMPLES_DIR)/chain/firewall/target-port.sig
	@-cp $(TARGET_DIR)/round-robin $(EXAMPLES_DIR)/chain/load-balancer/round-robin
	@-cp $(TARGET_DIR)/round-robin.sig $(EXAMPLES_DIR)/chain/load-balancer/round-robin.sig

	@-cp $(TARGET_DIR)/pass $(EXAMPLES_DIR)/showcase/rootfs/pass
	@-cp $(TARGET_DIR)/pass.sig $(EXAMPLES_DIR)/showcase/rootfs/pass.sig
	@-cp $(TARGET_DIR)/drop $(EXAMPLES_DIR)/showcase/rootfs/drop
	@-cp $(TARGET_DIR)/drop.sig $(EXAMPLES_DIR)/showcase/rootfs/drop.sig

	@-cp $(TARGET_DIR)/pass $(BENCHMARK_DIR)/pass
	@-cp $(TARGET_DIR)/pass.sig $(BENCHMARK_DIR)/pass.sig
	@-cp $(TARGET_DIR)/drop $(BENCHMARK_DIR)/drop
	@-cp $(TARGET_DIR)/drop.sig $(BENCHMARK_DIR)/drop.sig
	@-cp $(TARGET_DIR)/ether-mirror $(BENCHMARK_DIR)/ether-mirror
	@-cp $(TARGET_DIR)/ether-mirror.sig $(BENCHMARK_DIR)/ether-mirror.sig
	@-cp $(TARGET_DIR)/target-port $(BENCHMARK_DIR)/target-port
	@-cp $(TARGET_DIR)/target-port.sig $(BENCHMARK_DIR)/target-port.sig
	@-cp $(TARGET_DIR)/round-robin $(BENCHMARK_DIR)/round-robin
	@-cp $(TARGET_DIR)/round-robin.sig $(BENCHMARK_DIR)/round-robin.sig
	@-cp $(TARGET_DIR)/rate-limiter $(BENCHMARK_DIR)/rate-limiter
	@-cp $(TARGET_DIR)/rate-limiter.sig $(BENCHMARK_DIR)/rate-limiter.sig
	@-cp $(TARGET_DIR)/udp-tcp-classifier $(BENCHMARK_DIR)/udp-tcp-classifier
	@-cp $(TARGET_DIR)/udp-tcp-classifier.sig $(BENCHMARK_DIR)/udp-tcp-classifier.sig
	@-cp $(TARGET_DIR)/strip-ether-vlan-header $(BENCHMARK_DIR)/strip-ether-vlan-header
	@-cp $(TARGET_DIR)/strip-ether-vlan-header.sig $(BENCHMARK_DIR)/strip-ether-vlan-header.sig
	@-cp $(TARGET_DIR)/stringmatcher $(BENCHMARK_DIR)/
	@-cp $(TARGET_DIR)/nat.sig $(BENCHMARK_DIR)/
	@-cp $(TARGET_DIR)/nat $(BENCHMARK_DIR)/
	@-cp $(TARGET_DIR)/stringmatcher.sig $(BENCHMARK_DIR)/
