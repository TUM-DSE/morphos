// https://github.com/tbarbette/fastclick/blob/main/conf/switch/switch-2ports-vanilla.click
// Slightly modified to be compatible with Click on Unikraft. Functionality not guaranteed.

/*
 * Configuration for a simple 2-ports switch without STP support
 * it is intended for userlevel Click using standard Kernel I/O
 * (eg PCAP or Socket)
 * Eg launch with:
 *     click switch-2ports-vanilla.click dev1=eth0 dev2=eth1
 * or omit dev1 and dev2 parameters and change it below
 */


//Define the name of the ports
define( $dev1  0,
        $dev2  1,
        $print false)

s :: EtherSwitch;

w0 :: Queue -> ToDevice($dev1);
w1 :: Queue -> ToDevice($dev2);

elementclass Input { $label |
    input -> Print('Received packet from device') -> c :: Classifier(12/0800, 12/0806, -);
    c[0] -> Strip(14) ->  CheckIPHeader() -> IPPrint($label, ACTIVE $print) -> Unstrip(14) -> output;
    c[1] -> output;
    c[2] -> Print(NONIP) -> Discard;
}

FromDevice($dev1) -> Input(FROMH1) -> [0]s[0] -> w0;
FromDevice($dev2) -> Input(FROMH2) -> [1]s[1] -> w1;

DriverManager(wait 15s);