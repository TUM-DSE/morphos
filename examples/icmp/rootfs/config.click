// === Data network ===
FromDevice(0)
 -> c1 :: Classifier(12/0806 20/0001,
                     12/0800);

// Answer ARP requests
c1[0] -> ARPResponder(172.44.0.2 $MAC0)
      -> ToDevice(0);

// Handle IP Packets
c1[1] -> CheckIPHeader(14)
 // -> BPFilter(ID 1, FILE pass, SIGNATURE pass.sig)
 -> ICMPPingResponder()
 -> EtherMirror()
 -> ToDevice(0);
