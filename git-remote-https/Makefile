WUNIX_CC := $(abspath ../../wunix/tools/wunix-cc)
CFLAGS ?= -D_GNU_SOURCE -O2
LDFLAGS ?= -lssl -lcrypto

all: git-remote-https

git-remote-https: git-remote-https.c
	$(WUNIX_CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

clean:
	rm -f git-remote-https

.PHONY: all clean
