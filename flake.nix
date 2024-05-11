{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
    unstablepkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, unstablepkgs, flake-utils }:
    (flake-utils.lib.eachDefaultSystem
        (system:
            let
              pkgs = import nixpkgs {
                  inherit system;
              };
              unstable = import unstablepkgs {
                   inherit system;
              };
              buildDeps = with pkgs; [
                pkg-config
                gnumake
                flex
                bison
                git
                wget
                libuuid
                gcc
                qemu
                qemu_kvm
                cmake
                unzip
                clang
                openssl
              ];
            in
            {
              devShells.default = pkgs.mkShell {
                name = "devShell";
                buildInputs = buildDeps ++ [
                    unstable.kraft
                    unstable.rustup
                ];
                KRAFTKIT_NO_WARN_SUDO = "1";
                KRAFTKIT_NO_CHECK_UPDATES = "true";
              };
            }
        )
    );
}
