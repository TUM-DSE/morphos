{ self
, pkgs
, lib
, selfpkgs
, ...
}:
let
  dpdk = selfpkgs.dpdk23; # needed for ice package thingy
  # dpdk = self.inputs.nixpkgs.legacyPackages.x86_64-linux.dpdk; # needed to build with flow-api
  debug = false;
in
pkgs.stdenv.mkDerivation {
  pname = "fastclick";
  version = "2024.01.15-21";

  src = self.inputs.fastclick;

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
    dpdk
    hyperscan
    jansson
  ];

  patches = [
    ./fastclick.per-thread-counter.patch
    ./fastclick.dpdk23.patch # patch needed for FromDPDKDevice(FLOW_RULES_FILE) with dpdk 23
  ];

  postPatch = ''
    # sln /bin/echo ${pkgs.coreutils}/bin/echo
    find . -type f -exec sed -i 's/\/bin\/echo/echo/g' {} \;
    find . -type f -exec sed -i 's/\/bin\/rm/rm/g' {} \;
    find . -type f -exec sed -i 's/\/bin\/ln/ln/g' {} \;
    # substituteInPlace ./userlevel/Makefile.in \
    #   --replace "/bin/echo" "echo"

    # some variables are not correctly subsituted with our dpdk install. Substitute the values.
    substituteInPlace ./userlevel/Makefile.in \
      --replace "@RTE_VER_MINOR@" "0"
    substituteInPlace ./userlevel/Makefile.in \
      --replace "@RTE_VER_YEAR@" "22"
    substituteInPlace ./userlevel/Makefile.in \
      --replace "@RTE_VER_MONTH@" "11"

    mkdir /build/rte_sdk
    cp -r ${dpdk}/* /build/rte_sdk

    # fastlick FromDPDKDevice(FLOW_RULES_FILE) requires dpdk 21.11 or older.
    # We attempt to patch fastclick for dpdk 22. 
    substituteInPlace ./include/click/flowruleparser.hh \
      --replace "main_ctx" "builtin_ctx"
    # patch needed for dpdk 23.
    substituteInPlace ./lib/flowrulemanager.cc \
      --replace "(const uint32_t *) int_rule_ids" "(const uint64_t *) int_rule_ids, true"

  '';

  RTE_SDK = "/build/rte_sdk";
  # RTE_KERNELDIR = "${pkgs.linux.dev}/lib/modules/${pkgs.linux.modDirVersion}/build";

  configureFlags = [
    "--enable-all-elements"
    "--enable-etherswitch"
    # fastclick light config
    "--enable-dpdk"
    "--enable-intel-cpu"
    "--verbose"
    "--enable-select=poll"
    "--disable-dynamic-linking"
    "--enable-poll"
    "--enable-bound-port-transfer"
    "--enable-local"
    "--enable-flow"
    "--disable-task-stats"
    "--disable-cpu-load"
    "--enable-dpdk-packet"
    "--disable-clone"
    # "--disable-dpdk-softqueue" # enable IQUEUE for debugging

    # added by me
    "--disable-sse42"
    "--enable-flow-api"

    # middleclick
    "--enable-multithread"
    "--disable-linuxmodule"
    "--enable-intel-cpu"
    "--enable-user-multithread"
    "--disable-dynamic-linking"
    "--enable-poll"
    "--enable-bound-port-transfer"
    "--enable-dpdk"
    "--enable-batch"
    "--with-netmap=no"
    "--enable-zerocopy"
    "--disable-dpdk-pool"
    "--disable-dpdk-packet"
    "--enable-user-timestamp"
    "--enable-flow"
    "--enable-ctx"
    # runtime tells me to add:
    "--enable-flow-dynamic"

  ];

  CFLAGS = "-msse4.1 -mavx" + lib.optionalString (!debug) " -O3" + lib.optionalString debug " -g";
  CXXFLAGS = "-std=c++11 -msse4.1 -mavx" + lib.optionalString (!debug) " -O3" + lib.optionalString debug " -g";
  NIX_LDFLAGS = "-lrte_eal -lrte_ring -lrte_mempool -lrte_ethdev -lrte_mbuf -lrte_net -lrte_latencystats -lrte_cmdline -lrte_net_bond -lrte_metrics -lrte_gso -lrte_gro -lrte_net_ixgbe -lrte_net_i40e -lrte_net_bnxt -lrte_net_dpaa -lrte_bpf -lrte_bitratestats -ljansson -lbsd";
  RTE_VER_YEAR = "21"; # does this bubble through to the makefile variable? i dont think so. Then we can remove it.
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
