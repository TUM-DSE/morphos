// === Control network ===
elementclass ControlReceiver { $deviceid |
    FromDevice($deviceid)
     -> c0 :: Classifier(12/0806 20/0001,
                         12/0800,
                         -);

    // Answer ARP requests
    c0[0] -> ARPResponder(173.44.0.2 $MAC1)
          -> ToDevice($deviceid);

    // Handle IP packets
    c0[1] -> StripEtherVLANHeader
     -> CheckIPHeader
     -> IPFilter(allow dst port 4444, deny all)
     -> IPReassembler
     -> SetUDPChecksum
     -> CheckUDPHeader
     -> Control;

    c0[2] -> Discard;
}

ControlReceiver(1);

// === Data network ===
FromDevice(0)
 -> c1 :: Classifier(12/0806 20/0001,
                     12/0806 20/0002,
                     12/0800);

// Answer ARP requests
c1[0] -> ARPResponder(172.44.0.3 $MAC0)
      -> ToDevice(0);

// Handle IP packets
arp_q :: ARPQuerier(172.44.0.3, $MAC0)
      -> ToDevice(0);

c1[1] -> [1]arp_q;

lb :: BPFClassifier(ID 1, FILE round-robin, SIGNATURE round-robin.sig, JIT true);
lb[0] -> SetIPAddress(172.44.0.4)
      -> Print('Sending to output 0', MAXLENGTH 6)
      -> [0]arp_q;

lb[1] -> SetIPAddress(172.44.0.5)
      -> Print('Sending to output 1', MAXLENGTH 6)
      -> [0]arp_q;

// Handle IP Packets
c1[2] -> processing :: Strip(14)
 -> CheckIPHeader
 -> DropBroadcasts
 -> ipgw :: IPGWOptions(172.44.0.3)
 -> FixIPSrc(172.44.0.3)
 -> ttl :: DecIPTTL
 -> frag :: IPFragmenter(1500)
 -> Print('Received packet (load-balancer)')
 -> lb;

ipgw[1] -> arp_q[0];
ttl[1] -> processing;
frag[1] -> processing;