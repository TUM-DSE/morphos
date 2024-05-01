# appclick-ubpf

This repository showcases the use of ubpf with Click Modular Router.

The repository extends the default appclick with following features:
- Added a new element `BPFilter` which uses ubpf to filter packets based upon a user-defined filter
- Changes the build system to be based on Kraftkit

## Installation
1. Ensure all dependencies are installed: `nix develop`
2. Build the project: `kraft build`
3. Setup the network interfaces: `./setup.sh`
4. Run the project: `./run.sh`
 