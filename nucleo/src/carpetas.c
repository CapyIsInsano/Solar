/*
 * ☀ Solar — carpetas personalizadas (color y logotipo de cada carpeta)
 *
 * Se guardan en ~/.local/share/solar/carpetas.tsv, una por línea:
 *     ruta <TAB> color <TAB> logotipo <TAB> color del logotipo
 * con "-" cuando no hay valor. Las rutas escapan \t, \n y \\.
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

typedef struct {
    char *ruta;
    unsigned long secuencia;   /* el más reciente gana si hay duplicados */
    SolarEstiloCarpeta estilo;
} Registro;

struct SolarCarpetas {
    char         *archivo;
    Registro     *v;
    size_t        n, cap;
    unsigned long contador;
};

static int solo_ruta(const void *a, const void *b)
{
    return strcmp(((const Registro *)a)->ruta, ((const Registro *)b)->ruta);
}

static int por_ruta(const void *a, const void *b)
{
    const Registro *x = a, *y = b;
    int r = strcmp(x->ruta, y->ruta);
    if (r) return r;
    return (x->secuencia > y->secuencia) - (x->secuencia < y->secuencia);
}

static Registro *buscar(const SolarCarpetas *db, const char *ruta)
{
    if (!db || db->n == 0) return NULL;
    Registro clave = { (char *)ruta, 0, { 0 } };
    return bsearch(&clave, db->v, db->n, sizeof db->v[0], solo_ruta);
}

static void escapar(Cadena *c, const char *s)
{
    for (; *s; s++) {
        if (*s == '\t') cadena_agregar(c, "\\t");
        else if (*s == '\n') cadena_agregar(c, "\\n");
        else if (*s == '\\') cadena_agregar(c, "\\\\");
        else cadena_agregar_n(c, s, 1);
    }
}

static void desescapar(char *s)
{
    char *w = s;
    for (const char *r = s; *r; r++) {
        if (r[0] == '\\' && r[1]) {
            r++;
            *w++ = *r == 't' ? '\t' : (*r == 'n' ? '\n' : *r);
        } else {
            *w++ = *r;
        }
    }
    *w = '\0';
}

static int leer_hex(const char *s, uint32_t *color)
{
    if (!s || s[0] != '#' || strlen(s) != 7) return 0;
    char *fin = NULL;
    unsigned long v = strtoul(s + 1, &fin, 16);
    if (!fin || *fin) return 0;
    *color = (uint32_t)v & 0xFFFFFF;
    return 1;
}

static int agregar(SolarCarpetas *db, const char *ruta, const SolarEstiloCarpeta *e)
{
    if (db->n == db->cap) {
        size_t nuevo = db->cap ? db->cap * 2 : 32;
        Registro *q = realloc(db->v, nuevo * sizeof *q);
        if (!q) return ENOMEM;
        db->v = q;
        db->cap = nuevo;
    }
    char *copia = solar_strdup(ruta);
    if (!copia) return ENOMEM;
    db->v[db->n].ruta = copia;
    db->v[db->n].secuencia = ++db->contador;
    db->v[db->n].estilo = *e;
    db->n++;
    return 0;
}

static void ordenar(SolarCarpetas *db)
{
    if (db->n > 1) qsort(db->v, db->n, sizeof db->v[0], por_ruta);
    /* Sin duplicados: nos quedamos con el último leído */
    size_t w = 0;
    for (size_t i = 0; i < db->n; i++) {
        if (w > 0 && !strcmp(db->v[w - 1].ruta, db->v[i].ruta)) {
            free(db->v[w - 1].ruta);
            db->v[w - 1] = db->v[i];
        } else {
            db->v[w++] = db->v[i];
        }
    }
    db->n = w;
}

SolarCarpetas *solar_carpetas_abrir(const char *archivo)
{
    SolarCarpetas *db = calloc(1, sizeof *db);
    if (!db) return NULL;
    db->archivo = solar_strdup(archivo);
    if (!db->archivo) { free(db); return NULL; }
    char *texto = leer_archivo(archivo, 16u << 20, NULL);
    if (!texto) return db;
    char *guardar = NULL;
    for (char *linea = strtok_r(texto, "\n", &guardar); linea; linea = strtok_r(NULL, "\n", &guardar)) {
        if (linea[0] == '#' || linea[0] == '\0') continue;
        char *campos[4] = { NULL, NULL, NULL, NULL };
        int nc = 0;
        char *p = linea;
        campos[nc++] = p;
        while (*p && nc < 4) {
            if (*p == '\\' && p[1]) { p += 2; continue; }
            if (*p == '\t') { *p = '\0'; campos[nc++] = p + 1; }
            p++;
        }
        if (nc < 2 || !campos[0][0]) continue;
        desescapar(campos[0]);
        SolarEstiloCarpeta e;
        memset(&e, 0, sizeof e);
        if (leer_hex(campos[1], &e.color)) e.tiene_color = 1;
        if (nc >= 3 && strcmp(campos[2], "-")) {
            desescapar(campos[2]);
            copiar_texto(e.icono, sizeof e.icono, campos[2]);
        }
        if (nc >= 4 && leer_hex(campos[3], &e.color_icono)) e.tiene_color_icono = 1;
        if (!e.tiene_color && !e.icono[0] && !e.tiene_color_icono) continue;
        if (agregar(db, campos[0], &e)) break;
    }
    free(texto);
    ordenar(db);
    return db;
}

static int guardar(const SolarCarpetas *db)
{
    Cadena c;
    cadena_iniciar(&c);
    cadena_agregar(&c, "# Solar — carpetas personalizadas. Formato: ruta<TAB>color<TAB>logotipo<TAB>color_logotipo\n");
    for (size_t i = 0; i < db->n; i++) {
        const SolarEstiloCarpeta *e = &db->v[i].estilo;
        escapar(&c, db->v[i].ruta);
        if (e->tiene_color) cadena_printf(&c, "\t#%06X", e->color & 0xFFFFFF);
        else cadena_agregar(&c, "\t-");
        cadena_agregar(&c, "\t");
        if (e->icono[0]) escapar(&c, e->icono);
        else cadena_agregar(&c, "-");
        if (e->tiene_color_icono) cadena_printf(&c, "\t#%06X\n", e->color_icono & 0xFFFFFF);
        else cadena_agregar(&c, "\t-\n");
    }
    if (c.sin_memoria) { cadena_liberar(&c); return ENOMEM; }
    char *dir = ruta_padre(db->archivo);
    if (dir) { crear_dirs(dir, 0755); free(dir); }
    int r = escribir_atomico(db->archivo, c.datos ? c.datos : "", c.largo);
    cadena_liberar(&c);
    return r;
}

static char *clave(const char *ruta)
{
    char *r = solar_strdup(ruta);
    if (r) ruta_normalizar(r);
    return r;
}

int solar_carpetas_obtener(const SolarCarpetas *db, const char *ruta, SolarEstiloCarpeta *estilo)
{
    if (!db || !ruta) return 0;
    char *k = clave(ruta);
    if (!k) return 0;
    Registro *r = buscar(db, k);
    free(k);
    if (!r) return 0;
    if (estilo) *estilo = r->estilo;
    return 1;
}

int solar_carpetas_poner(SolarCarpetas *db, const char *ruta, const SolarEstiloCarpeta *estilo)
{
    if (!db || !ruta || !estilo) return EINVAL;
    if (!estilo->tiene_color && !estilo->icono[0] && !estilo->tiene_color_icono)
        return solar_carpetas_quitar(db, ruta);
    char *k = clave(ruta);
    if (!k) return ENOMEM;
    SolarEstiloCarpeta e = *estilo;
    e.icono[sizeof e.icono - 1] = '\0';
    Registro *r = buscar(db, k);
    int res = 0;
    if (r) {
        r->estilo = e;
        r->secuencia = ++db->contador;
    } else {
        res = agregar(db, k, &e);
        if (!res) ordenar(db);
    }
    free(k);
    return res ? res : guardar(db);
}

int solar_carpetas_quitar(SolarCarpetas *db, const char *ruta)
{
    if (!db || !ruta) return EINVAL;
    char *k = clave(ruta);
    if (!k) return ENOMEM;
    Registro *r = buscar(db, k);
    free(k);
    if (!r) return 0;
    size_t i = (size_t)(r - db->v);
    free(db->v[i].ruta);
    memmove(&db->v[i], &db->v[i + 1], (db->n - i - 1) * sizeof db->v[0]);
    db->n--;
    return guardar(db);
}

int solar_carpetas_olvidar_arbol(SolarCarpetas *db, const char *ruta)
{
    if (!db || !ruta) return EINVAL;
    char *k = clave(ruta);
    if (!k) return ENOMEM;
    size_t w = 0, quitados = 0;
    for (size_t i = 0; i < db->n; i++) {
        if (ruta_es_dentro_de(db->v[i].ruta, k)) {
            free(db->v[i].ruta);
            quitados++;
        } else {
            db->v[w++] = db->v[i];
        }
    }
    db->n = w;
    free(k);
    return quitados ? guardar(db) : 0;
}

int solar_carpetas_renombrar(SolarCarpetas *db, const char *vieja, const char *nueva)
{
    if (!db || !vieja || !nueva) return EINVAL;
    char *kv = clave(vieja), *kn = clave(nueva);
    if (!kv || !kn) { free(kv); free(kn); return ENOMEM; }
    size_t lv = strlen(kv);
    int cambios = 0;
    for (size_t i = 0; i < db->n; i++) {
        if (!ruta_es_dentro_de(db->v[i].ruta, kv)) continue;
        const char *resto = db->v[i].ruta + (strcmp(kv, "/") ? lv : 0);
        size_t largo = strlen(kn) + strlen(resto) + 1;
        char *nueva_ruta = malloc(largo);
        if (!nueva_ruta) continue;
        snprintf(nueva_ruta, largo, "%s%s", kn, resto);
        free(db->v[i].ruta);
        db->v[i].ruta = nueva_ruta;
        db->v[i].secuencia = ++db->contador;
        cambios = 1;
    }
    free(kv);
    free(kn);
    if (!cambios) return 0;
    ordenar(db);
    return guardar(db);
}

int solar_carpetas_copiar(SolarCarpetas *db, const char *origen, const char *destino)
{
    if (!db || !origen || !destino) return EINVAL;
    char *ko = clave(origen), *kd = clave(destino);
    if (!ko || !kd) { free(ko); free(kd); return ENOMEM; }
    size_t lo = strlen(ko);
    size_t n_original = db->n;
    int cambios = 0;
    for (size_t i = 0; i < n_original; i++) {
        if (!ruta_es_dentro_de(db->v[i].ruta, ko)) continue;
        const char *resto = db->v[i].ruta + (strcmp(ko, "/") ? lo : 0);
        size_t largo = strlen(kd) + strlen(resto) + 1;
        char *nueva = malloc(largo);
        if (!nueva) continue;
        snprintf(nueva, largo, "%s%s", kd, resto);
        SolarEstiloCarpeta e = db->v[i].estilo;
        if (agregar(db, nueva, &e) == 0) cambios = 1;
        free(nueva);
    }
    free(ko);
    free(kd);
    if (!cambios) return 0;
    ordenar(db);
    return guardar(db);
}

size_t solar_carpetas_cantidad(const SolarCarpetas *db) { return db ? db->n : 0; }

void solar_carpetas_cerrar(SolarCarpetas *db)
{
    if (!db) return;
    for (size_t i = 0; i < db->n; i++) free(db->v[i].ruta);
    free(db->v);
    free(db->archivo);
    free(db);
}
