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

extern int __init_tp(void *p);
static char main_thread_area[4096] __attribute__((aligned(64)));
static int tls_initialized = 0;

void wunix_init_tls(void) {
    if (!tls_initialized) {
        // Прямая инициализация области локального хранилища потока (TLS)
        // без использования libc syscall(), требующего уже инициализированного canary в %fs:0x28
        __init_tp(main_thread_area);
        *(unsigned long *)((char *)main_thread_area + 0x28) = 0x595e9fbd94fda766UL;
        tls_initialized = 1;
    }
}

void *wunix_tls_connect(int fd, const char *hostname, const char *ca_file, int insecure) {
    wunix_init_tls();

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
