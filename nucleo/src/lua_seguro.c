/*
 * ☀ Solar — Lua en un entorno seguro
 *
 * La configuración y los temas son Lua de verdad (puedes usar variables,
 * funciones, os.getenv…), pero sin acceso a io, os.execute, require ni
 * carga de bytecode. Además se limita la memoria (32 MiB) y la cantidad de
 * instrucciones, así un bucle infinito en config.lua no congela Solar.
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <lauxlib.h>
#include <lua.h>
#include <lualib.h>

#if LUA_VERSION_NUM < 503
#error "Solar necesita Lua 5.3 o más nuevo (recomendado 5.4)"
#endif

#define LIMITE_MEMORIA (32u << 20)
#define LIMITE_INSTRUCCIONES 50000000L
#define CADA_INSTRUCCIONES 10000

typedef struct {
    size_t usado;
    size_t limite;
    long   instrucciones;
} Presupuesto;

static void *asignar(void *ud, void *ptr, size_t viejo, size_t nuevo)
{
    Presupuesto *p = ud;
    size_t antes = ptr ? viejo : 0;
    if (nuevo == 0) {
        free(ptr);
        p->usado -= antes;
        return NULL;
    }
    if (p->usado - antes + nuevo > p->limite) return NULL;
    void *q = realloc(ptr, nuevo);
    if (q) p->usado = p->usado - antes + nuevo;
    return q;
}

static void gancho(lua_State *L, lua_Debug *ar)
{
    (void)ar;
    void *ud = NULL;
    lua_getallocf(L, &ud);
    Presupuesto *p = ud;
    p->instrucciones += CADA_INSTRUCCIONES;
    if (p->instrucciones > LIMITE_INSTRUCCIONES)
        luaL_error(L, "la configuración tardó demasiado (¿hay un bucle infinito?)");
}

static void abrir(lua_State *L, const char *nombre, lua_CFunction f)
{
    luaL_requiref(L, nombre, f, 1);
    lua_pop(L, 1);
}

lua_State *lua_seguro_crear(void)
{
    Presupuesto *p = calloc(1, sizeof *p);
    if (!p) return NULL;
    p->limite = LIMITE_MEMORIA;
#if LUA_VERSION_NUM >= 505
    lua_State *L = lua_newstate(asignar, p, 0x50A12u);
#else
    lua_State *L = lua_newstate(asignar, p);
#endif
    if (!L) {
        free(p);
        return NULL;
    }
    abrir(L, "_G", luaopen_base);
    abrir(L, LUA_TABLIBNAME, luaopen_table);
    abrir(L, LUA_STRLIBNAME, luaopen_string);
    abrir(L, LUA_MATHLIBNAME, luaopen_math);
    abrir(L, LUA_UTF8LIBNAME, luaopen_utf8);
    abrir(L, LUA_OSLIBNAME, luaopen_os);

    /* Fuera lo peligroso */
    static const char *const QUITAR[] = { "dofile", "loadfile", "load", "require", "collectgarbage" };
    for (size_t i = 0; i < SOLAR_LEN(QUITAR); i++) {
        lua_pushnil(L);
        lua_setglobal(L, QUITAR[i]);
    }
    /* os: solo lo inofensivo */
    lua_getglobal(L, "os");
    lua_newtable(L);
    static const char *const OS_PERMITIDO[] = { "getenv", "date", "time", "clock", "difftime" };
    for (size_t i = 0; i < SOLAR_LEN(OS_PERMITIDO); i++) {
        lua_getfield(L, -2, OS_PERMITIDO[i]);
        lua_setfield(L, -2, OS_PERMITIDO[i]);
    }
    lua_setglobal(L, "os");
    lua_pop(L, 1);

    /* Tabla "solar" con datos útiles para la configuración */
    char hogar[PATH_MAX];
    obtener_hogar(hogar, sizeof hogar);
    lua_newtable(L);
    lua_pushstring(L, SOLAR_VERSION);
    lua_setfield(L, -2, "version");
    lua_pushstring(L, hogar);
    lua_setfield(L, -2, "hogar");
    lua_setglobal(L, "solar");

    lua_sethook(L, gancho, LUA_MASKCOUNT, CADA_INSTRUCCIONES);
    return L;
}

int lua_seguro_ejecutar(lua_State *L, const char *archivo, char *avisos, size_t avisos_tam)
{
    if (!existe(archivo)) return ENOENT;
    int base = lua_gettop(L);
    int r = luaL_loadfilex(L, archivo, "t");
    if (r != LUA_OK) {
        agregar_aviso(avisos, avisos_tam, "%s", lua_tostring(L, -1));
        lua_settop(L, base);
        return r == LUA_ERRFILE ? EIO : EINVAL;
    }
    r = lua_pcall(L, 0, LUA_MULTRET, 0);
    if (r != LUA_OK) {
        const char *msg = lua_tostring(L, -1);
        agregar_aviso(avisos, avisos_tam, "%s", msg ? msg : "error desconocido");
        lua_settop(L, base);
        return EINVAL;
    }
    if (lua_gettop(L) > base && lua_istable(L, base + 1)) {
        lua_settop(L, base + 1);
        return 0;
    }
    lua_settop(L, base);
    lua_pushglobaltable(L);
    return 0;
}

void lua_seguro_cerrar(lua_State *L)
{
    if (!L) return;
    void *ud = NULL;
    lua_getallocf(L, &ud);
    lua_close(L);
    free(ud);
}

static int hex(char c)
{
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

int lua_leer_color(lua_State *L, int indice, uint32_t *color)
{
    if (lua_type(L, indice) == LUA_TNUMBER) {
        lua_Integer v = lua_tointeger(L, indice);
        if (v < 0 || v > 0xFFFFFF) return 0;
        *color = (uint32_t)v;
        return 1;
    }
    if (lua_type(L, indice) != LUA_TSTRING) return 0;
    const char *s = lua_tostring(L, indice);
    if (*s == '#') s++;
    size_t n = strlen(s);
    if (n != 6 && n != 3) return 0;
    uint32_t v = 0;
    for (size_t i = 0; i < n; i++) {
        int h = hex(s[i]);
        if (h < 0) return 0;
        if (n == 3) v = (v << 8) | (uint32_t)(h * 17);
        else v = (v << 4) | (uint32_t)h;
    }
    *color = v;
    return 1;
}
