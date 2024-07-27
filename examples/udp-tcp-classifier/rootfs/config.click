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
                     12/0800,
                     -);

// Answer ARP requests
c1[0] -> ARPResponder(172.44.0.2 $MAC0)
      -> ToDevice(0);

// Handle IP Packets
c1[1] -> bpf_classifier :: BPFClassifier(FILE udp-tcp-classifier, SIGNATURE udp-tcp-classifier.sig);
bpf_classifier[0] -> Print('UDP packet received')
                  -> ToDevice(0);

bpf_classifier[1] -> Print('TCP packet received')
                  -> ToDevice(0);

bpf_classifier[2] -> Print('Packet received, but not UDP or TCP')
                  -> ToDevice(0);

c1[2] -> Discard;