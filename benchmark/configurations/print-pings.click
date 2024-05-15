// From: https://github.com/kohler/click/blob/master/conf/print-pings.click
// Slightly modified to be compatible with Click on Unikraft. Functionality not guaranteed.

FromDevice				// read packets from device
						// (assume Ethernet device)
   -> Print('Received packet from device')
   -> Classifier(12/0800)			// select IP-in-Ethernet
   -> Strip(14)					// strip Ethernet header
   -> CheckIPHeader				// check IP header, mark as IP
   -> IPFilter(allow icmp && icmp type echo)	// select ICMP echo requests
   -> IPPrint					// print them out
   -> Discard;