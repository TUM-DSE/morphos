from :: FromDevice(0)
 -> Print('Received packet from device') 
 -> BPFilter(ID 1, FILE firewall-2, SIGNATURE firewall-2.sig, JIT true)
 -> Discard;

Script(TYPE ACTIVE,
       print "sleeping first increases startup time"
       print "sleeping first increases startup time"
       print "sleeping first increases startup time"
       print "sleeping first increases startup time"
       wait 5ms,
       label start,
       wait 1s,
       goto start
       )
