build:
	rm -rf .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
	mkdir -p .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements/unikraft || true
	cp -r libs/click/unikraft .unikraft/build/libclick/origin/click-a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2/elements
	kraft build --no-configure --no-fetch --no-update --no-rootfs --sources-dir /scratch/paulz/.unikraft/sources --manifests-dir /scratch/paulz/.unikraft/manifests --log-type basic

run:
	./run.sh

build-run: build run

benchmark:
	cd benchmarks && cargo bench