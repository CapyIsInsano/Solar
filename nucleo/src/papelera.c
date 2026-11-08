/*
 * ☀ Solar — papelera, según la especificación de freedesktop.org
 * (compatible con la papelera de Nautilus, Dolphin, Thunar, etc.)
 *
 *   ~/.local/share/Trash/{files,info}      papelera de la carpeta personal
 *   <unidad>/.Trash-<uid>/{files,info}     papelera en memorias USB y otros discos
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <ctype.h>
#include <dirent.h>
#include <fcntl.h>
#include <time.h>
#include <unistd.h>

int solar_papelera_dir(char *salida, size_t tam)
{
    const char *xdg = getenv("XDG_DATA_HOME");
    int n;
    if (xdg && xdg[0] == '/') {
        n = snprintf(salida, tam, "%s/Trash", xdg);
    } else {
        char hogar[PATH_MAX];
        obtener_hogar(hogar, sizeof hogar);
        n = snprintf(salida, tam, "%s/.local/share/Trash", hogar);
    }
    return (n < 0 || (size_t)n >= tam) ? ENAMETOOLONG : 0;
}

static int asegurar_papelera(const char *dir)
{
    int r = crear_dirs(dir, 0700);
    if (r) return r;
    char sub[PATH_MAX + 8];
    snprintf(sub, sizeof sub, "%s/files", dir);
    if (mkdir(sub, 0700) != 0 && errno != EEXIST) return errno;
    snprintf(sub, sizeof sub, "%s/info", dir);
    if (mkdir(sub, 0700) != 0 && errno != EEXIST) return errno;
    return 0;
}

/* La carpeta más alta que sigue en el mismo dispositivo (el punto de montaje). */
static void raiz_dispositivo(const char *ruta, dev_t dispositivo, char *salida, size_t tam)
{
    char *actual = ruta_padre(ruta);
    if (!actual) { copiar_texto(salida, tam, "/"); return; }
    for (;;) {
        if (!strcmp(actual, "/")) break;
        char *padre = ruta_padre(actual);
        struct stat st;
        if (!padre || stat(padre, &st) != 0 || st.st_dev != dispositivo) {
            free(padre);
            break;
        }
        free(actual);
        actual = padre;
    }
    copiar_texto(salida, tam, actual);
    free(actual);
}

static void codificar_uri(Cadena *c, const char *ruta)
{
    static const char *const SEGUROS = "-_.~/!$&'()*+,;=:@";
    for (const unsigned char *p = (const unsigned char *)ruta; *p; p++) {
        if (isalnum(*p) || strchr(SEGUROS, *p)) cadena_agregar_n(c, (const char *)p, 1);
        else cadena_printf(c, "%%%02X", (unsigned)*p);
    }
}

static int valor_hex(char c)
{
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

static void decodificar_uri(char *s)
{
    char *w = s;
    for (const char *r = s; *r; r++) {
        int a, b;
        if (r[0] == '%' && (a = valor_hex(r[1])) >= 0 && (b = valor_hex(r[2])) >= 0) {
            *w++ = (char)(a * 16 + b);
            r += 2;
        } else {
            *w++ = *r;
        }
    }
    *w = '\0';
}

/* Lee Path y DeletionDate de un .trashinfo. */
static int leer_info(const char *archivo, char *ruta, size_t tam, int64_t *fecha)
{
    *fecha = 0;
    ruta[0] = '\0';
    char *texto = leer_archivo(archivo, 64 * 1024, NULL);
    if (!texto) return errno ? errno : EIO;
    char *guardar = NULL;
    for (char *linea = strtok_r(texto, "\n", &guardar); linea; linea = strtok_r(NULL, "\n", &guardar)) {
        size_t n = strlen(linea);
        if (n && linea[n - 1] == '\r') linea[n - 1] = '\0';
        if (!strncmp(linea, "Path=", 5)) {
            copiar_texto(ruta, tam, linea + 5);
            decodificar_uri(ruta);
        } else if (!strncmp(linea, "DeletionDate=", 13)) {
            struct tm tm;
            memset(&tm, 0, sizeof tm);
            if (sscanf(linea + 13, "%d-%d-%dT%d:%d:%d", &tm.tm_year, &tm.tm_mon, &tm.tm_mday,
                       &tm.tm_hour, &tm.tm_min, &tm.tm_sec) >= 3) {
                tm.tm_year -= 1900;
                tm.tm_mon -= 1;
                tm.tm_isdst = -1;
                *fecha = (int64_t)mktime(&tm);
            }
        }
    }
    free(texto);
    return ruta[0] ? 0 : EINVAL;
}

/* ===================================================================== */
/*  Enviar a la papelera                                                 */
/* ===================================================================== */

static int enviar_uno(const char *original, dev_t dev_hogar, const char *papelera_hogar,
                      char *en_papelera, size_t en_papelera_tam, char *error, size_t error_tam)
{
    char ruta[PATH_MAX];
    if (copiar_texto(ruta, sizeof ruta, original)) return ENAMETOOLONG;
    ruta_normalizar(ruta);
    if (ruta[0] != '/') return EINVAL;
    if (!strcmp(ruta, "/")) return EINVAL;

    struct stat st;
    if (lstat(ruta, &st) != 0) return errno;

    char papelera[PATH_MAX], raiz[PATH_MAX];
    int relativa = 0;
    if (st.st_dev == dev_hogar) {
        copiar_texto(papelera, sizeof papelera, papelera_hogar);
    } else {
        raiz_dispositivo(ruta, st.st_dev, raiz, sizeof raiz);
        unsigned uid = (unsigned)getuid();
        int lista = 0;
        char admin[PATH_MAX + 16];
        if (formatear(admin, sizeof admin, "%s/.Trash", strcmp(raiz, "/") ? raiz : "")) return ENAMETOOLONG;
        struct stat sa;
        if (lstat(admin, &sa) == 0 && S_ISDIR(sa.st_mode) && (sa.st_mode & S_ISVTX) &&
            formatear(papelera, sizeof papelera, "%s/%u", admin, uid) == 0) {
            lista = asegurar_papelera(papelera) == 0;
        }
        if (!lista) {
            if (formatear(papelera, sizeof papelera, "%s/.Trash-%u", strcmp(raiz, "/") ? raiz : "", uid))
                return ENAMETOOLONG;
            int r = asegurar_papelera(papelera);
            if (r) {
                poner_error(error, error_tam,
                            "La unidad donde está «%s» no admite papelera (%s). Puedes eliminarlo para siempre.",
                            ruta_base(ruta), solar_error_texto(r));
                return r;
            }
        }
        struct stat sp;
        if (stat(papelera, &sp) != 0 || sp.st_dev != st.st_dev) {
            poner_error(error, error_tam, "No se encontró una papelera en la unidad de «%s».", ruta_base(ruta));
            return EXDEV;
        }
        relativa = 1;
    }

    /* Reservamos el nombre creando el .trashinfo con O_EXCL */
    const char *base = ruta_base(ruta);
    char nombre[300], info[PATH_MAX], destino[PATH_MAX];
    int fd = -1;
    for (int i = 1; i < 100000; i++) {
        int r0 = i == 1 ? formatear(nombre, sizeof nombre, "%s", base)
                        : formatear(nombre, sizeof nombre, "%s.%d", base, i);
        if (r0 || formatear(info, sizeof info, "%s/info/%s.trashinfo", papelera, nombre) ||
            formatear(destino, sizeof destino, "%s/files/%s", papelera, nombre))
            return ENAMETOOLONG;
        fd = open(info, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0600);
        if (fd >= 0) {
            if (existe(destino)) { close(fd); unlink(info); fd = -1; continue; }
            break;
        }
        if (errno != EEXIST) return errno;
    }
    if (fd < 0) return EEXIST;

    char fecha[32];
    time_t ahora = time(NULL);
    struct tm tm;
    localtime_r(&ahora, &tm);
    strftime(fecha, sizeof fecha, "%Y-%m-%dT%H:%M:%S", &tm);

    Cadena c;
    cadena_iniciar(&c);
    cadena_agregar(&c, "[Trash Info]\nPath=");
    if (relativa) {
        size_t lr = strcmp(raiz, "/") ? strlen(raiz) : 0;
        codificar_uri(&c, ruta + lr + 1);
    } else {
        codificar_uri(&c, ruta);
    }
    cadena_printf(&c, "\nDeletionDate=%s\n", fecha);
    int r = 0;
    if (c.sin_memoria) r = ENOMEM;
    size_t escrito = 0;
    while (!r && escrito < c.largo) {
        ssize_t w = write(fd, c.datos + escrito, c.largo - escrito);
        if (w < 0) { if (errno == EINTR) continue; r = errno; break; }
        escrito += (size_t)w;
    }
    cadena_liberar(&c);
    if (close(fd) != 0 && !r) r = errno;
    if (!r && rename(ruta, destino) != 0) r = errno;
    if (r) {
        unlink(info);
        return r;
    }
    copiar_texto(en_papelera, en_papelera_tam, destino);
    return 0;
}

int solar_papelera_enviar(const char *const *rutas, size_t cantidad, const SolarOpciones *opciones)
{
    char *error = opciones ? opciones->error : NULL;
    size_t error_tam = opciones ? opciones->error_tam : 0;
    if (error && error_tam) error[0] = '\0';

    char hogar[PATH_MAX];
    int r = solar_papelera_dir(hogar, sizeof hogar);
    if (!r) r = asegurar_papelera(hogar);
    if (r) {
        poner_error(error, error_tam, "No se pudo preparar la papelera: %s.", solar_error_texto(r));
        return r;
    }
    struct stat sh;
    if (stat(hogar, &sh) != 0) return errno;

    for (size_t i = 0; i < cantidad; i++) {
        if (opciones && opciones->progreso &&
            opciones->progreso(opciones->datos, 0, 0, i, cantidad, ruta_base(rutas[i])))
            return ECANCELED;
        char destino[PATH_MAX];
        r = enviar_uno(rutas[i], sh.st_dev, hogar, destino, sizeof destino, error, error_tam);
        if (r) {
            if (error && error_tam && !error[0])
                poner_error(error, error_tam, "No se pudo mover «%s» a la papelera: %s.",
                            ruta_base(rutas[i]), solar_error_texto(r));
            return r;
        }
        if (opciones && opciones->resultado) opciones->resultado(opciones->datos, rutas[i], destino);
    }
    if (opciones && opciones->progreso) opciones->progreso(opciones->datos, 0, 0, cantidad, cantidad, "");
    return 0;
}

/* ===================================================================== */
/*  Todas las papeleras visibles                                         */
/* ===================================================================== */

typedef struct {
    char *dir;     /* .../Trash o <unidad>/.Trash-uid */
    char *raiz;    /* NULL para la del hogar */
} Papelera;

static int es_fs_virtual(const char *tipo)
{
    static const char *const VIRTUALES[] = {
        "proc", "sysfs", "devtmpfs", "devpts", "tmpfs", "securityfs", "debugfs", "tracefs",
        "pstore", "bpf", "mqueue", "hugetlbfs", "fusectl", "configfs", "autofs", "nsfs",
        "efivarfs", "binfmt_misc", "rpc_pipefs", "ramfs", "squashfs", "overlay", "cgroup",
        "cgroup2", "selinuxfs", "fuse.portal", "fuse.gvfsd-fuse", "nfsd", "zfs",
    };
    for (size_t i = 0; i < SOLAR_LEN(VIRTUALES); i++)
        if (!strcmp(tipo, VIRTUALES[i])) return 1;
    return 0;
}

static size_t recolectar_papeleras(Papelera *salida, size_t maximo)
{
    size_t n = 0;
    char hogar[PATH_MAX];
    if (solar_papelera_dir(hogar, sizeof hogar) == 0 && n < maximo) {
        salida[n].dir = solar_strdup(hogar);
        salida[n].raiz = NULL;
        n++;
    }
    Montaje *m = NULL;
    size_t nm = 0;
    if (leer_montajes(&m, &nm) != 0) return n;
    unsigned uid = (unsigned)getuid();
    for (size_t i = 0; i < nm && n < maximo; i++) {
        if (!m[i].punto || es_fs_virtual(m[i].tipo_fs ? m[i].tipo_fs : "")) continue;
        const char *raiz = strcmp(m[i].punto, "/") ? m[i].punto : "";
        char candidatos[2][PATH_MAX + 32];
        snprintf(candidatos[0], sizeof candidatos[0], "%s/.Trash-%u", raiz, uid);
        snprintf(candidatos[1], sizeof candidatos[1], "%s/.Trash/%u", raiz, uid);
        for (int k = 0; k < 2 && n < maximo; k++) {
            struct stat st;
            if (lstat(candidatos[k], &st) != 0 || !S_ISDIR(st.st_mode)) continue;
            int repetida = 0;
            for (size_t j = 0; j < n; j++)
                if (!strcmp(salida[j].dir, candidatos[k])) repetida = 1;
            if (repetida) continue;
            salida[n].dir = solar_strdup(candidatos[k]);
            salida[n].raiz = solar_strdup(m[i].punto);
            n++;
        }
    }
    liberar_montajes(m, nm);
    return n;
}

static void liberar_papeleras(Papelera *p, size_t n)
{
    for (size_t i = 0; i < n; i++) {
        free(p[i].dir);
        free(p[i].raiz);
    }
}

static void listar_una(const Papelera *p, SolarPapelera *salida, size_t *cap)
{
    char dir_info[PATH_MAX + 8];
    snprintf(dir_info, sizeof dir_info, "%s/info", p->dir);
    DIR *d = opendir(dir_info);
    if (!d) return;
    struct dirent *de;
    while ((de = readdir(d))) {
        if (!termina_con(de->d_name, ".trashinfo")) continue;
        size_t ln = strlen(de->d_name) - strlen(".trashinfo");
        if (ln == 0) continue;
        char nombre[512];
        if (ln >= sizeof nombre) continue;
        memcpy(nombre, de->d_name, ln);
        nombre[ln] = '\0';

        char archivo[PATH_MAX], info[PATH_MAX + 300];
        snprintf(archivo, sizeof archivo, "%s/files/%s", p->dir, nombre);
        snprintf(info, sizeof info, "%s/%s", dir_info, de->d_name);
        struct stat st;
        if (lstat(archivo, &st) != 0) continue;

        char original[PATH_MAX];
        int64_t fecha = 0;
        if (leer_info(info, original, sizeof original, &fecha) != 0) copiar_texto(original, sizeof original, nombre);
        if (original[0] != '/' && p->raiz) {
            char completa[PATH_MAX * 2];
            snprintf(completa, sizeof completa, "%s/%s", strcmp(p->raiz, "/") ? p->raiz : "", original);
            copiar_texto(original, sizeof original, completa);
        }

        if (salida->cantidad == *cap) {
            size_t nuevo = *cap ? *cap * 2 : 32;
            SolarElementoPapelera *q = realloc(salida->elementos, nuevo * sizeof *q);
            if (!q) break;
            salida->elementos = q;
            *cap = nuevo;
        }
        SolarElementoPapelera *e = &salida->elementos[salida->cantidad];
        memset(e, 0, sizeof *e);
        e->nombre = solar_strdup(ruta_base(original));
        e->ruta_original = solar_strdup(original);
        e->ruta = solar_strdup(archivo);
        e->fecha = fecha;
        e->es_carpeta = S_ISDIR(st.st_mode) ? 1 : 0;
        e->tamano = S_ISREG(st.st_mode) ? (uint64_t)st.st_size : 0;
        e->categoria = (uint8_t)solar_categoria(e->nombre ? e->nombre : "", (uint32_t)st.st_mode,
                                                e->es_carpeta ? SOLAR_TIPO_CARPETA : SOLAR_TIPO_ARCHIVO);
        if (!e->nombre || !e->ruta_original || !e->ruta) {
            free(e->nombre);
            free(e->ruta_original);
            free(e->ruta);
            continue;
        }
        salida->cantidad++;
    }
    closedir(d);
}

static int por_fecha(const void *a, const void *b)
{
    const SolarElementoPapelera *x = a, *y = b;
    if (x->fecha != y->fecha) return x->fecha < y->fecha ? 1 : -1;
    return solar_comparar_natural(x->nombre, y->nombre);
}

int solar_papelera_listar(SolarPapelera *salida)
{
    salida->elementos = NULL;
    salida->cantidad = 0;
    Papelera papeleras[64];
    size_t n = recolectar_papeleras(papeleras, SOLAR_LEN(papeleras));
    size_t cap = 0;
    for (size_t i = 0; i < n; i++) listar_una(&papeleras[i], salida, &cap);
    liberar_papeleras(papeleras, n);
    if (salida->cantidad > 1)
        qsort(salida->elementos, salida->cantidad, sizeof salida->elementos[0], por_fecha);
    return 0;
}

void solar_papelera_liberar(SolarPapelera *p)
{
    if (!p) return;
    for (size_t i = 0; i < p->cantidad; i++) {
        free(p->elementos[i].nombre);
        free(p->elementos[i].ruta_original);
        free(p->elementos[i].ruta);
    }
    free(p->elementos);
    p->elementos = NULL;
    p->cantidad = 0;
}

/* Desde ".../files/nombre" calcula la papelera, el .trashinfo y la raíz. */
static int ubicar(const char *ruta, char *papelera, char *info, char *raiz)
{
    char *files = ruta_padre(ruta);
    if (!files) return ENOMEM;
    if (strcmp(ruta_base(files), "files") != 0) { free(files); return EINVAL; }
    char *dir = ruta_padre(files);
    free(files);
    if (!dir) return ENOMEM;
    copiar_texto(papelera, PATH_MAX, dir);
    snprintf(info, PATH_MAX, "%s/info/%s.trashinfo", dir, ruta_base(ruta));
    raiz[0] = '\0';
    const char *base = ruta_base(dir);
    if (!strncmp(base, ".Trash-", 7)) {
        char *r = ruta_padre(dir);
        if (r) copiar_texto(raiz, PATH_MAX, r);
        free(r);
    } else {
        char *padre = ruta_padre(dir);
        if (padre && !strcmp(ruta_base(padre), ".Trash")) {
            char *r = ruta_padre(padre);
            if (r) copiar_texto(raiz, PATH_MAX, r);
            free(r);
        }
        free(padre);
    }
    free(dir);
    return 0;
}

int solar_papelera_restaurar(const char *ruta, char *restaurado, size_t restaurado_tam,
                             char *error, size_t error_tam)
{
    if (error && error_tam) error[0] = '\0';
    char papelera[PATH_MAX], info[PATH_MAX], raiz[PATH_MAX];
    int r = ubicar(ruta, papelera, info, raiz);
    if (r) {
        poner_error(error, error_tam, "«%s» no está en una papelera.", ruta_base(ruta));
        return r;
    }
    char original[PATH_MAX];
    int64_t fecha;
    if (leer_info(info, original, sizeof original, &fecha) != 0) {
        poner_error(error, error_tam, "No se encontró de dónde venía «%s».", ruta_base(ruta));
        return ENOENT;
    }
    char destino[PATH_MAX];
    if (original[0] != '/') {
        if (formatear(destino, sizeof destino, "%s/%s", strcmp(raiz, "/") ? raiz : "", original))
            return ENAMETOOLONG;
    } else {
        copiar_texto(destino, sizeof destino, original);
    }
    ruta_normalizar(destino);

    char *padre = ruta_padre(destino);
    if (!padre) return ENOMEM;
    if (!es_carpeta(padre) && (r = crear_dirs(padre, 0755)) != 0) {
        poner_error(error, error_tam, "No se pudo recrear la carpeta original de «%s»: %s.",
                    ruta_base(destino), solar_error_texto(r));
        free(padre);
        return r;
    }
    if (existe(destino)) {
        char libre[512];
        struct stat st;
        int es_dir = lstat(ruta, &st) == 0 && S_ISDIR(st.st_mode);
        (void)es_dir;
        r = solar_nombre_libre(padre, ruta_base(destino), 0, libre, sizeof libre);
        if (r) { free(padre); return r; }
        char *nuevo = ruta_unir(padre, libre);
        if (!nuevo) { free(padre); return ENOMEM; }
        copiar_texto(destino, sizeof destino, nuevo);
        free(nuevo);
    }
    free(padre);
    if (rename(ruta, destino) != 0) {
        r = errno;
        poner_error(error, error_tam, "No se pudo restaurar «%s»: %s.", ruta_base(destino), solar_error_texto(r));
        return r;
    }
    unlink(info);
    if (restaurado && restaurado_tam) copiar_texto(restaurado, restaurado_tam, destino);
    return 0;
}

int solar_papelera_borrar(const char *ruta)
{
    char papelera[PATH_MAX], info[PATH_MAX], raiz[PATH_MAX];
    int r = ubicar(ruta, papelera, info, raiz);
    if (r) return r;
    r = eliminar_arbol(ruta, NULL, NULL);
    if (r && r != ENOENT) return r;
    unlink(info);
    return 0;
}

typedef struct {
    const SolarOpciones *op;
    uint64_t hechos, total;
    int cancelado;
} Vaciado;

static int al_vaciar(void *datos, const char *ruta)
{
    Vaciado *v = datos;
    v->hechos++;
    if (v->op && v->op->progreso && (v->hechos % 64) == 0 &&
        v->op->progreso(v->op->datos, 0, 0, v->hechos, v->total, ruta_base(ruta))) {
        v->cancelado = 1;
        return 1;
    }
    return 0;
}

static int vaciar_subdir(const char *dir, Vaciado *v, int contar)
{
    DIR *d = opendir(dir);
    if (!d) return errno == ENOENT ? 0 : errno;
    int r = 0;
    struct dirent *de;
    while ((de = readdir(d))) {
        if (!strcmp(de->d_name, ".") || !strcmp(de->d_name, "..")) continue;
        char *ruta = ruta_unir(dir, de->d_name);
        if (!ruta) { r = ENOMEM; break; }
        if (contar) {
            uint64_t b = 0;
            contar_arbol(ruta, &b, &v->total);
        } else {
            r = eliminar_arbol(ruta, al_vaciar, v);
            if (r == ENOENT) r = 0;
        }
        free(ruta);
        if (r) break;
    }
    closedir(d);
    return r;
}

int solar_papelera_vaciar(const SolarOpciones *opciones)
{
    Vaciado v = { opciones, 0, 0, 0 };
    Papelera papeleras[64];
    size_t n = recolectar_papeleras(papeleras, SOLAR_LEN(papeleras));
    char sub[PATH_MAX + 16];
    for (size_t i = 0; i < n; i++) {
        snprintf(sub, sizeof sub, "%s/files", papeleras[i].dir);
        vaciar_subdir(sub, &v, 1);
    }
    int r = 0;
    for (size_t i = 0; i < n && !r; i++) {
        snprintf(sub, sizeof sub, "%s/files", papeleras[i].dir);
        r = vaciar_subdir(sub, &v, 0);
        if (!r) {
            snprintf(sub, sizeof sub, "%s/info", papeleras[i].dir);
            r = vaciar_subdir(sub, &v, 0);
        }
        snprintf(sub, sizeof sub, "%s/directorysizes", papeleras[i].dir);
        unlink(sub);
    }
    liberar_papeleras(papeleras, n);
    if (v.cancelado) return ECANCELED;
    if (opciones && opciones->error && opciones->error_tam && r)
        poner_error(opciones->error, opciones->error_tam, "No se pudo vaciar la papelera: %s.", solar_error_texto(r));
    if (!r && opciones && opciones->progreso) opciones->progreso(opciones->datos, 0, 0, v.total, v.total, "");
    return r;
}

int solar_papelera_vacia(void)
{
    Papelera papeleras[64];
    size_t n = recolectar_papeleras(papeleras, SOLAR_LEN(papeleras));
    int vacia = 1;
    char sub[PATH_MAX + 16];
    for (size_t i = 0; i < n && vacia; i++) {
        snprintf(sub, sizeof sub, "%s/files", papeleras[i].dir);
        DIR *d = opendir(sub);
        if (!d) continue;
        struct dirent *de;
        while ((de = readdir(d))) {
            if (strcmp(de->d_name, ".") && strcmp(de->d_name, "..")) { vacia = 0; break; }
        }
        closedir(d);
    }
    liberar_papeleras(papeleras, n);
    return vacia;
}
