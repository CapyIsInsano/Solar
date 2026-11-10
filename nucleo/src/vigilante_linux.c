/*
 * ☀ Solar — vigilante de cambios en Linux (inotify + mountinfo)
 *
 * Avisa cuando cambia la carpeta abierta, la papelera, la tabla de montajes
 * (se montó o desmontó algo) o aparece/desaparece un disco en /dev (se
 * conectó una memoria USB). Todo con eventos: mientras no pase nada, el hilo
 * que espera duerme y no gasta batería.
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <fcntl.h>
#include <poll.h>
#include <sys/eventfd.h>
#include <sys/inotify.h>
#include <unistd.h>

struct SolarVigilante {
    int ifd;          /* inotify */
    int wd_carpeta;
    int wd_papelera;
    int mfd;          /* /proc/self/mountinfo (POLLPRI cuando cambia) */
    int efd;          /* eventfd para despertar la espera */
    int wd_dev;       /* /dev: discos que llegan o se van */
};

/* ¿Es un nombre de /dev que corresponde a un disco o partición? */
static int es_disco(const char *n)
{
    static const char *const prefijos[] = { "sd", "nvme", "mmcblk", "sr", "vd", "hd", "xvd" };
    for (size_t i = 0; i < sizeof prefijos / sizeof *prefijos; i++)
        if (strncmp(n, prefijos[i], strlen(prefijos[i])) == 0) return 1;
    return 0;
}

static void rearmar_montajes(int fd)
{
    char buf[4096];
    lseek(fd, 0, SEEK_SET);
    while (read(fd, buf, sizeof buf) > 0) {}
}

SolarVigilante *solar_vigilante_crear(void)
{
    SolarVigilante *v = calloc(1, sizeof *v);
    if (!v) return NULL;
    v->wd_carpeta = -1;
    v->wd_papelera = -1;
    v->wd_dev = -1;
    v->ifd = inotify_init1(IN_NONBLOCK | IN_CLOEXEC);
    v->mfd = open("/proc/self/mountinfo", O_RDONLY | O_CLOEXEC);
    if (v->mfd >= 0) rearmar_montajes(v->mfd);
    v->efd = eventfd(0, EFD_NONBLOCK | EFD_CLOEXEC);
    if (v->ifd >= 0) v->wd_dev = inotify_add_watch(v->ifd, "/dev", IN_CREATE | IN_DELETE | IN_ONLYDIR);

    char papelera[PATH_MAX], files[PATH_MAX + 8];
    if (v->ifd >= 0 && solar_papelera_dir(papelera, sizeof papelera) == 0) {
        snprintf(files, sizeof files, "%s/files", papelera);
        crear_dirs(files, 0700);
        v->wd_papelera = inotify_add_watch(v->ifd, files,
                                           IN_CREATE | IN_DELETE | IN_MOVED_FROM | IN_MOVED_TO | IN_ONLYDIR);
    }
    return v;
}

int solar_vigilante_observar(SolarVigilante *v, const char *ruta)
{
    if (!v || v->ifd < 0) return EINVAL;
    if (v->wd_carpeta >= 0 && v->wd_carpeta != v->wd_papelera) inotify_rm_watch(v->ifd, v->wd_carpeta);
    v->wd_carpeta = -1;
    if (!ruta) return 0;
    uint32_t mascara = IN_CREATE | IN_DELETE | IN_MOVED_FROM | IN_MOVED_TO | IN_ATTRIB |
                       IN_CLOSE_WRITE | IN_DELETE_SELF | IN_MOVE_SELF | IN_ONLYDIR;
    if (v->wd_papelera >= 0) mascara |= IN_MASK_ADD; /* por si es la misma carpeta */
    int wd = inotify_add_watch(v->ifd, ruta, mascara);
    if (wd < 0) return errno;
    v->wd_carpeta = wd;
    return 0;
}

int solar_vigilante_esperar(SolarVigilante *v, int milisegundos)
{
    if (!v) return -1;
    struct pollfd pfd[3];
    int n = 0, i_ino = -1, i_mnt = -1, i_ev = -1;
    if (v->ifd >= 0) { pfd[n].fd = v->ifd; pfd[n].events = POLLIN; pfd[n].revents = 0; i_ino = n++; }
    if (v->mfd >= 0) { pfd[n].fd = v->mfd; pfd[n].events = POLLPRI; pfd[n].revents = 0; i_mnt = n++; }
    if (v->efd >= 0) { pfd[n].fd = v->efd; pfd[n].events = POLLIN; pfd[n].revents = 0; i_ev = n++; }
    if (n == 0) {
        usleep((useconds_t)milisegundos * 1000);
        return 0;
    }
    int r = poll(pfd, (nfds_t)n, milisegundos);
    if (r < 0) return errno == EINTR ? 0 : -1;
    if (r == 0) return 0;

    int cambios = 0;
    if (i_ev >= 0 && (pfd[i_ev].revents & POLLIN)) {
        uint64_t x;
        while (read(v->efd, &x, sizeof x) > 0) {}
        cambios |= SOLAR_CAMBIO_DESPERTADO;
    }
    if (i_mnt >= 0 && (pfd[i_mnt].revents & (POLLPRI | POLLERR))) {
        cambios |= SOLAR_CAMBIO_MONTAJES;
        rearmar_montajes(v->mfd);
    }
    if (i_ino >= 0 && (pfd[i_ino].revents & POLLIN)) {
        char buf[16384] __attribute__((aligned(__alignof__(struct inotify_event))));
        for (;;) {
            ssize_t leidos = read(v->ifd, buf, sizeof buf);
            if (leidos <= 0) break;
            for (char *p = buf; p < buf + leidos;) {
                struct inotify_event *ev = (struct inotify_event *)p;
                if (ev->wd == v->wd_carpeta) {
                    cambios |= SOLAR_CAMBIO_CARPETA;
                    if (ev->mask & IN_IGNORED) v->wd_carpeta = -1;
                }
                if (ev->wd == v->wd_papelera) cambios |= SOLAR_CAMBIO_PAPELERA;
                if (ev->wd == v->wd_dev && ev->len > 0 && es_disco(ev->name)) cambios |= SOLAR_CAMBIO_DISPOSITIVOS;
                if (ev->mask & IN_Q_OVERFLOW) cambios |= SOLAR_CAMBIO_CARPETA | SOLAR_CAMBIO_PAPELERA;
                p += sizeof(struct inotify_event) + ev->len;
            }
        }
    }
    return cambios;
}

void solar_vigilante_despertar(SolarVigilante *v)
{
    if (!v || v->efd < 0) return;
    uint64_t uno = 1;
    ssize_t r = write(v->efd, &uno, sizeof uno);
    (void)r;
}

void solar_vigilante_destruir(SolarVigilante *v)
{
    if (!v) return;
    if (v->ifd >= 0) close(v->ifd);
    if (v->mfd >= 0) close(v->mfd);
    if (v->efd >= 0) close(v->efd);
    free(v);
}
