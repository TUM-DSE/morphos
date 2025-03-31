from :: FromDevice(0)

            // stripping only makes sense, once we've looked at the ethernet header
            -> Classifier(12/0800)
            // check ip header doesn't want ethernet header
            -> Strip(14)
            // some elements like IPFilter segfault with some packets if we don't check them
            -> CheckIPHeader
-> Print("rx", MAXLENGTH 128)
        -> BPFilter(ID 1, FILE stringmatcher, SIGNATURE stringmatcher.sig, JIT true)
-> Print("dr")
-> ic0 :: AverageCounter()
-> Discard;

Script(TYPE ACTIVE,
       wait 5ms,
       label start,
       // print "Rx rate: $(ic0.rate)",
       write ic0.reset 1,
       write from.reset 1,
       wait 1s,
       goto start
       )
