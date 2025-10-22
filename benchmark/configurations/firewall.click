from :: FromDevice(0)
 -> Print('Received packet from device') 
 -> IPFilter(
    allow dst port 1000,
    deny dst port 1001,
 )
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
