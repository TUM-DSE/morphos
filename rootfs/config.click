FromDevice
  -> Print('Running packet through BPFilter')
  -> IPPrint(CONTENTS HEX)
  -> BPFilter(filter-rs)
  -> Print('Packet passed BPFilter')
  -> ToDevice;
