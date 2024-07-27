FromDevice(0)
 -> c1 :: Classifier(12/0806 20/0001,
                     12/0800);

// Answer ARP requests
c1[0] -> ARPResponder(172.44.0.5 $MAC0)
      -> ToDevice(0);

// Handle IP Packets
c1[1] -> processing :: Strip(14)
 -> CheckIPHeader
 -> DropBroadcasts
 -> ipgw :: IPGWOptions(172.44.0.5)
 -> FixIPSrc(172.44.0.5)
 -> ttl :: DecIPTTL
 -> frag :: IPFragmenter(1500)
 -> Print('Hello World (app-2)')
 -> Discard;

ipgw[1] -> processing;
ttl[1] -> processing;
frag[1] -> processing;