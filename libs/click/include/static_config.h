static char CONFIGSTRING[] = "    define($IP 10.0.10.123);\n\
\n\
    source :: FromDevice;\n\
    sink   :: ToDevice;\n\
    // classifies packets \n\
    c :: Classifier(\n\
        12/0806 20/0001, // ARP Requests goes to output 0\n\
        12/0806 20/0002, // ARP Replies to output 1\n\
        12/0800 14/45 34/08, // ICMP Requests to output 2\n\
        -); // without a match to output 3\n\
\n\
    arpq :: ARPQuerier($IP, $MAC0);\n\
    arpr :: ARPResponder($IP $MAC0);\n\
\n\
    source -> c;\n\
    c[0] -> CheckARPHeader(14) -> ARPPrint -> arpr -> ARPPrint -> sink;\n\
    c[1] -> [1]arpq;\n\
    Idle -> [0]arpq;\n\
    arpq -> sink;\n\
    c[2] -> CheckIPHeader(14) -> IPPrint -> ICMPPingResponder() -> EtherMirror() -> IPPrint -> sink;\n\
    c[3] -> Discard;";
