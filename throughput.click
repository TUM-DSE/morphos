
// need this to initialize the device 0
FromDevice(0)
// -> Print("rx")
-> ic0 :: AverageCounter()
-> Discard;
// encap_then_out :: EtherEncap(0x0800, $MAC0, 76:7E:90:D4:98:54) -> tx:: ToDevice(0);

// InfiniteSource(DATA \<0800>, LENGTH 1460, LIMIT -1, BURST 100000)
// -> ic0 :: AverageCounter()
// -> UDPIPEncap(172.44.0.2, 5678, 172.44.0.1, 5678)
// -> encap_then_out;

Script(TYPE ACTIVE,
 				wait 5ms,
				label start,
				print "Rx rate: $(ic0.count)",
				write ic0.reset 1,
				wait 1s,
				goto start
				)

