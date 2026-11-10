/*
 * ☀ Solar — operaciones con archivos: copiar, mover, eliminar, renombrar, crear
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <pthread.h>

#include <dirent.h>
#include <fcntl.h>
#include <stdio.h>
#include <time.h>
#include <unistd.h>

typedef struct {
    const SolarOpciones *op;
    uint64_t bytes_hechos, bytes_total;
    uint64_t hechos, total;
    int64_t  ultimo_ms;
    int      cancelado;
} Tarea;

static int64_t ms_ahora(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

static void tarea_iniciar(Tarea *t, const SolarOpciones *op)
{
    memset(t, 0, sizeof *t);
    t->op = op;
    if (op && op->error && op->error_tam) op->error[0] = '\0';
}

/* Llama al callback de progreso (como mucho cada 40 ms, salvo 'forzar'). */
static int informar(Tarea *t, const char *actual, int forzar)
{
    if (t->cancelado) return ECANCELED;
    if (!t->op || !t->op->progreso) return 0;
    int64_t ahora = ms_ahora();
    if (!forzar && ahora - t->ultimo_ms < 40) return 0;
    t->ultimo_ms = ahora;
    if (t->op->progreso(t->op->datos, t->bytes_hechos, t->bytes_total, t->hechos, t->total,
                        actual ? actual : "")) {
        t->cancelado = 1;
        return ECANCELED;
    }
    return 0;
}

static void anotar_error(Tarea *t, int codigo, const char *accion, const char *ruta)
{
    if (!t->op || !t->op->error || !t->op->error_tam || t->op->error[0]) return;
    if (codigo == ECANCELED) return;
    poner_error(t->op->error, t->op->error_tam, "No se pudo %s «%s»: %s.", accion,
                ruta_base(ruta), solar_error_texto(codigo));
}

static void anotar_mensaje(Tarea *t, const char *mensaje, const char *ruta)
{
    if (!t->op || !t->op->error || !t->op->error_tam || t->op->error[0]) return;
    poner_error(t->op->error, t->op->error_tam, mensaje, ruta_base(ruta));
}

static void avisar_resultado(Tarea *t, const char *origen, const char *destino)
{
    if (t->op && t->op->resultado) t->op->resultado(t->op->datos, origen, destino);
}

/* ===================================================================== */
/*  Nombres                                                              */
/* ===================================================================== */

int solar_validar_nombre(const char *nombre)
{
    if (!nombre || !*nombre) return EINVAL;
    if (!strcmp(nombre, ".") || !strcmp(nombre, "..")) return EINVAL;
    if (strchr(nombre, '/')) return EINVAL;
    if (strlen(nombre) > 255) return ENAMETOOLONG;
    return 0;
}

/* "foto.tar.gz" -> base "foto", ext ".tar.gz". Las carpetas no tienen ext. */
static void separar_nombre(const char *nombre, int es_dir, char *base, size_t tam, const char **ext)
{
    static const char *const DOBLES[] = { ".pkg.tar.zst", ".pkg.tar.xz", ".tar.gz", ".tar.xz",
                                          ".tar.bz2", ".tar.zst", ".tar.lz4" };
    size_t largo = strlen(nombre);
    *ext = nombre + largo;
    if (!es_dir) {
        for (size_t i = 0; i < SOLAR_LEN(DOBLES); i++) {
            size_t ld = strlen(DOBLES[i]);
            if (largo > ld && termina_con(nombre, DOBLES[i])) { *ext = nombre + largo - ld; break; }
        }
        if (!**ext) {
            const char *punto = strrchr(nombre, '.');
            if (punto && punto != nombre && punto[1]) *ext = punto;
        }
    }
    size_t lb = (size_t)(*ext - nombre);
    if (lb >= tam) lb = tam - 1;
    memcpy(base, nombre, lb);
    base[lb] = '\0';
}

static int solo_digitos(const char *s, size_t n, int *valor)
{
    if (n == 0 || n > 6) return 0;
    int v = 0;
    for (size_t i = 0; i < n; i++) {
        if (s[i] < '0' || s[i] > '9') return 0;
        v = v * 10 + (s[i] - '0');
    }
    *valor = v;
    return 1;
}

/* Reconoce y quita " (N)", " (copia)" o " (copia N)" al final de 'base'. */
static void quitar_sufijo(char *base, int *numero, int *era_copia)
{
    *numero = 0;
    *era_copia = 0;
    size_t n = strlen(base);
    if (n < 4 || base[n - 1] != ')') return;
    char *abre = strrchr(base, '(');
    if (!abre || abre == base || abre[-1] != ' ') return;
    const char *dentro = abre + 1;
    size_t ld = (size_t)((base + n - 1) - dentro);
    int v;
    if (ld == 5 && !strncmp(dentro, "copia", 5)) {
        *era_copia = 1;
        *numero = 1;
        abre[-1] = '\0';
    } else if (ld > 6 && !strncmp(dentro, "copia ", 6) && solo_digitos(dentro + 6, ld - 6, &v)) {
        *era_copia = 1;
        *numero = v;
        abre[-1] = '\0';
    } else if (solo_digitos(dentro, ld, &v)) {
        *numero = v;
        abre[-1] = '\0';
    }
}

static int nombre_libre(const char *dir, const char *nombre, int es_dir, int es_copia,
                        char *salida, size_t tam)
{
    char base[512];
    const char *ext;
    separar_nombre(nombre, es_dir, base, sizeof base, &ext);
    int numero, era_copia;
    char original[512];
    copiar_texto(original, sizeof original, base);
    quitar_sufijo(base, &numero, &era_copia);

    int inicio;
    if (es_copia) {
        if (!era_copia) { copiar_texto(base, sizeof base, original); inicio = 1; }
        else inicio = numero + 1;
    } else {
        if (numero > 0 && !era_copia) inicio = numero + 1;
        else { copiar_texto(base, sizeof base, original); inicio = 2; }
    }

    for (int i = inicio; i < 100000; i++) {
        char candidato[1024];
        if (es_copia && i == 1) snprintf(candidato, sizeof candidato, "%s (copia)%s", base, ext);
        else if (es_copia) snprintf(candidato, sizeof candidato, "%s (copia %d)%s", base, i, ext);
        else snprintf(candidato, sizeof candidato, "%s (%d)%s", base, i, ext);
        if (strlen(candidato) > 255) return ENAMETOOLONG;
        char *completo = ruta_unir(dir, candidato);
        if (!completo) return ENOMEM;
        int ocupado = existe(completo);
        free(completo);
        if (!ocupado) return copiar_texto(salida, tam, candidato);
    }
    return EEXIST;
}

int solar_nombre_libre(const char *dir, const char *nombre, int es_copia, char *salida, size_t tam)
{
    char *completo = ruta_unir(dir, nombre);
    if (!completo) return ENOMEM;
    int ocupado = existe(completo);
    int es_dir = es_carpeta(completo);
    free(completo);
    if (!ocupado && !es_copia) return copiar_texto(salida, tam, nombre);
    return nombre_libre(dir, nombre, es_dir, es_copia, salida, tam);
}

/* ===================================================================== */
/*  Copiar                                                               */
/* ===================================================================== */

/* Búfer de 1 MiB para copiar cuando copy_file_range no se puede (p. ej. a
 * una memoria USB en FAT). Uno por hilo y reutilizado: antes se pedía y se
 * soltaba en cada archivo, y copiar miles de archivos chiquitos hacía miles
 * de mmap/munmap. Se libera solo cuando termina el hilo. */
enum { TAM_BUF = 1 << 20 };
static pthread_key_t clave_buf;
static pthread_once_t buf_una_vez = PTHREAD_ONCE_INIT;
static void crear_clave_buf(void) { pthread_key_create(&clave_buf, free); }

static char *bufer_del_hilo(void)
{
    pthread_once(&buf_una_vez, crear_clave_buf);
    char *b = pthread_getspecific(clave_buf);
    if (!b && (b = malloc(TAM_BUF)) != NULL) pthread_setspecific(clave_buf, b);
    return b;
}

static int copiar_contenido(Tarea *t, int in, int out, const char *nombre)
{
    int usar_cfr = 1;
    char *buf = NULL;
    int r = 0;
    for (;;) {
        ssize_t n;
        if (usar_cfr) {
            n = copy_file_range(in, NULL, out, NULL, 8u << 20, 0);
            if (n < 0 && (errno == EXDEV || errno == EINVAL || errno == ENOSYS || errno == EOPNOTSUPP ||
                          errno == EBADF || errno == EPERM || errno == ETXTBSY)) {
                usar_cfr = 0;
                continue;
            }
        } else {
            if (!buf && !(buf = bufer_del_hilo())) { r = ENOMEM; break; }
            n = read(in, buf, TAM_BUF);
            if (n > 0) {
                ssize_t escrito = 0;
                while (escrito < n) {
                    ssize_t w = write(out, buf + escrito, (size_t)(n - escrito));
                    if (w < 0) {
                        if (errno == EINTR) continue;
                        r = errno;
                        break;
                    }
                    escrito += w;
                }
                if (r) break;
            }
        }
        if (n < 0) {
            if (errno == EINTR) continue;
            r = errno;
            break;
        }
        if (n == 0) break;
        t->bytes_hechos += (uint64_t)n;
        if (informar(t, nombre, 0)) { r = ECANCELED; break; }
    }
    return r;
}

static int copiar_archivo(Tarea *t, const char *origen, const char *destino, const struct stat *st)
{
    int in = open(origen, O_RDONLY | O_CLOEXEC);
    if (in < 0) {
        int e = errno;
        anotar_error(t, e, "leer", origen);
        return e;
    }
    int out = open(destino, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0600);
    if (out < 0) {
        int e = errno;
        close(in);
        anotar_error(t, e, "crear", destino);
        return e;
    }
    posix_fadvise(in, 0, 0, POSIX_FADV_SEQUENTIAL);
    int r = copiar_contenido(t, in, out, ruta_base(origen));
    close(in);
    if (r == 0) {
        fchmod(out, st->st_mode & 07777);
        struct timespec tiempos[2] = { st->st_atim, st->st_mtim };
        futimens(out, tiempos);
    }
    if (close(out) != 0 && r == 0) r = errno;
    if (r) {
        unlink(destino);
        anotar_error(t, r, "copiar", origen);
    }
    return r;
}

static int copiar_arbol(Tarea *t, const char *origen, const char *destino, int combinar)
{
    if (t->cancelado) return ECANCELED;
    struct stat st;
    if (lstat(origen, &st) != 0) {
        int e = errno;
        anotar_error(t, e, "leer", origen);
        return e;
    }
    struct stat sd;
    int destino_existe = lstat(destino, &sd) == 0;
    int combinar_carpetas = 0;
    if (destino_existe) {
        if (!combinar) {
            anotar_error(t, EEXIST, "copiar", destino);
            return EEXIST;
        }
        if (sd.st_dev == st.st_dev && sd.st_ino == st.st_ino) {
            /* Es el mismo archivo: nada que hacer */
            uint64_t b = 0, e = 0;
            contar_arbol(origen, &b, &e);
            t->hechos += e;
            t->bytes_hechos += b;
            return informar(t, ruta_base(origen), 0);
        }
        if (S_ISDIR(st.st_mode) && S_ISDIR(sd.st_mode)) {
            combinar_carpetas = 1;
        } else {
            int r = eliminar_arbol(destino, NULL, NULL);
            if (r) {
                anotar_error(t, r, "reemplazar", destino);
                return r;
            }
        }
    }

    int r = 0;
    if (S_ISLNK(st.st_mode)) {
        char objetivo[PATH_MAX];
        ssize_t n = readlink(origen, objetivo, sizeof objetivo - 1);
        if (n < 0) {
            r = errno;
            anotar_error(t, r, "leer el enlace", origen);
            return r;
        }
        objetivo[n] = '\0';
        if (symlink(objetivo, destino) != 0) {
            r = errno;
            /* Memorias USB en FAT/exFAT no admiten enlaces: copiamos lo que apunta */
            struct stat apuntado;
            if ((r == EPERM || r == ENOTSUP || r == EOPNOTSUPP || r == EINVAL) &&
                stat(origen, &apuntado) == 0 && S_ISREG(apuntado.st_mode)) {
                r = copiar_archivo(t, origen, destino, &apuntado);
            } else if (r == EPERM || r == ENOTSUP || r == EOPNOTSUPP) {
                r = 0; /* enlace a carpeta en FAT: lo saltamos */
            } else {
                anotar_error(t, r, "crear el enlace", destino);
            }
        }
        t->hechos++;
    } else if (S_ISDIR(st.st_mode)) {
        if (!combinar_carpetas && mkdir(destino, 0700) != 0) {
            r = errno;
            anotar_error(t, r, "crear la carpeta", destino);
            return r;
        }
        t->hechos++;
        if ((r = informar(t, ruta_base(origen), 0))) return r;
        DIR *d = opendir(origen);
        if (!d) {
            r = errno;
            anotar_error(t, r, "abrir", origen);
            return r;
        }
        struct dirent *de;
        while ((de = readdir(d))) {
            if (!strcmp(de->d_name, ".") || !strcmp(de->d_name, "..")) continue;
            char *o = ruta_unir(origen, de->d_name);
            char *dd = ruta_unir(destino, de->d_name);
            if (!o || !dd) {
                free(o);
                free(dd);
                r = ENOMEM;
                break;
            }
            r = copiar_arbol(t, o, dd, combinar);
            free(o);
            free(dd);
            if (r) break;
        }
        closedir(d);
        if (r == 0) {
            chmod(destino, st.st_mode & 07777);
            struct timespec tiempos[2] = { st.st_atim, st.st_mtim };
            utimensat(AT_FDCWD, destino, tiempos, 0);
        }
    } else if (S_ISREG(st.st_mode)) {
        r = copiar_archivo(t, origen, destino, &st);
        t->hechos++;
    } else {
        t->hechos++; /* fifo, socket o dispositivo: no se copian */
    }
    if (r == 0) r = informar(t, ruta_base(origen), 0);
    return r;
}

/* Copia normalizada de una ruta (para comparar). */
static char *normalizada(const char *ruta)
{
    char *r = solar_strdup(ruta);
    if (r) ruta_normalizar(r);
    return r;
}

int solar_copiar(const char *const *origenes, size_t cantidad, const char *destino,
                 const SolarOpciones *opciones)
{
    Tarea t;
    tarea_iniciar(&t, opciones);
    int conflicto = opciones ? opciones->conflicto : SOLAR_CONFLICTO_RENOMBRAR;
    if (!es_carpeta(destino)) {
        anotar_error(&t, ENOTDIR, "copiar en", destino);
        return ENOTDIR;
    }
    char *dir = normalizada(destino);
    uint64_t *bytes = calloc(cantidad ? cantidad : 1, sizeof *bytes);
    uint64_t *elementos = calloc(cantidad ? cantidad : 1, sizeof *elementos);
    if (!dir || !bytes || !elementos) {
        free(dir);
        free(bytes);
        free(elementos);
        return ENOMEM;
    }

    int r = 0;
    for (size_t i = 0; i < cantidad && !r; i++) {
        struct stat st;
        if (lstat(origenes[i], &st) != 0) {
            r = errno;
            anotar_error(&t, r, "copiar", origenes[i]);
            break;
        }
        if (S_ISDIR(st.st_mode) && ruta_es_dentro_de(dir, origenes[i])) {
            anotar_mensaje(&t, "No puedes copiar «%s» dentro de sí misma.", origenes[i]);
            r = EINVAL;
            break;
        }
        contar_arbol(origenes[i], &bytes[i], &elementos[i]);
        t.bytes_total += bytes[i];
        t.total += elementos[i];
    }
    if (!r) informar(&t, "", 1);

    for (size_t i = 0; i < cantidad && !r; i++) {
        const char *origen = origenes[i];
        char *padre = ruta_padre(origen);
        if (padre) ruta_normalizar(padre);
        int mismo_dir = padre && strcmp(padre, dir) == 0;
        free(padre);

        struct stat st;
        lstat(origen, &st);
        const char *nombre = ruta_base(origen);
        char *final = ruta_unir(dir, nombre);
        if (!final) { r = ENOMEM; break; }
        int combinar = 0, omitir = 0;
        if (mismo_dir || existe(final)) {
            if (mismo_dir || conflicto == SOLAR_CONFLICTO_RENOMBRAR) {
                char libre[512];
                r = nombre_libre(dir, nombre, S_ISDIR(st.st_mode), mismo_dir, libre, sizeof libre);
                if (r) { anotar_error(&t, r, "copiar", origen); free(final); break; }
                free(final);
                final = ruta_unir(dir, libre);
                if (!final) { r = ENOMEM; break; }
            } else if (conflicto == SOLAR_CONFLICTO_OMITIR) {
                omitir = 1;
            } else {
                combinar = 1;
            }
        }
        if (omitir) {
            t.hechos += elementos[i];
            t.bytes_hechos += bytes[i];
            informar(&t, nombre, 0);
        } else {
            r = copiar_arbol(&t, origen, final, combinar);
            if (!r) avisar_resultado(&t, origen, final);
        }
        free(final);
    }
    if (!r) informar(&t, "", 1);
    free(dir);
    free(bytes);
    free(elementos);
    return r;
}

/* ===================================================================== */
/*  Mover                                                                */
/* ===================================================================== */

/* Mueve 'origen' a 'destino' (que no existe). Si están en distintos
 * dispositivos, copia y luego borra. */
static int mover_uno(Tarea *t, const char *origen, const char *destino, int combinar)
{
    if (!combinar && rename(origen, destino) == 0) {
        t->hechos++;
        return informar(t, ruta_base(origen), 0);
    }
    if (!combinar && errno != EXDEV) {
        int e = errno;
        anotar_error(t, e, "mover", origen);
        return e;
    }
    uint64_t b = 0, e = 0;
    contar_arbol(origen, &b, &e);
    t->bytes_total += b;
    if (e > 0) t->total += e - 1;
    int r = copiar_arbol(t, origen, destino, combinar);
    if (r) return r;
    r = eliminar_arbol(origen, NULL, NULL);
    if (r) {
        anotar_mensaje(t, "Se copió «%s», pero no se pudo borrar el original.", origen);
        return r;
    }
    return 0;
}

/* Combina la carpeta 'origen' dentro de 'destino' (que ya existe). */
static int mover_combinando(Tarea *t, const char *origen, const char *destino)
{
    DIR *d = opendir(origen);
    if (!d) {
        int e = errno;
        anotar_error(t, e, "abrir", origen);
        return e;
    }
    int r = 0;
    struct dirent *de;
    while ((de = readdir(d))) {
        if (!strcmp(de->d_name, ".") || !strcmp(de->d_name, "..")) continue;
        char *o = ruta_unir(origen, de->d_name);
        char *dd = ruta_unir(destino, de->d_name);
        if (!o || !dd) { free(o); free(dd); r = ENOMEM; break; }
        struct stat so, sdd;
        if (lstat(o, &so) != 0) { free(o); free(dd); continue; }
        if (lstat(dd, &sdd) == 0) {
            if (S_ISDIR(so.st_mode) && S_ISDIR(sdd.st_mode)) {
                r = mover_combinando(t, o, dd);
            } else {
                r = eliminar_arbol(dd, NULL, NULL);
                if (r) anotar_error(t, r, "reemplazar", dd);
                else r = mover_uno(t, o, dd, 0);
            }
        } else {
            r = mover_uno(t, o, dd, 0);
        }
        free(o);
        free(dd);
        if (r) break;
    }
    closedir(d);
    if (!r && rmdir(origen) != 0 && errno != ENOENT) {
        r = errno;
        anotar_error(t, r, "quitar la carpeta", origen);
    }
    return r;
}

int solar_mover(const char *const *origenes, size_t cantidad, const char *destino,
                const SolarOpciones *opciones)
{
    Tarea t;
    tarea_iniciar(&t, opciones);
    int conflicto = opciones ? opciones->conflicto : SOLAR_CONFLICTO_RENOMBRAR;
    if (!es_carpeta(destino)) {
        anotar_error(&t, ENOTDIR, "mover a", destino);
        return ENOTDIR;
    }
    char *dir = normalizada(destino);
    if (!dir) return ENOMEM;
    t.total = cantidad;
    informar(&t, "", 1);

    int r = 0;
    for (size_t i = 0; i < cantidad && !r; i++) {
        const char *origen = origenes[i];
        struct stat st;
        if (lstat(origen, &st) != 0) {
            r = errno;
            anotar_error(&t, r, "mover", origen);
            break;
        }
        char *padre = ruta_padre(origen);
        if (padre) ruta_normalizar(padre);
        int mismo_dir = padre && strcmp(padre, dir) == 0;
        free(padre);
        if (mismo_dir) { /* ya está ahí */
            t.hechos++;
            avisar_resultado(&t, origen, origen);
            continue;
        }
        if (S_ISDIR(st.st_mode) && ruta_es_dentro_de(dir, origen)) {
            anotar_mensaje(&t, "No puedes mover «%s» dentro de sí misma.", origen);
            r = EINVAL;
            break;
        }
        const char *nombre = ruta_base(origen);
        char *final = ruta_unir(dir, nombre);
        if (!final) { r = ENOMEM; break; }
        struct stat sd;
        if (lstat(final, &sd) == 0) {
            if (conflicto == SOLAR_CONFLICTO_OMITIR) {
                t.hechos++;
                free(final);
                continue;
            }
            if (conflicto == SOLAR_CONFLICTO_RENOMBRAR) {
                char libre[512];
                r = nombre_libre(dir, nombre, S_ISDIR(st.st_mode), 0, libre, sizeof libre);
                free(final);
                if (r) { anotar_error(&t, r, "mover", origen); break; }
                final = ruta_unir(dir, libre);
                if (!final) { r = ENOMEM; break; }
                r = mover_uno(&t, origen, final, 0);
            } else if (S_ISDIR(st.st_mode) && S_ISDIR(sd.st_mode)) {
                r = mover_combinando(&t, origen, final);
                if (!r) t.hechos++;
            } else {
                r = eliminar_arbol(final, NULL, NULL);
                if (r) anotar_error(&t, r, "reemplazar", final);
                else r = mover_uno(&t, origen, final, 0);
            }
        } else {
            r = mover_uno(&t, origen, final, 0);
        }
        if (!r) avisar_resultado(&t, origen, final);
        free(final);
    }
    if (!r) informar(&t, "", 1);
    free(dir);
    return r;
}

/* ===================================================================== */
/*  Eliminar para siempre                                                */
/* ===================================================================== */

static int al_borrar(void *datos, const char *ruta)
{
    Tarea *t = datos;
    t->hechos++;
    return informar(t, ruta_base(ruta), 0) != 0;
}

int solar_eliminar(const char *const *rutas, size_t cantidad, const SolarOpciones *opciones)
{
    Tarea t;
    tarea_iniciar(&t, opciones);
    for (size_t i = 0; i < cantidad; i++) {
        uint64_t b = 0;
        contar_arbol(rutas[i], &b, &t.total);
    }
    informar(&t, "", 1);
    int r = 0;
    for (size_t i = 0; i < cantidad && !r; i++) {
        r = eliminar_arbol(rutas[i], al_borrar, &t);
        if (r == ENOENT) r = 0;
        if (r) anotar_error(&t, r, "eliminar", rutas[i]);
        else avisar_resultado(&t, rutas[i], "");
    }
    if (!r) informar(&t, "", 1);
    return r;
}

/* ===================================================================== */
/*  Renombrar y crear                                                    */
/* ===================================================================== */

int solar_renombrar(const char *ruta, const char *nombre_nuevo)
{
    int r = solar_validar_nombre(nombre_nuevo);
    if (r) return r;
    char *padre = ruta_padre(ruta);
    if (!padre) return ENOMEM;
    char *destino = ruta_unir(padre, nombre_nuevo);
    free(padre);
    if (!destino) return ENOMEM;
    if (strcmp(destino, ruta) == 0) { free(destino); return 0; }

    struct stat a, b;
    if (lstat(ruta, &a) != 0) { r = errno; free(destino); return r; }
    int mismo = 0;
    if (lstat(destino, &b) == 0) {
        /* En FAT, "Foto.JPG" -> "foto.jpg" es el mismo archivo: se permite */
        if (a.st_dev == b.st_dev && a.st_ino == b.st_ino) mismo = 1;
        else { free(destino); return EEXIST; }
    }
#if defined(__GLIBC__) && defined(RENAME_NOREPLACE)
    if (!mismo) {
        if (renameat2(AT_FDCWD, ruta, AT_FDCWD, destino, RENAME_NOREPLACE) == 0) { free(destino); return 0; }
        if (errno != EINVAL && errno != ENOSYS) { r = errno; free(destino); return r; }
    }
#endif
    (void)mismo;
    r = rename(ruta, destino) == 0 ? 0 : errno;
    free(destino);
    return r;
}

int solar_crear_carpeta(const char *dir, const char *nombre)
{
    int r = solar_validar_nombre(nombre);
    if (r) return r;
    char *ruta = ruta_unir(dir, nombre);
    if (!ruta) return ENOMEM;
    r = mkdir(ruta, 0777) == 0 ? 0 : errno;
    free(ruta);
    return r;
}

int solar_crear_archivo(const char *dir, const char *nombre)
{
    int r = solar_validar_nombre(nombre);
    if (r) return r;
    char *ruta = ruta_unir(dir, nombre);
    if (!ruta) return ENOMEM;
    int fd = open(ruta, O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0666);
    r = fd >= 0 ? 0 : errno;
    if (fd >= 0) close(fd);
    free(ruta);
    return r;
}
