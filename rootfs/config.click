// === Control network ===
FromDevice(1)
 -> c0 :: Classifier(12/0806 20/0001,
                     12/0800,
                     -);

// Answer ARP requests
c0[0] -> ARPResponder(173.44.0.2 $MAC1)
      -> ToDevice(1);

// Handle IP packets
c0[1] -> StripEtherVLANHeader
 -> CheckIPHeader
 -> IPFilter(allow dst port 4444, deny all)
 -> IPReassembler
 -> SetUDPChecksum
 -> CheckUDPHeader
 -> Control;

c0[2] -> Discard;

// === Data network ===
FromDevice(0)
  -> BPFilter(filter-rs)
  -> ToDevice(0);
