// === Control network ===
elementclass ControlReceiver { $deviceid |
    FromDevice($deviceid)
     -> c0 :: Classifier(12/0806 20/0001,
                         12/0800,
                         -);

    // Answer ARP requests
    c0[0] -> ARPResponder(173.44.0.2 $MAC2)
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

ControlReceiver(2);

// === Input ===
FromDevice(0)
 -> c1 :: Classifier(12/0806 20/0001,
                     12/0800,
                     -);

// Answer ARP requests
c1[0] -> ARPResponder(172.44.0.2 $MAC0)
      -> ToDevice(0);

// Handle IP Packets
c1[1] -> StripEtherVLANHeader
 -> CheckIPHeader
 -> IPReassembler
 -> Print('Received packet (pre-filtering)')
 // -> BPFilter(ID 1, FILE target-port, SIGNATURE target-port.sig, JIT true, DUMP_JIT false)
 -> Print('Received packet (post-filtering)')
 -> ToDevice(1);

c1[2] -> Discard;

// === Output ===
FromDevice(1)
 -> c2 :: Classifier(12/0806 20/0001,
                     12/0800,
                     -);

// Answer ARP requests
c2[0] -> ARPResponder(174.44.0.2 $MAC1)
      -> ToDevice(1);

// Handle IP Packets
c2[1] -> StripEtherVLANHeader
 -> CheckIPHeader
 -> IPReassembler
 -> CheckUDPHeader
 -> Print('Received packet on output')
 -> ToDevice(1);

c2[2] -> Discard;
