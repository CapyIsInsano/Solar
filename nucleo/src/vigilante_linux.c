/*
 * ☀ Solar — vigilante de cambios en Linux (inotify + mountinfo)
 *
 * Avisa cuando cambia la carpeta abierta, la papelera o la tabla de montajes
 * (se conectó o se expulsó una memoria USB).
 *
 * Licencia: GPL-3.0-or-later
 */
#include "interno.h"

#include <fcntl.h>
#include <poll.h>
#include <sys/inotify.h>
#include <unistd.h>

struct SolarVigilante {
    int ifd;          /* inotify */
    int wd_carpeta;
    int wd_papelera;
    int mfd;          /* /proc/self/mountinfo (POLLPRI cuando cambia) */
};

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
    v->ifd = inotify_init1(IN_NONBLOCK | IN_CLOEXEC);
    v->mfd = open("/proc/self/mountinfo", O_RDONLY | O_CLOEXEC);
    if (v->mfd >= 0) rearmar_montajes(v->mfd);

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
    struct pollfd pfd[2];
    int n = 0, i_ino = -1, i_mnt = -1;
    if (v->ifd >= 0) { pfd[n].fd = v->ifd; pfd[n].events = POLLIN; pfd[n].revents = 0; i_ino = n++; }
    if (v->mfd >= 0) { pfd[n].fd = v->mfd; pfd[n].events = POLLPRI; pfd[n].revents = 0; i_mnt = n++; }
    if (n == 0) {
        usleep((useconds_t)milisegundos * 1000);
        return 0;
    }
    int r = poll(pfd, (nfds_t)n, milisegundos);
    if (r < 0) return errno == EINTR ? 0 : -1;
    if (r == 0) return 0;

    int cambios = 0;
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
                if (ev->mask & IN_Q_OVERFLOW) cambios |= SOLAR_CAMBIO_CARPETA | SOLAR_CAMBIO_PAPELERA;
                p += sizeof(struct inotify_event) + ev->len;
            }
        }
    }
    return cambios;
}

void solar_vigilante_destruir(SolarVigilante *v)
{
    if (!v) return;
    if (v->ifd >= 0) close(v->ifd);
    if (v->mfd >= 0) close(v->mfd);
    free(v);
}
