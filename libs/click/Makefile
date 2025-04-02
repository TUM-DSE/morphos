UK_ROOT ?= $(PWD)/../../.unikraft/unikraft
UK_LIBS ?= $(PWD)/../../.unikraft/libs
LIBS := $(UK_LIBS)/musl:$(UK_LIBS)/lwip:$(PWD)/../ubpf:$(UK_LIBS)/lib-openssl:$(PWD)/../../.unikraft/build/libukpku:$(UK_LIBS)/ukpku

all:
	@$(MAKE) -C $(UK_ROOT) A=$(PWD) L=$(LIBS)

$(MAKECMDGOALS):
	@$(MAKE) -C $(UK_ROOT) A=$(PWD) L=$(LIBS) $(MAKECMDGOALS)
