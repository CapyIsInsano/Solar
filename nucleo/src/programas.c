/*
 * ☀ Solar — tipos de archivo (MIME) y programas instalados
 *
 * Para "Abrir con…": averigua el tipo MIME de cada archivo con la base de
 * shared-mime-info (mime/globs2, subclasses y aliases), lee los lanzadores
 * .desktop de las carpetas XDG (incluidos Flatpak y Snap) y ordena los
 * programas: primero el predeterminado (mimeapps.list), después los que
 * dicen abrir ese tipo exacto, luego los que abren un tipo "padre" (por
 * ejemplo, cualquier editor de texto para un .cpp) y al final, si se pide,
 * todos los demás.
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <ctype.h>
#include <dirent.h>
#include <fcntl.h>
#include <fnmatch.h>
#include <pthread.h>
#include <unistd.h>

/* ===================================================================== */
/*  Carpetas XDG de datos                                                */
/* ===================================================================== */

typedef struct {
    char  **dirs;
    size_t  n;
} ListaDirs;

static void lista_agregar(ListaDirs *l, const char *dir)
{
    if (!dir || !*dir) return;
    for (size_t i = 0; i < l->n; i++)
        if (!strcmp(l->dirs[i], dir)) return;
    char **p = realloc(l->dirs, (l->n + 1) * sizeof *p);
    if (!p) return;
    l->dirs = p;
    l->dirs[l->n] = solar_strdup(dir);
    if (l->dirs[l->n]) l->n++;
}

static void lista_liberar(ListaDirs *l)
{
    for (size_t i = 0; i < l->n; i++) free(l->dirs[i]);
    free(l->dirs);
    l->dirs = NULL;
    l->n = 0;
}

/* $XDG_DATA_HOME, luego $XDG_DATA_DIRS (en orden de prioridad) */
static void dirs_datos(ListaDirs *l)
{
    char buf[PATH_MAX];
    if (solar_dir_datos(buf, sizeof buf) == 0) {
        /* solar_dir_datos devuelve …/solar; queremos el padre */
        char *padre = ruta_padre(buf);
        lista_agregar(l, padre);
        free(padre);
    }
    const char *env = getenv("XDG_DATA_DIRS");
    char *copia = solar_strdup(env && *env ? env : "/usr/local/share:/usr/share");
    if (copia) {
        char *guardar = NULL;
        for (char *d = strtok_r(copia, ":", &guardar); d; d = strtok_r(NULL, ":", &guardar))
            lista_agregar(l, d);
        free(copia);
    }
    /* Por si la sesión no los puso en XDG_DATA_DIRS */
    char hogar[PATH_MAX];
    if (obtener_hogar(hogar, sizeof hogar) == 0 &&
        formatear(buf, sizeof buf, "%s/.local/share/flatpak/exports/share", hogar) == 0)
        lista_agregar(l, buf);
    lista_agregar(l, "/var/lib/flatpak/exports/share");
}

/* ===================================================================== */
/*  Base de tipos MIME (shared-mime-info)                                 */
/* ===================================================================== */

typedef struct {
    int   peso;
    int   mayusculas;     /* ":cs" — distingue mayúsculas */
    int   literal;        /* sin comodines */
    char *mime;
    char *patron;
} Glob;

typedef struct { char *hijo; char *padre; } Par;

static struct {
    pthread_once_t una_vez;
    Glob  *globs;  size_t n_globs;
    Par   *padres; size_t n_padres;
    Par   *alias;  size_t n_alias;
} base = { .una_vez = PTHREAD_ONCE_INIT };

static void agregar_par(Par **v, size_t *n, const char *a, const char *b)
{
    Par *p = realloc(*v, (*n + 1) * sizeof *p);
    if (!p) return;
    *v = p;
    p[*n].hijo = solar_strdup(a);
    p[*n].padre = solar_strdup(b);
    if (p[*n].hijo && p[*n].padre) (*n)++;
    else { free(p[*n].hijo); free(p[*n].padre); }
}

static void leer_pares(const char *archivo, Par **v, size_t *n)
{
    char *texto = leer_archivo(archivo, 4u << 20, NULL);
    if (!texto) return;
    char *guardar = NULL;
    for (char *linea = strtok_r(texto, "\n", &guardar); linea; linea = strtok_r(NULL, "\n", &guardar)) {
        if (*linea == '#') continue;
        char *esp = strchr(linea, ' ');
        if (!esp) continue;
        *esp = '\0';
        agregar_par(v, n, linea, esp + 1);
    }
    free(texto);
}

static void leer_globs(const char *archivo)
{
    char *texto = leer_archivo(archivo, 8u << 20, NULL);
    if (!texto) return;
    char *guardar = NULL;
    for (char *linea = strtok_r(texto, "\n", &guardar); linea; linea = strtok_r(NULL, "\n", &guardar)) {
        if (*linea == '#') continue;
        /* peso:mime:patrón[:banderas] */
        char *c1 = strchr(linea, ':');
        if (!c1) continue;
        char *c2 = strchr(c1 + 1, ':');
        if (!c2) continue;
        *c1 = *c2 = '\0';
        char *patron = c2 + 1;
        char *banderas = strchr(patron, ':');
        int cs = 0;
        if (banderas) { *banderas = '\0'; cs = strstr(banderas + 1, "cs") != NULL; }
        if (!*patron || !strcmp(c1 + 1, "__NOGLOBS__")) continue;
        Glob *g = realloc(base.globs, (base.n_globs + 1) * sizeof *g);
        if (!g) break;
        base.globs = g;
        g = &base.globs[base.n_globs];
        g->peso = atoi(linea);
        g->mayusculas = cs;
        g->literal = strpbrk(patron, "*?[") == NULL;
        g->mime = solar_strdup(c1 + 1);
        g->patron = solar_strdup(patron);
        if (g->mime && g->patron) base.n_globs++;
        else { free(g->mime); free(g->patron); }
    }
    free(texto);
}

static void cargar_base(void)
{
    ListaDirs dirs = { 0 };
    dirs_datos(&dirs);
    char ruta[PATH_MAX];
    for (size_t i = 0; i < dirs.n; i++) {
        if (formatear(ruta, sizeof ruta, "%s/mime/globs2", dirs.dirs[i]) == 0) leer_globs(ruta);
        if (formatear(ruta, sizeof ruta, "%s/mime/subclasses", dirs.dirs[i]) == 0) leer_pares(ruta, &base.padres, &base.n_padres);
        if (formatear(ruta, sizeof ruta, "%s/mime/aliases", dirs.dirs[i]) == 0) leer_pares(ruta, &base.alias, &base.n_alias);
    }
    lista_liberar(&dirs);
}

static const char *canonico(const char *mime)
{
    for (size_t i = 0; i < base.n_alias; i++)
        if (!strcmp(base.alias[i].hijo, mime)) return base.alias[i].padre;
    return mime;
}

/* Tipos de respaldo si la distro no trae shared-mime-info */
static const struct { const char *ext; const char *mime; } RESPALDO[] = {
    { "c", "text/x-csrc" }, { "h", "text/x-chdr" }, { "cpp", "text/x-c++src" },
    { "cc", "text/x-c++src" }, { "hpp", "text/x-c++hdr" }, { "rs", "text/rust" },
    { "py", "text/x-python3" }, { "lua", "text/x-lua" }, { "sh", "application/x-shellscript" },
    { "txt", "text/plain" }, { "md", "text/markdown" }, { "json", "application/json" },
    { "html", "text/html" }, { "css", "text/css" }, { "js", "text/javascript" },
    { "png", "image/png" }, { "jpg", "image/jpeg" }, { "jpeg", "image/jpeg" },
    { "gif", "image/gif" }, { "webp", "image/webp" }, { "svg", "image/svg+xml" },
    { "mp3", "audio/mpeg" }, { "flac", "audio/flac" }, { "ogg", "audio/ogg" },
    { "wav", "audio/x-wav" }, { "mp4", "video/mp4" }, { "mkv", "video/x-matroska" },
    { "webm", "video/webm" }, { "pdf", "application/pdf" }, { "zip", "application/zip" },
};

static int parece_texto(const char *ruta)
{
    int fd = open(ruta, O_RDONLY | O_CLOEXEC | O_NONBLOCK);
    if (fd < 0) return -1;
    unsigned char buf[1024];
    ssize_t n = read(fd, buf, sizeof buf);
    close(fd);
    if (n < 0) return -1;
    if (n >= 4 && buf[0] == 0x7f && buf[1] == 'E' && buf[2] == 'L' && buf[3] == 'F') return 2;
    for (ssize_t i = 0; i < n; i++)
        if (buf[i] == 0) return 0;
    return 1;
}

int solar_mime_de(const char *ruta, char *mime, size_t tam)
{
    if (!ruta || !mime || tam == 0) return EINVAL;
    pthread_once(&base.una_vez, cargar_base);
    struct stat st;
    if (stat(ruta, &st) != 0) return errno;
    if (S_ISDIR(st.st_mode)) return copiar_texto(mime, tam, "inode/directory");
    if (!S_ISREG(st.st_mode)) return copiar_texto(mime, tam, "application/octet-stream");

    const char *nombre = ruta_base(ruta);
    const Glob *mejor = NULL;
    for (size_t i = 0; i < base.n_globs; i++) {
        const Glob *g = &base.globs[i];
        int coincide = g->literal
            ? (g->mayusculas ? strcmp(g->patron, nombre) : strcasecmp(g->patron, nombre)) == 0
            : fnmatch(g->patron, nombre, g->mayusculas ? 0 : FNM_CASEFOLD) == 0;
        if (!coincide) continue;
        /* Gana el literal, luego el de más peso, luego el patrón más largo */
        if (!mejor ||
            (g->literal && !mejor->literal) ||
            (g->literal == mejor->literal &&
             (g->peso > mejor->peso || (g->peso == mejor->peso && strlen(g->patron) > strlen(mejor->patron)))))
            mejor = g;
    }
    if (mejor) return copiar_texto(mime, tam, canonico(mejor->mime));

    const char *punto = strrchr(nombre, '.');
    if (punto && punto != nombre) {
        for (size_t i = 0; i < SOLAR_LEN(RESPALDO); i++)
            if (!strcasecmp(punto + 1, RESPALDO[i].ext)) return copiar_texto(mime, tam, RESPALDO[i].mime);
    }
    if (st.st_size == 0) return copiar_texto(mime, tam, "text/plain");
    switch (parece_texto(ruta)) {
    case 1:  return copiar_texto(mime, tam, "text/plain");
    case 2:  return copiar_texto(mime, tam, "application/x-executable");
    default: return copiar_texto(mime, tam, "application/octet-stream");
    }
}

/* El tipo y todos sus "padres" (text/x-c++src → text/x-csrc → text/plain).
 * 'tipos' se llena hasta 'max'. Devuelve cuántos. */
static size_t familia(const char *mime, char tipos[][128], size_t max)
{
    size_t n = 0;
    if (max == 0) return 0;
    copiar_texto(tipos[n++], 128, canonico(mime));
    for (size_t i = 0; i < n && n < max; i++) {
        for (size_t k = 0; k < base.n_padres && n < max; k++) {
            if (strcmp(base.padres[k].hijo, tipos[i])) continue;
            const char *p = canonico(base.padres[k].padre);
            int ya = 0;
            for (size_t j = 0; j < n; j++) if (!strcmp(tipos[j], p)) ya = 1;
            if (!ya) copiar_texto(tipos[n++], 128, p);
        }
    }
    /* Todo tipo de texto es también text/plain (aunque la base no lo diga) */
    if (!strncmp(tipos[0], "text/", 5) && n < max) {
        int ya = 0;
        for (size_t j = 0; j < n; j++) if (!strcmp(tipos[j], "text/plain")) ya = 1;
        if (!ya) copiar_texto(tipos[n++], 128, "text/plain");
    }
    return n;
}

/* ===================================================================== */
/*  Lanzadores .desktop                                                  */
/* ===================================================================== */

typedef struct {
    char *id;           /* org.kde.kate.desktop */
    char *archivo;      /* ruta completa del .desktop */
    char *nombre;
    char *generico;
    char *comentario;
    char *exec;
    char *icono;
    char *categorias;
    char *mimes;        /* "text/plain;text/x-csrc;" */
    char *dir_trabajo;
    int   terminal;
    int   oculto;       /* Hidden=true: borrado */
    int   no_mostrar;   /* NoDisplay=true */
} Lanzador;

typedef struct {
    Lanzador *v;
    size_t    n;
} Lanzadores;

static void lanzador_liberar(Lanzador *l)
{
    free(l->id); free(l->archivo); free(l->nombre); free(l->generico);
    free(l->comentario); free(l->exec); free(l->icono); free(l->categorias);
    free(l->mimes); free(l->dir_trabajo);
}

/* Prefijos de idioma a buscar: es_MX, es (en ese orden) */
static void idiomas(char pref[2][32])
{
    pref[0][0] = pref[1][0] = '\0';
    const char *vars[] = { "LC_ALL", "LC_MESSAGES", "LANG" };
    const char *v = NULL;
    for (size_t i = 0; i < 3; i++) {
        v = getenv(vars[i]);
        if (v && *v) break;
    }
    if (!v || !*v || !strcmp(v, "C") || !strcmp(v, "POSIX")) return;
    size_t k = strcspn(v, ".@");
    if (k >= 32) return;
    memcpy(pref[0], v, k);
    pref[0][k] = '\0';
    size_t g = strcspn(pref[0], "_");
    memcpy(pref[1], pref[0], g);
    pref[1][g] = '\0';
    if (!strcmp(pref[0], pref[1])) pref[0][0] = '\0';
}

/* Quita los escapes del formato desktop (\s \n \t \r \\) */
static char *valor_desktop(const char *v)
{
    char *s = malloc(strlen(v) + 1);
    if (!s) return NULL;
    char *o = s;
    for (const char *p = v; *p; p++) {
        if (*p == '\\' && p[1]) {
            p++;
            switch (*p) {
            case 's': *o++ = ' '; break;
            case 'n': *o++ = '\n'; break;
            case 't': *o++ = '\t'; break;
            case 'r': *o++ = '\r'; break;
            default:  *o++ = '\\'; *o++ = *p; break;   /* \\ y \" los resuelve Exec */
            }
        } else {
            *o++ = *p;
        }
    }
    *o = '\0';
    return s;
}

static int es_verdad(const char *v) { return !strcmp(v, "true") || !strcmp(v, "1"); }

/* Lee un .desktop. Devuelve 0 si es una aplicación usable. */
static int leer_lanzador(const char *archivo, const char *id, char pref[2][32], Lanzador *l)
{
    memset(l, 0, sizeof *l);
    char *texto = leer_archivo(archivo, 512u * 1024, NULL);
    if (!texto) return -1;

    int en_grupo = 0, tipo_app = 0, rango_nombre = 99, rango_gen = 99, rango_com = 99;
    char *probar = NULL;
    char *guardar = NULL;
    for (char *linea = strtok_r(texto, "\n", &guardar); linea; linea = strtok_r(NULL, "\n", &guardar)) {
        while (*linea == ' ' || *linea == '\t') linea++;
        if (*linea == '#' || !*linea) continue;
        if (*linea == '[') {
            if (en_grupo) break;            /* solo nos importa [Desktop Entry] */
            en_grupo = !strncmp(linea, "[Desktop Entry]", 15);
            continue;
        }
        if (!en_grupo) continue;
        char *igual = strchr(linea, '=');
        if (!igual) continue;
        char *fin = igual;
        while (fin > linea && (fin[-1] == ' ' || fin[-1] == '\t')) fin--;
        *fin = '\0';
        char *valor = igual + 1;
        while (*valor == ' ' || *valor == '\t') valor++;
        size_t lv = strlen(valor);
        while (lv && (valor[lv - 1] == '\r' || valor[lv - 1] == ' ')) valor[--lv] = '\0';

        /* Claves localizadas: Name[es_MX] */
        char clave[64];
        int rango = 3;                       /* 0 = es_MX, 1 = es, 3 = sin idioma */
        char *corchete = strchr(linea, '[');
        if (corchete) {
            char *cierre = strchr(corchete, ']');
            if (!cierre) continue;
            *cierre = '\0';
            const char *loc = corchete + 1;
            if (pref[0][0] && !strcmp(loc, pref[0])) rango = 0;
            else if (pref[1][0] && !strcmp(loc, pref[1])) rango = 1;
            else continue;
            *corchete = '\0';
        }
        if (copiar_texto(clave, sizeof clave, linea)) continue;

        char **destino = NULL;
        int *rango_dest = NULL;
        if (!strcmp(clave, "Name")) { destino = &l->nombre; rango_dest = &rango_nombre; }
        else if (!strcmp(clave, "GenericName")) { destino = &l->generico; rango_dest = &rango_gen; }
        else if (!strcmp(clave, "Comment")) { destino = &l->comentario; rango_dest = &rango_com; }
        if (destino) {
            if (rango < *rango_dest) {
                free(*destino);
                *destino = valor_desktop(valor);
                *rango_dest = rango;
            }
            continue;
        }
        if (rango != 3) continue;
        if (!strcmp(clave, "Type")) tipo_app = !strcmp(valor, "Application");
        else if (!strcmp(clave, "Exec")) { free(l->exec); l->exec = valor_desktop(valor); }
        else if (!strcmp(clave, "TryExec")) { free(probar); probar = valor_desktop(valor); }
        else if (!strcmp(clave, "Icon")) { free(l->icono); l->icono = valor_desktop(valor); }
        else if (!strcmp(clave, "Categories")) { free(l->categorias); l->categorias = solar_strdup(valor); }
        else if (!strcmp(clave, "MimeType")) { free(l->mimes); l->mimes = solar_strdup(valor); }
        else if (!strcmp(clave, "Path")) { free(l->dir_trabajo); l->dir_trabajo = valor_desktop(valor); }
        else if (!strcmp(clave, "Terminal")) l->terminal = es_verdad(valor);
        else if (!strcmp(clave, "Hidden")) l->oculto = es_verdad(valor);
        else if (!strcmp(clave, "NoDisplay")) l->no_mostrar = es_verdad(valor);
    }
    free(texto);

    l->id = solar_strdup(id);
    l->archivo = solar_strdup(archivo);
    int usable = l->id && l->archivo && !l->oculto && tipo_app && l->exec && l->nombre;
    if (usable && probar && *probar) {
        char exe[PATH_MAX];
        if (probar[0] == '/') usable = access(probar, X_OK) == 0;
        else usable = buscar_en_path(probar, exe, sizeof exe) == 0;
    }
    free(probar);
    if (l->oculto) return 1;                 /* oculta las de menor prioridad */
    return usable ? 0 : -1;
}

static int lanzador_ya(const Lanzadores *ls, const char *id)
{
    for (size_t i = 0; i < ls->n; i++)
        if (!strcmp(ls->v[i].id, id)) return 1;
    return 0;
}

static void escanear(const char *dir, const char *prefijo, int profundidad, char pref[2][32],
                     Lanzadores *ls, Lanzadores *ocultos)
{
    DIR *d = opendir(dir);
    if (!d) return;
    struct dirent *e;
    while ((e = readdir(d))) {
        if (e->d_name[0] == '.') continue;
        char *ruta = ruta_unir(dir, e->d_name);
        if (!ruta) continue;
        if (es_carpeta(ruta)) {
            if (profundidad < 3) {
                char sub[256];
                if (formatear(sub, sizeof sub, "%s%s-", prefijo, e->d_name) == 0)
                    escanear(ruta, sub, profundidad + 1, pref, ls, ocultos);
            }
        } else if (termina_con(e->d_name, ".desktop")) {
            char id[512];
            if (formatear(id, sizeof id, "%s%s", prefijo, e->d_name) == 0 &&
                !lanzador_ya(ls, id) && !lanzador_ya(ocultos, id)) {
                Lanzador l;
                int r = leer_lanzador(ruta, id, pref, &l);
                Lanzadores *dest = r == 0 ? ls : r == 1 ? ocultos : NULL;
                if (dest) {
                    Lanzador *nv = realloc(dest->v, (dest->n + 1) * sizeof *nv);
                    if (nv) { dest->v = nv; dest->v[dest->n++] = l; }
                    else lanzador_liberar(&l);
                } else {
                    lanzador_liberar(&l);
                }
            }
        }
        free(ruta);
    }
    closedir(d);
}

static void cargar_lanzadores(Lanzadores *ls)
{
    ListaDirs dirs = { 0 };
    dirs_datos(&dirs);
    lista_agregar(&dirs, "/var/lib/snapd/desktop");
    char pref[2][32];
    idiomas(pref);
    Lanzadores ocultos = { 0 };
    char ruta[PATH_MAX];
    for (size_t i = 0; i < dirs.n; i++)
        if (formatear(ruta, sizeof ruta, "%s/applications", dirs.dirs[i]) == 0)
            escanear(ruta, "", 0, pref, ls, &ocultos);
    for (size_t i = 0; i < ocultos.n; i++) lanzador_liberar(&ocultos.v[i]);
    free(ocultos.v);
    lista_liberar(&dirs);
}

static int declara(const Lanzador *l, const char *mime)
{
    if (!l->mimes) return 0;
    size_t n = strlen(mime);
    for (const char *p = l->mimes; *p;) {
        const char *fin = strchr(p, ';');
        size_t k = fin ? (size_t)(fin - p) : strlen(p);
        if (k == n && !strncmp(p, mime, n)) return 1;
        if (k > 0 && k == n && !strncasecmp(p, mime, n)) return 1;
        if (!fin) break;
        p = fin + 1;
    }
    return 0;
}

/* ===================================================================== */
/*  mimeapps.list                                                        */
/* ===================================================================== */

typedef struct {
    char *pref_defecto;    /* lista "a.desktop;b.desktop;" del predeterminado */
    char *agregados;       /* [Added Associations] */
    char *quitados;        /* [Removed Associations] */
} Asociaciones;

static void anexar(char **dest, const char *valor)
{
    if (!valor || !*valor) return;
    size_t a = *dest ? strlen(*dest) : 0, b = strlen(valor);
    char *n = realloc(*dest, a + b + 2);
    if (!n) return;
    memcpy(n + a, valor, b);
    n[a + b] = ';';
    n[a + b + 1] = '\0';
    *dest = n;
}

static void leer_mimeapps(const char *archivo, const char *mime, Asociaciones *as)
{
    char *texto = leer_archivo(archivo, 2u << 20, NULL);
    if (!texto) return;
    enum { NADA, DEFECTO, AGREGADOS, QUITADOS } seccion = NADA;
    size_t n = strlen(mime);
    char *guardar = NULL;
    for (char *linea = strtok_r(texto, "\n", &guardar); linea; linea = strtok_r(NULL, "\n", &guardar)) {
        if (*linea == '[') {
            seccion = !strncmp(linea, "[Default Applications]", 22) ? DEFECTO
                    : !strncmp(linea, "[Added Associations]", 20) ? AGREGADOS
                    : !strncmp(linea, "[Removed Associations]", 22) ? QUITADOS : NADA;
            continue;
        }
        if (seccion == NADA || strncmp(linea, mime, n) || linea[n] != '=') continue;
        const char *valor = linea + n + 1;
        if (seccion == DEFECTO) anexar(&as->pref_defecto, valor);
        else if (seccion == AGREGADOS) anexar(&as->agregados, valor);
        else anexar(&as->quitados, valor);
    }
    free(texto);
}

static int lista_contiene(const char *lista, const char *id)
{
    if (!lista) return 0;
    size_t n = strlen(id);
    for (const char *p = lista; *p;) {
        const char *fin = strchr(p, ';');
        size_t k = fin ? (size_t)(fin - p) : strlen(p);
        if (k == n && !strncmp(p, id, n)) return 1;
        if (!fin) break;
        p = fin + 1;
    }
    return 0;
}

static void archivos_mimeapps(ListaDirs *l)
{
    char dir[PATH_MAX], buf[PATH_MAX];
    const char *escritorio = getenv("XDG_CURRENT_DESKTOP");
    char esc[64] = "";
    if (escritorio && *escritorio) {
        size_t k = strcspn(escritorio, ":");
        if (k < sizeof esc) { memcpy(esc, escritorio, k); esc[k] = '\0'; a_minusculas(esc); }
    }
    if (solar_dir_config(dir, sizeof dir) == 0) {
        char *padre = ruta_padre(dir);       /* ~/.config */
        if (padre) {
            if (*esc && formatear(buf, sizeof buf, "%s/%s-mimeapps.list", padre, esc) == 0) lista_agregar(l, buf);
            if (formatear(buf, sizeof buf, "%s/mimeapps.list", padre) == 0) lista_agregar(l, buf);
            free(padre);
        }
    }
    lista_agregar(l, "/etc/xdg/mimeapps.list");
    ListaDirs datos = { 0 };
    dirs_datos(&datos);
    for (size_t i = 0; i < datos.n; i++) {
        if (*esc && formatear(buf, sizeof buf, "%s/applications/%s-mimeapps.list", datos.dirs[i], esc) == 0)
            lista_agregar(l, buf);
        if (formatear(buf, sizeof buf, "%s/applications/mimeapps.list", datos.dirs[i]) == 0) lista_agregar(l, buf);
        if (formatear(buf, sizeof buf, "%s/applications/defaults.list", datos.dirs[i]) == 0) lista_agregar(l, buf);
    }
    lista_liberar(&datos);
}

/* ===================================================================== */
/*  API                                                                  */
/* ===================================================================== */

typedef struct {
    size_t indice;
    int    afinidad;
} Candidato;

static int comparar_candidatos(const void *a, const void *b, void *datos)
{
    const Candidato *x = a, *y = b;
    const Lanzadores *ls = datos;
    if (x->afinidad != y->afinidad) return y->afinidad - x->afinidad;
    return solar_comparar_natural(ls->v[x->indice].nombre, ls->v[y->indice].nombre);
}

int solar_programas_para(const char *const *rutas, size_t cantidad, int32_t incluir_todos, SolarProgramas *salida)
{
    if (!salida) return EINVAL;
    memset(salida, 0, sizeof *salida);
    pthread_once(&base.una_vez, cargar_base);

    /* Tipos de cada archivo (con sus padres) */
    enum { MAX_TIPOS = 12, MAX_ARCHIVOS = 64 };
    size_t n_arch = cantidad < MAX_ARCHIVOS ? cantidad : MAX_ARCHIVOS;
    char (*tipos)[MAX_TIPOS][128] = calloc(n_arch ? n_arch : 1, sizeof *tipos);
    size_t *n_tipos = calloc(n_arch ? n_arch : 1, sizeof *n_tipos);
    if (!tipos || !n_tipos) { free(tipos); free(n_tipos); return ENOMEM; }
    for (size_t i = 0; i < n_arch; i++) {
        char mime[128];
        if (solar_mime_de(rutas[i], mime, sizeof mime)) copiar_texto(mime, sizeof mime, "application/octet-stream");
        n_tipos[i] = familia(mime, tipos[i], MAX_TIPOS);
    }
    salida->mime = solar_strdup(n_arch ? tipos[0][0] : "");

    Asociaciones as = { 0 };
    if (n_arch) {
        ListaDirs listas = { 0 };
        archivos_mimeapps(&listas);
        for (size_t i = 0; i < listas.n; i++) leer_mimeapps(listas.dirs[i], tipos[0][0], &as);
        lista_liberar(&listas);
    }

    Lanzadores ls = { 0 };
    cargar_lanzadores(&ls);

    Candidato *cand = calloc(ls.n ? ls.n : 1, sizeof *cand);
    size_t n_cand = 0;
    char predeterminado[512] = "";
    /* El predeterminado es el primero de la lista que esté instalado */
    if (as.pref_defecto) {
        for (const char *p = as.pref_defecto; *p && !predeterminado[0];) {
            const char *fin = strchr(p, ';');
            size_t k = fin ? (size_t)(fin - p) : strlen(p);
            if (k > 0 && k < sizeof predeterminado) {
                char id[512];
                memcpy(id, p, k);
                id[k] = '\0';
                if (lanzador_ya(&ls, id)) copiar_texto(predeterminado, sizeof predeterminado, id);
            }
            if (!fin) break;
            p = fin + 1;
        }
    }

    for (size_t i = 0; cand && i < ls.n; i++) {
        const Lanzador *l = &ls.v[i];
        int afinidad = 0;
        if (n_arch) {
            int peor = 3;
            for (size_t a = 0; a < n_arch; a++) {
                int mejor = 0;
                for (size_t t = 0; t < n_tipos[a]; t++) {
                    if (declara(l, tipos[a][t]) || (a == 0 && lista_contiene(as.agregados, l->id))) {
                        int nivel = t == 0 ? 2 : 1;
                        if (nivel > mejor) mejor = nivel;
                    }
                }
                if (mejor < peor) peor = mejor;
            }
            afinidad = peor;
            if (lista_contiene(as.quitados, l->id)) afinidad = 0;
            if (predeterminado[0] && !strcmp(l->id, predeterminado)) afinidad = 3;
        }
        if (afinidad == 0 && (!incluir_todos || l->no_mostrar)) continue;
        cand[n_cand].indice = i;
        cand[n_cand].afinidad = afinidad;
        n_cand++;
    }
    if (cand) qsort_r(cand, n_cand, sizeof *cand, comparar_candidatos, &ls);

    int r = 0;
    salida->programas = calloc(n_cand ? n_cand : 1, sizeof *salida->programas);
    if (!salida->programas || !cand) r = ENOMEM;
    for (size_t i = 0; !r && i < n_cand; i++) {
        Lanzador *l = &ls.v[cand[i].indice];
        SolarPrograma *p = &salida->programas[salida->cantidad++];
        /* Le pasamos los textos (nadie más los usa) */
        p->id = l->id; l->id = NULL;
        p->archivo = l->archivo; l->archivo = NULL;
        p->nombre = l->nombre; l->nombre = NULL;
        p->descripcion = l->generico ? l->generico : l->comentario;
        if (l->generico) { l->generico = NULL; } else { l->comentario = NULL; }
        p->icono = l->icono; l->icono = NULL;
        p->categorias = l->categorias; l->categorias = NULL;
        p->terminal = l->terminal;
        p->afinidad = cand[i].afinidad;
    }

    for (size_t i = 0; i < ls.n; i++) lanzador_liberar(&ls.v[i]);
    free(ls.v);
    free(cand);
    free(as.pref_defecto); free(as.agregados); free(as.quitados);
    free(tipos);
    free(n_tipos);
    if (r) solar_programas_liberar(salida);
    return r;
}

void solar_programas_liberar(SolarProgramas *p)
{
    if (!p) return;
    for (size_t i = 0; i < p->cantidad; i++) {
        SolarPrograma *x = &p->programas[i];
        free(x->id); free(x->archivo); free(x->nombre); free(x->descripcion);
        free(x->icono); free(x->categorias);
    }
    free(p->programas);
    free(p->mime);
    memset(p, 0, sizeof *p);
}

/* ---------------------------------------------------------------------- */
/*  Lanzar un programa con archivos                                       */
/* ---------------------------------------------------------------------- */

/* Divide Exec según la especificación (comillas dobles con \" \\ \` \$). */
static int dividir_exec(const char *exec, char ***argv, size_t *n)
{
    *argv = NULL;
    *n = 0;
    size_t cap = 0;
    const char *p = exec;
    while (*p) {
        while (*p == ' ' || *p == '\t') p++;
        if (!*p) break;
        Cadena arg;
        cadena_iniciar(&arg);
        cadena_agregar(&arg, "");
        if (*p == '"') {
            p++;
            while (*p && *p != '"') {
                if (*p == '\\' && p[1] && strchr("\"`$\\", p[1])) p++;
                cadena_agregar_n(&arg, p, 1);
                p++;
            }
            if (*p == '"') p++;
        }
        while (*p && *p != ' ' && *p != '\t') {
            cadena_agregar_n(&arg, p, 1);
            p++;
        }
        if (arg.sin_memoria) { cadena_liberar(&arg); liberar_argv(*argv, *n); return ENOMEM; }
        if (*n + 1 >= cap) {
            cap = cap ? cap * 2 : 8;
            char **v = realloc(*argv, cap * sizeof *v);
            if (!v) { cadena_liberar(&arg); liberar_argv(*argv, *n); return ENOMEM; }
            *argv = v;
        }
        (*argv)[(*n)++] = arg.datos;
        (*argv)[*n] = NULL;
    }
    return *n ? 0 : EINVAL;
}

/* Cómo le dice cada terminal "ejecuta esto" */
static void prefijo_terminal(const char *nombre, const char **opcion)
{
    *opcion = "-e";
    if (!strcmp(nombre, "foot") || !strcmp(nombre, "kitty")) *opcion = NULL;
    else if (!strcmp(nombre, "gnome-terminal") || !strcmp(nombre, "ptyxis") || !strcmp(nombre, "kgx")) *opcion = "--";
    else if (!strcmp(nombre, "xfce4-terminal") || !strcmp(nombre, "terminator") || !strcmp(nombre, "mate-terminal")) *opcion = "-x";
}

int solar_programa_lanzar(const SolarPrograma *prog, const char *const *rutas, size_t cantidad, const char *terminal)
{
    if (!prog || !prog->archivo) return EINVAL;
    char pref[2][32];
    idiomas(pref);
    Lanzador l;
    int lr = leer_lanzador(prog->archivo, prog->id ? prog->id : "", pref, &l);
    if (lr != 0) { lanzador_liberar(&l); return ENOENT; }

    char **base_argv = NULL;
    size_t n = 0;
    int r = dividir_exec(l.exec, &base_argv, &n);
    if (r) { lanzador_liberar(&l); return r; }

    /* Si el programa solo acepta un archivo (%f/%u) y hay varios, se abre
     * una vez por archivo, como hacen los demás gestores. */
    int uno_por_vez = 0, tiene_campo = 0;
    for (size_t i = 0; i < n; i++) {
        if (strstr(base_argv[i], "%f") || strstr(base_argv[i], "%u")) { uno_por_vez = 1; tiene_campo = 1; }
        if (strstr(base_argv[i], "%F") || strstr(base_argv[i], "%U")) tiene_campo = 1;
    }
    size_t vueltas = uno_por_vez && cantidad > 1 ? cantidad : 1;

    char term[PATH_MAX] = "";
    char **term_argv = NULL;
    size_t term_n = 0;
    const char *opcion_term = NULL;
    if (l.terminal) {
        if (terminal && *terminal) copiar_texto(term, sizeof term, terminal);
        else if (solar_detectar_terminal(term, sizeof term)) { r = ENOENT; goto fin; }
        if ((r = dividir_comando(term, &term_argv, &term_n)) || term_n == 0) { r = r ? r : EINVAL; goto fin; }
        prefijo_terminal(ruta_base(term_argv[0]), &opcion_term);
    }

    for (size_t v = 0; v < vueltas && !r; v++) {
        size_t max = term_n + 2 + n + cantidad + 4;
        const char **argv = calloc(max, sizeof *argv);
        char **propios = calloc(n + 4, sizeof *propios);   /* textos armados aquí */
        size_t k = 0, np = 0;
        if (!argv || !propios) { free(argv); free(propios); r = ENOMEM; break; }
        for (size_t i = 0; i < term_n; i++) argv[k++] = term_argv[i];
        if (opcion_term) argv[k++] = opcion_term;
        for (size_t i = 0; i < n && k < max - 1; i++) {
            const char *a = base_argv[i];
            if (!strcmp(a, "%f") || !strcmp(a, "%u")) {
                if (cantidad) argv[k++] = rutas[uno_por_vez ? v : 0];
            } else if (!strcmp(a, "%F") || !strcmp(a, "%U")) {
                for (size_t j = 0; j < cantidad && k < max - 1; j++) argv[k++] = rutas[j];
            } else if (!strcmp(a, "%i")) {
                if (l.icono && *l.icono && k < max - 2) { argv[k++] = "--icon"; argv[k++] = l.icono; }
            } else if (!strcmp(a, "%c")) {
                argv[k++] = l.nombre;
            } else if (!strcmp(a, "%k")) {
                argv[k++] = l.archivo;
            } else if (strchr(a, '%')) {
                /* %% → %, y se quitan los códigos viejos (%d %n %m…) */
                Cadena c;
                cadena_iniciar(&c);
                cadena_agregar(&c, "");
                for (const char *p = a; *p; p++) {
                    if (*p == '%' && p[1]) {
                        p++;
                        if (*p == '%') cadena_agregar_n(&c, "%", 1);
                        else if ((*p == 'f' || *p == 'u') && cantidad) cadena_agregar(&c, rutas[uno_por_vez ? v : 0]);
                        continue;
                    }
                    cadena_agregar_n(&c, p, 1);
                }
                if (c.datos && *c.datos) { argv[k++] = c.datos; propios[np++] = c.datos; }
                else cadena_liberar(&c);
            } else {
                argv[k++] = a;
            }
        }
        if (!tiene_campo)
            for (size_t j = 0; j < cantidad && k < max - 1; j++) argv[k++] = rutas[j];
        argv[k] = NULL;

        const char *dir = l.dir_trabajo && *l.dir_trabajo ? l.dir_trabajo : NULL;
        char *padre = NULL;
        if (!dir && cantidad) {
            padre = ruta_padre(rutas[uno_por_vez ? v : 0]);
            if (padre && es_carpeta(padre)) dir = padre;
        }
        r = solar_lanzar(argv, dir);
        free(padre);
        for (size_t i = 0; i < np; i++) free(propios[i]);
        free(propios);
        free(argv);
    }

fin:
    liberar_argv(term_argv, term_n);
    liberar_argv(base_argv, n);
    lanzador_liberar(&l);
    return r;
}

/* ---------------------------------------------------------------------- */
/*  Hacerlo el predeterminado (~/.config/mimeapps.list)                   */
/* ---------------------------------------------------------------------- */

int solar_programa_predeterminar(const char *id, const char *mime)
{
    if (!id || !*id || !mime || !*mime || strchr(id, '\n') || strchr(mime, '\n') || strchr(mime, '=')) return EINVAL;
    char dir[PATH_MAX], archivo[PATH_MAX];
    if (solar_dir_config(dir, sizeof dir)) return ENOENT;
    char *config = ruta_padre(dir);
    if (!config) return ENOMEM;
    int r = formatear(archivo, sizeof archivo, "%s/mimeapps.list", config);
    if (!r) r = crear_dirs(config, 0700);
    free(config);
    if (r) return r;

    char *texto = leer_archivo(archivo, 4u << 20, NULL);
    Cadena s;
    cadena_iniciar(&s);
    size_t n = strlen(mime);
    int en_defecto = 0, puesto_defecto = 0, en_agregados = 0, puesto_agregado = 0, hubo_defecto = 0, hubo_agregados = 0;

    if (texto) {
        char *guardar = NULL;
        for (char *linea = strtok_r(texto, "\n", &guardar); linea; linea = strtok_r(NULL, "\n", &guardar)) {
            if (*linea == '[') {
                /* Al salir de una sección, agregar la línea si no estaba */
                if (en_defecto && !puesto_defecto) { cadena_printf(&s, "%s=%s;\n", mime, id); puesto_defecto = 1; }
                if (en_agregados && !puesto_agregado) { cadena_printf(&s, "%s=%s;\n", mime, id); puesto_agregado = 1; }
                en_defecto = !strncmp(linea, "[Default Applications]", 22);
                en_agregados = !strncmp(linea, "[Added Associations]", 20);
                hubo_defecto |= en_defecto;
                hubo_agregados |= en_agregados;
                cadena_printf(&s, "%s\n", linea);
                continue;
            }
            if ((en_defecto || en_agregados) && !strncmp(linea, mime, n) && linea[n] == '=') {
                if (en_defecto) { cadena_printf(&s, "%s=%s;\n", mime, id); puesto_defecto = 1; continue; }
                /* Asociaciones: el elegido primero y luego los demás */
                cadena_printf(&s, "%s=%s;", mime, id);
                const char *p = linea + n + 1;
                while (*p) {
                    const char *fin = strchr(p, ';');
                    size_t k = fin ? (size_t)(fin - p) : strlen(p);
                    if (k && !(k == strlen(id) && !strncmp(p, id, k))) { cadena_agregar_n(&s, p, k); cadena_agregar(&s, ";"); }
                    if (!fin) break;
                    p = fin + 1;
                }
                cadena_agregar(&s, "\n");
                puesto_agregado = 1;
                continue;
            }
            cadena_printf(&s, "%s\n", linea);
        }
        free(texto);
    }
    if (en_defecto && !puesto_defecto) { cadena_printf(&s, "%s=%s;\n", mime, id); puesto_defecto = 1; }
    if (en_agregados && !puesto_agregado) { cadena_printf(&s, "%s=%s;\n", mime, id); puesto_agregado = 1; }
    if (!hubo_defecto) cadena_printf(&s, "\n[Default Applications]\n%s=%s;\n", mime, id);
    if (!hubo_agregados) cadena_printf(&s, "\n[Added Associations]\n%s=%s;\n", mime, id);

    r = s.sin_memoria ? ENOMEM : escribir_atomico(archivo, s.datos ? s.datos : "", s.largo);
    cadena_liberar(&s);
    return r;
}
