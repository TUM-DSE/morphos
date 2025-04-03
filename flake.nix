{
  nixConfig.extra-substituters = [
    "https://tum-dse.cachix.org"
  ];

  nixConfig.extra-trusted-public-keys = [
    "tum-dse.cachix.org-1:v67rK18oLwgO0Z4b69l30SrV1yRtqxKpiHodG4YxhNM="
  ];

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
    unstablepkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    unikraft = {
      flake = false;
      url = "github:TUM-DSE/unibpf-unikraft/release-0.16.3-mpk";
    };

    lib-musl = {
      flake = false;
      url = "github:unikraft/lib-musl/stable";
    };
    musl = {
      flake = false;
      url = "file+https://www.musl-libc.org/releases/musl-1.2.3.tar.gz";
    };

    lib-libunwind = {
      flake = false;
      url = "github:unikraft/lib-libunwind/stable";
    };
    libunwind = {
      flake = false;
      url = "file+https://github.com/llvm/llvm-project/releases/download/llvmorg-14.0.6/libunwind-14.0.6.src.tar.xz";
    };

    lib-libcxxabi = {
      flake = false;
      url = "github:unikraft/lib-libcxxabi/stable";
    };
    libcxxabi = {
      flake = false;
      url = "file+https://github.com/llvm/llvm-project/releases/download/llvmorg-14.0.6/libcxxabi-14.0.6.src.tar.xz";
    };

    lib-libcxx = {
      flake = false;
      url = "github:unikraft/lib-libcxx/stable";
    };
    libcxx = {
      flake = false;
      url = "file+https://github.com/llvm/llvm-project/releases/download/llvmorg-14.0.6/libcxx-14.0.6.src.tar.xz";
    };

    lib-openssl = {
      flake = false;
      url = "github:unikraft/lib-openssl/stable";
    };
    openssl = {
      flake = false;
      url = "file+https://www.openssl.org/source/old/1.1.1/openssl-1.1.1c.tar.gz";
    };

    lib-compiler-rt = {
      flake = false;
      url = "github:unikraft/lib-compiler-rt/stable";
    };
    compiler-rt = {
      flake = false;
      url = "file+https://github.com/llvm/llvm-project/releases/download/llvmorg-14.0.6/compiler-rt-14.0.6.src.tar.xz";
    };

    unikraft_click = {
      flake = false;
      url = "file+https://codeload.github.com/kohler/click/zip/a5384835a6cac10f8d44da4eeea8eaa8f8e6a0c2";
    };

    og-click = {
      url = "git+https://github.com/kohler/click.git";
      # url = "git+file:///home/okelmann/click";
      flake = false;
    };

    fastclick = {
      url = "git+https://github.com/tbarbette/fastclick.git";
      flake = false;
    };

    vmux.url = "github:vmuxIO/vmuxIO/dev/update-moongen";
  };

  outputs =
    {
      self,
      nixpkgs,
      unstablepkgs,
      flake-utils,
      ...
    }@inputs:
    (flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        unstable = unstablepkgs.legacyPackages.${system};
        flakepkgs = self.packages.${system};
        rustToolchain =
          with inputs.fenix.packages.${system};
          combine [
            latest.cargo
            latest.rustc
            latest.rust-src
            latest.rust-std
            latest.clippy
            latest.rustfmt
            #targets.x86_64-unknown-linux-musl.stable.rust-std
            # fenix.packages.x86_64-linux.targets.aarch64-unknown-linux-gnu.latest.rust-std
          ];
        bpfDeps =
          pkgs:
          (with pkgs; [
            # ((bpf-linker.override { rustPlatform = makeRustPlatform {
            #     cargo = rustToolchain;
            #     rustc = rustToolchain;
            #     # stdenv = (overrideCC stdenv rustToolchain);
            # };}).overrideAttrs (final: old: {
            #     nativeBuildInputs = old.nativeBuildInputs ++ [ rustToolchain ];
            #     fixupPhase = ''
            #         ls $out/bin
            #         patchelf --add-rpath ${zlib}/lib $out/bin/bpf-linker $out/bin/bpf-linker
            #         patchelf --add-rpath ${ncurses}/lib $out/bin/bpf-linker $out/bin/bpf-linker
            #         patchelf --add-rpath ${libxml2}/lib $out/bin/bpf-linker $out/bin/bpf-linker
            #         # patchelf --add-rpath ${rustToolchain}/lib $out/bin/bpf-linker $out/bin/bpf-linker
            #         patchelf --add-rpath ${libgcc.lib}/lib $out/bin/bpf-linker $out/bin/bpf-linker
            #     '';
            # }))
          ]);
        buildDeps =
          pkgs:
          (with pkgs; [
            pkg-config
            gnumake
            flex
            bison
            git
            wget
            libuuid
            gcc
            # qemu
            (qemu_kvm.overrideAttrs (
              new: old: {
                patches =
                  old.patches
                  ++ [
                  ];
              }
            ))
            cmake
            unzip
            clang
            openssl
            ncurses
            bridge-utils
            python3
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
            (pkgs.runCommand "gcc-nm" { } ''
              # only bring in gcc-nm from libgcc.out, because it otherwise prevents crt1.so from musl to be found
              mkdir -p $out/bin
              cp ${pkgs.libgcc.out}/bin/gcc-nm $out/bin
              cp -r ${pkgs.libgcc.out}/libexec/ $out/
            '')
            gdb
            bpftrace
          ]);
        unikraftDeps =
          pkgs:
          (
            with pkgs;
            [
            ]
          );
        prevailDeps =
          pkgs:
          (with pkgs; [
            gcc
            git
            cmake
            boost
            yaml-cpp
          ]);
        make-disk-image = import (./nix/make-disk-image.nix);
      in
      {
        packages = {
          unikraft = pkgs.callPackage ./nix/unikraft.nix {
            inherit pkgs;
            inherit unstable;
            inherit inputs;
            unikraftDeps = (buildDeps pkgs) ++ (unikraftDeps pkgs) ++ (prevailDeps pkgs);
          };

          guest-image = make-disk-image {
            config = self.nixosConfigurations.guest.config;
            inherit (pkgs) lib;
            inherit pkgs;
            format = "qcow2";
          };

          click = pkgs.callPackage ./nix/click.nix {
            linux = pkgs.linuxPackages_6_6.kernel;
            selfpkgs = flakepkgs;
            inherit self;
          };

          fastclick = pkgs.callPackage ./nix/fastclick.nix {
            linux = pkgs.linuxPackages_6_6.kernel;
            selfpkgs = flakepkgs;
            inherit self;
          };

          dpdk20 = pkgs.callPackage ./nix/dpdk20.nix {
            kernel = pkgs.linuxPackages_5_10.kernel;
            inherit (flakepkgs) linux-firmware-pinned;
          };

          dpdk23 = pkgs.callPackage ./nix/dpdk23.nix {
            kernel = pkgs.linuxPackages_6_6.kernel;
            inherit (flakepkgs) linux-firmware-pinned;
          };

          dpdk24 = pkgs.callPackage ./nix/dpdk24.nix {
            kernel = pkgs.linuxPackages_6_6.kernel;
            inherit (flakepkgs) linux-firmware-pinned;
          };

          vpp = unstable.vpp.override { dpdk = flakepkgs.dpdk24; };
          vpp2 = unstable.vpp.override { dpdk = flakepkgs.dpdkX; };

          dpdkX = unstable.dpdk.overrideAttrs (
            new: old: {
              postPatch =
                old.postPatch
                + ''
                  substituteInPlace drivers/net/ice/ice_ethdev.h \
                    --replace '#define ICE_PKG_FILE_DEFAULT "/lib/firmware/intel/ice/ddp/ice.pkg"' \
                    '#define ICE_PKG_FILE_DEFAULT "${flakepkgs.linux-firmware-pinned}/lib/firmware/intel/ice/ddp/ice-1.3.26.0.pkg"'
                  substituteInPlace drivers/net/ice/ice_ethdev.h --replace \
                    '#define ICE_PKG_FILE_SEARCH_PATH_DEFAULT "/lib/firmware/intel/ice/ddp/"' \
                    '#define ICE_PKG_FILE_SEARCH_PATH_DEFAULT "${flakepkgs.linux-firmware-pinned}/lib/firmware/intel/ice/ddp/"'
                '';
            }
          );

          moongen-lachnit = inputs.vmux.packages.${system}.moongen-lachnit;

          linux-pktgen = pkgs.callPackage ./nix/linux-pktgen.nix {
            kernel = pkgs.linuxPackages_6_6.kernel;
          };

          linux-firmware-pinned = (
            pkgs.linux-firmware.overrideAttrs (
              old: new: {
                src = fetchGit {
                  url = "git://git.kernel.org/pub/scm/linux/kernel/git/firmware/linux-firmware.git";
                  ref = "main";
                  rev = "8a2d811764e7fcc9e2862549f91487770b70563b";
                };
                version = "8a2d81";
                outputHash = "sha256-dVvfwgto9Pgpkukf/IoJ298MUYzcsV1G/0jTxVcdFGw=";
              }
            )
          );
        };

        devShells = {
          default = pkgs.mkShell {
            name = "devShell";
            RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
            buildInputs =
              (bpfDeps pkgs)
              ++ (buildDeps pkgs)
              ++ (prevailDeps pkgs)
              ++ [
                unstable.kraft
                unstable.bmon
                unstable.gh
                unstable.just
                unstable.bridge-utils
                unstable.ack
                rustToolchain

                # deps for tests
                (pkgs.python3.withPackages (ps: [
                  # deps for tests/autotest
                  ps.colorlog
                  ps.netaddr
                  ps.pandas
                  ps.tqdm
                  ps.requests
                  ps.argcomplete

                  # dependencies for hosts/prepare.py
                  ps.pyyaml

                  # deps for deathstarbench/socialNetwork
                  ps.aiohttp

                  # linting
                  ps.black
                  ps.flake8
                  ps.isort
                  ps.mypy
                ]))

              ];
            KRAFTKIT_NO_WARN_SUDO = "1";
            KRAFTKIT_NO_CHECK_UPDATES = "true";
          };
          fhs =
            (pkgs.buildFHSEnv {
              name = "devShell";
              targetPkgs =
                pkgs:
                (
                  (buildDeps pkgs)
                  ++ (prevailDeps pkgs)
                  ++ [
                    unstable.kraft
                    unstable.rustup
                    unstable.bmon
                    unstable.gh
                    unstable.just
                  ]
                );
              runScript = "bash";
              # KRAFTKIT_NO_WARN_SUDO = "1";
              # KRAFTKIT_NO_CHECK_UPDATES = "true";
            }).env;
          fhsMake =
            (pkgs.buildFHSEnv {
              name = "devShell";
              targetPkgs =
                pkgs:
                (
                  (buildDeps pkgs)
                  ++ (prevailDeps pkgs)
                  ++ [
                    unstable.kraft
                    unstable.rustup
                    unstable.bmon
                    unstable.gh
                    unstable.just
                  ]
                );
              runScript = "bash -c \"KRAFTKIT_NO_CHECK_UPDATES=true make\"";
              # runScript = "bash -c \"NIX_LDFLAGS=' --trace-symbol=pkey_mprotect ' KRAFTKIT_NO_CHECK_UPDATES=true make\"";
              # runScript = "bash -c \"NIX_LDFLAGS=' --trace-symbol=pkey_mprotect --verbose=1 ' KRAFTKIT_NO_CHECK_UPDATES=true make\"";
            }).env;
        };

      }
    ))
    // (
      let
        pkgs = nixpkgs.legacyPackages.x86_64-linux;
        flakepkgs = self.packages.x86_64-linux;
      in
      {
        nixosConfigurations = {
          guest = nixpkgs.lib.nixosSystem {
            system = "x86_64-linux";
            modules = [
              (import ./nix/guest-config.nix {
                inherit pkgs;
                inherit (pkgs) lib;
                inherit flakepkgs;
              })
              ./nix/nixos-generators-qcow.nix
            ];
          };
        };

        # checks used by CI (buildbot)
        checks =
          let
            system = "x86_64-linux";
            nixosMachines = pkgs.lib.mapAttrs' (
              name: config: pkgs.lib.nameValuePair "nixos-${name}" config.config.system.build.toplevel
            ) ((pkgs.lib.filterAttrs (_: config: config.pkgs.system == system)) self.nixosConfigurations);
            blacklistPackages = [ ];
            packages = pkgs.lib.mapAttrs' (n: pkgs.lib.nameValuePair "package-${n}") (
              pkgs.lib.filterAttrs (n: _v: !(builtins.elem n blacklistPackages)) self.packages.x86_64-linux
            );
            homeConfigurations = pkgs.lib.mapAttrs' (
              name: config: pkgs.lib.nameValuePair "home-manager-${name}" config.activation-script
            ) (self.legacyPackages.x86_64-linux.homeConfigurations or { });
          in
          nixosMachines // packages // homeConfigurations;
      }
    );
}
