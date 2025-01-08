# NOTE THIS IS STILL BROKEN
{ self
, pkgs
, lib
, selfpkgs
, ...
}:
let
  # dpdk = pkgs.dpdk;
  dpdk = selfpkgs.dpdk20; # click doesn't build with dpdk >= 22 as of 2024.11.29
  # dpdk = self.inputs.nixpkgs.legacyPackages.x86_64-linux.dpdk; # needed to build with flow-api
  debug = false;
in
pkgs.stdenv.mkDerivation rec {
  pname = "click";
  version = "2024.01.15-21";

  src = self.inputs.og-click;
  thisgit = ../.;

  nativeBuildInputs = with pkgs; [
    coreutils
    perl
    python3Packages.pyelftools
    openssl
    (writeScriptBin "git" ''
      echo ignoring git command
    '')
  ];
  buildInputs = with pkgs; [
    openssl
    libbsd
    numactl
    luajit
    libpcap
    hyperscan
    jansson
  ] ++ [ dpdk ];

  postPatch = ''
    # sln /bin/echo ${pkgs.coreutils}/bin/echo
    find . -type f -exec sed -i 's/\/bin\/echo/echo/g' {} \;
    find . -type f -exec sed -i 's/\/bin\/rm/rm/g' {} \;
    find . -type f -exec sed -i 's/\/bin\/ln/ln/g' {} \;
    # substituteInPlace ./userlevel/Makefile.in \
    #   --replace "/bin/echo" "echo"

    cp ${thisgit}/libs/click/unikraft/ipfilter2.* ./elements/ip/
    cp ${thisgit}/libs/click/unikraft/stringmatcher.* ./elements/ip/
    cp ${thisgit}/libs/click/unikraft/replace.* ./elements/ip/
    cp ${thisgit}/libs/click/unikraft/node.* ./elements/ip/
    cp ${thisgit}/libs/click/unikraft/mpool.* ./elements/ip/
    cp ${thisgit}/libs/click/unikraft/ahocorasickplus.* ./elements/ip/
    cp ${thisgit}/libs/click/unikraft/ahocorasick.* ./elements/ip/
    cp ${thisgit}/libs/click/unikraft/actypes.* ./elements/ip/

    mkdir -p $RTE_SDK_BIN
    cp -r ${dpdk}/* $RTE_SDK_BIN/
      '';

    /*
    # some variables are not correctly subsituted with our dpdk install. Substitute the values.
    substituteInPlace ./userlevel/Makefile.in \
      --replace "@RTE_VER_MINOR@" "0"
    substituteInPlace ./userlevel/Makefile.in \
      --replace "@RTE_VER_YEAR@" "22"
    substituteInPlace ./userlevel/Makefile.in \
      --replace "@RTE_VER_MONTH@" "11"


    # fastlick FromDPDKDevice(FLOW_RULES_FILE) requires dpdk 21.11 or older.
    # We attempt to patch fastclick for dpdk 22.
    substituteInPlace ./include/click/flowruleparser.hh \
      --replace "main_ctx" "builtin_ctx"
    # patch needed for dpdk 23.
    substituteInPlace ./lib/flowrulemanager.cc \
      --replace "(const uint32_t *) int_rule_ids" "(const uint64_t *) int_rule_ids, true"
    */

  RTE_SDK = "/build/rte_sdk";
  RTE_SDK_BIN = "/build/rte_sdk";
  RTE_TARGET = "x86_64-native-linuxapp-gcc";
  # RTE_KERNELDIR = "${pkgs.linux.dev}/lib/modules/${pkgs.linux.modDirVersion}/build";

  shellHook = ''
    export RTE_SDK=$(pwd)/rte_sdk
    export RTE_SDK_BIN=$RTE_SDK/$RTE_TARGET
  '';

  configureFlags = [
    "--enable-all-elements"
    "--enable-etherswitch"
    # fastclick light config
    #"--enable-dpdk"
    "--enable-intel-cpu"
    "--verbose"
    "--enable-select=poll"
    "--disable-dynamic-linking"
    "--enable-poll"
    "--enable-bound-port-transfer"
    "--enable-local"
    #"--enable-flow"
    #"--disable-task-stats"
    #"--disable-cpu-load"
    #"--enable-dpdk-packet"
    #"--disable-clone"
    # "--disable-dpdk-softqueue" # enable IQUEUE for debugging

    # added by me
    #"--disable-sse42"
    #"--enable-flow-api"

    # middleclick
    "--enable-multithread"
    "--disable-linuxmodule"
    "--enable-intel-cpu"
    "--enable-user-multithread"
    "--disable-dynamic-linking"
    "--enable-poll"
    "--enable-bound-port-transfer"
    "--enable-dpdk"
    #"--enable-batch"
    "--with-netmap=no"
    #"--enable-zerocopy"
    #"--disable-dpdk-pool"
    #"--disable-dpdk-packet"
    #"--enable-user-timestamp"
    #"--enable-flow"
    #"--enable-ctx"
    # runtime tells me to add:
    #"--enable-flow-dynamic"

  ];

  #CFLAGS = "-msse4.1 -mavx" + lib.optionalString (!debug) " -O3" + lib.optionalString debug " -g";
  CFLAGS = "-msse4.1 -mavx -O3";
  #CXXFLAGS = "-std=c++11 -msse4.1 -mavx" + lib.optionalString (!debug) " -O3" + lib.optionalString debug " -g";
  CXXFLAGS = "-std=c++11 -msse4.1 -mavx -O3";
  NIX_LDFLAGS = "-lrte_eal -lrte_ring -lrte_mempool -lrte_ethdev -lrte_mbuf -lrte_net -lrte_latencystats -lrte_cmdline -lrte_net_bond -lrte_metrics -lrte_gso -lrte_gro -lrte_net_ixgbe -lrte_net_i40e -lrte_net_bnxt -lrte_net_dpaa -lrte_bpf -lrte_bitratestats -ljansson -lbsd";
  #RTE_VER_YEAR = "21"; # does this bubble through to the makefile variable? i dont think so. Then we can remove it.
  enableParallelBuilding = true;
  hardeningDisable = [ "all" ];
  preBuild = ''
    echo foobar
    echo $enableParallelBuilding
    gzip --version
  '';

  dontFixup = debug;
  dontStrip = debug;
}
