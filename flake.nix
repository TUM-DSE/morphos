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
              buildDeps = pkgs: (with pkgs; [
                pkg-config
                gnumake
                flex
                bison
                git
                wget
                libuuid
                gcc
                # qemu
                (qemu_kvm.overrideAttrs (new: old: {
                  patches = old.patches ++ [
                  ];
                }))
                cmake
                unzip
                clang
                openssl
                ncurses
                bridge-utils
                python3Packages.numpy
                python3Packages.matplotlib
                python3Packages.scipy
                gnuplot
                llvmPackages_15.bintools
                perl
                doxygen
                gzip
                ncurses
                ncurses.dev
                (pkgs.runCommand "gcc-nm" {} ''
                  # only bring in gcc-nm from libgcc.out, because it otherwise prevents crt1.so from musl to be found
                  mkdir -p $out/bin
                  cp ${pkgs.libgcc.out}/bin/gcc-nm $out/bin
                  cp -r ${pkgs.libgcc.out}/libexec/ $out/
                '')
                gdb
                musl
              ]);
              prevailDeps = pkgs: (with pkgs; [
                gcc
                git
                cmake
                boost
                yaml-cpp
              ]);
            in
            {
              devShells.default = pkgs.mkShell {
                name = "devShell";
                buildInputs = (buildDeps pkgs) ++ (prevailDeps pkgs) ++ [
                    unstable.kraft
                    unstable.rustup
                    unstable.bmon
                    unstable.gh
                ];
                KRAFTKIT_NO_WARN_SUDO = "1";
                KRAFTKIT_NO_CHECK_UPDATES = "true";
              };
              devShells.fhs = (pkgs.buildFHSEnv {
                name = "devShell";
                targetPkgs = pkgs: (
                  (buildDeps pkgs) ++ (prevailDeps pkgs) ++ [
                    unstable.kraft
                    unstable.rustup
                    unstable.bmon
                    unstable.gh
                  ]
                );
                runScript = "bash";
                # KRAFTKIT_NO_WARN_SUDO = "1";
                # KRAFTKIT_NO_CHECK_UPDATES = "true";
              }).env;
            }
        )
    );
}
