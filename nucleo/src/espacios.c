/*
 * ☀ Solar — espacios de trabajo (~/.config/solar/espacios.lua)
 *
 * Cada espacio tiene su nombre, sus pestañas (con la vista de cada una) y,
 * si quiere, su propio tema y sus propios ajustes de archivos. Solar
 * reescribe el archivo solo; es Lua normal y se puede editar con Solar
 * cerrado. Lo que no se entiende se ignora y se explica en los avisos.
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <lauxlib.h>
#include <lua.h>

#define MAX_ESPACIOS 64
#define MAX_PESTANAS 128

static void liberar_espacio(SolarEspacioTrabajo *w)
{
    free(w->id);
    free(w->nombre);
    free(w->icono);
    free(w->color);
    free(w->tema);
    for (size_t i = 0; i < w->n_pestanas; i++) free(w->pestanas[i].ruta);
    free(w->pestanas);
    memset(w, 0, sizeof *w);
}

void solar_espacios_liberar(SolarEspacios *e)
{
    if (!e) return;
    free(e->activo);
    for (size_t i = 0; i < e->n_espacios; i++) liberar_espacio(&e->espacios[i]);
    free(e->espacios);
    memset(e, 0, sizeof *e);
}

/* ===================================================================== */
/*  Lectura                                                              */
/* ===================================================================== */

static void leer_pestanas(lua_State *L, SolarEspacioTrabajo *w, int numero, char *avisos, size_t tam)
{
    lua_getfield(L, -1, "pestanas");
    if (lua_isnil(L, -1)) { lua_pop(L, 1); return; }
    if (!lua_istable(L, -1)) {
        agregar_aviso(avisos, tam, "En el espacio %d, «pestanas» debe ser una lista.", numero);
        lua_pop(L, 1);
        return;
    }
    lua_Integer total = (lua_Integer)lua_rawlen(L, -1);
    if (total > MAX_PESTANAS) total = MAX_PESTANAS;
    w->pestanas = calloc(total > 0 ? (size_t)total : 1, sizeof *w->pestanas);
    if (!w->pestanas) { lua_pop(L, 1); return; }
    for (lua_Integer i = 1; i <= total; i++) {
        lua_rawgeti(L, -1, i);
        SolarPestana p = { NULL, 0, w->vista };
        if (lua_type(L, -1) == LUA_TSTRING) {
            p.ruta = solar_strdup(lua_tostring(L, -1));
        } else if (lua_istable(L, -1)) {
            Lector l = { L, lua_gettop(L), avisos, tam };
            p.ruta = solar_strdup("");
            lector_texto(&l, "ruta", &p.ruta);
            lector_bool(&l, "papelera", &p.papelera);
            lector_opcion(&l, "vista", &p.vista, OPCIONES_VISTA, N_OPCIONES_VISTA);
        } else {
            agregar_aviso(avisos, tam, "La pestaña %d del espacio %d no se entiende.", (int)i, numero);
        }
        if (p.ruta && (p.papelera || p.ruta[0])) {
            w->pestanas[w->n_pestanas++] = p;
        } else {
            if (p.ruta && !p.papelera) agregar_aviso(avisos, tam, "La pestaña %d del espacio %d no tiene «ruta».", (int)i, numero);
            free(p.ruta);
        }
        lua_pop(L, 1);
    }
    lua_pop(L, 1);
}

static int id_repetido(const SolarEspacios *e, const char *id)
{
    for (size_t i = 0; i < e->n_espacios; i++)
        if (!strcmp(e->espacios[i].id, id)) return 1;
    return 0;
}

static void leer_espacio(lua_State *L, SolarEspacios *e, const SolarAjustesVista *base, int numero,
                         char *avisos, size_t tam)
{
    SolarEspacioTrabajo w;
    memset(&w, 0, sizeof w);
    w.id = solar_strdup("");
    w.nombre = solar_strdup("");
    w.icono = solar_strdup("");
    w.color = solar_strdup("");
    w.tema = solar_strdup("");
    w.vista = SOLAR_VISTA_CUADRICULA;
    w.ajustes = *base;
    if (!w.id || !w.nombre || !w.icono || !w.color || !w.tema) {
        liberar_espacio(&w);
        return;
    }
    Lector l = { L, lua_gettop(L), avisos, tam };
    lector_texto(&l, "id", &w.id);
    lector_texto(&l, "nombre", &w.nombre);
    lector_texto(&l, "icono", &w.icono);
    lector_texto(&l, "color", &w.color);
    lector_texto(&l, "tema", &w.tema);
    if (w.tema && strcmp(solar_tema_id_actual(w.tema), w.tema)) {
        char *nuevo = solar_strdup(solar_tema_id_actual(w.tema));
        if (nuevo) { free(w.tema); w.tema = nuevo; }
    }
    lector_opcion(&l, "vista", &w.vista, OPCIONES_VISTA, N_OPCIONES_VISTA);
    lector_bool(&l, "ajustes_propios", &w.ajustes_propios);
    lua_getfield(L, -1, "ajustes");
    if (lua_istable(L, -1)) {
        Lector la = { L, lua_gettop(L), avisos, tam };
        leer_ajustes_vista(&la, &w.ajustes);
    } else if (!lua_isnil(L, -1)) {
        agregar_aviso(avisos, tam, "En el espacio %d, «ajustes» debe ser una tabla.", numero);
    }
    lua_pop(L, 1);
    leer_pestanas(L, &w, numero, avisos, tam);
    int32_t activa = 1;
    lector_entero(&l, "activa", &activa, 1, MAX_PESTANAS);
    w.activa = w.n_pestanas && (size_t)activa <= w.n_pestanas ? activa - 1 : 0;

    /* Sin id (escrito a mano) o repetido: se inventa uno */
    if (!w.id[0] || id_repetido(e, w.id)) {
        char nuevo[48];
        int n = numero;
        do snprintf(nuevo, sizeof nuevo, "espacio-%d", n++);
        while (id_repetido(e, nuevo));
        char *id = solar_strdup(nuevo);
        if (id) { free(w.id); w.id = id; }
    }
    if (!w.nombre[0]) {
        char nombre[48];
        snprintf(nombre, sizeof nombre, "Espacio %d", numero);
        char *n = solar_strdup(nombre);
        if (n) { free(w.nombre); w.nombre = n; }
    }
    e->espacios[e->n_espacios++] = w;
}

int solar_espacios_cargar(const char *archivo, const SolarAjustesVista *base, SolarEspacios *e,
                          char *avisos, size_t avisos_tam)
{
    memset(e, 0, sizeof *e);
    if (avisos && avisos_tam) avisos[0] = '\0';
    SolarAjustesVista defecto;
    if (!base) {
        solar_ajustes_vista_defecto(&defecto);
        base = &defecto;
    }
    e->activo = solar_strdup("");
    if (!e->activo) return ENOMEM;
    if (!existe(archivo)) return ENOENT;
    lua_State *L = lua_seguro_crear();
    if (!L) return ENOMEM;
    int r = lua_seguro_ejecutar(L, archivo, avisos, avisos_tam);
    if (r) {
        lua_seguro_cerrar(L);
        return r;
    }
    Lector l = { L, lua_gettop(L), avisos, avisos_tam };
    lector_texto(&l, "activo", &e->activo);
    lua_getfield(L, l.tabla, "espacios");
    if (lua_istable(L, -1)) {
        lua_Integer total = (lua_Integer)lua_rawlen(L, -1);
        if (total > MAX_ESPACIOS) {
            agregar_aviso(avisos, avisos_tam, "Hay más de %d espacios; los demás se ignoran.", MAX_ESPACIOS);
            total = MAX_ESPACIOS;
        }
        e->espacios = calloc(total > 0 ? (size_t)total : 1, sizeof *e->espacios);
        if (!e->espacios) {
            lua_seguro_cerrar(L);
            return ENOMEM;
        }
        for (lua_Integer i = 1; i <= total; i++) {
            lua_rawgeti(L, -1, i);
            if (lua_istable(L, -1)) leer_espacio(L, e, base, (int)i, avisos, avisos_tam);
            else agregar_aviso(avisos, avisos_tam, "El espacio número %d no se entiende.", (int)i);
            lua_pop(L, 1);
        }
    } else if (!lua_isnil(L, -1)) {
        agregar_aviso(avisos, avisos_tam, "«espacios» debe ser una lista: { { nombre = \"…\", pestanas = { … } }, … }.");
    }
    lua_pop(L, 1);
    lua_seguro_cerrar(L);
    return 0;
}

/* ===================================================================== */
/*  Escritura                                                            */
/* ===================================================================== */

static void escribir_numero(Cadena *s, double v)
{
    char buf[64];
    snprintf(buf, sizeof buf, "%.2f", v);
    for (char *c = buf; *c; c++)
        if (*c == ',') *c = '.';   /* por si el locale usa coma decimal */
    cadena_agregar(s, buf);
}

static void escribir_texto(Cadena *s, const char *sangria, const char *clave, const char *valor)
{
    cadena_printf(s, "%s%s = ", sangria, clave);
    cadena_agregar_lua(s, valor ? valor : "");
    cadena_agregar(s, ",\n");
}

static const char *nombre_vista(int v)
{
    return NOMBRES_VISTA[v >= 0 && v <= 3 ? v : 0];
}

int solar_espacios_guardar(const char *archivo, const SolarEspacios *e)
{
    Cadena s;
    cadena_iniciar(&s);
    cadena_agregar(&s,
        "-- ☀ Solar " SOLAR_VERSION " — espacios de trabajo\n"
        "--\n"
        "-- Aquí se guardan tus espacios de trabajo y las pestañas que tenías\n"
        "-- abiertas en cada uno (con la vista de cada pestaña). Solar lo\n"
        "-- reescribe solo; puedes editarlo a mano con Solar cerrado.\n"
        "--\n"
        "-- tema = \"\"               usa el tema de config.lua\n"
        "-- ajustes_propios = false  usa los ajustes de archivos de config.lua\n"
        "--                          (con true, usa los de «ajustes»)\n"
        "-- vista: cuadricula | lista | mosaicos | orbita\n\n"
        "return {\n");
    escribir_texto(&s, "  ", "activo", e->activo);
    cadena_agregar(&s, "  espacios = {\n");
    for (size_t i = 0; i < e->n_espacios; i++) {
        const SolarEspacioTrabajo *w = &e->espacios[i];
        const SolarAjustesVista *a = &w->ajustes;
        cadena_agregar(&s, "    {\n");
        escribir_texto(&s, "      ", "id", w->id);
        escribir_texto(&s, "      ", "nombre", w->nombre);
        escribir_texto(&s, "      ", "icono", w->icono);
        escribir_texto(&s, "      ", "color", w->color);
        escribir_texto(&s, "      ", "tema", w->tema);
        escribir_texto(&s, "      ", "vista", nombre_vista(w->vista));
        cadena_printf(&s, "      ajustes_propios = %s,\n", bool_lua(w->ajustes_propios));
        cadena_printf(&s, "      ajustes = {\n"
                          "        tamano_iconos = %d, mostrar_ocultos = %s, carpetas_primero = %s,\n"
                          "        orden = \"%s\", orden_descendente = %s,\n"
                          "        orbita_girar = %s, orbita_velocidad = ",
                      (int)a->tamano_iconos, bool_lua(a->mostrar_ocultos), bool_lua(a->carpetas_primero),
                      nombre_orden(a->orden), bool_lua(a->orden_descendente), bool_lua(a->orbita_girar));
        escribir_numero(&s, a->orbita_velocidad);
        cadena_printf(&s, ", orbita_orden = \"%s\", orbita_electricidad = %s,\n"
                          "      },\n",
                      nombre_orden_orbita(a->orbita_orden), bool_lua(a->orbita_electricidad));
        cadena_agregar(&s, "      pestanas = {\n");
        for (size_t j = 0; j < w->n_pestanas; j++) {
            const SolarPestana *p = &w->pestanas[j];
            if (p->papelera) {
                cadena_printf(&s, "        { papelera = true, vista = \"%s\" },\n", nombre_vista(p->vista));
            } else {
                cadena_agregar(&s, "        { ruta = ");
                cadena_agregar_lua(&s, p->ruta ? p->ruta : "");
                cadena_printf(&s, ", vista = \"%s\" },\n", nombre_vista(p->vista));
            }
        }
        cadena_agregar(&s, "      },\n");
        cadena_printf(&s, "      activa = %d,\n", (int)(w->activa >= 0 ? w->activa + 1 : 1));
        cadena_agregar(&s, "    },\n");
    }
    cadena_agregar(&s, "  },\n}\n");

    if (s.sin_memoria) {
        cadena_liberar(&s);
        return ENOMEM;
    }
    char *dir = ruta_padre(archivo);
    if (dir) { crear_dirs(dir, 0755); free(dir); }
    int r = escribir_atomico(archivo, s.datos, s.largo);
    cadena_liberar(&s);
    return r;
}
