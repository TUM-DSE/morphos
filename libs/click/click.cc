/* SPDX-License-Identifier: BSD-3-Clause */
/*
 * Copyright (c) 2014 NEC Europe Ltd,
 *               2019, NEC Laboratories Europe GmbH, NEC Corporation.
 *               All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from
 *    this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
 * LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

/* To be implemented at a later point */
#define CLICK_CONSOLE_SUPPORT_IMPLEMENTED 0

extern "C"{
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <poll.h>
#include <dirent.h>
}

#include <click/config.h>
#include <click/lexer.hh>
#include <click/router.hh>
#include <click/master.hh>
#include <click/error.hh>
#include <click/timer.hh>
#include <click/string.hh>
#include <click/straccum.hh>
#include <click/driver.hh>

#include <static_config.h>

#include <uk/sched.h>
#include <uk/thread.h>
#include <uk/netdev.h>
#include <uk/plat/memory.h>

int click_nthreads = 1;
void *__dso_handle = NULL;

#define NLOG(fmt, ...)
#define LOG(fmt, ...) \
	printf("[%s:%d] " fmt "\n", \
		__FUNCTION__, __LINE__, ##__VA_ARGS__)

u_int _shutdown = 0;
u_int _reason = 0;

char*
event_token(char **key)
{
	char *p = *key, *p2;
	char *token;
	u_int sz;

	p2 = strchr(++p, '/');
	sz = (p2 != NULL ? p2 - p : strlen(p));
	token = strndup(p, sz);
	*key = (p+sz);

	return token;
}

char*
read_rid(char *path)
{
	char *id = strdup(path + 1);
	char *id_end = strchr(id, '/');
	*id_end = '\0';
	return id;
}

#if CLICK_CONSOLE_SUPPORT_IMPLEMENTED
String *
read_config(u_int rid = 0)
{
	char path[PATH_MAX_LEN];
	String *cfg = new String();
	char *token;

	for (int i = 0;; i++) {
		snprintf(path, PATH_MAX_LEN, "%s/%d/config/%d",
				PATH_ROOT, rid, i);
		xenbus_read(XBT_NIL, path, &token);
		if (token == NULL)
			break;
		*cfg += token;
	}

	return cfg;
}
#endif /* CLICK_CONSOLE_SUPPORT_IMPLEMENTED */
/*
 * click glue
 */

#define MAX_ROUTERS	64
static ErrorHandler *errh;
static Master master(1);
static String macaddr_preamble;

static void
make_macaddr_preamble()
{
	unsigned int ndev = uk_netdev_count();
	const size_t buflen = 64;
	char buf[buflen];
	const struct uk_hwaddr *mac;
	StringAccum acc;

	UK_ASSERT(macaddr_preamble.empty());

	uk_pr_info("Found %d network device(s)\n", uk_netdev_count());
	for (unsigned int i = 0; i < ndev; ++i) {
		mac = uk_netdev_hwaddr_get(uk_netdev_get(i));
		snprintf(buf, buflen,
			"define($MAC%d %02x:%02x:%02x:%02x:%02x:%02x);\n",
			i, mac->addr_bytes[0], mac->addr_bytes[1],
			mac->addr_bytes[2], mac->addr_bytes[3],
			mac->addr_bytes[4], mac->addr_bytes[5]);
		uk_pr_info("appending %s", buf);
		acc.append(buf);
	}
	acc.append("/* End unikraft-provided MAC preamble */\n");
	macaddr_preamble = acc.take_string();
	uk_pr_info("MAC address macros:\n%s\n", macaddr_preamble.c_str());
}

static void listdir(const char *name, int indent)
{
    DIR *dir;
    struct dirent *entry;

    if (!(dir = opendir(name)))
        return;

    while ((entry = readdir(dir)) != NULL) {
        if (entry->d_type == DT_DIR) {
            char path[1024];
            if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0)
                continue;
            snprintf(path, sizeof(path), "%s/%s", name, entry->d_name);
            uk_pr_info("%*s[%s]\n", indent, "", entry->d_name);
            listdir(path, indent + 2);
        } else {
            uk_pr_info("%*s- %s\n", indent, "", entry->d_name);
        }
    }
    closedir(dir);
}

static String *
get_config()
{
	String *cfg = new String(macaddr_preamble);
	struct ukplat_memregion_desc *img;
	char *cstr;
	size_t cstr_len;

	UK_ASSERT(!macaddr_preamble.empty());

    uk_pr_info("Files @ /:\n");
    listdir("/", 0);

    /* First, try to read from "config.click" */
    FILE* file = fopen("/config.click", "r");
    if (file) {
        uk_pr_info("Reading config from /config.click\n");
        fseek(file, 0, SEEK_END);
        size_t file_size = (size_t) ftell(file);
        fseek(file, 0, SEEK_SET);

        cstr = (char*)malloc(file_size);
        cstr_len = fread(cstr, 1, file_size, file);
    } else if (ukplat_memregion_find_initrd0(&img) >= 0) {
        /* Then, try initrd */
        uk_pr_info("Reading config from initrd0\n");

		cstr = (char *)img->pbase;
		cstr_len = img->len;
	} else {
		/* If we can't find a config: use a fallback one statically
		 * compiled in.
		 */
		uk_pr_warn("Could not find a config, using standard config!\n");
		cstr = CONFIGSTRING;
		cstr_len = strlen(CONFIGSTRING);
	}
	cfg->append(cstr, cstr_len);
	printf("Received config (length %d):\n", cfg->length());
	printf("%s\n", cfg->c_str());
	return cfg;
}

struct router_instance {
	Router *r;
	u_int f_stop;
} router_list[MAX_ROUTERS];

void
router_thread(void *thread_data)
{
	struct router_instance *ri = &router_list[(unsigned long)thread_data];
	String *config = get_config();

	ri->r = click_read_router(*config, true, errh, false, &master);
	if (ri->r->initialize(errh) < 0) {
		LOG("Router init failed!");
		ri->f_stop = 1;
		return;
	}

	ri->r->use();
	ri->r->activate(errh);

	LOG("Starting driver...\n\n");
	ri->r->master()->thread(0)->driver();

	LOG("Stopping driver...\n\n");
	ri->r->unuse();
	ri->f_stop = 1;

	LOG("Master/driver stopped, closing router_thread");
	free(config);
}

void
router_stop(int n = MAX_ROUTERS)
{
	LOG("Stopping all routers...\n\n");
	for (int i = n - 1; i >= 0; --i) {
		Router *r = router_list[i].r;

		if (router_list[i].f_stop)
			continue;

		LOG("Stopping instance = %d...\n\n", i);

		do {
			r->please_stop_driver();
			uk_sched_yield();
		} while (!(router_list[i].f_stop));
	}
	LOG("Stopped all routers...\n\n");
}

/* Initialize all netdev devices to the point where you can get a MAC
 * address from them.
 */
static int
uk_netdev_early_init(ErrorHandler *errh)
{
	struct uk_netdev *netdev;
	struct uk_netdev_conf netdev_conf;
	int ret;

	netdev_conf.nb_rx_queues = 1;
	netdev_conf.nb_tx_queues = 1;
	for (unsigned int i = 0; i < uk_netdev_count(); ++i) {
		netdev = uk_netdev_get(i);

		if (!netdev)
			continue;
		if (uk_netdev_state_get(netdev) != UK_NETDEV_UNCONFIGURED &&
				uk_netdev_state_get(netdev) != UK_NETDEV_UNPROBED) {
			uk_pr_info("Skipping to add network device %u to lwIP: Not in unconfigured state\n", i);
			continue;
		}

		if (uk_netdev_state_get(netdev) == UK_NETDEV_UNPROBED) {
			ret = uk_netdev_probe(netdev);
			if (ret < 0) {
				uk_pr_err("Failed to probe network device %u %d", i, ret);
				continue;
			}
		}
		uk_pr_info("netdev %d early init\n", i);
		if (uk_netdev_configure(netdev, &netdev_conf) < 0)
			return errh->error("Failed to configure device %d\n", i);
	}
	return 0;
}

#if CLICK_CONSOLE_SUPPORT_IMPLEMENTED
void
router_suspend(int n = MAX_ROUTERS)
{
}

void
router_resume(int n = MAX_ROUTERS)
{
}

u_int
on_status(int rid, char *key, void *data)
{
	String status = (char *) data;
	struct router_instance *r = &router_list[rid];

	LOG("router id %d", rid);
	LOG("status change to %s", data);
	if (status == "Running") {
		u_int *arg0;

		if (!r->f_stop)
			goto status_err;

		arg0 = (u_int*) malloc(sizeof(u_int));
		*arg0 = rid;
		r->f_stop = 0;
		create_thread((char*)"click", router_thread, arg0);
	} else if (status == "Halted") {
		if (r->f_stop)
			goto status_err;

		router_stop(rid);
	}

	return 0;
status_err:
	return -EINVAL;
}

static void
read_cname(char *path, Element **e, const Handler **h, int rid)
{
	int sep = strchr(path, '/') - path;
	String ename(path, sep), hname(path + sep + 1);
	Router *r = router_list[rid].r;

	NLOG("sep=%d element=%s handler=%s", sep,
			ename.c_str(), hname.c_str());

	*e = r->find(ename);
	*h = Router::handler(*e, hname);
	NLOG("ename %s=%p hname %s=%p", ename.c_str(), e, hname.c_str(), h);
}

u_int
on_elem_readh(int rid, char *key, void *data)
{
	Element *e;
	const Handler* h;
	String val, h_path, lock_path;
	int f_stop = router_list[rid].f_stop;

	if (strncmp(key, "/read/", 6))
		return 0;

	if (f_stop)
		return 0;

	read_cname(key+6, &e, &h, rid);

	if (!h || !h->readable())
		return EINVAL;

	val = h->call_read(e, "", ErrorHandler::default_handler());
	h_path = String(PATH_ROOT) + "/0/elements/" + e->name() + "/" + h->name();
	lock_path = h_path + "/lock";

	xenbus_write(XBT_NIL, h_path.c_str(), val.c_str());
	xenbus_write(XBT_NIL, lock_path.c_str(), "0");

	NLOG("element handler read %s", val.c_str());
	return 0;
}

u_int
on_elem_writeh(int rid, char *key, void *data)
{
	Element *e;
	const Handler* h;
	int f_stop = router_list[rid].f_stop;

	if (f_stop || strlen((char *) data) == 0)
		return 0;

	read_cname(key+1, &e, &h, rid);

	if (!h || !h->writable())
		return EINVAL;

	h->call_write(String(data), e, ErrorHandler::default_handler());
	NLOG("element handler write %s value %s", data, "");
	return 0;
}

u_int
on_router(char *key, void *data)
{
	char *p = key;
	String rev;
	u_int rid;

	if (strlen(key) == 0)
		return 0;

	rid = atoi(event_token(&p));
	rev = event_token(&p);

	if (rev == "control")
		on_elem_readh(rid, p, data);
	else if (rev == "elements")
		on_elem_writeh(rid, p, data);
	else if (rev == "status")
		on_status(rid, key, data);

	return 0;
}
#endif /* CLICK_CONSOLE_SUPPORT_IMPLEMENTED */

#if CONFIG_LIBCLICK_MAIN
#define CLICK_MAIN main
#else
#define CLICK_MAIN click_main
extern "C" int click_main(int argc, char **argv);
#endif

/* runs the event loop */
int CLICK_MAIN(int argc, char **argv)
{
	struct uk_thread *router;

	click_static_initialize();
	errh = ErrorHandler::default_handler();

	memset(router_list, 0, MAX_ROUTERS * sizeof(struct router_instance));
	for (int i = 0; i < MAX_ROUTERS; ++i) {
		router_list[i].f_stop = 1;
	}

	if (uk_netdev_early_init(errh))
		return -EINVAL;
	make_macaddr_preamble();

#if CLICK_CONSOLE_SUPPORT_IMPLEMENTED
/* TODO: This interaction between xenbus and click should be replaced with a
 * generic unikraft interaction scheme, for example, via a console.
 */
	while (!_shutdown) {
		String value;
		char *err, *val, **path;

		path = xenbus_wait_for_watch_return(&xsdev->events);
		if (!path || _shutdown)
			continue;

		err = xenbus_read(XBT_NIL, *path, &val);
		if (err)
			continue;

		on_router((*path + strlen(PATH_ROOT)), val);
	}
#else
	router = uk_sched_thread_create(uk_sched_current(), router_thread, 0, "click-router");
	while (!uk_thread_is_exited(router))
		uk_sched_yield();
#endif
	LOG("Shutting down...");

	return _reason;
}

/* app shutdown hook from minios */
/* TODO: could be reused as call-in from unikraft */
#if CLICK_CONSOLE_SUPPORT_IMPLEMENTED
extern "C" {
int app_shutdown(unsigned reason)
{
	switch (reason) {
	case SHUTDOWN_poweroff:
		LOG("Requested shutdown reason=poweroff");
		_shutdown = 1;
		_reason = reason;
		router_stop();
		break;
	case SHUTDOWN_reboot:
		LOG("Requested shutdown reason=reboot");
		_shutdown = 1;
		_reason = reason;
		router_stop();
		break;
	case SHUTDOWN_suspend:
		LOG("Requested shutdown reason=suspend");
		router_suspend();
		kernel_suspend();
		router_resume();
	default:
		LOG("Requested shutdown with invalid reason (%d)", reason);
		break;
	}

	return 0;
}
}
#endif /* CLICK_CONSOLE_SUPPORT_IMPLEMENTED */
