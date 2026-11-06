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

void *wunix_tls_connect(int fd, const char *hostname, const char *ca_file, int insecure) {
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

    // Минимальная версия TLS 1.2
    SSL_CTX_set_min_proto_version(ctx, TLS1_2_VERSION);

    if (insecure) {
        SSL_CTX_set_verify(ctx, SSL_VERIFY_NONE, NULL);
    } else {
        SSL_CTX_set_verify(ctx, SSL_VERIFY_PEER, NULL);
        const char *ca = (ca_file && *ca_file) ? ca_file : "/etc/ssl/certs/ca-certificates.crt";
        if (SSL_CTX_load_verify_locations(ctx, ca, NULL) <= 0) {
            // Запасной стандартный путь к сертификатам
            SSL_CTX_load_verify_locations(ctx, "/etc/ssl/cert.pem", NULL);
        }
    }

    SSL *ssl = SSL_new(ctx);
    if (!ssl) {
        SSL_CTX_free(ctx);
        return NULL;
    }

    // Установка SNI (Server Name Indication) и проверка имени хоста
    if (hostname && *hostname) {
        SSL_set_tlsext_host_name(ssl, hostname);
        if (!insecure) {
            SSL_set1_host(ssl, hostname);
        }
    }

    // Привязываем сокет файлового дескриптора
    if (SSL_set_fd(ssl, fd) <= 0) {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        return NULL;
    }

    // Выполняем рукопожатие TLS Handshake
    if (SSL_connect(ssl) <= 0) {
        SSL_free(ssl);
        SSL_CTX_free(ctx);
        return NULL;
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
