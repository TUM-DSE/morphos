FromDevice
  -> Print('Running packet through BPFilter')
  -> IPPrint
  -> EtherMirror
  -> BPFilter("filter.o")
  -> ToDevice;
