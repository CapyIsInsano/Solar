/*
 * ☀ Solar — temas de colores
 *
 * Hay temas integrados y puedes crear los tuyos en ~/.config/solar/temas/
 * como archivos .lua:
 *
 *     return {
 *       nombre = "Mi tema",
 *       base = "eclipse",          -- hereda lo que no pongas
 *       colores = { acento = "#22C55E", carpeta = "#16A34A" },
 *     }
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <dirent.h>
#include <lauxlib.h>
#include <lua.h>

static const char *const CLAVES[SOLAR_COLORES_TOTAL] = {
    "fondo", "superficie", "elevado", "borde", "texto", "texto_tenue", "acento",
    "texto_acento", "seleccion", "peligro", "exito", "aviso", "carpeta",
};

static const char *const DESCRIPCIONES[SOLAR_COLORES_TOTAL] = {
    "fondo de la vista de archivos",
    "barras (lateral, herramientas, estado)",
    "menús, diálogos y botones",
    "líneas y bordes",
    "texto principal",
    "texto secundario",
    "color de acento",
    "texto encima del acento",
    "fondo de lo seleccionado",
    "acciones peligrosas (eliminar)",
    "todo salió bien",
    "advertencias",
    "color de carpeta por defecto",
};

const char *solar_color_clave(int indice)
{
    if (indice < 0 || indice >= SOLAR_COLORES_TOTAL) return "";
    return CLAVES[indice];
}

typedef struct {
    const char *id;
    const char *nombre;
    int oscuro;
    uint32_t c[SOLAR_COLORES_TOTAL];
} TemaIntegrado;

/*                       fondo     superf.   elevado   borde     texto     tenue     acento    t.acento  selecc.   peligro   éxito     aviso     carpeta */
static const TemaIntegrado INTEGRADOS[] = {
    { "solar", "Solar", 1,
      { 0x15100D, 0x1E1712, 0x2A2019, 0x3B2D23, 0xF6EEE7, 0xA8978A, 0xF97316, 0x1C0F06, 0xF97316, 0xEF4444, 0x22C55E, 0xEAB308, 0xF97316 } },
    { "amanecer", "Amanecer", 0,
      { 0xFFF9F3, 0xFBEFE3, 0xFFFFFF, 0xEAD6C3, 0x2B1C11, 0x7B6555, 0xEA580C, 0xFFFFFF, 0xEA580C, 0xDC2626, 0x16A34A, 0xCA8A04, 0xF97316 } },
    { "eclipse", "Eclipse", 1,
      { 0x000000, 0x0A0A0A, 0x161616, 0x272727, 0xEDEDED, 0x8C8C8C, 0xF59E0B, 0x000000, 0xF59E0B, 0xF87171, 0x4ADE80, 0xFBBF24, 0xF59E0B } },
    { "nebulosa", "Nebulosa", 1,
      { 0x110E1B, 0x191527, 0x231D36, 0x352C4E, 0xEFEAFB, 0x9D93B9, 0xA855F7, 0xFFFFFF, 0xA855F7, 0xF43F5E, 0x34D399, 0xFBBF24, 0xA855F7 } },
    { "aurora", "Aurora", 1,
      { 0x0A1514, 0x0F1F1D, 0x162B28, 0x22403B, 0xE6F4F1, 0x8BAFA9, 0x2DD4BF, 0x042F2E, 0x2DD4BF, 0xF87171, 0x4ADE80, 0xFACC15, 0x14B8A6 } },
    { "oceano", "Océano", 1,
      { 0x0A111F, 0x10192C, 0x17223B, 0x24324E, 0xE5ECF8, 0x8C9BB5, 0x3B82F6, 0xFFFFFF, 0x3B82F6, 0xF87171, 0x34D399, 0xFBBF24, 0x3B82F6 } },
    { "marte", "Marte", 1,
      { 0x160C0A, 0x20120F, 0x2C1914, 0x45281F, 0xF8E9E4, 0xB49187, 0xEF4444, 0xFFFFFF, 0xEF4444, 0xFB7185, 0x4ADE80, 0xFBBF24, 0xDC2626 } },
    { "grafito", "Grafito", 1,
      { 0x18181B, 0x1F1F23, 0x2A2A30, 0x3A3A42, 0xF4F4F5, 0xA1A1AA, 0x38BDF8, 0x082F49, 0x38BDF8, 0xF87171, 0x4ADE80, 0xFACC15, 0x38BDF8 } },
    { "luna", "Luna", 0,
      { 0xF7F8FA, 0xEDF0F4, 0xFFFFFF, 0xD8DDE5, 0x1A1F27, 0x636B78, 0x6366F1, 0xFFFFFF, 0x6366F1, 0xDC2626, 0x16A34A, 0xCA8A04, 0x5B8DEF } },
};

static int agregar_tema(SolarTemas *t, size_t *cap, const SolarTema *nuevo)
{
    /* Un tema del usuario con el mismo id reemplaza al anterior */
    for (size_t i = 0; i < t->cantidad; i++) {
        if (!strcmp(t->temas[i].id, nuevo->id)) {
            free(t->temas[i].id);
            free(t->temas[i].nombre);
            free(t->temas[i].ruta);
            t->temas[i] = *nuevo;
            return 0;
        }
    }
    if (t->cantidad == *cap) {
        size_t n = *cap ? *cap * 2 : 16;
        SolarTema *q = realloc(t->temas, n * sizeof *q);
        if (!q) return ENOMEM;
        t->temas = q;
        *cap = n;
    }
    t->temas[t->cantidad++] = *nuevo;
    return 0;
}

static const SolarTema *buscar_tema(const SolarTemas *t, const char *id)
{
    for (size_t i = 0; i < t->cantidad; i++)
        if (!strcmp(t->temas[i].id, id)) return &t->temas[i];
    return NULL;
}

static int id_valido(const char *id)
{
    if (!*id || strlen(id) > 48) return 0;
    for (const char *p = id; *p; p++) {
        char c = *p;
        if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' || c == '-'))
            return 0;
    }
    return 1;
}

static int por_nombre(const void *a, const void *b)
{
    return strcmp(*(char *const *)a, *(char *const *)b);
}

static void cargar_archivo(SolarTemas *t, size_t *cap, const char *dir, const char *archivo,
                           char *avisos, size_t avisos_tam)
{
    char id[64];
    size_t largo = strlen(archivo) - 4;
    if (largo >= sizeof id) return;
    memcpy(id, archivo, largo);
    id[largo] = '\0';
    if (!id_valido(id)) {
        agregar_aviso(avisos, avisos_tam, "Tema «%s»: el nombre del archivo solo puede tener letras sin acento, números, _ y -.", archivo);
        return;
    }
    char *ruta = ruta_unir(dir, archivo);
    if (!ruta) return;
    lua_State *L = lua_seguro_crear();
    if (!L) { free(ruta); return; }
    char error[1024] = "";
    if (lua_seguro_ejecutar(L, ruta, error, sizeof error) != 0) {
        agregar_aviso(avisos, avisos_tam, "Tema «%s»: %s", archivo, error);
        lua_seguro_cerrar(L);
        free(ruta);
        return;
    }
    int tabla = lua_gettop(L);

    const SolarTema *base = NULL;
    lua_getfield(L, tabla, "base");
    if (lua_type(L, -1) == LUA_TSTRING) {
        base = buscar_tema(t, lua_tostring(L, -1));
        if (!base) agregar_aviso(avisos, avisos_tam, "Tema «%s»: no existe el tema base «%s», uso «solar».",
                                 archivo, lua_tostring(L, -1));
    }
    lua_pop(L, 1);
    if (!base) base = buscar_tema(t, "solar");

    SolarTema nuevo;
    memset(&nuevo, 0, sizeof nuevo);
    if (base) {
        memcpy(nuevo.colores, base->colores, sizeof nuevo.colores);
        nuevo.oscuro = base->oscuro;
    }
    nuevo.id = solar_strdup(id);
    nuevo.ruta = ruta;
    nuevo.integrado = 0;

    lua_getfield(L, tabla, "nombre");
    nuevo.nombre = solar_strdup(lua_type(L, -1) == LUA_TSTRING ? lua_tostring(L, -1) : id);
    lua_pop(L, 1);
    lua_getfield(L, tabla, "oscuro");
    if (lua_isboolean(L, -1)) nuevo.oscuro = lua_toboolean(L, -1);
    lua_pop(L, 1);

    /* Los colores pueden ir dentro de "colores = {…}" o sueltos */
    lua_getfield(L, tabla, "colores");
    int colores = lua_istable(L, -1) ? lua_gettop(L) : tabla;
    int puesto[SOLAR_COLORES_TOTAL] = { 0 };
    for (int i = 0; i < SOLAR_COLORES_TOTAL; i++) {
        lua_getfield(L, colores, CLAVES[i]);
        if (!lua_isnil(L, -1)) {
            uint32_t c;
            if (lua_leer_color(L, -1, &c)) {
                nuevo.colores[i] = c;
                puesto[i] = 1;
            } else {
                agregar_aviso(avisos, avisos_tam, "Tema «%s»: el color «%s» debe ser como \"#F97316\".", archivo, CLAVES[i]);
            }
        }
        lua_pop(L, 1);
    }
    lua_pop(L, 1);
    /* Si cambiaste el acento, la selección (y la carpeta, si la base la
     * tenía igual al acento) lo siguen, salvo que digas otra cosa. */
    if (base && puesto[SOLAR_COLOR_ACENTO]) {
        if (!puesto[SOLAR_COLOR_SELECCION] && base->colores[SOLAR_COLOR_SELECCION] == base->colores[SOLAR_COLOR_ACENTO])
            nuevo.colores[SOLAR_COLOR_SELECCION] = nuevo.colores[SOLAR_COLOR_ACENTO];
        if (!puesto[SOLAR_COLOR_CARPETA] && base->colores[SOLAR_COLOR_CARPETA] == base->colores[SOLAR_COLOR_ACENTO])
            nuevo.colores[SOLAR_COLOR_CARPETA] = nuevo.colores[SOLAR_COLOR_ACENTO];
    }
    lua_seguro_cerrar(L);
    if (!nuevo.id || !nuevo.nombre || agregar_tema(t, cap, &nuevo)) {
        free(nuevo.id);
        free(nuevo.nombre);
        free(nuevo.ruta);
    }
}

int solar_temas_cargar(const char *dir_usuario, SolarTemas *salida, char *avisos, size_t avisos_tam)
{
    if (avisos && avisos_tam) avisos[0] = '\0';
    salida->temas = NULL;
    salida->cantidad = 0;
    size_t cap = 0;
    for (size_t i = 0; i < SOLAR_LEN(INTEGRADOS); i++) {
        SolarTema t;
        memset(&t, 0, sizeof t);
        t.id = solar_strdup(INTEGRADOS[i].id);
        t.nombre = solar_strdup(INTEGRADOS[i].nombre);
        t.oscuro = INTEGRADOS[i].oscuro;
        t.integrado = 1;
        memcpy(t.colores, INTEGRADOS[i].c, sizeof t.colores);
        if (!t.id || !t.nombre || agregar_tema(salida, &cap, &t)) {
            free(t.id);
            free(t.nombre);
            return ENOMEM;
        }
    }
    if (!dir_usuario) return 0;
    DIR *d = opendir(dir_usuario);
    if (!d) return 0;
    char *nombres[256];
    size_t n = 0;
    struct dirent *de;
    while ((de = readdir(d)) && n < SOLAR_LEN(nombres)) {
        if (de->d_name[0] == '.' || !termina_con(de->d_name, ".lua") || strlen(de->d_name) <= 4) continue;
        nombres[n] = solar_strdup(de->d_name);
        if (nombres[n]) n++;
    }
    closedir(d);
    qsort(nombres, n, sizeof nombres[0], por_nombre);
    for (size_t i = 0; i < n; i++) {
        cargar_archivo(salida, &cap, dir_usuario, nombres[i], avisos, avisos_tam);
        free(nombres[i]);
    }
    return 0;
}

void solar_temas_liberar(SolarTemas *t)
{
    if (!t) return;
    for (size_t i = 0; i < t->cantidad; i++) {
        free(t->temas[i].id);
        free(t->temas[i].nombre);
        free(t->temas[i].ruta);
    }
    free(t->temas);
    t->temas = NULL;
    t->cantidad = 0;
}

int solar_tema_guardar(const char *archivo, const SolarTema *tema)
{
    Cadena s;
    cadena_iniciar(&s);
    cadena_agregar(&s, "-- ☀ Tema de Solar: ");
    cadena_agregar(&s, tema->nombre ? tema->nombre : "sin nombre");
    cadena_agregar(&s, "\n-- Guardado desde Ajustes → Apariencia. Puedes editarlo a mano;\n"
                       "-- los colores van como \"#RRGGBB\". Si borras alguno, se usa el de «base».\n\n"
                       "return {\n  nombre = ");
    cadena_agregar_lua(&s, tema->nombre ? tema->nombre : "Mi tema");
    cadena_printf(&s, ",\n  oscuro = %s,\n  base = \"%s\",\n  colores = {\n",
                  tema->oscuro ? "true" : "false", tema->oscuro ? "solar" : "amanecer");
    for (int i = 0; i < SOLAR_COLORES_TOTAL; i++) {
        cadena_printf(&s, "    %-13s = \"#%06X\",  -- %s\n", CLAVES[i], tema->colores[i] & 0xFFFFFF, DESCRIPCIONES[i]);
    }
    cadena_agregar(&s, "  },\n}\n");
    if (s.sin_memoria) { cadena_liberar(&s); return ENOMEM; }
    char *dir = ruta_padre(archivo);
    if (dir) { crear_dirs(dir, 0755); free(dir); }
    int r = escribir_atomico(archivo, s.datos, s.largo);
    cadena_liberar(&s);
    return r;
}
