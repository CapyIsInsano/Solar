/*
 * ☀ Solar — pruebas del núcleo en C
 *
 *   make pruebas            (con AddressSanitizer y UBSan)
 *
 * Todo pasa dentro de una carpeta temporal; HOME, XDG_DATA_HOME y
 * XDG_CONFIG_HOME apuntan ahí para no tocar tu papelera ni tu config.
 *
 * Licencia: GPL-3.0-or-later
 */
#include "../src/interno.h"

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

static int fallos = 0, total = 0;
static char raiz[PATH_MAX];

#define REVISAR(cond)                                                              \
    do {                                                                           \
        total++;                                                                   \
        if (!(cond)) {                                                             \
            fallos++;                                                              \
            fprintf(stderr, "  ✗ %s:%d: %s\n", __FILE__, __LINE__, #cond);        \
        }                                                                          \
    } while (0)

#define SECCION(nombre) printf("• %s\n", nombre)

/* Rutas dentro de la carpeta temporal (se liberan al final). */
static char *pozo[2048];
static size_t en_pozo = 0;

static char *en_raiz(const char *rel)
{
    char buf[PATH_MAX];
    snprintf(buf, sizeof buf, "%s/%s", raiz, rel);
    char *r = solar_strdup(buf);
    if (en_pozo < SOLAR_LEN(pozo)) pozo[en_pozo++] = r;
    return r;
}

static void escribir(const char *ruta, const char *texto)
{
    FILE *f = fopen(ruta, "w");
    if (!f) { perror(ruta); exit(2); }
    fputs(texto, f);
    fclose(f);
}

static long tamano_de(const char *ruta)
{
    struct stat st;
    return stat(ruta, &st) == 0 ? (long)st.st_size : -1;
}

static const SolarEntrada *buscar(const SolarListado *l, const char *nombre)
{
    for (size_t i = 0; i < l->cantidad; i++)
        if (!strcmp(l->entradas[i].nombre, nombre)) return &l->entradas[i];
    return NULL;
}

static int contar_progreso(void *datos, uint64_t bh, uint64_t bt, uint64_t eh, uint64_t et, const char *a)
{
    (void)bh; (void)bt; (void)et; (void)a;
    *(uint64_t *)datos = eh;
    return 0;
}

static int cancelar_siempre(void *datos, uint64_t bh, uint64_t bt, uint64_t eh, uint64_t et, const char *a)
{
    (void)datos; (void)bh; (void)bt; (void)eh; (void)et; (void)a;
    return 1;
}

static char ultimo_destino[PATH_MAX];
static void guardar_destino(void *datos, const char *o, const char *d)
{
    (void)datos; (void)o;
    copiar_texto(ultimo_destino, sizeof ultimo_destino, d);
}

/* ------------------------------------------------------------------ */

static void prueba_natural(void)
{
    SECCION("orden natural");
    REVISAR(solar_comparar_natural("foto2", "foto10") < 0);
    REVISAR(solar_comparar_natural("foto10", "foto2") > 0);
    REVISAR(solar_comparar_natural("Árbol", "arbol") == 0);
    REVISAR(solar_comparar_natural("nube", "ñandú") < 0);
    REVISAR(solar_comparar_natural("ñandú", "oso") < 0);
    REVISAR(solar_comparar_natural("a", "B") < 0);
    REVISAR(solar_comparar_natural("capítulo 007", "capítulo 7") == 0);
    REVISAR(solar_comparar_natural("x", "x1") < 0);
    /* Camino rápido ASCII: mayúsculas, números a mitad del nombre, signos */
    REVISAR(solar_comparar_natural("Zeta", "alfa") > 0);
    REVISAR(solar_comparar_natural("archivo_10.txt", "Archivo_9.txt") > 0);
    REVISAR(solar_comparar_natural("a19", "a1x") > 0);
    REVISAR(solar_comparar_natural("Informe-Final", "informe-final") == 0);
    REVISAR(solar_comparar_natural("anz", "año") < 0);
    REVISAR(solar_comparar_natural("cafe", "Café") == 0);
}

static void prueba_tipos(void)
{
    SECCION("tipos de archivo");
    char d[128];
    REVISAR(solar_categoria("foto.PNG", 0644, SOLAR_TIPO_ARCHIVO) == SOLAR_CAT_IMAGEN);
    solar_describir_tipo("foto.PNG", SOLAR_CAT_IMAGEN, SOLAR_TIPO_ARCHIVO, 0, d, sizeof d);
    REVISAR(!strcmp(d, "Imagen PNG"));
    solar_describir_tipo("respaldo.tar.gz", SOLAR_CAT_COMPRIMIDO, SOLAR_TIPO_ARCHIVO, 0, d, sizeof d);
    REVISAR(!strcmp(d, "Archivo TAR comprimido (gz)"));
    REVISAR(solar_categoria("Makefile", 0644, SOLAR_TIPO_ARCHIVO) == SOLAR_CAT_CODIGO);
    REVISAR(solar_categoria("instalar", 0755, SOLAR_TIPO_ARCHIVO) == SOLAR_CAT_EJECUTABLE);
    REVISAR(solar_categoria("main.rs", 0644, SOLAR_TIPO_ARCHIVO) == SOLAR_CAT_CODIGO);
    REVISAR(solar_categoria("yay-12.pkg.tar.zst", 0644, SOLAR_TIPO_ARCHIVO) == SOLAR_CAT_PAQUETE);
    solar_describir_tipo("raro.xyz", SOLAR_CAT_GENERICO, SOLAR_TIPO_ARCHIVO, 0, d, sizeof d);
    REVISAR(!strcmp(d, "Archivo XYZ"));
    solar_describir_tipo("Juegos", SOLAR_CAT_CARPETA, SOLAR_TIPO_CARPETA, 0, d, sizeof d);
    REVISAR(!strcmp(d, "Carpeta"));
}

static void prueba_listado(void)
{
    SECCION("listado y orden");
    mkdir(en_raiz("lista"), 0755);
    mkdir(en_raiz("lista/Juegos"), 0755);
    escribir(en_raiz("lista/b10.txt"), "1234567890");
    escribir(en_raiz("lista/b2.txt"), "12");
    escribir(en_raiz("lista/.oculto"), "x");
    escribir(en_raiz("lista/respaldo~"), "x");
    escribir(en_raiz("lista/secreto.txt"), "x");
    escribir(en_raiz("lista/.hidden"), "secreto.txt\n");
    REVISAR(symlink("b2.txt", en_raiz("lista/enlace")) == 0);
    REVISAR(symlink("no-existe", en_raiz("lista/roto")) == 0);

    SolarListado l;
    REVISAR(solar_listar(en_raiz("lista"), &l) == 0);
    REVISAR(l.cantidad == 9);
    const SolarEntrada *e = buscar(&l, "Juegos");
    REVISAR(e && e->tipo == SOLAR_TIPO_CARPETA && e->categoria == SOLAR_CAT_CARPETA);
    e = buscar(&l, ".oculto");
    REVISAR(e && e->es_oculto);
    e = buscar(&l, "respaldo~");
    REVISAR(e && e->es_oculto);
    e = buscar(&l, "secreto.txt");
    REVISAR(e && e->es_oculto);
    e = buscar(&l, "enlace");
    REVISAR(e && e->es_enlace && e->tipo == SOLAR_TIPO_ARCHIVO && e->tamano == 2);
    e = buscar(&l, "roto");
    REVISAR(e && e->tipo == SOLAR_TIPO_ENLACE_ROTO);

    solar_ordenar(&l, SOLAR_ORDEN_NOMBRE, 0, 1);
    REVISAR(!strcmp(l.entradas[0].nombre, "Juegos"));
    size_t i2 = 0, i10 = 0;
    for (size_t i = 0; i < l.cantidad; i++) {
        if (!strcmp(l.entradas[i].nombre, "b2.txt")) i2 = i;
        if (!strcmp(l.entradas[i].nombre, "b10.txt")) i10 = i;
    }
    REVISAR(i2 < i10);
    solar_ordenar(&l, SOLAR_ORDEN_TAMANO, 1, 1);
    REVISAR(!strcmp(l.entradas[0].nombre, "Juegos"));
    size_t j2 = 0, j10 = 0;
    for (size_t i = 0; i < l.cantidad; i++) {
        if (!strcmp(l.entradas[i].nombre, "b2.txt")) j2 = i;
        if (!strcmp(l.entradas[i].nombre, "b10.txt")) j10 = i;
    }
    REVISAR(j10 < j2);
    solar_listado_liberar(&l);

    SolarListado vacio;
    REVISAR(solar_listar(en_raiz("no-existe"), &vacio) == ENOENT);
    solar_listado_liberar(&vacio);

    SolarInfo info;
    REVISAR(solar_info(en_raiz("lista/b10.txt"), &info) == 0);
    REVISAR(info.tamano == 10 && info.tipo == SOLAR_TIPO_ARCHIVO && info.categoria == SOLAR_CAT_TEXTO);
    REVISAR(info.propietario[0] != '\0');
    REVISAR(solar_info(en_raiz("lista/enlace"), &info) == 0 && info.es_enlace && !strcmp(info.destino_enlace, "b2.txt"));
    REVISAR(solar_cambiar_permisos(en_raiz("lista/b2.txt"), 0600) == 0);
    REVISAR(solar_info(en_raiz("lista/b2.txt"), &info) == 0 && (info.modo & 0777) == 0600);

    uint64_t bytes = 0, archivos = 0, carpetas = 0;
    REVISAR(solar_tamano_total(en_raiz("lista"), &bytes, &archivos, &carpetas, NULL, NULL) == 0);
    REVISAR(carpetas == 1 && bytes >= 12);
}

static void prueba_nombres(void)
{
    SECCION("nombres libres y validación");
    mkdir(en_raiz("nombres"), 0755);
    escribir(en_raiz("nombres/foto.png"), "x");
    escribir(en_raiz("nombres/foto (copia).png"), "x");
    escribir(en_raiz("nombres/a.tar.gz"), "x");
    mkdir(en_raiz("nombres/Juegos"), 0755);
    char n[512];
    REVISAR(solar_nombre_libre(en_raiz("nombres"), "foto.png", 0, n, sizeof n) == 0 && !strcmp(n, "foto (2).png"));
    REVISAR(solar_nombre_libre(en_raiz("nombres"), "foto.png", 1, n, sizeof n) == 0 && !strcmp(n, "foto (copia 2).png"));
    REVISAR(solar_nombre_libre(en_raiz("nombres"), "foto (copia).png", 1, n, sizeof n) == 0 && !strcmp(n, "foto (copia 2).png"));
    REVISAR(solar_nombre_libre(en_raiz("nombres"), "a.tar.gz", 0, n, sizeof n) == 0 && !strcmp(n, "a (2).tar.gz"));
    REVISAR(solar_nombre_libre(en_raiz("nombres"), "Juegos", 0, n, sizeof n) == 0 && !strcmp(n, "Juegos (2)"));
    REVISAR(solar_nombre_libre(en_raiz("nombres"), "nuevo.txt", 0, n, sizeof n) == 0 && !strcmp(n, "nuevo.txt"));
    REVISAR(solar_validar_nombre("") == EINVAL);
    REVISAR(solar_validar_nombre("..") == EINVAL);
    REVISAR(solar_validar_nombre("a/b") == EINVAL);
    REVISAR(solar_validar_nombre("Mis cosas ☀") == 0);

    REVISAR(solar_crear_carpeta(en_raiz("nombres"), "Nueva") == 0);
    REVISAR(solar_crear_carpeta(en_raiz("nombres"), "Nueva") == EEXIST);
    REVISAR(solar_crear_archivo(en_raiz("nombres"), "vacío.txt") == 0);
    REVISAR(tamano_de(en_raiz("nombres/vacío.txt")) == 0);
    REVISAR(solar_renombrar(en_raiz("nombres/vacío.txt"), "foto.png") == EEXIST);
    REVISAR(solar_renombrar(en_raiz("nombres/vacío.txt"), "listo.txt") == 0);
    REVISAR(existe(en_raiz("nombres/listo.txt")) && !existe(en_raiz("nombres/vacío.txt")));
}

static void prueba_copiar_mover(void)
{
    SECCION("copiar, mover y eliminar");
    mkdir(en_raiz("origen"), 0755);
    mkdir(en_raiz("origen/Juegos"), 0755);
    mkdir(en_raiz("origen/Juegos/Retro"), 0755);
    escribir(en_raiz("origen/Juegos/Retro/rom.bin"), "0123456789ABCDEF");
    escribir(en_raiz("origen/Juegos/leeme.txt"), "hola");
    REVISAR(symlink("leeme.txt", en_raiz("origen/Juegos/acceso")) == 0);
    mkdir(en_raiz("destino"), 0755);

    char error[512];
    uint64_t hechos = 0;
    SolarOpciones op = { SOLAR_CONFLICTO_RENOMBRAR, contar_progreso, guardar_destino, &hechos, error, sizeof error };
    const char *uno[] = { en_raiz("origen/Juegos") };
    REVISAR(solar_copiar(uno, 1, en_raiz("destino"), &op) == 0);
    REVISAR(tamano_de(en_raiz("destino/Juegos/Retro/rom.bin")) == 16);
    REVISAR(tamano_de(en_raiz("destino/Juegos/leeme.txt")) == 4);
    char enlace[64] = "";
    REVISAR(readlink(en_raiz("destino/Juegos/acceso"), enlace, sizeof enlace - 1) == 9);
    REVISAR(hechos == 5);
    REVISAR(!strcmp(ultimo_destino, en_raiz("destino/Juegos")));

    /* Otra vez: conflicto -> "Juegos (2)" */
    REVISAR(solar_copiar(uno, 1, en_raiz("destino"), &op) == 0);
    REVISAR(es_carpeta(en_raiz("destino/Juegos (2)")));
    /* Duplicar en la misma carpeta -> "(copia)" */
    const char *dup[] = { en_raiz("destino/Juegos") };
    REVISAR(solar_copiar(dup, 1, en_raiz("destino"), &op) == 0);
    REVISAR(es_carpeta(en_raiz("destino/Juegos (copia)")));
    /* Dentro de sí misma -> error */
    REVISAR(solar_copiar(dup, 1, en_raiz("destino/Juegos/Retro"), &op) == EINVAL);
    REVISAR(strstr(error, "dentro de sí misma") != NULL);
    /* Omitir */
    op.conflicto = SOLAR_CONFLICTO_OMITIR;
    escribir(en_raiz("origen/Juegos/leeme.txt"), "contenido nuevo");
    REVISAR(solar_copiar(uno, 1, en_raiz("destino"), &op) == 0);
    REVISAR(tamano_de(en_raiz("destino/Juegos/leeme.txt")) == 4);
    /* Reemplazar combina carpetas */
    op.conflicto = SOLAR_CONFLICTO_REEMPLAZAR;
    escribir(en_raiz("destino/Juegos/extra.txt"), "se queda");
    int rr = solar_copiar(uno, 1, en_raiz("destino"), &op);
    if (rr) fprintf(stderr, "    reemplazar: %d %s\n", rr, error);
    REVISAR(rr == 0);
    REVISAR(tamano_de(en_raiz("destino/Juegos/leeme.txt")) == 15);
    REVISAR(existe(en_raiz("destino/Juegos/extra.txt")));
    /* Cancelar */
    SolarOpciones cancelar = { SOLAR_CONFLICTO_RENOMBRAR, cancelar_siempre, NULL, NULL, error, sizeof error };
    rr = solar_copiar(uno, 1, en_raiz("destino"), &cancelar);
    if (rr != ECANCELED) fprintf(stderr, "    cancelar: %d %s\n", rr, error);
    REVISAR(rr == ECANCELED);

    /* Mover */
    mkdir(en_raiz("movidos"), 0755);
    const char *mov[] = { en_raiz("destino/Juegos (2)") };
    op.conflicto = SOLAR_CONFLICTO_RENOMBRAR;
    REVISAR(solar_mover(mov, 1, en_raiz("movidos"), &op) == 0);
    REVISAR(!existe(en_raiz("destino/Juegos (2)")) && es_carpeta(en_raiz("movidos/Juegos (2)")));
    REVISAR(!strcmp(ultimo_destino, en_raiz("movidos/Juegos (2)")));
    const char *mov2[] = { en_raiz("destino/Juegos (copia)") };
    REVISAR(solar_mover(mov2, 1, en_raiz("destino/Juegos (copia)/Retro"), &op) == EINVAL);

    /* Mover reemplazando (combinar) */
    mkdir(en_raiz("movidos/Juegos"), 0755);
    escribir(en_raiz("movidos/Juegos/viejo.txt"), "viejo");
    const char *mov3[] = { en_raiz("destino/Juegos") };
    op.conflicto = SOLAR_CONFLICTO_REEMPLAZAR;
    REVISAR(solar_mover(mov3, 1, en_raiz("movidos"), &op) == 0);
    REVISAR(existe(en_raiz("movidos/Juegos/viejo.txt")) && existe(en_raiz("movidos/Juegos/Retro/rom.bin")));
    REVISAR(!existe(en_raiz("destino/Juegos")));

    /* Eliminar con progreso, incluyendo una carpeta sin permisos */
    chmod(en_raiz("movidos/Juegos/Retro"), 0500);
    hechos = 0;
    const char *borrar[] = { en_raiz("movidos") };
    SolarOpciones opb = { 0, contar_progreso, NULL, &hechos, error, sizeof error };
    REVISAR(solar_eliminar(borrar, 1, &opb) == 0);
    REVISAR(!existe(en_raiz("movidos")));
    REVISAR(hechos > 5);
}

static void prueba_papelera(void)
{
    SECCION("papelera");
    escribir(en_raiz("tirar.txt"), "adiós");
    mkdir(en_raiz("tirar dir"), 0755);
    escribir(en_raiz("tirar dir/a.txt"), "a");
    char error[512];
    SolarOpciones op = { 0, NULL, guardar_destino, NULL, error, sizeof error };
    const char *rutas[] = { en_raiz("tirar.txt"), en_raiz("tirar dir") };
    REVISAR(solar_papelera_vacia() == 1);
    REVISAR(solar_papelera_enviar(rutas, 2, &op) == 0);
    REVISAR(!existe(en_raiz("tirar.txt")));
    REVISAR(solar_papelera_vacia() == 0);

    SolarPapelera p;
    REVISAR(solar_papelera_listar(&p) == 0);
    REVISAR(p.cantidad == 2);
    const SolarElementoPapelera *txt = NULL;
    for (size_t i = 0; i < p.cantidad; i++)
        if (!strcmp(p.elementos[i].nombre, "tirar.txt")) txt = &p.elementos[i];
    REVISAR(txt && !strcmp(txt->ruta_original, en_raiz("tirar.txt")) && txt->fecha > 0 && txt->tamano == 6);

    /* Mismo nombre otra vez: debe convivir */
    escribir(en_raiz("tirar.txt"), "otra vez");
    const char *otra[] = { en_raiz("tirar.txt") };
    REVISAR(solar_papelera_enviar(otra, 1, &op) == 0);

    if (txt) {
        char restaurado[PATH_MAX];
        escribir(en_raiz("tirar.txt"), "ocupa el lugar");
        REVISAR(solar_papelera_restaurar(txt->ruta, restaurado, sizeof restaurado, error, sizeof error) == 0);
        REVISAR(!strcmp(restaurado, en_raiz("tirar (2).txt")));
        REVISAR(tamano_de(en_raiz("tirar (2).txt")) == 6);
    }
    solar_papelera_liberar(&p);

    REVISAR(solar_papelera_listar(&p) == 0 && p.cantidad == 2);
    if (p.cantidad) REVISAR(solar_papelera_borrar(p.elementos[0].ruta) == 0);
    solar_papelera_liberar(&p);
    REVISAR(solar_papelera_vaciar(NULL) == 0);
    REVISAR(solar_papelera_vacia() == 1);
    REVISAR(solar_papelera_listar(&p) == 0 && p.cantidad == 0);
    solar_papelera_liberar(&p);
}

static void prueba_carpetas(void)
{
    SECCION("carpetas personalizadas");
    char archivo[PATH_MAX];
    snprintf(archivo, sizeof archivo, "%s/datos/carpetas.tsv", raiz);
    SolarCarpetas *db = solar_carpetas_abrir(archivo);
    REVISAR(db != NULL);
    SolarEstiloCarpeta e;
    memset(&e, 0, sizeof e);
    e.tiene_color = 1;
    e.color = 0xF97316;
    e.tiene_color_icono = 1;
    e.color_icono = 0x14532D;
    strcpy(e.icono, "mando");
    REVISAR(solar_carpetas_poner(db, "/home/capy/Juegos", &e) == 0);
    strcpy(e.icono, "musica");
    e.tiene_color_icono = 0;
    REVISAR(solar_carpetas_poner(db, "/home/capy/Juegos/Bandas sonoras", &e) == 0);
    strcpy(e.icono, "txt:★\tcon tab");
    REVISAR(solar_carpetas_poner(db, "/home/capy/Raro\tnombre", &e) == 0);
    REVISAR(solar_carpetas_cantidad(db) == 3);

    SolarEstiloCarpeta leido;
    REVISAR(solar_carpetas_obtener(db, "/home/capy/Juegos/", &leido) == 1);
    REVISAR(!strcmp(leido.icono, "mando") && leido.color == 0xF97316 && leido.color_icono == 0x14532D);
    REVISAR(solar_carpetas_obtener(db, "/home/capy/Otra", &leido) == 0);

    REVISAR(solar_carpetas_renombrar(db, "/home/capy/Juegos", "/home/capy/Videojuegos") == 0);
    REVISAR(solar_carpetas_obtener(db, "/home/capy/Videojuegos/Bandas sonoras", &leido) == 1 &&
            !strcmp(leido.icono, "musica"));
    REVISAR(solar_carpetas_obtener(db, "/home/capy/Juegos", &leido) == 0);
    REVISAR(solar_carpetas_copiar(db, "/home/capy/Videojuegos", "/home/capy/Videojuegos (copia)") == 0);
    REVISAR(solar_carpetas_cantidad(db) == 5);
    solar_carpetas_cerrar(db);

    db = solar_carpetas_abrir(archivo);
    REVISAR(solar_carpetas_cantidad(db) == 5);
    REVISAR(solar_carpetas_obtener(db, "/home/capy/Raro\tnombre", &leido) == 1 && !strcmp(leido.icono, "txt:★\tcon tab"));
    REVISAR(solar_carpetas_olvidar_arbol(db, "/home/capy/Videojuegos") == 0);
    REVISAR(solar_carpetas_cantidad(db) == 3);
    REVISAR(solar_carpetas_quitar(db, "/home/capy/Raro\tnombre") == 0);
    REVISAR(solar_carpetas_cantidad(db) == 2);
    solar_carpetas_cerrar(db);
}

static void prueba_config(void)
{
    SECCION("configuración en Lua");
    char archivo[PATH_MAX], avisos[2048];
    snprintf(archivo, sizeof archivo, "%s/config/solar/config.lua", raiz);

    SolarConfig c;
    solar_config_defecto(&c);
    REVISAR(!strcmp(c.tema, "sol") && c.n_atajos > 20 && c.n_lugares >= 1);
    REVISAR(c.sonidos == 1 && c.volumen > 0.5 && c.animaciones == 1 && !strcmp(c.repo_actualizaciones, "CapyIsInsano/Solar"));
    REVISAR(!strcmp(c.lugares[0].ruta, "~"));
    REVISAR(solar_config_cargar(archivo, &c, avisos, sizeof avisos) == ENOENT);

    free(c.tema);
    c.tema = solar_strdup("eclipse");
    c.transparencia = 1;
    c.opacidad = 0.7;
    c.barra_ubicacion = SOLAR_POS_ABAJO;
    c.vista = SOLAR_VISTA_LISTA;
    c.tamano_iconos = 96;
    c.sonidos = 0;
    c.volumen = 0.25;
    c.animaciones = 0;
    free(c.repo_actualizaciones);
    c.repo_actualizaciones = solar_strdup("capy/prueba");
    free(c.atajos[0].teclas);
    c.atajos[0].teclas = solar_strdup("Ctrl+Shift+\"raro\"");
    REVISAR(solar_config_guardar(archivo, &c) == 0);
    solar_config_liberar(&c);

    solar_config_defecto(&c);
    REVISAR(solar_config_cargar(archivo, &c, avisos, sizeof avisos) == 0);
    REVISAR(avisos[0] == '\0');
    if (avisos[0]) fprintf(stderr, "    avisos: %s\n", avisos);
    REVISAR(!strcmp(c.tema, "eclipse") && c.transparencia == 1 && c.opacidad > 0.69 && c.opacidad < 0.71);
    REVISAR(c.barra_ubicacion == SOLAR_POS_ABAJO && c.vista == SOLAR_VISTA_LISTA && c.tamano_iconos == 96);
    REVISAR(!strcmp(c.atajos[0].teclas, "Ctrl+Shift+\"raro\""));
    REVISAR(c.sonidos == 0 && c.volumen > 0.24 && c.volumen < 0.26 && c.animaciones == 0);
    REVISAR(!strcmp(c.repo_actualizaciones, "capy/prueba"));
    solar_config_liberar(&c);

    /* Valores raros: avisos, pero se carga lo demás */
    escribir(archivo,
             "local azul = 'oceano'\n"
             "return { tema = azul, opacidad = 7, vista = 'mosaico', transparencia = 'si',\n"
             "  lugares = { '~/Proyectos', { nombre = 'Juegos', ruta = '~/Juegos', icono = 'mando' } },\n"
             "  atajos = { copiar = 'Ctrl+Insert', volar = 'F1', buscar = false }, tipo_raro = 1 }\n");
    solar_config_defecto(&c);
    REVISAR(solar_config_cargar(archivo, &c, avisos, sizeof avisos) == 0);
    REVISAR(!strcmp(c.tema, "neptuno") && c.opacidad == 0.85 && c.vista == SOLAR_VISTA_CUADRICULA);
    REVISAR(strstr(avisos, "opacidad") && strstr(avisos, "mosaico") && strstr(avisos, "volar") && strstr(avisos, "tipo_raro"));
    REVISAR(c.n_lugares == 2 && !strcmp(c.lugares[0].nombre, "Proyectos") && !strcmp(c.lugares[1].icono, "mando"));
    int encontrado = 0;
    for (size_t i = 0; i < c.n_atajos; i++) {
        if (!strcmp(c.atajos[i].accion, "copiar")) encontrado += !strcmp(c.atajos[i].teclas, "Ctrl+Insert");
        if (!strcmp(c.atajos[i].accion, "buscar")) encontrado += !strcmp(c.atajos[i].teclas, "");
    }
    REVISAR(encontrado == 2);
    solar_config_liberar(&c);

    /* Estilo "variables globales" */
    escribir(archivo, "tema = 'luna'\nescala = 1.25\n");
    solar_config_defecto(&c);
    REVISAR(solar_config_cargar(archivo, &c, avisos, sizeof avisos) == 0);
    REVISAR(!strcmp(c.tema, "luna") && c.escala == 1.25 && avisos[0] == '\0');
    solar_config_liberar(&c);

    /* Errores de sintaxis, bucles infinitos, memoria y cosas prohibidas */
    const char *malos[] = {
        "return { tema = 'solar' ",
        "while true do end",
        "local t = {} for i = 1, 1e9 do t[i] = ('x'):rep(1000) end",
        "os.execute('touch /tmp/solar-no-deberia')",
        "io.open('/etc/passwd')",
        "load('return 1')()",
    };
    for (size_t i = 0; i < SOLAR_LEN(malos); i++) {
        escribir(archivo, malos[i]);
        solar_config_defecto(&c);
        int r = solar_config_cargar(archivo, &c, avisos, sizeof avisos);
        REVISAR(r == EINVAL && avisos[0] != '\0' && !strcmp(c.tema, "sol"));
        solar_config_liberar(&c);
    }
}

static void prueba_temas(void)
{
    SECCION("temas");
    char dir[PATH_MAX], avisos[2048];
    snprintf(dir, sizeof dir, "%s/config/solar/temas", raiz);
    crear_dirs(dir, 0755);
    SolarTemas t;
    REVISAR(solar_temas_cargar(NULL, &t, avisos, sizeof avisos) == 0 && t.cantidad == 9);
    REVISAR(!strcmp(t.temas[0].id, "sol") && t.temas[0].colores[SOLAR_COLOR_ACENTO] == 0xF97316);
    {
        static const char *const NOMBRES[] = { "Sol", "Aurora", "Eclipse", "Nebulosa", "Venus",
                                               "Neptuno", "Marte", "Mercurio", "Luna" };
        int bien = 1;
        for (size_t i = 0; i < 9; i++) bien &= !strcmp(t.temas[i].nombre, NOMBRES[i]);
        REVISAR(bien);
        REVISAR(!strcmp(solar_tema_id_actual("amanecer"), "aurora") && !strcmp(solar_tema_id_actual("grafito"), "mercurio"));
        REVISAR(!strcmp(solar_tema_id_actual("venus"), "venus") && !strcmp(solar_tema_id_actual("solar"), "sol"));
    }
    solar_temas_liberar(&t);

    char archivo[PATH_MAX + 32];
    snprintf(archivo, sizeof archivo, "%s/verde.lua", dir);
    escribir(archivo, "return { nombre = 'Verde', base = 'eclipse', colores = { acento = '#22C55E', borde = '#333' } }");
    snprintf(archivo, sizeof archivo, "%s/roto.lua", dir);
    escribir(archivo, "return { colores = { acento = 'verde' } }");
    snprintf(archivo, sizeof archivo, "%s/mal nombre.lua", dir);
    escribir(archivo, "return {}");
    REVISAR(solar_temas_cargar(dir, &t, avisos, sizeof avisos) == 0);
    REVISAR(t.cantidad == 11);
    const SolarTema *verde = NULL;
    for (size_t i = 0; i < t.cantidad; i++)
        if (!strcmp(t.temas[i].id, "verde")) verde = &t.temas[i];
    REVISAR(verde && !strcmp(verde->nombre, "Verde") && verde->oscuro && !verde->integrado);
    REVISAR(verde && verde->colores[SOLAR_COLOR_ACENTO] == 0x22C55E && verde->colores[SOLAR_COLOR_BORDE] == 0x333333);
    REVISAR(verde && verde->colores[SOLAR_COLOR_SELECCION] == 0x22C55E && verde->colores[SOLAR_COLOR_CARPETA] == 0x22C55E);
    REVISAR(verde && verde->colores[SOLAR_COLOR_FONDO] == 0x000000);
    REVISAR(strstr(avisos, "acento") && strstr(avisos, "mal nombre"));

    if (verde) {
        snprintf(archivo, sizeof archivo, "%s/copia.lua", dir);
        REVISAR(solar_tema_guardar(archivo, verde) == 0);
    }
    solar_temas_liberar(&t);
    REVISAR(solar_temas_cargar(dir, &t, avisos, sizeof avisos) == 0 && t.cantidad == 12);
    for (size_t i = 0; i < t.cantidad; i++)
        if (!strcmp(t.temas[i].id, "copia")) REVISAR(t.temas[i].colores[SOLAR_COLOR_ACENTO] == 0x22C55E);
    solar_temas_liberar(&t);
}

static void prueba_procesos(void)
{
    SECCION("procesos y comandos");
    char **argv = NULL;
    size_t n = 0;
    REVISAR(dividir_comando("kitty --hold 'a b' \"c \\\"d\\\"\" e\\ f", &argv, &n) == 0);
    REVISAR(n == 5 && !strcmp(argv[2], "a b") && !strcmp(argv[3], "c \"d\"") && !strcmp(argv[4], "e f"));
    liberar_argv(argv, n);
    REVISAR(dividir_comando("mal 'cerrado", &argv, &n) == EINVAL);

    char salida[256];
    const char *eco[] = { "sh", "-c", "echo hola; echo error >&2", NULL };
    REVISAR(ejecutar_y_esperar(eco, salida, sizeof salida, 5000) == 0);
    REVISAR(strstr(salida, "hola") && strstr(salida, "error"));
    const char *falla[] = { "sh", "-c", "exit 3", NULL };
    REVISAR(ejecutar_y_esperar(falla, salida, sizeof salida, 5000) == 3);
    const char *lento[] = { "sleep", "5", NULL };
    REVISAR(ejecutar_y_esperar(lento, salida, sizeof salida, 200) == -ETIMEDOUT);
    const char *nada[] = { "programa-que-no-existe-solar", NULL };
    REVISAR(solar_lanzar(nada, NULL) == ENOENT);
    const char *ok[] = { "true", NULL };
    REVISAR(solar_lanzar(ok, raiz) == 0);
}

static void prueba_dispositivos_vigilante(void)
{
    SECCION("dispositivos y vigilante");
    SolarDispositivos d;
    REVISAR(solar_dispositivos_listar(1, &d) == 0);
    printf("    (%zu dispositivos visibles en este sistema)\n", d.cantidad);
    for (size_t i = 0; i < d.cantidad; i++) REVISAR(d.dispositivos[i].nombre && d.dispositivos[i].dispositivo);
    solar_dispositivos_liberar(&d);
    SolarEspacio e;
    REVISAR(solar_espacio(raiz, &e) == 0 && e.total > 0);

    SolarVigilante *v = solar_vigilante_crear();
    REVISAR(v != NULL);
    mkdir(en_raiz("vigilada"), 0755);
    REVISAR(solar_vigilante_observar(v, en_raiz("vigilada")) == 0);
    REVISAR(solar_vigilante_esperar(v, 50) == 0);
    escribir(en_raiz("vigilada/nuevo.txt"), "x");
    int cambios = solar_vigilante_esperar(v, 1000);
    REVISAR(cambios > 0 && (cambios & SOLAR_CAMBIO_CARPETA));
    REVISAR(solar_vigilante_observar(v, NULL) == 0);
    /* Despertar desde otro lado: la espera larga termina enseguida */
    solar_vigilante_despertar(v);
    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    cambios = solar_vigilante_esperar(v, 5000);
    clock_gettime(CLOCK_MONOTONIC, &t1);
    double ms = (double)(t1.tv_sec - t0.tv_sec) * 1000 + (double)(t1.tv_nsec - t0.tv_nsec) / 1e6;
    REVISAR(cambios > 0 && (cambios & SOLAR_CAMBIO_DESPERTADO) && ms < 1000);
    /* Y ya no queda despierta: la siguiente espera corta no ve nada */
    REVISAR(solar_vigilante_esperar(v, 30) == 0);
    solar_vigilante_destruir(v);
}


static int esperar_archivo(const char *ruta, int ms)
{
    for (int i = 0; i < ms / 20; i++) {
        if (tamano_de(ruta) > 0) return 1;
        usleep(20000);
    }
    return 0;
}

static void prueba_programas(void)
{
    SECCION("tipos de archivo y programas (Abrir con)");
    /* Una "distro" falsa en raiz/sistema: base MIME y lanzadores */
    mkdir(en_raiz("sistema"), 0755);
    mkdir(en_raiz("sistema/mime"), 0755);
    mkdir(en_raiz("sistema/applications"), 0755);
    mkdir(en_raiz("sistema/applications/kde4"), 0755);
    escribir(en_raiz("sistema/mime/globs2"),
             "# comentario\n50:text/x-c++src:*.cpp\n50:text/x-csrc:*.c\n50:text/plain:*.txt\n"
             "50:application/x-tar:*.tar\n50:application/x-compressed-tar:*.tar.gz\n"
             "50:image/png:*.png\n10:text/x-makefile:Makefile:cs\n");
    escribir(en_raiz("sistema/mime/subclasses"), "text/x-c++src text/x-csrc\ntext/x-csrc text/plain\n");
    escribir(en_raiz("sistema/mime/aliases"), "text/x-c text/x-csrc\n");
    escribir(en_raiz("sistema/applications/nvim.desktop"),
             "[Desktop Entry]\nType=Application\nName=Neovim\nGenericName=Text Editor\nGenericName[es]=Editor de texto\n"
             "Exec=nvim %F\nTerminal=true\nIcon=nvim\nMimeType=text/plain;text/x-c++src;\n");
    escribir(en_raiz("sistema/applications/codium.desktop"),
             "[Desktop Entry]\nType=Application\nName=VSCodium\nExec=codium --unity-launch %F\nMimeType=text/plain;\n"
             "[Desktop Action new-window]\nName=Ventana nueva\nExec=codium --new-window %F\n");
    escribir(en_raiz("sistema/applications/code.desktop"),
             "[Desktop Entry]\nType=Application\nName=Visual Studio Code\nName[es_MX]=VS Code\nExec=code %F\n"
             "MimeType=text/plain;inode/directory;\n");
    escribir(en_raiz("sistema/applications/gimp.desktop"),
             "[Desktop Entry]\nType=Application\nName=GIMP\nExec=gimp %U\nMimeType=image/png;\n");
    escribir(en_raiz("sistema/applications/kde4/kate.desktop"),
             "[Desktop Entry]\nType=Application\nName=Kate\nExec=kate %U\nMimeType=text/x-csrc;\n");
    escribir(en_raiz("sistema/applications/fantasma.desktop"),
             "[Desktop Entry]\nType=Application\nName=Fantasma\nExec=x %F\nTryExec=no-existe-solar-xyz\nMimeType=text/plain;\n");
    escribir(en_raiz("sistema/applications/borrado.desktop"),
             "[Desktop Entry]\nType=Application\nName=Borrado\nExec=x %F\nHidden=true\nMimeType=text/plain;\n");
    escribir(en_raiz("sistema/applications/enlace.desktop"),
             "[Desktop Entry]\nType=Link\nName=Enlace\nURL=https://example.com\n");
    char copia[PATH_MAX];
    snprintf(copia, sizeof copia, "%s/copiado.txt", raiz);
    char lanzador[PATH_MAX * 2];
    snprintf(lanzador, sizeof lanzador,
             "[Desktop Entry]\nType=Application\nName=Copiador\nExec=cp %%f \"%s\"\nMimeType=text/x-c++src;\n", copia);
    escribir(en_raiz("sistema/applications/copiador.desktop"), lanzador);

    char *cpp = en_raiz("hola.cpp"), *tgz = en_raiz("cosa.tar.gz"), *raro = en_raiz("LEEME");
    char *bin = en_raiz("datos.bin"), *mk = en_raiz("Makefile"), *png = en_raiz("foto.png");
    escribir(cpp, "int main() {}\n");
    escribir(tgz, "x");
    escribir(raro, "solo texto\n");
    escribir(mk, "all:\n");
    escribir(png, "x");
    int fd = open(bin, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd >= 0) { (void)!write(fd, "\x01\x00\x02\x00", 4); close(fd); }

    char mime[128];
    REVISAR(solar_mime_de(cpp, mime, sizeof mime) == 0 && !strcmp(mime, "text/x-c++src"));
    REVISAR(solar_mime_de(tgz, mime, sizeof mime) == 0 && !strcmp(mime, "application/x-compressed-tar"));
    REVISAR(solar_mime_de(raro, mime, sizeof mime) == 0 && !strcmp(mime, "text/plain"));
    REVISAR(solar_mime_de(bin, mime, sizeof mime) == 0 && !strcmp(mime, "application/octet-stream"));
    REVISAR(solar_mime_de(mk, mime, sizeof mime) == 0 && !strcmp(mime, "text/x-makefile"));
    REVISAR(solar_mime_de(raiz, mime, sizeof mime) == 0 && !strcmp(mime, "inode/directory"));

    /* Para un .cpp: exacto primero (Copiador, Neovim), luego los padres */
    SolarProgramas ps;
    const char *uno[] = { cpp };
    REVISAR(solar_programas_para(uno, 1, 0, &ps) == 0);
    REVISAR(ps.mime && !strcmp(ps.mime, "text/x-c++src"));
    REVISAR(ps.cantidad == 5);
    if (ps.cantidad == 5) {
        REVISAR(!strcmp(ps.programas[0].nombre, "Copiador") && ps.programas[0].afinidad == SOLAR_AFINIDAD_EXACTA);
        REVISAR(!strcmp(ps.programas[1].nombre, "Neovim") && ps.programas[1].terminal == 1);
        REVISAR(ps.programas[1].descripcion && !strcmp(ps.programas[1].descripcion, "Editor de texto"));
        REVISAR(!strcmp(ps.programas[2].nombre, "Kate") && !strcmp(ps.programas[2].id, "kde4-kate.desktop"));
        REVISAR(ps.programas[2].afinidad == SOLAR_AFINIDAD_PADRE);
        REVISAR(!strcmp(ps.programas[3].nombre, "VS Code") && !strcmp(ps.programas[4].nombre, "VSCodium"));
    }
    solar_programas_liberar(&ps);

    /* Con todos: también GIMP, al final */
    REVISAR(solar_programas_para(uno, 1, 1, &ps) == 0);
    REVISAR(ps.cantidad == 6 && !strcmp(ps.programas[5].nombre, "GIMP") && ps.programas[5].afinidad == SOLAR_AFINIDAD_OTRO);
    solar_programas_liberar(&ps);

    /* Varios archivos: solo los que abren todos */
    const char *dos[] = { cpp, png };
    REVISAR(solar_programas_para(dos, 2, 0, &ps) == 0 && ps.cantidad == 0);
    solar_programas_liberar(&ps);

    /* Predeterminado */
    REVISAR(solar_programa_predeterminar("codium.desktop", "text/x-c++src") == 0);
    REVISAR(solar_programa_predeterminar("nvim.desktop", "text/plain") == 0);
    REVISAR(solar_programa_predeterminar("codium.desktop", "text/x-c++src") == 0);
    REVISAR(solar_programas_para(uno, 1, 0, &ps) == 0);
    REVISAR(ps.cantidad > 0 && !strcmp(ps.programas[0].id, "codium.desktop") && ps.programas[0].afinidad == SOLAR_AFINIDAD_DEFECTO);
    solar_programas_liberar(&ps);
    {
        char mimeapps[PATH_MAX + 32];
        snprintf(mimeapps, sizeof mimeapps, "%s/config/mimeapps.list", raiz);
        char *t = leer_archivo(mimeapps, 1 << 16, NULL);
        REVISAR(t && strstr(t, "[Default Applications]") && strstr(t, "text/x-c++src=codium.desktop;"));
        REVISAR(t && strstr(t, "text/plain=nvim.desktop;"));
        /* sin líneas repetidas */
        int veces = 0;
        for (const char *q = t; q && (q = strstr(q, "text/x-c++src=")); q++) veces++;
        REVISAR(veces == 2);   /* una en Default, otra en Added */
        free(t);
    }

    /* Lanzar sigue la línea Exec (%f con comillas) */
    REVISAR(solar_programas_para(uno, 1, 0, &ps) == 0);
    const SolarPrograma *cop = NULL;
    for (size_t i = 0; i < ps.cantidad; i++) if (!strcmp(ps.programas[i].id, "copiador.desktop")) cop = &ps.programas[i];
    REVISAR(cop && solar_programa_lanzar(cop, uno, 1, NULL) == 0);
    REVISAR(esperar_archivo(copia, 3000));
    solar_programas_liberar(&ps);
}

static void prueba_sonido(void)
{
    SECCION("efectos de sonido");
    size_t tam = 0;
    char *mp3 = leer_archivo("../recursos/sonidos/click.mp3", 4u << 20, &tam);
    REVISAR(mp3 != NULL);
    if (!mp3) return;
    REVISAR(solar_sonido_cargar(0, (const uint8_t *)mp3, tam) == 0);
    double d = solar_sonido_duracion(0);
    REVISAR(d > 0.05 && d < 0.5);          /* sin el segundo de silencio final */
    REVISAR(solar_sonido_cargar(1, (const uint8_t *)"no es mp3", 9) == EINVAL);
    REVISAR(solar_sonido_cargar(99, (const uint8_t *)mp3, tam) == EINVAL);
    int r = solar_sonido_reproducir(0, 0.0f);
    REVISAR(r == 0);
    r = solar_sonido_reproducir(0, 0.1f);
    printf("    (salida de audio: %s)\n", *solar_sonido_salida() ? solar_sonido_salida() : "ninguna");
    REVISAR(r == 0 || r == ENOTSUP);
    usleep(400000);
    free(mp3);
}

int main(void)
{
    char plantilla[] = "/tmp/solar-pruebas-XXXXXX";
    if (!mkdtemp(plantilla)) { perror("mkdtemp"); return 2; }
    copiar_texto(raiz, sizeof raiz, plantilla);
    char var[PATH_MAX + 32];
    setenv("HOME", raiz, 1);
    snprintf(var, sizeof var, "%s/datos", raiz);
    setenv("XDG_DATA_HOME", var, 1);
    snprintf(var, sizeof var, "%s/config", raiz);
    setenv("XDG_CONFIG_HOME", var, 1);
    snprintf(var, sizeof var, "%s/sistema", raiz);
    setenv("XDG_DATA_DIRS", var, 1);
    setenv("LANG", "es_MX.UTF-8", 1);
    unsetenv("LC_ALL");
    unsetenv("LC_MESSAGES");
    unsetenv("XDG_CURRENT_DESKTOP");
    printf("☀ Pruebas del núcleo de Solar %s (en %s)\n", solar_version(), raiz);

    prueba_natural();
    prueba_tipos();
    prueba_listado();
    prueba_nombres();
    prueba_copiar_mover();
    prueba_papelera();
    prueba_carpetas();
    prueba_config();
    prueba_temas();
    prueba_procesos();
    prueba_dispositivos_vigilante();
    prueba_programas();
    prueba_sonido();

    eliminar_arbol(raiz, NULL, NULL);
    for (size_t i = 0; i < en_pozo; i++) free(pozo[i]);
    printf("\n%d de %d revisiones bien%s\n", total - fallos, total, fallos ? "" : " ✓");
    return fallos ? 1 : 0;
}
