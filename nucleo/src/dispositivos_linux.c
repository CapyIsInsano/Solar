/*
 * ☀ Solar — dispositivos en Linux: memorias USB, tarjetas SD y discos
 *
 * Lee /sys/class/block, /proc/self/mountinfo y (si hay) la base de datos de
 * udev en /run/udev/data. Para montar y expulsar usa udisksctl (udisks2),
 * así no hace falta ser root; si no está, intenta con mount/umount.
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <ctype.h>
#include <dirent.h>
#include <sys/statvfs.h>
#include <unistd.h>

/* ===================================================================== */
/*  /proc/self/mountinfo                                                 */
/* ===================================================================== */

static void decodificar_octal(char *s)
{
    char *w = s;
    for (const char *r = s; *r; r++) {
        if (r[0] == '\\' && r[1] >= '0' && r[1] <= '3' && r[2] >= '0' && r[2] <= '7' && r[3] >= '0' && r[3] <= '7') {
            *w++ = (char)((r[1] - '0') * 64 + (r[2] - '0') * 8 + (r[3] - '0'));
            r += 3;
        } else {
            *w++ = *r;
        }
    }
    *w = '\0';
}

static int tiene_opcion(const char *opciones, const char *opcion)
{
    size_t n = strlen(opcion);
    const char *p = opciones;
    while (p && *p) {
        if (!strncmp(p, opcion, n) && (p[n] == ',' || p[n] == '\0')) return 1;
        p = strchr(p, ',');
        if (p) p++;
    }
    return 0;
}

int leer_montajes(Montaje **salida, size_t *cantidad)
{
    *salida = NULL;
    *cantidad = 0;
    char *texto = leer_archivo("/proc/self/mountinfo", 8u << 20, NULL);
    if (!texto) return errno ? errno : EIO;
    size_t cap = 32, n = 0;
    Montaje *v = calloc(cap, sizeof *v);
    if (!v) { free(texto); return ENOMEM; }

    char *guardar_linea = NULL;
    for (char *linea = strtok_r(texto, "\n", &guardar_linea); linea;
         linea = strtok_r(NULL, "\n", &guardar_linea)) {
        char *campos[16];
        int nc = 0;
        char *guardar = NULL;
        for (char *c = strtok_r(linea, " ", &guardar); c && nc < 16; c = strtok_r(NULL, " ", &guardar))
            campos[nc++] = c;
        if (nc < 7) continue;
        int guion = -1;
        for (int i = 6; i < nc; i++)
            if (!strcmp(campos[i], "-")) { guion = i; break; }
        if (guion < 0 || guion + 2 >= nc) continue;

        Montaje m;
        memset(&m, 0, sizeof m);
        if (sscanf(campos[2], "%u:%u", &m.mayor, &m.menor) != 2) continue;
        decodificar_octal(campos[4]);
        decodificar_octal(campos[guion + 2]);
        m.punto = solar_strdup(campos[4]);
        m.tipo_fs = solar_strdup(campos[guion + 1]);
        m.fuente = solar_strdup(campos[guion + 2]);
        m.solo_lectura = tiene_opcion(campos[5], "ro");
        if (!m.punto || !m.tipo_fs || !m.fuente) {
            free(m.punto);
            free(m.tipo_fs);
            free(m.fuente);
            continue;
        }
        if (n == cap) {
            cap *= 2;
            Montaje *q = realloc(v, cap * sizeof *q);
            if (!q) { free(m.punto); free(m.tipo_fs); free(m.fuente); break; }
            v = q;
        }
        v[n++] = m;
    }
    free(texto);
    *salida = v;
    *cantidad = n;
    return 0;
}

void liberar_montajes(Montaje *m, size_t cantidad)
{
    if (!m) return;
    for (size_t i = 0; i < cantidad; i++) {
        free(m[i].punto);
        free(m[i].tipo_fs);
        free(m[i].fuente);
    }
    free(m);
}

/* ===================================================================== */
/*  Ayudas de sysfs y udev                                               */
/* ===================================================================== */

static int leer_linea(const char *ruta, char *salida, size_t tam)
{
    char *t = leer_archivo(ruta, 4096, NULL);
    if (!t) return 0;
    char *fin = strpbrk(t, "\r\n");
    if (fin) *fin = '\0';
    copiar_texto(salida, tam, t);
    free(t);
    return 1;
}

static uint64_t leer_numero(const char *ruta)
{
    char buf[64];
    if (!leer_linea(ruta, buf, sizeof buf)) return 0;
    return strtoull(buf, NULL, 10);
}

typedef struct {
    char tipo_fs[32];
    char etiqueta[128];
    char uso[32];
    char tipo_particion[64];
    char modelo[128];
    char nombre_particion[128];
    int  ignorar;
    int  hay_datos;
} DatosUdev;

static void decodificar_hex(char *s)
{
    char *w = s;
    for (const char *r = s; *r; r++) {
        if (r[0] == '\\' && r[1] == 'x' && isxdigit((unsigned char)r[2]) && isxdigit((unsigned char)r[3])) {
            char hex[3] = { r[2], r[3], 0 };
            *w++ = (char)strtol(hex, NULL, 16);
            r += 3;
        } else {
            *w++ = *r;
        }
    }
    *w = '\0';
}

static void leer_udev(unsigned mayor, unsigned menor, DatosUdev *u)
{
    memset(u, 0, sizeof *u);
    char ruta[64];
    snprintf(ruta, sizeof ruta, "/run/udev/data/b%u:%u", mayor, menor);
    char *texto = leer_archivo(ruta, 256 * 1024, NULL);
    if (!texto) return;
    u->hay_datos = 1;
    char etiqueta_simple[128] = "";
    char *guardar = NULL;
    for (char *l = strtok_r(texto, "\n", &guardar); l; l = strtok_r(NULL, "\n", &guardar)) {
        if (strncmp(l, "E:", 2)) continue;
        l += 2;
        char *igual = strchr(l, '=');
        if (!igual) continue;
        *igual = '\0';
        const char *v = igual + 1;
        if (!strcmp(l, "ID_FS_TYPE")) copiar_texto(u->tipo_fs, sizeof u->tipo_fs, v);
        else if (!strcmp(l, "ID_FS_LABEL_ENC")) { copiar_texto(u->etiqueta, sizeof u->etiqueta, v); decodificar_hex(u->etiqueta); }
        else if (!strcmp(l, "ID_FS_LABEL")) copiar_texto(etiqueta_simple, sizeof etiqueta_simple, v);
        else if (!strcmp(l, "ID_FS_USAGE")) copiar_texto(u->uso, sizeof u->uso, v);
        else if (!strcmp(l, "ID_PART_ENTRY_TYPE")) { copiar_texto(u->tipo_particion, sizeof u->tipo_particion, v); a_minusculas(u->tipo_particion); }
        else if (!strcmp(l, "ID_PART_ENTRY_NAME")) { copiar_texto(u->nombre_particion, sizeof u->nombre_particion, v); decodificar_hex(u->nombre_particion); }
        else if (!strcmp(l, "ID_MODEL")) {
            copiar_texto(u->modelo, sizeof u->modelo, v);
            for (char *p = u->modelo; *p; p++) if (*p == '_') *p = ' ';
        } else if (!strcmp(l, "UDISKS_IGNORE") && !strcmp(v, "1")) u->ignorar = 1;
    }
    if (!u->etiqueta[0] && etiqueta_simple[0]) copiar_texto(u->etiqueta, sizeof u->etiqueta, etiqueta_simple);
    free(texto);
}

/* Sin udev: buscamos la etiqueta en /dev/disk/by-label. */
static void etiqueta_por_enlaces(const char *nombre_dev, char *salida, size_t tam)
{
    DIR *d = opendir("/dev/disk/by-label");
    if (!d) return;
    struct dirent *de;
    while ((de = readdir(d))) {
        if (de->d_name[0] == '.') continue;
        char ruta[PATH_MAX], destino[PATH_MAX];
        snprintf(ruta, sizeof ruta, "/dev/disk/by-label/%s", de->d_name);
        ssize_t n = readlink(ruta, destino, sizeof destino - 1);
        if (n <= 0) continue;
        destino[n] = '\0';
        if (!strcmp(ruta_base(destino), nombre_dev)) {
            copiar_texto(salida, tam, de->d_name);
            decodificar_hex(salida);
            break;
        }
    }
    closedir(d);
}

static int es_punto_de_sistema(const char *punto)
{
    static const char *const SISTEMA[] = { "/", "/boot", "/efi", "/home", "/usr", "/var", "/opt",
                                           "/srv", "/tmp", "/nix", "/gnu", "/root", "[SWAP]" };
    for (size_t i = 0; i < SOLAR_LEN(SISTEMA); i++)
        if (!strcmp(punto, SISTEMA[i])) return 1;
    return !strncmp(punto, "/boot/", 6) || !strncmp(punto, "/var/", 5) || !strncmp(punto, "/snap", 5) ||
           !strncmp(punto, "/usr/", 5) || !strncmp(punto, "/nix/", 5);
}

static int es_particion_de_sistema(const char *tipo)
{
    static const char *const TIPOS[] = {
        "c12a7328-f81f-11d2-ba4b-00a0c93ec93b", /* EFI */
        "e3c9e316-0b5c-4db8-817d-f92df00215ae", /* reservada de Microsoft */
        "de94bba4-06d1-4d40-a16a-bfd50179d6ac", /* recuperación de Windows */
        "21686148-6449-6e6f-744e-656564454649", /* BIOS boot */
        "0657fd6d-a4ab-43c4-84e5-0933c84b4f4f", /* swap de Linux */
        "0xef", "0x27", "0x82",
    };
    for (size_t i = 0; i < SOLAR_LEN(TIPOS); i++)
        if (!strcmp(tipo, TIPOS[i])) return 1;
    return 0;
}

static void tamano_legible(uint64_t bytes, char *salida, size_t tam)
{
    const char *unidades[] = { "B", "kB", "MB", "GB", "TB", "PB" };
    double v = (double)bytes;
    int u = 0;
    while (v >= 1000.0 && u < 5) { v /= 1000.0; u++; }
    if (u == 0 || v >= 100) snprintf(salida, tam, "%.0f %s", v, unidades[u]);
    else {
        snprintf(salida, tam, "%.1f %s", v, unidades[u]);
        char *punto = strchr(salida, '.');
        if (punto) *punto = ',';
    }
}

/* Nombre del disco al que pertenece una partición (sda1 -> sda). */
static int disco_de(const char *nombre, char *disco, size_t tam)
{
    char ruta[PATH_MAX], destino[PATH_MAX];
    snprintf(ruta, sizeof ruta, "/sys/class/block/%s", nombre);
    ssize_t n = readlink(ruta, destino, sizeof destino - 1);
    if (n <= 0) return copiar_texto(disco, tam, nombre);
    destino[n] = '\0';
    char *ultima = strrchr(destino, '/');
    if (!ultima) return copiar_texto(disco, tam, nombre);
    *ultima = '\0';
    return copiar_texto(disco, tam, ruta_base(destino));
}

static int tiene_particiones(const char *disco)
{
    char ruta[PATH_MAX];
    snprintf(ruta, sizeof ruta, "/sys/block/%s", disco);
    DIR *d = opendir(ruta);
    if (!d) return 0;
    int si = 0;
    size_t n = strlen(disco);
    struct dirent *de;
    while ((de = readdir(d))) {
        if (strncmp(de->d_name, disco, n) || !de->d_name[n]) continue;
        char part[PATH_MAX + 300];
        snprintf(part, sizeof part, "%s/%s/partition", ruta, de->d_name);
        if (existe(part)) { si = 1; break; }
    }
    closedir(d);
    return si;
}

static int es_extraible(const char *disco)
{
    char ruta[PATH_MAX], real[PATH_MAX];
    snprintf(ruta, sizeof ruta, "/sys/block/%s/removable", disco);
    if (leer_numero(ruta) == 1) return 1;
    snprintf(ruta, sizeof ruta, "/sys/block/%s", disco);
    if (realpath(ruta, real) && strstr(real, "/usb")) return 1;
    if (!strncmp(disco, "mmcblk", 6)) {
        char tipo[32];
        snprintf(ruta, sizeof ruta, "/sys/block/%s/device/type", disco);
        if (leer_linea(ruta, tipo, sizeof tipo) && (!strcmp(tipo, "SD") || !strcmp(tipo, "MMC"))) return 1;
    }
    return 0;
}

static int ignorar_nombre(const char *n)
{
    static const char *const PREFIJOS[] = { "loop", "ram", "zram", "dm-", "md", "sr", "fd", "nbd", "zd", "rbd" };
    for (size_t i = 0; i < SOLAR_LEN(PREFIJOS); i++)
        if (!strncmp(n, PREFIJOS[i], strlen(PREFIJOS[i]))) return 1;
    return 0;
}

static int ordenar_dispositivos(const void *a, const void *b)
{
    const SolarDispositivo *x = a, *y = b;
    if (x->extraible != y->extraible) return y->extraible - x->extraible;
    return solar_comparar_natural(x->nombre ? x->nombre : "", y->nombre ? y->nombre : "");
}

int solar_dispositivos_listar(int incluir_internos, SolarDispositivos *salida)
{
    salida->dispositivos = NULL;
    salida->cantidad = 0;
    Montaje *montajes = NULL;
    size_t nm = 0;
    leer_montajes(&montajes, &nm);

    DIR *d = opendir("/sys/class/block");
    if (!d) {
        int e = errno;
        liberar_montajes(montajes, nm);
        return e;
    }
    size_t cap = 0;
    struct dirent *de;
    while ((de = readdir(d))) {
        const char *nombre = de->d_name;
        if (nombre[0] == '.' || ignorar_nombre(nombre)) continue;
        char base[PATH_MAX], ruta[PATH_MAX + 32], numeros[32];
        snprintf(base, sizeof base, "/sys/class/block/%s", nombre);
        snprintf(ruta, sizeof ruta, "%s/dev", base);
        unsigned mayor, menor;
        if (!leer_linea(ruta, numeros, sizeof numeros) || sscanf(numeros, "%u:%u", &mayor, &menor) != 2) continue;

        snprintf(ruta, sizeof ruta, "%s/partition", base);
        int es_particion = existe(ruta);
        char disco[128];
        if (es_particion) disco_de(nombre, disco, sizeof disco);
        else copiar_texto(disco, sizeof disco, nombre);
        if (!es_particion && tiene_particiones(disco)) continue;

        snprintf(ruta, sizeof ruta, "%s/size", base);
        uint64_t bytes = leer_numero(ruta) * 512;
        if (bytes == 0) continue;

        int extraible = es_extraible(disco);
        DatosUdev u;
        leer_udev(mayor, menor, &u);
        if (u.ignorar) continue;

        const Montaje *mt = NULL;
        for (size_t i = 0; i < nm; i++) {
            if (montajes[i].mayor == mayor && montajes[i].menor == menor) {
                if (!mt || strlen(montajes[i].punto) < strlen(mt->punto)) mt = &montajes[i];
            }
        }
        const char *tipo_fs = u.tipo_fs[0] ? u.tipo_fs : (mt ? mt->tipo_fs : NULL);
        if (u.uso[0] && strcmp(u.uso, "filesystem")) continue;
        if (tipo_fs && (!strcmp(tipo_fs, "swap") || !strcmp(tipo_fs, "crypto_LUKS") ||
                        !strcmp(tipo_fs, "LVM2_member") || !strcmp(tipo_fs, "linux_raid_member")))
            continue;
        if (!tipo_fs && !mt && (!extraible || u.hay_datos)) continue;
        if (!extraible) {
            if (!incluir_internos) continue;
            if (mt && es_punto_de_sistema(mt->punto)) continue;
            if (es_particion_de_sistema(u.tipo_particion)) continue;
            if (!mt && bytes < (256ull << 20)) continue;
        }

        if (salida->cantidad == cap) {
            size_t nuevo = cap ? cap * 2 : 8;
            SolarDispositivo *q = realloc(salida->dispositivos, nuevo * sizeof *q);
            if (!q) break;
            salida->dispositivos = q;
            cap = nuevo;
        }
        SolarDispositivo *dv = &salida->dispositivos[salida->cantidad];
        memset(dv, 0, sizeof *dv);
        char texto[300];
        snprintf(texto, sizeof texto, "/dev/%s", nombre);
        dv->dispositivo = solar_strdup(texto);

        char etiqueta[128];
        copiar_texto(etiqueta, sizeof etiqueta, u.etiqueta);
        if (!etiqueta[0]) etiqueta_por_enlaces(nombre, etiqueta, sizeof etiqueta);
        if (!etiqueta[0] && u.nombre_particion[0] && !extraible)
            copiar_texto(etiqueta, sizeof etiqueta, u.nombre_particion);
        if (etiqueta[0]) {
            dv->nombre = solar_strdup(etiqueta);
        } else {
            char legible[32];
            tamano_legible(bytes, legible, sizeof legible);
            if (extraible && u.modelo[0]) snprintf(texto, sizeof texto, "%s (%s)", u.modelo, legible);
            else snprintf(texto, sizeof texto, "%s de %s", extraible ? "Unidad" : "Disco", legible);
            dv->nombre = solar_strdup(texto);
        }
        dv->sistema_archivos = tipo_fs ? solar_strdup(tipo_fs) : NULL;
        dv->tamano = bytes;
        dv->extraible = (uint8_t)extraible;
        if (mt) {
            dv->punto_montaje = solar_strdup(mt->punto);
            dv->montado = 1;
            dv->solo_lectura = (uint8_t)mt->solo_lectura;
            struct statvfs sv;
            if (statvfs(mt->punto, &sv) == 0) {
                dv->tamano = (uint64_t)sv.f_blocks * sv.f_frsize;
                dv->libre = (uint64_t)sv.f_bavail * sv.f_frsize;
            }
        }
        if (!dv->dispositivo || !dv->nombre) {
            free(dv->dispositivo);
            free(dv->nombre);
            free(dv->punto_montaje);
            free(dv->sistema_archivos);
            continue;
        }
        salida->cantidad++;
    }
    closedir(d);
    liberar_montajes(montajes, nm);
    if (salida->cantidad > 1)
        qsort(salida->dispositivos, salida->cantidad, sizeof salida->dispositivos[0], ordenar_dispositivos);
    return 0;
}

void solar_dispositivos_liberar(SolarDispositivos *d)
{
    if (!d) return;
    for (size_t i = 0; i < d->cantidad; i++) {
        free(d->dispositivos[i].dispositivo);
        free(d->dispositivos[i].nombre);
        free(d->dispositivos[i].punto_montaje);
        free(d->dispositivos[i].sistema_archivos);
    }
    free(d->dispositivos);
    d->dispositivos = NULL;
    d->cantidad = 0;
}

/* ===================================================================== */
/*  Montar y expulsar                                                    */
/* ===================================================================== */

static void ultima_linea(char *texto)
{
    size_t n = strlen(texto);
    while (n && (texto[n - 1] == '\n' || texto[n - 1] == ' ' || texto[n - 1] == '.')) texto[--n] = '\0';
    char *ultima = strrchr(texto, '\n');
    if (ultima) memmove(texto, ultima + 1, strlen(ultima + 1) + 1);
}

static void explicar(const char *salida, const char *accion, const char *dispositivo, char *error, size_t tam)
{
    const char *nombre = ruta_base(dispositivo);
    if (strstr(salida, "NotAuthorized") || strstr(salida, "not authorized"))
        poner_error(error, tam, "No tienes permiso para %s «%s» (polkit lo negó). ¿Tienes un agente de polkit corriendo?", accion, nombre);
    else if (strstr(salida, "busy") || strstr(salida, "Busy"))
        poner_error(error, tam, "No se pudo %s «%s»: la unidad está ocupada. Cierra los programas que estén usando sus archivos.", accion, nombre);
    else if (strstr(salida, "wrong fs type") || strstr(salida, "unknown filesystem") || strstr(salida, "NotSupported"))
        poner_error(error, tam, "No se pudo %s «%s»: el sistema de archivos no es compatible.", accion, nombre);
    else {
        char copia[512];
        copiar_texto(copia, sizeof copia, salida);
        ultima_linea(copia);
        if (copia[0]) poner_error(error, tam, "No se pudo %s «%s»: %s", accion, nombre, copia);
        else poner_error(error, tam, "No se pudo %s «%s».", accion, nombre);
    }
}

int solar_dispositivo_montar(const char *dispositivo, char *error, size_t error_tam)
{
    if (error && error_tam) error[0] = '\0';
    char salida[2048];
    const char *argv[] = { "udisksctl", "mount", "-b", dispositivo, NULL };
    int r = ejecutar_y_esperar(argv, salida, sizeof salida, 90000);
    if (r == 0 || strstr(salida, "AlreadyMounted")) return 0;
    if (r == -ENOENT) {
        const char *alterno[] = { "mount", dispositivo, NULL };
        if (ejecutar_y_esperar(alterno, salida, sizeof salida, 30000) == 0) return 0;
        poner_error(error, error_tam, "No se encontró «udisksctl». Instala udisks2 para montar memorias USB sin ser root.");
        return ENOENT;
    }
    if (r < 0) {
        poner_error(error, error_tam, "No se pudo montar «%s»: %s.", ruta_base(dispositivo), solar_error_texto(-r));
        return -r;
    }
    explicar(salida, "montar", dispositivo, error, error_tam);
    return EIO;
}

int solar_dispositivo_desmontar(const char *dispositivo, int expulsar, char *error, size_t error_tam)
{
    if (error && error_tam) error[0] = '\0';
    char salida[2048];
    const char *argv[] = { "udisksctl", "unmount", "-b", dispositivo, NULL };
    int r = ejecutar_y_esperar(argv, salida, sizeof salida, 120000);
    if (r == -ENOENT) {
        const char *alterno[] = { "umount", dispositivo, NULL };
        if (ejecutar_y_esperar(alterno, salida, sizeof salida, 60000) == 0) return 0;
        explicar(salida, "desmontar", dispositivo, error, error_tam);
        return EIO;
    }
    if (r < 0) {
        poner_error(error, error_tam, "No se pudo desmontar «%s»: %s.", ruta_base(dispositivo), solar_error_texto(-r));
        return -r;
    }
    if (r != 0 && !strstr(salida, "NotMounted") && !strstr(salida, "not mounted")) {
        explicar(salida, "desmontar", dispositivo, error, error_tam);
        return EIO;
    }
    if (expulsar) {
        char disco[128], ruta[256];
        disco_de(ruta_base(dispositivo), disco, sizeof disco);
        if (es_extraible(disco)) {
            snprintf(ruta, sizeof ruta, "/dev/%s", disco);
            const char *apagar[] = { "udisksctl", "power-off", "-b", ruta, NULL };
            ejecutar_y_esperar(apagar, salida, sizeof salida, 30000);
        }
    }
    return 0;
}

int solar_espacio(const char *ruta, SolarEspacio *espacio)
{
    struct statvfs sv;
    if (statvfs(ruta, &sv) != 0) return errno;
    espacio->total = (uint64_t)sv.f_blocks * sv.f_frsize;
    espacio->libre = (uint64_t)sv.f_bfree * sv.f_frsize;
    espacio->disponible = (uint64_t)sv.f_bavail * sv.f_frsize;
    return 0;
}
