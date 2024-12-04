/*
 * This file implements a fast L2 UDP packet generator
 *
 * This can be used to test the throughput of a DUT, using a receiver-l2.click
 * on some other end of a switch
 *
 * A launch line would be :
 *   sudo bin/click -c 0x1 -n 4 -- conf/fastclick/pktgen-l2.click L=60 S=1000000 N=100
 */

//Default values for packet length, number of packets and amountfs of time to replay them
define($L 60, $R 100, $S 100000);

//You do not need to change these to the real ones, just have the dmac match the receiver's one
define($mymac 90:e2:ba:c3:79:66)
define($dmac 90:e2:ba:c3:76:6e)
//Ip are just for a convenient payload as this is l2
define($myip 192.168.130.13)
define($dstip 192.168.128.13)

//Explained in loop.click
define($verbose 3)
define($blocking true)


InfiniteSource(DATA \<0800>, LENGTH 1460, LIMIT -1, BURST 100000)
-> UDPIPEncap($myip, 5678, $dstip, 5678)
-> EtherEncap(0x0800, $mymac, $dmac)
-> ic0 :: AverageCounter()
-> ToDevice(ens7);



// //###################
// // TX
// //###################
// //Create a UDP flow
// FastUDPFlows(RATE 0, LIMIT -1, LENGTH $L, SRCETH $mymac, DSTETH $dmac, SRCIP $myip, DSTIP $dstip, FLOWS 1, FLOWSIZE $S)
// -> ic0 :: AverageCounter()
// -> td :: ToDevice(ens7)

//###################
// RX
//###################
fd :: FromDevice(ens7) -> Print('rx') -> Discard

Script(TYPE ACTIVE,
 				wait 5ms,
				label start,
				print "Number of packets sent : $(ic0.rate)",
				wait 1s,
				goto start
				)

