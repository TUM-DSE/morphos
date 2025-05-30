--- Forward packets between two ports
local mg     = require "moongen"
local device = require "device"
local stats  = require "stats"
local log    = require "log"
local memory = require "memory"
local timer  = require "timer"

function configure(parser)
	parser:argument("dev", "Device to receive on."):args(1):convert(tonumber)
	parser:option("-t --threads", "Number of threads to receive on with RSS."):args(1):convert(tonumber):default(1)
	parser:option("-c --csv", "Filename of the output csv."):default("")
	parser:option("-T --time", "Time to transmit for in seconds."):default(60):convert(tonumber)
	return parser:parse()
end

function master(args)
	-- configure devices
	-- for i, dev in ipairs(args.dev) do
		args.dev = device.config{
			port = args.dev,
			txQueues = args.threads,
			rxQueues = args.threads,
			rssQueues = args.threads
		}
	-- end
	device.waitForLinks()

	-- start receiving threads
	for i = 1, args.threads do
		mg.startTask("receive", args.dev:getRxQueue(i - 1), args.dev:getTxQueue(i - 1))
	end

	mg.sleepMillis(2000) -- make sure receiver is running

	-- measure stats
	print("Measuring receive rate...")
	-- stats.startStatsTask{dev} -- we can only have one stats task at a time
	if args.csv ~= "" then
		stats.startStatsTask{devices={args.dev}, format="csv", file=args.csv}
	end

	-- stop test after time
	if args.time >= 0 then
		local runtime = timer:new(args.time)
		runtime:wait()
		mg:stop()
	end

	mg.waitForTasks()
end

function receive(rxQueue, txQueue)
	-- a bufArray is just a list of buffers that we will use for batched forwarding
	local bufs = memory.bufArray()
	while mg.running() do -- check if Ctrl+c was pressed
		-- receive one or more packets from the queue
		local count = rxQueue:recv(bufs)
		-- print(count)
		-- send out all received bufs on the other queue
		-- the bufs are free'd implicitly by this function
		-- txQueue:sendN(bufs, count)
		bufs:freeAll()
	end
end
