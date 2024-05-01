#include <errno.h>
#include <uk/essentials.h>
#include <click/config.h>
#include <click/error.hh>
#include <click/string.hh>

/* Unikraft does not support files yet, this function is to
   be able to read Click configs from files. */
String file_string(String s __unused, ErrorHandler *errh)
{
	errh->error("no support for files yet!");
	return String();
}

String clickpath_find_file(const String &filename __unused,
		const char *subdir __unused, String default_path __unused,
		ErrorHandler *errh)
{
	errh->error("no support for finding files!");
	return String();
}

void click_signal(int signum __unused, void (*handler)(int) __unused,
		bool resethand __unused)
{
}

extern "C" {
int dup(int oldfd); /* we don't have a header file, but the symbol is there */

int pipe(int pipefd[2])
{
	pipefd[0] = dup(0);
	pipefd[1] = dup(1);
	return 0;
}
} /* extern "C" */
