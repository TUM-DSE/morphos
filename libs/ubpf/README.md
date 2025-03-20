# ubpf for Unikraft

This is the port of ubpf for Unikraft as external library.

Originally based on https://github.com/iovisor/ubpf/tree/34100804b4dfff20dc24d53ed95a1debbb279611 (that commit differs by 7 lines in ubpf_vm.c though).
In June 2024 (3be9b9271becd4792203285c2088601b953241e4) updated to https://github.com/iovisor/ubpf/tree/2c2a68a2d9d3d9c4db159a03391a2841e8baa964. Again, only roughly. The version vendored here differd by ~13 lines. The changes introduce a bug fixed in f4d4e8dabd774e6a27fe0c6d51d95e5add85a349 and the page flag changes as described by Zhang et. al..

Please refer to the [`README.md`](https://github.com/unikraft/unikraft/tree/staging/README.md)
as well as the documentation in the [`doc/`](https://github.com/unikraft/unikraft/tree/staging/doc/)
subdirectory of the main unikraft repository.
