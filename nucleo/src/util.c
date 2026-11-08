/*
 * ☀ Solar — utilidades generales del núcleo
 * Errores en español, cadenas, rutas, XDG, procesos y árboles de archivos.
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <dirent.h>
#include <fcntl.h>
#include <poll.h>
#include <pthread.h>
#include <pwd.h>
#include <signal.h>
#include <stdint.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

extern char **environ;

const char *solar_version(void) { return SOLAR_VERSION; }

const char *solar_error_texto(int codigo)
{
    switch (codigo) {
    case 0:            return "Todo salió bien";
    case EACCES:       return "Permiso denegado";
    case EPERM:        return "Operación no permitida";
    case ENOENT:       return "No existe el archivo o la carpeta";
    case EEXIST:       return "Ya existe un elemento con ese nombre";
    case ENOTDIR:      return "No es una carpeta";
    case EISDIR:       return "Es una carpeta";
    case ENOTEMPTY:    return "La carpeta no está vacía";
    case ENOSPC:       return "No queda espacio en el dispositivo";
    case EDQUOT:       return "Se superó la cuota de disco";
    case EROFS:        return "El dispositivo es de solo lectura";
    case EXDEV:        return "No se puede mover entre dispositivos distintos";
    case EBUSY:        return "El dispositivo o recurso está ocupado";
    case EINVAL:       return "Nombre o valor no válido";
    case ENAMETOOLONG: return "El nombre o la ruta son demasiado largos";
    case ELOOP:        return "Demasiados enlaces simbólicos";
    case EIO:          return "Error de lectura/escritura en el dispositivo";
    case ENOMEM:       return "No hay memoria suficiente";
    case EMFILE:
    case ENFILE:       return "Hay demasiados archivos abiertos";
    case ECANCELED:    return "Operación cancelada";
    case ETXTBSY:      return "El archivo está en uso";
    case EFBIG:        return "El archivo es demasiado grande para este dispositivo";
    case ENODEV:       return "No existe el dispositivo";
    case ENXIO:        return "El dispositivo no está disponible";
    case ETIMEDOUT:    return "Se agotó el tiempo de espera";
    case ENOTSUP:      return "Operación no soportada";
    case ENOEXEC:      return "No es un programa válido";
    case EAGAIN:       return "Recurso no disponible por ahora, inténtalo de nuevo";
    case EFAULT:       return "Dirección de memoria no válida";
    case ENOSYS:       return "Función no implementada en este sistema";
    case ESRCH:        return "No existe el proceso";
    case ECHILD:       return "El proceso hijo terminó con error";
    default:           return "Error desconocido";
    }
}

/* ===================================================================== */
/*  Cadenas dinámicas                                                    */
/* ===================================================================== */

void cadena_iniciar(Cadena *c)
{
    c->datos = NULL;
    c->largo = 0;
    c->capacidad = 0;
    c->sin_memoria = 0;
}

static int cadena_reservar(Cadena *c, size_t extra)
{
    if (c->sin_memoria) return 0;
    size_t necesario = c->largo + extra + 1;
    if (necesario <= c->capacidad) return 1;
    size_t nueva = c->capacidad ? c->capacidad : 256;
    while (nueva < necesario) nueva *= 2;
    char *p = realloc(c->datos, nueva);
    if (!p) {
        c->sin_memoria = 1;
        return 0;
    }
    c->datos = p;
    c->capacidad = nueva;
    return 1;
}

void cadena_agregar_n(Cadena *c, const char *texto, size_t n)
{
    if (!cadena_reservar(c, n)) return;
    memcpy(c->datos + c->largo, texto, n);
    c->largo += n;
    c->datos[c->largo] = '\0';
}

void cadena_agregar(Cadena *c, const char *texto)
{
    if (texto) cadena_agregar_n(c, texto, strlen(texto));
}

void cadena_printf(Cadena *c, const char *formato, ...)
{
    va_list ap;
    va_start(ap, formato);
    char pequeno[256];
    int n = vsnprintf(pequeno, sizeof pequeno, formato, ap);
    va_end(ap);
    if (n < 0) return;
    if ((size_t)n < sizeof pequeno) {
        cadena_agregar_n(c, pequeno, (size_t)n);
        return;
    }
    if (!cadena_reservar(c, (size_t)n)) return;
    va_start(ap, formato);
    vsnprintf(c->datos + c->largo, (size_t)n + 1, formato, ap);
    va_end(ap);
    c->largo += (size_t)n;
}

void cadena_agregar_lua(Cadena *c, const char *texto)
{
    cadena_agregar_n(c, "\"", 1);
    for (const unsigned char *p = (const unsigned char *)(texto ? texto : ""); *p; p++) {
        switch (*p) {
        case '"':  cadena_agregar(c, "\\\""); break;
        case '\\': cadena_agregar(c, "\\\\"); break;
        case '\n': cadena_agregar(c, "\\n"); break;
        case '\r': cadena_agregar(c, "\\r"); break;
        case '\t': cadena_agregar(c, "\\t"); break;
        default:
            if (*p < 0x20 || *p == 0x7f) {
                cadena_printf(c, "\\%03u", (unsigned)*p);
            } else {
                cadena_agregar_n(c, (const char *)p, 1);
            }
        }
    }
    cadena_agregar_n(c, "\"", 1);
}

void cadena_liberar(Cadena *c)
{
    free(c->datos);
    cadena_iniciar(c);
}

/* ===================================================================== */
/*  Memoria y texto                                                      */
/* ===================================================================== */

char *solar_strdup(const char *s)
{
    if (!s) return NULL;
    size_t n = strlen(s);
    char *r = malloc(n + 1);
    if (r) memcpy(r, s, n + 1);
    return r;
}

char *solar_strndup(const char *s, size_t n)
{
    if (!s) return NULL;
    size_t largo = strnlen(s, n);
    char *r = malloc(largo + 1);
    if (!r) return NULL;
    memcpy(r, s, largo);
    r[largo] = '\0';
    return r;
}

int copiar_texto(char *destino, size_t tam, const char *origen)
{
    if (!destino || tam == 0) return EINVAL;
    if (!origen) origen = "";
    size_t n = strlen(origen);
    if (n >= tam) {
        memcpy(destino, origen, tam - 1);
        destino[tam - 1] = '\0';
        return ENAMETOOLONG;
    }
    memcpy(destino, origen, n + 1);
    return 0;
}

void poner_error(char *buf, size_t tam, const char *formato, ...)
{
    if (!buf || tam == 0) return;
    va_list ap;
    va_start(ap, formato);
    vsnprintf(buf, tam, formato, ap);
    va_end(ap);
}

void agregar_aviso(char *buf, size_t tam, const char *formato, ...)
{
    if (!buf || tam == 0) return;
    size_t usado = strnlen(buf, tam);
    if (usado + 2 >= tam) return;
    va_list ap;
    va_start(ap, formato);
    int n = vsnprintf(buf + usado, tam - usado, formato, ap);
    va_end(ap);
    if (n < 0) return;
    usado = strnlen(buf, tam);
    if (usado + 1 < tam && (usado == 0 || buf[usado - 1] != '\n')) {
        buf[usado] = '\n';
        buf[usado + 1] = '\0';
    }
}

int formatear(char *buf, size_t tam, const char *formato, ...)
{
    va_list ap;
    va_start(ap, formato);
    int n = vsnprintf(buf, tam, formato, ap);
    va_end(ap);
    return (n < 0 || (size_t)n >= tam) ? ENAMETOOLONG : 0;
}

int termina_con(const char *s, const char *sufijo)
{
    size_t a = strlen(s), b = strlen(sufijo);
    return a >= b && strcmp(s + a - b, sufijo) == 0;
}

void a_minusculas(char *s)
{
    for (; *s; s++) {
        if (*s >= 'A' && *s <= 'Z') *s = (char)(*s + 32);
    }
}

/* ===================================================================== */
/*  Rutas                                                                */
/* ===================================================================== */

char *ruta_unir(const char *dir, const char *nombre)
{
    size_t a = strlen(dir), b = strlen(nombre);
    int barra = !(a > 0 && dir[a - 1] == '/');
    char *r = malloc(a + (size_t)barra + b + 1);
    if (!r) return NULL;
    memcpy(r, dir, a);
    if (barra) r[a] = '/';
    memcpy(r + a + (size_t)barra, nombre, b + 1);
    return r;
}

const char *ruta_base(const char *ruta)
{
    const char *barra = strrchr(ruta, '/');
    if (!barra) return ruta;
    if (barra[1] == '\0' && barra != ruta) {
        /* "algo/" -> buscar la barra anterior */
        const char *p = barra - 1;
        while (p > ruta && *p != '/') p--;
        return *p == '/' ? p + 1 : p;
    }
    return barra[1] ? barra + 1 : barra;
}

char *ruta_padre(const char *ruta)
{
    const char *barra = strrchr(ruta, '/');
    if (!barra) return solar_strdup(".");
    if (barra == ruta) return solar_strdup("/");
    return solar_strndup(ruta, (size_t)(barra - ruta));
}

void ruta_normalizar(char *ruta)
{
    if (!ruta || !*ruta) return;
    char *escribir = ruta;
    const char *leer = ruta;
    while (*leer) {
        if (leer[0] == '/' && leer[1] == '/') { leer++; continue; }
        if (leer[0] == '/' && leer[1] == '.' && (leer[2] == '/' || leer[2] == '\0')) {
            leer += 2;
            if (*leer == '\0' && escribir == ruta) *escribir++ = '/';
            continue;
        }
        *escribir++ = *leer++;
    }
    *escribir = '\0';
    size_t n = strlen(ruta);
    while (n > 1 && ruta[n - 1] == '/') ruta[--n] = '\0';
}

int ruta_es_dentro_de(const char *hija, const char *padre)
{
    size_t n = strlen(padre);
    while (n > 1 && padre[n - 1] == '/') n--;
    if (strncmp(hija, padre, n) != 0) return 0;
    if (n == 1 && padre[0] == '/') return 1;
    return hija[n] == '\0' || hija[n] == '/';
}

int existe(const char *ruta)
{
    struct stat st;
    return lstat(ruta, &st) == 0;
}

int es_carpeta(const char *ruta)
{
    struct stat st;
    return stat(ruta, &st) == 0 && S_ISDIR(st.st_mode);
}

int crear_dirs(const char *ruta, mode_t modo)
{
    char tmp[PATH_MAX];
    if (copiar_texto(tmp, sizeof tmp, ruta)) return ENAMETOOLONG;
    size_t n = strlen(tmp);
    if (n == 0) return EINVAL;
    for (size_t i = 1; i <= n; i++) {
        if (tmp[i] == '/' || tmp[i] == '\0') {
            char guardado = tmp[i];
            tmp[i] = '\0';
            if (mkdir(tmp, modo) != 0 && errno != EEXIST) return errno;
            tmp[i] = guardado;
        }
    }
    return es_carpeta(ruta) ? 0 : ENOTDIR;
}

int obtener_hogar(char *salida, size_t tam)
{
    const char *h = getenv("HOME");
    if (h && h[0] == '/') return copiar_texto(salida, tam, h);
    struct passwd pw, *res = NULL;
    char buf[4096];
    if (getpwuid_r(getuid(), &pw, buf, sizeof buf, &res) == 0 && res && res->pw_dir)
        return copiar_texto(salida, tam, res->pw_dir);
    return copiar_texto(salida, tam, "/");
}

int solar_dir_hogar(char *salida, size_t tam) { return obtener_hogar(salida, tam); }

void solar_fecha_local(int64_t instante, int32_t campos[7])
{
    time_t t = (time_t)instante;
    struct tm tm;
    memset(&tm, 0, sizeof tm);
    localtime_r(&t, &tm);
    campos[0] = tm.tm_year + 1900;
    campos[1] = tm.tm_mon + 1;
    campos[2] = tm.tm_mday;
    campos[3] = tm.tm_hour;
    campos[4] = tm.tm_min;
    campos[5] = tm.tm_sec;
    campos[6] = tm.tm_wday;
}

int solar_expandir_ruta(const char *ruta, char *salida, size_t tam)
{
    if (!ruta) return EINVAL;
    if (ruta[0] == '~' && (ruta[1] == '\0' || ruta[1] == '/')) {
        char hogar[PATH_MAX];
        obtener_hogar(hogar, sizeof hogar);
        int n = snprintf(salida, tam, "%s%s", hogar, ruta + 1);
        if (n < 0 || (size_t)n >= tam) return ENAMETOOLONG;
        ruta_normalizar(salida);
        return 0;
    }
    int r = copiar_texto(salida, tam, ruta);
    if (r == 0) ruta_normalizar(salida);
    return r;
}

static int dir_xdg(const char *variable, const char *defecto, char *salida, size_t tam)
{
    const char *v = getenv(variable);
    if (v && v[0] == '/') return copiar_texto(salida, tam, v);
    char hogar[PATH_MAX];
    obtener_hogar(hogar, sizeof hogar);
    int n = snprintf(salida, tam, "%s/%s", hogar, defecto);
    return (n < 0 || (size_t)n >= tam) ? ENAMETOOLONG : 0;
}

int solar_dir_config(char *salida, size_t tam)
{
    char base[PATH_MAX];
    int r = dir_xdg("XDG_CONFIG_HOME", ".config", base, sizeof base);
    if (r) return r;
    int n = snprintf(salida, tam, "%s/solar", base);
    if (n < 0 || (size_t)n >= tam) return ENAMETOOLONG;
    return crear_dirs(salida, 0755);
}

int solar_dir_datos(char *salida, size_t tam)
{
    char base[PATH_MAX];
    int r = dir_xdg("XDG_DATA_HOME", ".local/share", base, sizeof base);
    if (r) return r;
    int n = snprintf(salida, tam, "%s/solar", base);
    if (n < 0 || (size_t)n >= tam) return ENAMETOOLONG;
    return crear_dirs(salida, 0755);
}

int xdg_dir_usuario(const char *clave, char *salida, size_t tam)
{
    char config[PATH_MAX], archivo[PATH_MAX + 32], hogar[PATH_MAX];
    if (dir_xdg("XDG_CONFIG_HOME", ".config", config, sizeof config)) return ENOENT;
    snprintf(archivo, sizeof archivo, "%s/user-dirs.dirs", config);
    obtener_hogar(hogar, sizeof hogar);
    char *texto = leer_archivo(archivo, 64 * 1024, NULL);
    if (!texto) return ENOENT;

    char buscado[64];
    snprintf(buscado, sizeof buscado, "XDG_%s_DIR=", clave);
    int resultado = ENOENT;
    for (char *linea = strtok(texto, "\n"); linea; linea = strtok(NULL, "\n")) {
        while (*linea == ' ' || *linea == '\t') linea++;
        if (strncmp(linea, buscado, strlen(buscado)) != 0) continue;
        char *v = linea + strlen(buscado);
        if (*v == '"') v++;
        char *fin = strrchr(v, '"');
        if (fin) *fin = '\0';
        int n;
        if (strncmp(v, "$HOME", 5) == 0) n = snprintf(salida, tam, "%s%s", hogar, v + 5);
        else n = snprintf(salida, tam, "%s", v);
        if (n > 0 && (size_t)n < tam) {
            ruta_normalizar(salida);
            /* XDG dice que si apunta a $HOME, el directorio está "desactivado" */
            resultado = strcmp(salida, hogar) == 0 ? ENOENT : 0;
        }
        break;
    }
    free(texto);
    return resultado;
}

char *leer_archivo(const char *ruta, size_t limite, size_t *largo)
{
    int fd = open(ruta, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return NULL;
    size_t cap = 4096, n = 0;
    char *buf = malloc(cap + 1);
    if (!buf) { close(fd); return NULL; }
    for (;;) {
        if (n == cap) {
            if (cap >= limite) break;
            size_t nueva = cap * 2 > limite ? limite : cap * 2;
            char *p = realloc(buf, nueva + 1);
            if (!p) { free(buf); close(fd); return NULL; }
            buf = p;
            cap = nueva;
        }
        ssize_t r = read(fd, buf + n, cap - n);
        if (r < 0) {
            if (errno == EINTR) continue;
            free(buf);
            close(fd);
            return NULL;
        }
        if (r == 0) break;
        n += (size_t)r;
    }
    close(fd);
    buf[n] = '\0';
    if (largo) *largo = n;
    return buf;
}

int escribir_atomico(const char *ruta, const char *datos, size_t largo)
{
    char tmp[PATH_MAX];
    int n = snprintf(tmp, sizeof tmp, "%s.tmp-XXXXXX", ruta);
    if (n < 0 || (size_t)n >= sizeof tmp) return ENAMETOOLONG;
    int fd = mkostemp(tmp, O_CLOEXEC);
    if (fd < 0) return errno;
    size_t escrito = 0;
    while (escrito < largo) {
        ssize_t r = write(fd, datos + escrito, largo - escrito);
        if (r < 0) {
            if (errno == EINTR) continue;
            int e = errno;
            close(fd);
            unlink(tmp);
            return e;
        }
        escrito += (size_t)r;
    }
    fchmod(fd, 0644);
    if (fsync(fd) != 0 && errno != EINVAL) {
        int e = errno;
        close(fd);
        unlink(tmp);
        return e;
    }
    if (close(fd) != 0) {
        int e = errno;
        unlink(tmp);
        return e;
    }
    if (rename(tmp, ruta) != 0) {
        int e = errno;
        unlink(tmp);
        return e;
    }
    return 0;
}

/* ===================================================================== */
/*  Procesos                                                             */
/* ===================================================================== */

int buscar_en_path(const char *programa, char *salida, size_t tam)
{
    if (!programa || !*programa) return EINVAL;
    if (strchr(programa, '/')) {
        if (access(programa, X_OK) != 0) return errno;
        return copiar_texto(salida, tam, programa);
    }
    const char *path = getenv("PATH");
    if (!path || !*path) path = "/usr/local/bin:/usr/bin:/bin";
    const char *p = path;
    while (*p) {
        const char *fin = strchr(p, ':');
        size_t largo = fin ? (size_t)(fin - p) : strlen(p);
        char candidato[PATH_MAX];
        int n;
        if (largo == 0) n = snprintf(candidato, sizeof candidato, "./%s", programa);
        else n = snprintf(candidato, sizeof candidato, "%.*s/%s", (int)largo, p, programa);
        if (n > 0 && (size_t)n < sizeof candidato) {
            struct stat st;
            if (stat(candidato, &st) == 0 && S_ISREG(st.st_mode) && access(candidato, X_OK) == 0)
                return copiar_texto(salida, tam, candidato);
        }
        if (!fin) break;
        p = fin + 1;
    }
    return ENOENT;
}

void liberar_argv(char **argv, size_t n)
{
    if (!argv) return;
    for (size_t i = 0; i < n; i++) free(argv[i]);
    free(argv);
}

int dividir_comando(const char *comando, char ***argv_salida, size_t *n_salida)
{
    *argv_salida = NULL;
    *n_salida = 0;
    if (!comando) return EINVAL;
    size_t cap = 8, n = 0;
    char **argv = calloc(cap, sizeof *argv);
    if (!argv) return ENOMEM;
    Cadena actual;
    cadena_iniciar(&actual);
    int hay_palabra = 0;
    char comilla = 0;
    for (const char *p = comando;; p++) {
        char c = *p;
        if (c == '\0' || (!comilla && (c == ' ' || c == '\t' || c == '\n'))) {
            if (hay_palabra) {
                if (n + 1 >= cap) {
                    cap *= 2;
                    char **q = realloc(argv, cap * sizeof *q);
                    if (!q) { cadena_liberar(&actual); liberar_argv(argv, n); return ENOMEM; }
                    argv = q;
                }
                argv[n++] = actual.datos ? actual.datos : solar_strdup("");
                cadena_iniciar(&actual);
                hay_palabra = 0;
            }
            if (c == '\0') break;
            continue;
        }
        if (comilla) {
            if (c == comilla) { comilla = 0; continue; }
            if (comilla == '"' && c == '\\' && (p[1] == '"' || p[1] == '\\')) { p++; c = *p; }
            cadena_agregar_n(&actual, &c, 1);
            continue;
        }
        if (c == '\'' || c == '"') { comilla = c; hay_palabra = 1; continue; }
        if (c == '\\' && p[1]) { p++; c = *p; }
        cadena_agregar_n(&actual, &c, 1);
        hay_palabra = 1;
    }
    cadena_liberar(&actual);
    if (comilla) { liberar_argv(argv, n); return EINVAL; }
    argv[n] = NULL;
    *argv_salida = argv;
    *n_salida = n;
    return 0;
}

static void *recolector(void *arg)
{
    pid_t pid = (pid_t)(intptr_t)arg;
    int estado;
    while (waitpid(pid, &estado, 0) < 0 && errno == EINTR) {}
    return NULL;
}

/* Prepara el hijo: sesión nueva, señales por defecto y E/S a /dev/null. Solo
 * usa funciones seguras tras fork(). */
static void preparar_hijo(int redirigir)
{
    sigset_t vacio;
    sigemptyset(&vacio);
    sigprocmask(SIG_SETMASK, &vacio, NULL);
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = SIG_DFL;
    sigaction(SIGPIPE, &sa, NULL);
    if (redirigir) {
        int nulo = open("/dev/null", O_RDWR);
        if (nulo >= 0) {
            dup2(nulo, 0);
            dup2(nulo, 1);
            dup2(nulo, 2);
            if (nulo > 2) close(nulo);
        }
    }
}

int solar_lanzar(const char *const *argv, const char *dir_trabajo)
{
    if (!argv || !argv[0] || !argv[0][0]) return EINVAL;
    char exe[PATH_MAX];
    int r = buscar_en_path(argv[0], exe, sizeof exe);
    if (r) return r;
    if (dir_trabajo && *dir_trabajo && !es_carpeta(dir_trabajo)) return ENOENT;

    pid_t pid = fork();
    if (pid < 0) return errno;
    if (pid == 0) {
        if (dir_trabajo && *dir_trabajo && chdir(dir_trabajo) != 0) _exit(126);
        setsid();
        preparar_hijo(1);
        execve(exe, (char *const *)argv, environ);
        _exit(127);
    }
    pthread_t hilo;
    pthread_attr_t atributos;
    pthread_attr_init(&atributos);
    pthread_attr_setdetachstate(&atributos, PTHREAD_CREATE_DETACHED);
    pthread_attr_setstacksize(&atributos, 64 * 1024);
    if (pthread_create(&hilo, &atributos, recolector, (void *)(intptr_t)pid) != 0) {
        /* Sin hilo recolector el proceso quedaría zombi al terminar; no es
         * grave, pero lo intentamos limpiar sin bloquear. */
        waitpid(pid, NULL, WNOHANG);
    }
    pthread_attr_destroy(&atributos);
    return 0;
}

static int64_t ms_ahora(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

int ejecutar_y_esperar(const char *const *argv, char *salida, size_t tam, int timeout_ms)
{
    if (salida && tam) salida[0] = '\0';
    char exe[PATH_MAX];
    int r = buscar_en_path(argv[0], exe, sizeof exe);
    if (r) return -r;
    int tubo[2];
    if (pipe2(tubo, O_CLOEXEC) != 0) return -errno;
    pid_t pid = fork();
    if (pid < 0) {
        int e = errno;
        close(tubo[0]);
        close(tubo[1]);
        return -e;
    }
    if (pid == 0) {
        preparar_hijo(0);
        int nulo = open("/dev/null", O_RDONLY);
        if (nulo >= 0) { dup2(nulo, 0); if (nulo > 2) close(nulo); }
        dup2(tubo[1], 1);
        dup2(tubo[1], 2);
        execve(exe, (char *const *)argv, environ);
        _exit(127);
    }
    close(tubo[1]);
    size_t usado = 0;
    int64_t limite = ms_ahora() + timeout_ms;
    int agotado = 0;
    for (;;) {
        int64_t resta = limite - ms_ahora();
        if (resta <= 0) { agotado = 1; break; }
        struct pollfd pfd = { .fd = tubo[0], .events = POLLIN };
        int p = poll(&pfd, 1, (int)resta);
        if (p < 0) { if (errno == EINTR) continue; break; }
        if (p == 0) { agotado = 1; break; }
        char buf[512];
        ssize_t n = read(tubo[0], buf, sizeof buf);
        if (n < 0) { if (errno == EINTR) continue; break; }
        if (n == 0) break;
        if (salida && tam > 1 && usado < tam - 1) {
            size_t copia = (size_t)n;
            if (copia > tam - 1 - usado) copia = tam - 1 - usado;
            memcpy(salida + usado, buf, copia);
            usado += copia;
            salida[usado] = '\0';
        }
    }
    close(tubo[0]);
    if (agotado) kill(pid, SIGTERM);
    int estado = 0;
    while (waitpid(pid, &estado, 0) < 0 && errno == EINTR) {}
    if (agotado) return -ETIMEDOUT;
    if (WIFEXITED(estado)) {
        int codigo = WEXITSTATUS(estado);
        return codigo == 127 ? -ENOENT : codigo;
    }
    return -ECHILD;
}

int solar_abrir(const char *ruta)
{
    if (!ruta || !*ruta) return EINVAL;
    char *dir = ruta_padre(ruta);
    const char *argv[] = { "xdg-open", ruta, NULL };
    int r = solar_lanzar(argv, dir && es_carpeta(dir) ? dir : NULL);
    free(dir);
    return r;
}

int solar_abrir_con(const char *comando, const char *const *rutas, size_t cantidad)
{
    char **base = NULL;
    size_t n = 0;
    int r = dividir_comando(comando, &base, &n);
    if (r) return r;
    if (n == 0) { liberar_argv(base, n); return EINVAL; }

    const char **argv = calloc(n + cantidad + 1, sizeof *argv);
    if (!argv) { liberar_argv(base, n); return ENOMEM; }
    size_t k = 0;
    int puesto = 0;
    for (size_t i = 0; i < n; i++) {
        if (!strcmp(base[i], "%f") || !strcmp(base[i], "%F") ||
            !strcmp(base[i], "%u") || !strcmp(base[i], "%U")) {
            for (size_t j = 0; j < cantidad; j++) argv[k++] = rutas[j];
            puesto = 1;
        } else {
            argv[k++] = base[i];
        }
    }
    if (!puesto)
        for (size_t j = 0; j < cantidad; j++) argv[k++] = rutas[j];
    argv[k] = NULL;

    char *dir = cantidad > 0 ? ruta_padre(rutas[0]) : NULL;
    r = solar_lanzar(argv, dir && es_carpeta(dir) ? dir : NULL);
    free(dir);
    free(argv);
    liberar_argv(base, n);
    return r;
}

static const char *const TERMINALES[] = {
    "foot", "kitty", "alacritty", "wezterm", "ghostty", "konsole", "gnome-terminal",
    "kgx", "ptyxis", "xfce4-terminal", "tilix", "terminator", "lxterminal",
    "mate-terminal", "qterminal", "urxvt", "st", "xterm",
};

int solar_detectar_terminal(char *salida, size_t tam)
{
    char exe[PATH_MAX];
    const char *env = getenv("TERMINAL");
    if (env && *env) {
        char **argv = NULL;
        size_t n = 0;
        if (dividir_comando(env, &argv, &n) == 0 && n > 0 && buscar_en_path(argv[0], exe, sizeof exe) == 0) {
            liberar_argv(argv, n);
            return copiar_texto(salida, tam, env);
        }
        liberar_argv(argv, n);
    }
    for (size_t i = 0; i < SOLAR_LEN(TERMINALES); i++) {
        if (buscar_en_path(TERMINALES[i], exe, sizeof exe) == 0)
            return copiar_texto(salida, tam, TERMINALES[i]);
    }
    return ENOENT;
}

int solar_abrir_terminal(const char *terminal, const char *dir)
{
    char detectado[PATH_MAX];
    if (!terminal || !*terminal) {
        int r = solar_detectar_terminal(detectado, sizeof detectado);
        if (r) return r;
        terminal = detectado;
    }
    char **base = NULL;
    size_t n = 0;
    int r = dividir_comando(terminal, &base, &n);
    if (r) return r;
    if (n == 0) { liberar_argv(base, n); return EINVAL; }

    const char *nombre = ruta_base(base[0]);
    const char **argv = calloc(n + 4, sizeof *argv);
    if (!argv) { liberar_argv(base, n); return ENOMEM; }
    size_t k = 0;
    for (size_t i = 0; i < n; i++) argv[k++] = base[i];

    char opcion[PATH_MAX + 32];
    if (dir && *dir) {
        if (!strcmp(nombre, "gnome-terminal") || !strcmp(nombre, "xfce4-terminal") ||
            !strcmp(nombre, "kgx") || !strcmp(nombre, "ptyxis") || !strcmp(nombre, "mate-terminal")) {
            snprintf(opcion, sizeof opcion, "--working-directory=%s", dir);
            argv[k++] = opcion;
        } else if (!strcmp(nombre, "konsole")) {
            argv[k++] = "--workdir";
            argv[k++] = dir;
        } else if (!strcmp(nombre, "wezterm") && n == 1) {
            argv[k++] = "start";
            argv[k++] = "--cwd";
            argv[k++] = dir;
        }
    }
    argv[k] = NULL;
    r = solar_lanzar(argv, dir);
    free(argv);
    liberar_argv(base, n);
    return r;
}

/* ===================================================================== */
/*  Árboles de archivos                                                  */
/* ===================================================================== */

#include <sys/stat.h>

static int eliminar_dir(int padre_fd, const char *nombre, const char *ruta,
                        CadaElementoFn cada, void *datos)
{
    int fd = openat(padre_fd, nombre, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0 && errno == EACCES) {
        /* Carpeta sin permiso de lectura pero nuestra: intentamos dárnoslo */
        if (fchmodat(padre_fd, nombre, 0700, 0) != 0) { /* seguimos igual */ }
        fd = openat(padre_fd, nombre, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    }
    if (fd < 0) return errno;
    struct stat st;
    if (fstat(fd, &st) == 0 && st.st_uid == geteuid() && (st.st_mode & S_IRWXU) != S_IRWXU)
        fchmod(fd, (st.st_mode & 07777) | S_IRWXU);

    DIR *d = fdopendir(fd);
    if (!d) {
        int e = errno;
        close(fd);
        return e;
    }
    int resultado = 0;
    struct dirent *de;
    while ((de = readdir(d))) {
        if (!strcmp(de->d_name, ".") || !strcmp(de->d_name, "..")) continue;
        char *hija = ruta_unir(ruta, de->d_name);
        if (!hija) { resultado = ENOMEM; break; }
        struct stat sh;
        if (fstatat(dirfd(d), de->d_name, &sh, AT_SYMLINK_NOFOLLOW) != 0) {
            if (errno == ENOENT) { free(hija); continue; }
            resultado = errno;
            free(hija);
            break;
        }
        if (S_ISDIR(sh.st_mode)) {
            resultado = eliminar_dir(dirfd(d), de->d_name, hija, cada, datos);
        } else if (unlinkat(dirfd(d), de->d_name, 0) != 0) {
            resultado = errno == ENOENT ? 0 : errno;
        } else if (cada && cada(datos, hija)) {
            resultado = ECANCELED;
        }
        free(hija);
        if (resultado) break;
    }
    closedir(d);
    if (resultado) return resultado;
    if (unlinkat(padre_fd, nombre, AT_REMOVEDIR) != 0 && errno != ENOENT) return errno;
    if (cada && cada(datos, ruta)) return ECANCELED;
    return 0;
}

int eliminar_arbol(const char *ruta, CadaElementoFn cada, void *datos)
{
    struct stat st;
    if (lstat(ruta, &st) != 0) return errno;
    if (!S_ISDIR(st.st_mode)) {
        if (unlink(ruta) != 0) return errno;
        if (cada && cada(datos, ruta)) return ECANCELED;
        return 0;
    }
    return eliminar_dir(AT_FDCWD, ruta, ruta, cada, datos);
}

static int contar_dir(int fd, uint64_t *bytes, uint64_t *elementos, int profundidad)
{
    if (profundidad > 4096) return ELOOP;
    DIR *d = fdopendir(fd);
    if (!d) {
        int e = errno;
        close(fd);
        return e;
    }
    struct dirent *de;
    while ((de = readdir(d))) {
        if (!strcmp(de->d_name, ".") || !strcmp(de->d_name, "..")) continue;
        struct stat st;
        if (fstatat(dirfd(d), de->d_name, &st, AT_SYMLINK_NOFOLLOW) != 0) continue;
        (*elementos)++;
        if (S_ISREG(st.st_mode)) {
            *bytes += (uint64_t)st.st_size;
        } else if (S_ISDIR(st.st_mode)) {
            int hijo = openat(dirfd(d), de->d_name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
            if (hijo >= 0) contar_dir(hijo, bytes, elementos, profundidad + 1);
        }
    }
    closedir(d);
    return 0;
}

int contar_arbol(const char *ruta, uint64_t *bytes, uint64_t *elementos)
{
    struct stat st;
    if (lstat(ruta, &st) != 0) return errno;
    (*elementos)++;
    if (S_ISREG(st.st_mode)) {
        *bytes += (uint64_t)st.st_size;
        return 0;
    }
    if (!S_ISDIR(st.st_mode)) return 0;
    int fd = open(ruta, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0) return 0; /* sin permiso: contamos solo la carpeta */
    return contar_dir(fd, bytes, elementos, 0);
}
