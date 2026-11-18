/*
 * git-remote-https / git-remote-http — Вспомогательный удаленный помощник Git для Wunix OS
 * Реализует протокол stateless-connect (wire protocol v2) поверх HTTPS (OpenSSL) и HTTP.
 */

#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <netdb.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <openssl/ssl.h>
#include <openssl/err.h>

typedef struct {
    int is_https;
    char host[256];
    int port;
    char path[1024];
} UrlParts;

static int parse_url(const char *url_str, UrlParts *parts) {
    memset(parts, 0, sizeof(*parts));
    const char *p = url_str;
    if (strncmp(p, "https://", 8) == 0) {
        parts->is_https = 1;
        parts->port = 443;
        p += 8;
    } else if (strncmp(p, "http://", 7) == 0) {
        parts->is_https = 0;
        parts->port = 80;
        p += 7;
    } else {
        return -1;
    }

    const char *slash = strchr(p, '/');
    char hostport[256];
    if (slash) {
        size_t hlen = slash - p;
        if (hlen >= sizeof(hostport)) hlen = sizeof(hostport) - 1;
        memcpy(hostport, p, hlen);
        hostport[hlen] = 0;
        strncpy(parts->path, slash, sizeof(parts->path) - 1);
    } else {
        strncpy(hostport, p, sizeof(hostport) - 1);
        strcpy(parts->path, "/");
    }

    char *colon = strchr(hostport, ':');
    if (colon) {
        *colon = 0;
        parts->port = atoi(colon + 1);
    }
    strncpy(parts->host, hostport, sizeof(parts->host) - 1);

    size_t plen = strlen(parts->path);
    while (plen > 1 && parts->path[plen - 1] == '/') {
        parts->path[plen - 1] = 0;
        plen--;
    }
    return 0;
}

typedef struct {
    int fd;
    SSL_CTX *ctx;
    SSL *ssl;
    int is_https;
} HttpConn;

static int conn_open(HttpConn *c, const UrlParts *url) {
    memset(c, 0, sizeof(*c));
    c->is_https = url->is_https;

    struct hostent *he = gethostbyname(url->host);
    if (!he) return -1;

    c->fd = socket(AF_INET, SOCK_STREAM, 0);
    if (c->fd < 0) return -1;

    struct sockaddr_in sa;
    memset(&sa, 0, sizeof(sa));
    sa.sin_family = AF_INET;
    sa.sin_port = htons(url->port);
    memcpy(&sa.sin_addr, he->h_addr_list[0], sizeof(sa.sin_addr));

    if (connect(c->fd, (struct sockaddr *)&sa, sizeof(sa)) < 0) {
        close(c->fd);
        return -1;
    }

    if (c->is_https) {
        c->ctx = SSL_CTX_new(TLS_client_method());
        if (!c->ctx) {
            close(c->fd);
            return -1;
        }
        c->ssl = SSL_new(c->ctx);
        SSL_set_fd(c->ssl, c->fd);
        SSL_set_tlsext_host_name(c->ssl, url->host);
        if (SSL_connect(c->ssl) <= 0) {
            SSL_free(c->ssl);
            SSL_CTX_free(c->ctx);
            close(c->fd);
            return -1;
        }
    }
    return 0;
}

static int conn_write(HttpConn *c, const void *buf, int len) {
    if (c->is_https) {
        return SSL_write(c->ssl, buf, len);
    } else {
        return write(c->fd, buf, len);
    }
}

static int conn_read(HttpConn *c, void *buf, int len) {
    if (c->is_https) {
        return SSL_read(c->ssl, buf, len);
    } else {
        return read(c->fd, buf, len);
    }
}

static void conn_close(HttpConn *c) {
    if (c->ssl) {
        SSL_shutdown(c->ssl);
        SSL_free(c->ssl);
        c->ssl = NULL;
    }
    if (c->ctx) {
        SSL_CTX_free(c->ctx);
        c->ctx = NULL;
    }
    if (c->fd >= 0) {
        close(c->fd);
        c->fd = -1;
    }
}

static int read_http_headers(HttpConn *c, int *status_out, int *is_chunked_out, long *content_len_out) {
    char hbuf[8192];
    int hlen = 0;
    *status_out = 200;
    *is_chunked_out = 0;
    *content_len_out = -1;

    while (hlen < (int)sizeof(hbuf) - 1) {
        int n = conn_read(c, hbuf + hlen, 1);
        if (n <= 0) break;
        hlen += n;
        hbuf[hlen] = 0;
        if (hlen >= 4 && memcmp(hbuf + hlen - 4, "\r\n\r\n", 4) == 0) {
            break;
        }
    }

    char *p = strstr(hbuf, "HTTP/1.");
    if (p) {
        p = strchr(p, ' ');
        if (p) *status_out = atoi(p + 1);
    }

    if (strcasestr(hbuf, "Transfer-Encoding: chunked")) {
        *is_chunked_out = 1;
    }
    p = strcasestr(hbuf, "Content-Length:");
    if (p) {
        *content_len_out = atol(p + 15);
    }

    return 0;
}

static void stream_http_body(HttpConn *c, int is_chunked, long content_len) {
    char buf[4096];
    if (is_chunked) {
        while (1) {
            char line[64];
            int pos = 0;
            while (pos < (int)sizeof(line) - 1) {
                int n = conn_read(c, line + pos, 1);
                if (n <= 0) return;
                pos += n;
                if (pos >= 2 && line[pos - 2] == '\r' && line[pos - 1] == '\n') {
                    break;
                }
            }
            line[pos] = 0;
            long chunk_size = strtol(line, NULL, 16);
            if (chunk_size <= 0) {
                char dummy[2];
                conn_read(c, dummy, 2);
                break;
            }

            long remaining = chunk_size;
            while (remaining > 0) {
                int to_read = remaining > (long)sizeof(buf) ? (long)sizeof(buf) : remaining;
                int n = conn_read(c, buf, to_read);
                if (n <= 0) return;
                fwrite(buf, 1, n, stdout);
                remaining -= n;
            }
            char crlf[2];
            conn_read(c, crlf, 2);
        }
    } else if (content_len >= 0) {
        long remaining = content_len;
        while (remaining > 0) {
            int to_read = remaining > (long)sizeof(buf) ? (long)sizeof(buf) : remaining;
            int n = conn_read(c, buf, to_read);
            if (n <= 0) break;
            fwrite(buf, 1, n, stdout);
            remaining -= n;
        }
    } else {
        while (1) {
            int n = conn_read(c, buf, sizeof(buf));
            if (n <= 0) break;
            fwrite(buf, 1, n, stdout);
        }
    }
    fflush(stdout);
}

/* Reads entire body into allocated memory (for discovery header stripping) */
static char *read_all_body(HttpConn *c, int is_chunked, long content_len, size_t *out_len) {
    size_t cap = 16384;
    size_t len = 0;
    char *buf = malloc(cap);

    if (is_chunked) {
        while (1) {
            char line[64];
            int pos = 0;
            while (pos < (int)sizeof(line) - 1) {
                int n = conn_read(c, line + pos, 1);
                if (n <= 0) break;
                pos += n;
                if (pos >= 2 && line[pos - 2] == '\r' && line[pos - 1] == '\n') {
                    break;
                }
            }
            line[pos] = 0;
            long chunk_size = strtol(line, NULL, 16);
            if (chunk_size <= 0) {
                char dummy[2];
                conn_read(c, dummy, 2);
                break;
            }

            long rem = chunk_size;
            while (rem > 0) {
                if (len + rem >= cap) {
                    cap = (len + rem) * 2;
                    buf = realloc(buf, cap);
                }
                int n = conn_read(c, buf + len, rem);
                if (n <= 0) break;
                len += n;
                rem -= n;
            }
            char crlf[2];
            conn_read(c, crlf, 2);
        }
    } else if (content_len >= 0) {
        if ((size_t)content_len + 1 > cap) {
            cap = content_len + 1;
            buf = realloc(buf, cap);
        }
        long rem = content_len;
        while (rem > 0) {
            int n = conn_read(c, buf + len, rem);
            if (n <= 0) break;
            len += n;
            rem -= n;
        }
    } else {
        while (1) {
            if (len + 4096 >= cap) {
                cap *= 2;
                buf = realloc(buf, cap);
            }
            int n = conn_read(c, buf + len, 4096);
            if (n <= 0) break;
            len += n;
        }
    }
    *out_len = len;
    return buf;
}

int main(int argc, char **argv) {
    if (argc < 3) {
        return 1;
    }
    const char *remote = argv[1];
    (void)remote;
    const char *url_str = argv[2];
    UrlParts url;
    if (parse_url(url_str, &url) < 0) {
        return 1;
    }

    char line[1024];
    while (fgets(line, sizeof(line), stdin)) {
        size_t len = strlen(line);
        while (len > 0 && (line[len - 1] == '\r' || line[len - 1] == '\n')) {
            line[--len] = 0;
        }
        if (strcmp(line, "capabilities") == 0) {
            printf("stateless-connect\n\n");
            fflush(stdout);
        } else if (strncmp(line, "option", 6) == 0) {
            printf("ok\n");
            fflush(stdout);
        } else if (strncmp(line, "stateless-connect ", 18) == 0) {
            const char *service = line + 18;
            while (*service == ' ') service++;

            // Step 1: GET info/refs?service=...
            HttpConn c;
            if (conn_open(&c, &url) < 0) {
                return 1;
            }

            char req[2048];
            int rlen = snprintf(req, sizeof(req),
                "GET %s/info/refs?service=%s HTTP/1.1\r\n"
                "Host: %s\r\n"
                "User-Agent: git/2.43.0\r\n"
                "Git-Protocol: version=2\r\n"
                "Accept: */*\r\n"
                "Connection: close\r\n\r\n",
                url.path, service, url.host);
            conn_write(&c, req, rlen);

            int status, is_chunked;
            long content_len;
            read_http_headers(&c, &status, &is_chunked, &content_len);

            size_t body_len = 0;
            char *body = read_all_body(&c, is_chunked, content_len, &body_len);
            conn_close(&c);

            // Strip initial "# service=..." and "0000" if present
            size_t pos = 0;
            if (body_len >= 4) {
                char hx[5];
                memcpy(hx, body, 4);
                hx[4] = 0;
                long pkt = strtol(hx, NULL, 16);
                if (pkt > 4 && (size_t)pkt <= body_len && memmem(body, pkt, "# service=", 10)) {
                    pos = pkt;
                    if (pos + 4 <= body_len && memcmp(body + pos, "0000", 4) == 0) {
                        pos += 4;
                    }
                }
            }

            // Connection established positive response
            printf("\n");
            fflush(stdout);

            if (pos < body_len) {
                fwrite(body + pos, 1, body_len - pos, stdout);
                fflush(stdout);
            }
            free(body);

            // Step 2: Loop handling POST requests
            while (1) {
                size_t req_cap = 65536;
                char *post_buf = malloc(req_cap);
                size_t post_len = 0;

                while (1) {
                    char len_str[5];
                    if (fread(len_str, 1, 4, stdin) != 4) {
                        free(post_buf);
                        return 0; // EOF from git
                    }
                    len_str[4] = 0;
                    memcpy(post_buf + post_len, len_str, 4);
                    post_len += 4;

                    int pkt_len = (int)strtol(len_str, NULL, 16);
                    if (pkt_len == 0) {
                        break; // Flush packet
                    }
                    if (pkt_len > 4) {
                        int payload_len = pkt_len - 4;
                        if (post_len + payload_len >= req_cap) {
                            req_cap = (post_len + payload_len) * 2;
                            post_buf = realloc(post_buf, req_cap);
                        }
                        if (fread(post_buf + post_len, 1, payload_len, stdin) != (size_t)payload_len) {
                            free(post_buf);
                            return 0;
                        }
                        post_len += payload_len;
                    }
                }

                // Send POST to server
                if (conn_open(&c, &url) < 0) {
                    free(post_buf);
                    return 1;
                }

                rlen = snprintf(req, sizeof(req),
                    "POST %s/%s HTTP/1.1\r\n"
                    "Host: %s\r\n"
                    "User-Agent: git/2.43.0\r\n"
                    "Content-Type: application/x-%s-request\r\n"
                    "Accept: application/x-%s-result\r\n"
                    "Git-Protocol: version=2\r\n"
                    "Content-Length: %zu\r\n"
                    "Connection: close\r\n\r\n",
                    url.path, service, url.host, service, service, post_len);
                conn_write(&c, req, rlen);
                conn_write(&c, post_buf, post_len);
                free(post_buf);

                read_http_headers(&c, &status, &is_chunked, &content_len);
                stream_http_body(&c, is_chunked, content_len);
                conn_close(&c);

                // Write response-end packet (0002) as required by stateless-connect
                fwrite("0002", 1, 4, stdout);
                fflush(stdout);
            }
        }
    }
    return 0;
}
