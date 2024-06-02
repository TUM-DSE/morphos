// need this to initialize the device 0
FromDevice(0) -> Discard;

// bmon -p 'clicknet' -o format:fmt='rx=$(attr:rx:packets) rx_pps=$(attr:rxrate:packets)\n' -r 0.1 -R 1
// baseline: 50kpps
// bpfilter pass: 50kpps
// ipfilter: 50kpps

InfiniteSource(DATA <>, LENGTH 0, BURST 10000)
-> UDPIPEncap(172.44.0.2, 5678, 172.44.0.1, 5678)
-> EtherEncap(0x0800, $MAC0, 76:7e:90:d4:98:54)
// -> IPFilter(deny src port 1234, allow all)
//-> BPFilter(ID 1, FILE filter-rs)
-> ToDevice(0)