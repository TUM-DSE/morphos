build:
	# Check which unikraft is checked out
	# @if [[ $$(git submodule status -- libs/unikraft) = *"$$(jq -r '.nodes.unikraft.locked.rev' flake.lock)"* ]]; then \
	# Check which version of the unikraft submodule is committed to git
	@if [[ $$(git ls-tree HEAD libs/unikraft) = *"$$(jq -r '.nodes.unikraft.locked.rev' flake.lock)"* ]]; then \
		echo "lib/unikraft in sync"; \
	else \
		echo "ERROR: git specifies lib/unikraft to be at another commit than expected by flake.nix/.lock"; \
		exit 1; \
	fi
	# Hack to fix missing invalidation of copied Click elements
	rm -rf .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
	mkdir -p .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
	cp -r libs/click/unikraft .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements

	kraft build --log-type basic $(EXTRA_KRAFT_ARGS)

cleanbuild:
	sudo rm -rf .unikraft || true
	sudo rm .config.click_qemu-x86_64 || true
	just downloadLibs
	nix develop .#fhsMake
