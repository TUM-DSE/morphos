local mg     = require "moongen"
local memory = require "memory"
local device = require "device"
local ts     = require "timestamping"
local stats  = require "stats"
local hist   = require "histogram"
local timer  = require "timer"
local log    = require "log"

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
	for i = 0, args.threads - 1 do
	    dev:getTxQueue(i):setRate(args.rate * (args.size + 4) * 8 / 1000)
	    mg.startTask("loadSlave", dev:getTxQueue(i), dev:getMac(true), args.mac, args.size, args.macs, args.ethertypes)
    end
	stats.startStatsTask{dev}
	if args.csv ~= "" then
		stats.startStatsTask{devices={dev}, format="csv", file=args.csv}
	end
	mg.startSharedTask("timerSlave", dev:getTxQueue(args.threads), dev:getRxQueue(args.threads), args.mac, args.file)
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
	while mg.running() do
    bufs:alloc(pktSize)
    queue:send(bufs)
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

function loadSlave(queue, srcMac, dstMac, pktSize, numDstMacs, numEthertypes)
	-- local mac_nr = parseMacAddress(dstMac, 1)
	-- print(srcMac)
	-- print(mac_nr)
	local mem = memory.createMemPool(function(buf)
		buf:getEthernetPacket():fill{
			ethSrc = srcMac,
			ethDst = dstMac,
			ethType = 0x1234
		}
	end)
	local bufs = mem:bufArray()
	if numDstMacs > 1 or numEthertypes > 1 then
		-- error("Sending to multiple MACs and ethertypes at the same time is not supproted.") -- no it is
		sendMacs(queue, bufs, pktSize, dstMac, numDstMacs, numEthertypes)
	else
			sendSimple(queue, bufs, pktSize)
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
