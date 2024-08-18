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
c1[0] -> ARPResponder(172.44.0.2 $MAC0)
      -> ToDevice(0);

// Handle IP packets
arp_q :: ARPQuerier(172.44.0.2, $MAC0)
      -> ToDevice(0);

c1[1] -> [1]arp_q;

// Handle IP Packets
c1[2] -> processing :: Strip(14)
 -> CheckIPHeader
 -> DropBroadcasts
 -> ipgw :: IPGWOptions(172.44.0.2)
 -> FixIPSrc(172.44.0.2)
 -> ttl :: DecIPTTL
 -> frag :: IPFragmenter(1500)
 -> Print('Received packet (firewall)')
 -> BPFilter(ID 1, FILE target-port, SIGNATURE target-port.sig, JIT true)
 -> BPFilter(ID 2, FILE rate-limiter, SIGNATURE rate-limiter.sig, JIT true)
 -> SetIPAddress(172.44.0.3)
 -> [0]arp_q;

ipgw[1] -> processing;
ttl[1] -> processing;
frag[1] -> processing;

FromDevice(0)
 -> c0 :: Classifier(12/0806 20/0001, // ARP
                     12/0800,         // IP
                     -);              // Everything else

// Answer ARP requests
c0[0] -> ARPResponder(173.44.0.2 $MAC1)
      -> ToDevice(0);

// Print IP packets
c0[1] -> StripEtherVLANHeader
 -> CheckIPHeader
 -> Print('Received packet')
 -> Discard;

c0[2] -> Discard;