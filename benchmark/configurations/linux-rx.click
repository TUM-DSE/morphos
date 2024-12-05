FromDevice(ens7)
-> ic0 :: AverageCounter()
-> Discard;

Script(TYPE ACTIVE,
    wait 5ms,
    label start,
    print "Rx rate: $(ic0.count)",
    write ic0.reset 1,
    wait 1s,
    goto start
    )
