local mg     = require "moongen"
local memory = require "memory"
local device = require "device"
local ts     = require "timestamping"
local stats  = require "stats"
local hist   = require "histogram"
local timer  = require "timer"
local log    = require "log"
local dpdkc  = require "dpdkc"
local eth    = require "proto.ethernet"
local ffi = require "ffi"

local function getRstFile(...)
	local args = { ... }
	for i, v in ipairs(args) do
		result, count = string.gsub(v, "%-%-result%=", "")
		if (count == 1) then
			return i, result
		end
	end
	return nil, nil
end

function configure(parser)
	parser:description("Generates bidirectional CBR traffic with hardware rate control and measure latencies.")
	parser:argument("dev", "Device to transmit/receive from."):convert(tonumber)
	parser:argument("mac", "MAC address of the destination device. (bytes in inverse order)")
	parser:option("--srcIp", "Source IPv4 address."):default("10.0.0.1")
	parser:option("--dstIp", "Destination IPv4 address."):default("10.0.0.2")
	parser:option("--srcPort", "Source port."):default(2):convert(tonumber)
	parser:option("--dstPort", "Destination port."):default(3):convert(tonumber)
	parser:option("-r --rate", "Transmit rate in kpps."):default(14000):convert(tonumber)
  parser:option("-s --size", "Packet size in bytes."):default(60):convert(tonumber)
	parser:option("-f --file", "Filename of the latency histogram."):default("histogram.csv")
	parser:option("-c --csv", "Filename of the output csv."):default("")
	parser:option("-t --threads", "Number of threads per device."):args(1):convert(tonumber):default(1)
	parser:option("-T --time", "Time to transmit for in seconds."):default(60):convert(tonumber)
	parser:option("-m --macs", "Send in round robin to (ethDst...ethDst+macs)."):default(1):convert(tonumber)
	parser:option("-e --ethertypes", "Send in round robin to (0x1234...0x1234+ethertypes)."):default(1):convert(tonumber)
end

function master(args)
	local dev = device.config({port = args.dev, rxQueues = args.threads + 1, txQueues = args.threads + 1})
	device.waitForLinks()
	-- for i = 0, args.threads - 1 do
	--     dev:getTxQueue(i):setRate(args.rate * (args.size + 4) * 8 / 1000)
	--     mg.startTask("loadSlave",
	--       dev:getTxQueue(i),
	--     	dev:getMac(true),
	--       args.mac,
	--       args.srcIp,
	--       args.dstIp,
	--       args.srcPort,
	--       args.dstPort,
	--       args.size,
	--       args.macs,
	--       args.ethertypes
	--     )
 --  end
	-- -- warmup
	-- print("Wraming up...")
	-- -- mg.startSharedTask("timerSlave", dev:getTxQueue(args.threads), dev:getRxQueue(args.threads), args.mac, args.file)
	-- mg.startTask("txTimestampThread", dev:getTxQueue(args.threads), args.size, args.mac)
	-- mg.startTask("rxTimestamps", dev:getRxQueue(0), args.mac, "")
	-- mg.sleepMillis(5000)
	-- print("Starting benchmark...")
	-- mg:stop()
	-- mg.waitForTasks()
	mg.setRuntime(9999)

	-- for i = 0, args.threads - 1 do
	--     dev:getTxQueue(i):setRate(args.rate * (args.size + 4) * 8 / 1000)
	--     mg.startTask("loadSlave",
	--       dev:getTxQueue(i),
	--     	dev:getMac(true),
	--       args.mac,
	--       args.srcIp,
	--       args.dstIp,
	--       args.srcPort,
	--       args.dstPort,
	--       args.size,
	--       args.macs,
	--       args.ethertypes
	--     )
 --  end
	mg.startTask("txTimestampThread",
			dev:getTxQueue(args.threads),
	    dev:getMac(true),
	    args.mac,
	    args.srcIp,
	    args.dstIp,
	    args.srcPort,
	    args.dstPort,
	    args.size
	)
	mg.startTask("rxTimestamps", dev:getRxQueue(0), args.mac, args.file)
	-- start measuring
	stats.startStatsTask{dev}
	if args.csv ~= "" then
		stats.startStatsTask{devices={dev}, format="csv", file=args.csv}
	end

	if args.time >= 0 then
		runtime = timer:new(args.time)
		runtime:wait()
		mg:stop()
	end

	mg.waitForTasks()
end

function setMac(buf, mac_nr, small_offset)
  local pl = buf:getRawPacket().payload
  pl.uint8[0] = bit.band(mac_nr, 0xFF)
  pl.uint8[1] = bit.band(bit.rshift(mac_nr, 8), 0xFF)
  pl.uint8[2] = bit.band(bit.rshift(mac_nr, 16), 0xFF)
  pl.uint8[3] = bit.band(bit.rshift(mac_nr, 24), 0xFF)
  pl.uint8[4] = bit.band(bit.rshift(mac_nr + 0ULL, 32ULL), 0xFF)
  pl.uint8[5] = bit.band(bit.rshift(mac_nr + 0ULL, 40ULL), 0xFF) + small_offset
end

function setEthertype(buf, type)
  local pl = buf:getRawPacket().payload
  pl.uint8[13] = bit.band(type, 0xFF)
  pl.uint8[12] = bit.band(bit.rshift(type, 8), 0xFF)
end

function sendSimple(queue, bufs, pktSize)
	-- local rateLimit = timer:new(0.00001)
	while mg.running() do
		-- print("load")
    bufs:alloc(pktSize)
    queue:send(bufs)
		-- bufs:offloadIP4Checksums()
		-- bufs:offloadUdpChecksums()
   --  rateLimit:wait()
  	-- rateLimit:reset()
  end
end

function sendMacs(queue, bufs, pktSize, dstMac, numDstMacs, numEthertypes)
	if numDstMacs > 0xFF then
		error("Sending packets with this many mac addresses is unsupported!")
	end
	local mac_nr = parseMacAddress(dstMac, 1)
	while mg.running() do
    bufs:alloc(pktSize)
    for i, buf in ipairs(bufs) do -- usually there are 63 bufs
      -- local dst = mac_nr + ((i-1) % numDstMacs)
      local mac_offset = ((i-1) % numDstMacs)
      local type = 0x1234 + (math.floor((i-1) / numDstMacs) % (numEthertypes))
  		-- local e = buf:getEthPacket()
  		-- e.type = 0x123
      setMac(buf, mac_nr, mac_offset)
      setEthertype(buf, type)
      -- buf:dump()
    end
    queue:send(bufs)
  end
end

function loadSlave(queue, srcMac, dstMac, srcIp, dstIp, srcPort, dstPort, pktSize, numDstMacs, numEthertypes)
	-- local mac_nr = parseMacAddress(dstMac, 1)
	-- print(srcMac)
	-- print(mac_nr)
	local mem = memory.createMemPool(function(buf)
		buf:getUdpPacket():fill{
			ethSrc = srcMac,
			ethDst = dstMac,
			ethType = 0x0800,
			ip4Src = srcIp,
			ip4Dst = dstIp,
			portSrc = srcPort,
			portDst = dstPort,
			pktLength = pktSize,
		}
		buf:getIP4Packet().ip4:calculateChecksum()
		-- buf.getUdpPacket().udp:calculateChecksum() -- not implemented but also never checked by our click
	end)
	local bufs = mem:bufArray()
	if numDstMacs > 1 or numEthertypes > 1 then
		-- error("Sending to multiple MACs and ethertypes at the same time is not supproted.") -- no it is
		sendMacs(queue, bufs, pktSize, dstMac, numDstMacs, numEthertypes)
	else
			sendSimple(queue, bufs, pktSize)
	end
end

function txTimestampThread(txQueue, srcMac, dstMac, srcIp, dstIp, srcPort, dstPort, pktSize)
	local pktSize = 84
	print(pktSize)
	local mem = memory.createMemPool(function(buf)
		buf:getUdpPtpPacket():fill{
			ethSrc = txQueue,
			ethDst = dstMac,
			ethType = 0x0800,
			ip4Src = srcIp,
			ip4Dst = dstIp,
			udpSrc = srcPort,
			udpDst = 0x10,
			pktLength = pktSize,
		}
		buf:getIP4Packet().ip4:calculateChecksum()
	end)
	-- mg.sleepMillis(1000) -- ensure that the load task is running
	local rateLimit = timer:new(0.001) -- 1000 latency samples per second
	local bufs = mem:bufArray(1)
	while mg.running() do
    bufs:alloc(pktSize)
    -- bufs[1]:dump()
    txQueue:sendWithTimestamp(bufs) -- see for a full software example: https://github.com/emmericp/MoonGen/blob/master/examples/timestamping-tests/timestamps-software.lua
    rateLimit:wait()
  	rateLimit:reset()
  end
end

local uint64Ptr = ffi.typeof("uint64_t*")
local uint8Ptr = ffi.typeof("uint8_t*")

function findTimestampOffset(buf, rxTs)
	-- try to find the offset of the timestamp field in the packet
	local data = buf:getData()
	-- local matcher = ffi.new("uint16_t", ffi.new("uint64_t", txTs) >> 48)
	local msb16 = tonumber(rxTs / 2^48)  -- convert to Lua number
	msb16 = math.floor(msb16)             -- truncate any fractional part
	for i = 0, buf:getSize() - 2, 1 do
		-- local val = ffi.cast("uint16_t*", data+i)[0]
		local val = ffi.cast("uint16_t*", ffi.cast("uint8_t*", data) + i)[0]
		if val == msb16 then
			return i - 6
		end
	end
	return 0
end

function rxTimestamps(rxQueue, dstMac, histfile)
	local tscFreq = mg.getCyclesFrequency()
	local bufs = memory.bufArray()
	-- use whatever filter appropriate for your packet type
	-- rxQueue:filterL2Timestamps()
	print("ok")

	local hist = hist:new()
	local offset = 0
	while mg.running() do
		-- local numPkts = rxQueue:tryRecv(bufs, 32)
		-- -- print("ok3")
		-- if numPkts > 0 then
    -- 		-- print(numPkts)
    -- 		bufs[1]:dump()
    -- end
    -- bufs:free(numPkts)
		local numPkts = rxQueue:recvWithTimestamps(bufs)
		for i = 1, numPkts do
		 bufs[i]:dump()
			-- if bufs[i]:getUdpPacket().udp:getDstPort() == 0x10 then
				local rxTs = dpdkc.get_timestamp_dynfield(bufs[i])
			  if offset == 0 then
			    offset = findTimestampOffset(bufs[i], rxTs)
				end
				local txTs = ffi.cast("uint64_t*", ffi.cast("uint8_t*", bufs[i]:getData()) + offset)[0]
				-- local txTs = bufs[i]:getSoftwareTxTimestamp()
				-- if txTs == 0 then
					-- txTs = uint64Ptr(bufs[i]:getData())[2] -- looks like the value we are looking for is here instead
					-- txTs = uint64Ptr(bufs[i]:getData())[9] -- looks like the value we are looking for is here instead
					-- txTs = uint64Ptr(uint8Ptr(bufs[i]:getData())[9*8+3]) -- looks like the value we are looking for is here instead
					-- txTs = ffi.cast("uint64_t*", ffi.cast("uint8_t*", bufs[i]:getData()) + 9*8+6)[0]
				-- end
				local latency = tonumber(rxTs - txTs) / tscFreq * 10^9 -- to nanoseconds
				print(" rxTs: ", rxTs, " txTs: ", txTs, "lat: ", latency)
				hist:update(latency) -- to nanoseconds
			-- end
		end
		bufs:free(numPkts)
	end
	hist:print()
	if histfile ~= "" then
		hist:save(histfile)
	end
end

function timerSlave(txQueue, rxQueue, dstMac, histfile)
	local timestamper = ts:newTimestamper(txQueue, rxQueue)
	local hist = hist:new()
	mg.sleepMillis(1000) -- ensure that the load task is running
	while mg.running() do
		hist:update(timestamper:measureLatency(function(buf) buf:getEthernetPacket().eth.dst:setString(dstMac) end))
	end
	hist:print()
	hist:save(histfile)
end

