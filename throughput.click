
// need this to initialize the device 0
from :: FromDevice(0)
// -> Print("rx")
-> ic0 :: AverageCounter()
-> Discard;
// encap_then_out :: EtherEncap(0x0800, $MAC0, 76:7E:90:D4:98:54) -> tx:: ToDevice(0);
// encap_then_out :: EtherEncap(0x0800, $MAC0, B4:96:91:B3:8B:04) -> tx:: ToDevice(0);

// if :: InfiniteSource(DATA \<0800>, LENGTH 1460, LIMIT -1, BURST 100000, ACTIVE 0)
// -> ic0 :: AverageCounter()
// -> UDPIPEncap(172.44.0.2, 5678, 172.44.0.1, 5678)
// -> encap_then_out;

Script(TYPE ACTIVE,
 				wait 1s,
// 				write if.active 1,
				label start,
				print "Rx rate: $(ic0.rate)   #$(ic0.count)",
				write ic0.reset 1,
				write from.reset 1,
				wait 1s,
				goto start
				)

