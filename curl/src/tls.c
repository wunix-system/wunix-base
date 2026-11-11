#include <openssl/ssl.h>
#include <openssl/err.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/syscall.h>

typedef struct {
    SSL_CTX *ctx;
    SSL *ssl;
} WunixTls;

void wunix_tls_close(void *handle);

extern void __init_libc(char **envp, char *pn);
static int tls_initialized = 0;

static unsigned long fallback_auxv[] = {
    25, (unsigned long)"\x59\x5e\x9f\xbd\x94\xfd\xa7\x66\x12\x34\x56\x78\x9a\xbc\xde\xf0", /* AT_RANDOM */
    6, 4096, /* AT_PAGESZ */
    0, 0     /* AT_NULL */
};

static char *fallback_envp[] = {
    "PATH=/bin:/usr/bin",
    NULL,
    (char *)fallback_auxv
};

void wunix_init_tls(long argc, char **argv) {
    if (!tls_initialized) {
        char **envp = NULL;
        char *pn = "curl";

        if (argv && argc > 0) {
            pn = argv[0];
            envp = argv + argc + 1;
        } else {
            envp = fallback_envp;
        }

        __init_libc(envp, pn);
        tls_initialized = 1;
    }
}

static void ssl_info_callback(const SSL *s, int where, int ret) {
    const char *str = "undefined";
    int w = where & ~SSL_ST_MASK;
    if (w & SSL_ST_CONNECT) str = "SSL_connect";
    else if (w & SSL_ST_ACCEPT) str = "SSL_accept";
    if (where & SSL_CB_LOOP) {
        char msg[256];
        int mlen = snprintf(msg, sizeof(msg), "* [TLS STATE] %s: %s\n", str, SSL_state_string_long(s));
        if (mlen > 0) write(2, msg, mlen);
    } else if (where & SSL_CB_ALERT) {
        str = (where & SSL_CB_READ) ? "read" : "write";
        char msg[256];
        int mlen = snprintf(msg, sizeof(msg), "* [TLS ALERT] %s: %s: %s\n", str, SSL_alert_type_string_long(ret), SSL_alert_desc_string_long(ret));
        if (mlen > 0) write(2, msg, mlen);
    } else if (where & SSL_CB_EXIT) {
        if (ret == 0) {
            char msg[256];
            int mlen = snprintf(msg, sizeof(msg), "* [TLS EXIT] %s: failed in %s\n", str, SSL_state_string_long(s));
            if (mlen > 0) write(2, msg, mlen);
        } else if (ret < 0) {
            char msg[256];
            int mlen = snprintf(msg, sizeof(msg), "* [TLS EXIT] %s: error in %s\n", str, SSL_state_string_long(s));
            if (mlen > 0) write(2, msg, mlen);
        }
    }
}

static int is_ip_literal(const char *h) {
    if (!h || !*h) return 0;
    int has_digit = 0;
    for (const char *p = h; *p; p++) {
        if (*p >= '0' && *p <= '9') has_digit = 1;
        else if (*p != '.' && *p != ':') return 0;
    }
    return has_digit;
}

void *wunix_tls_init_session(const char *hostname, const char *ca_file, int insecure, int verbose) {
    wunix_init_tls(0, NULL);

    // Инициализация контекста TLS клиента
    const SSL_METHOD *method = TLS_client_method();
    if (!method) {
        return NULL;
    }

    SSL_CTX *ctx = SSL_CTX_new(method);
    if (!ctx) {
        return NULL;
    }

    if (verbose) {
        SSL_CTX_set_info_callback(ctx, ssl_info_callback);
    }

    // Поддерживаем современные версии TLS (TLS 1.2 и TLS 1.3)
    SSL_CTX_set_min_proto_version(ctx, TLS1_2_VERSION);

    if (insecure) {
        SSL_CTX_set_verify(ctx, SSL_VERIFY_NONE, NULL);
    } else {
        SSL_CTX_set_verify(ctx, SSL_VERIFY_PEER, NULL);
        const char *ca = (ca_file && *ca_file) ? ca_file : "/etc/ssl/certs/ca-certificates.crt";
        int load_res = SSL_CTX_load_verify_locations(ctx, ca, NULL);
        if (load_res <= 0) {
            load_res = SSL_CTX_load_verify_locations(ctx, "/etc/ssl/cert.pem", NULL);
        }
        SSL_CTX_set_default_verify_paths(ctx);
    }

    SSL *ssl = SSL_new(ctx);
    if (!ssl) {
        SSL_CTX_free(ctx);
        return NULL;
    }

    // Установка SNI (Server Name Indication) и проверка имени хоста
    // По RFC 6066 literal IP-адреса запрещены в расширении server_name (SNI)
    if (hostname && *hostname) {
        if (!is_ip_literal(hostname)) {
            SSL_set_tlsext_host_name(ssl, hostname);
        }
        if (!insecure) {
            SSL_set1_host(ssl, hostname);
        }
    }

    WunixTls *handle = (WunixTls *)malloc(sizeof(WunixTls));
    if (!handle) {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        return NULL;
    }

    handle->ctx = ctx;
    handle->ssl = ssl;
    return (void *)handle;
}

int wunix_tls_handshake(void *handle, int fd, int verbose) {
    if (!handle) {
        return 0;
    }
    WunixTls *t = (WunixTls *)handle;

    // Привязываем сокет файлового дескриптора
    if (SSL_set_fd(t->ssl, fd) <= 0) {
        return 0;
    }

    // Выполняем рукопожатие TLS Handshake
    int conn_res = SSL_connect(t->ssl);
    if (conn_res <= 0) {
        int ssl_err = SSL_get_error(t->ssl, conn_res);
        long v_res = SSL_get_verify_result(t->ssl);
        const char *v_str = X509_verify_cert_error_string(v_res);
        if (verbose) {
            char msg[512];
            int mlen = snprintf(msg, sizeof(msg),
                "* [TLS] SSL_connect failed: ret=%d, ssl_err=%d, verify_result=%ld (%s)\n",
                conn_res, ssl_err, v_res, v_str ? v_str : "none");
            if (mlen > 0) write(2, msg, mlen);

            unsigned long err;
            while ((err = ERR_get_error()) != 0) {
                char err_str[256];
                ERR_error_string_n(err, err_str, sizeof(err_str));
                int elen = snprintf(msg, sizeof(msg), "* [TLS] OpenSSL error: %s\n", err_str);
                if (elen > 0) write(2, msg, elen);
            }
        }
        return 0;
    }

    return 1;
}

void *wunix_tls_connect(int fd, const char *hostname, const char *ca_file, int insecure, int verbose) {
    void *handle = wunix_tls_init_session(hostname, ca_file, insecure, verbose);
    if (!handle) {
        return NULL;
    }
    if (!wunix_tls_handshake(handle, fd, verbose)) {
        wunix_tls_close(handle);
        return NULL;
    }
    return handle;
}

int wunix_tls_write(void *handle, const void *buf, int len) {
    if (!handle || !buf || len <= 0) {
        return -1;
    }
    WunixTls *t = (WunixTls *)handle;
    return SSL_write(t->ssl, buf, len);
}

int wunix_tls_read(void *handle, void *buf, int len) {
    if (!handle || !buf || len <= 0) {
        return -1;
    }
    WunixTls *t = (WunixTls *)handle;
    return SSL_read(t->ssl, buf, len);
}

void wunix_tls_close(void *handle) {
    if (!handle) {
        return;
    }
    WunixTls *t = (WunixTls *)handle;
    if (t->ssl) {
        SSL_shutdown(t->ssl);
        SSL_free(t->ssl);
    }
    if (t->ctx) {
        SSL_CTX_free(t->ctx);
    }
    free(t);
}
