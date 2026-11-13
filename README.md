<a id="inicio"></a>

<p align="center">
  <img src="docs/readme/portada.svg" width="100%" alt="Solar, gestor de archivos para Linux: núcleo en C, interfaz en Rust y configuración en Lua">
</p>

<p align="center">
  <b>Un gestor de archivos donde cada carpeta tiene su propio color y logotipo.</b><br>
  <sub>Tu carpeta <code>Juegos</code> verde con un mando verde oscuro, <code>Proyectos</code> azul con <code>&lt;/&gt;</code>, <code>Escuela</code> amarilla con un birrete…</sub>
</p>

> [!IMPORTANT]
> **Solar por ahora es exclusivo de Linux.** El núcleo usa cosas propias de
> Linux (inotify, `/proc`, `/sys`, udisks2) y no compila en Windows ni macOS.

<table align="center">
<tr>
<td align="center" width="16.6%"><a href="#que-trae"><img src="docs/readme/carpetas/que-trae.svg" width="76" alt=""><br><sub><b>Qué trae</b></sub></a></td>
<td align="center" width="16.6%"><a href="#capturas"><img src="docs/readme/carpetas/capturas.svg" width="76" alt=""><br><sub><b>Capturas</b></sub></a></td>
<td align="center" width="16.6%"><a href="#instalar"><img src="docs/readme/carpetas/instalar.svg" width="76" alt=""><br><sub><b>Instalar</b></sub></a></td>
<td align="center" width="16.6%"><a href="#uso"><img src="docs/readme/carpetas/uso.svg" width="76" alt=""><br><sub><b>Uso rápido</b></sub></a></td>
<td align="center" width="16.6%"><a href="#configuracion"><img src="docs/readme/carpetas/configuracion.svg" width="76" alt=""><br><sub><b>Configuración</b></sub></a></td>
<td align="center" width="16.6%"><a href="#temas"><img src="docs/readme/carpetas/temas.svg" width="76" alt=""><br><sub><b>Temas</b></sub></a></td>
</tr>
<tr>
<td align="center" width="16.6%"><a href="#actualizar"><img src="docs/readme/carpetas/actualizar.svg" width="76" alt=""><br><sub><b>Actualizar</b></sub></a></td>
<td align="center" width="16.6%"><a href="#desinstalar"><img src="docs/readme/carpetas/desinstalar.svg" width="76" alt=""><br><sub><b>Desinstalar</b></sub></a></td>
<td align="center" width="16.6%"><a href="#como-esta-hecho"><img src="docs/readme/carpetas/como-esta-hecho.svg" width="76" alt=""><br><sub><b>Cómo está hecho</b></sub></a></td>
<td align="center" width="16.6%"><a href="#pruebas"><img src="docs/readme/carpetas/pruebas.svg" width="76" alt=""><br><sub><b>Pruebas</b></sub></a></td>
<td align="center" width="16.6%"><a href="#licencia"><img src="docs/readme/carpetas/licencia.svg" width="76" alt=""><br><sub><b>Licencia</b></sub></a></td>
<td align="center" width="16.6%"><a href="#creditos"><img src="docs/readme/carpetas/creditos.svg" width="76" alt=""><br><sub><b>Créditos</b></sub></a></td>
</tr>
</table>

<p align="center"><sub>12 carpetas · haz clic en una para abrirla</sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="que-trae"></a>

## <img src="docs/readme/carpetas/que-trae.svg" width="34" align="top" alt=""> Qué trae

<sub>⌂ Solar › <b>Qué trae</b></sub>

**Carpetas a tu gusto**

- Clic derecho → **Color de carpeta**: 21 colores listos, o el tuyo con
  selector y código hex.
- Clic derecho → **Logotipo**: 54 (mando, música, código, cámara, chip,
  cohete, sol…) o un texto/emoji, con su propio color.
- Todas las carpetas se dibujan con la forma del logo de Solar. El estilo
  sigue a la carpeta si la renombras o la mueves, y viaja con las copias.

**Se ve como tú quieras**

- **9 temas** con nombres de astros: Sol, Aurora, Eclipse, Nebulosa, Venus,
  Neptuno, Marte, Mercurio y Luna, y los tuyos en Lua.
- **Fondo transparente** con opacidad, tamaño de la interfaz y fuente.
- Barra de ubicación **arriba o abajo**, barra lateral **izquierda o derecha**.
- **Animaciones** suaves y **efectos de sonido** (los de Capyro). Las dos se
  pueden apagar.

**Órbita: tus carpetas como un sistema solar** (opcional)

- La carpeta abierta va en el **centro**, como el sol, y lo que tiene adentro
  le da vueltas en órbitas. Todo se ve con su ícono de siempre: las carpetas
  con su color y logotipo, y los archivos (`.txt`, `.mp4`, `.pdf`…) con el
  de su tipo. Los más pesados se ven un poco más grandes.
- Tú decides si **giran** y qué tan rápido, en qué **orden** se acomodan (los
  más grandes al centro, los más chicos, por nombre, por tipo, los más
  recientes o al azar) y si las órbitas son **eléctricas**: un rayo continuo
  que tiembla suave, sin parpadear.
- Se detiene al pasar el mouse para que le atines; con la rueda te acercas y
  con el botón de la rueda mueves la vista.

**Pestañas y espacios de trabajo**

- **Pestañas** como en Windows 11: arrástralas para reordenarlas, abre una
  carpeta en otra con el clic de la rueda y reabre la que cerraste.
- **Espacios de trabajo** («Trabajo», «Juegos», «Escuela»…): cada uno
  recuerda sus pestañas y la vista de cada una, y puede tener su propio tema
  y sus propios ajustes. Al abrir Solar vuelves a donde te quedaste.

**Hace lo que esperas de un gestor de archivos**

- **4 vistas** como en Windows: íconos (de pequeños a muy grandes), lista con
  detalles, mosaicos y Órbita. Al cambiar de vista no se pierde la selección.
- Orden natural (la ñ va después de la n), ocultos, búsqueda al escribir,
  selección con banda y teclado.
- Copiar, mover y pegar con progreso y conflictos; **papelera** compatible
  con la de tu escritorio; arrastrar y soltar, **también entre ventanas**:
  de un Solar a otro, o a una terminal para escribir la ruta.
- **Memorias USB** en la barra lateral: montar con un clic y expulsar seguro.
- **Abrir con** los programas que tienes (Neovim, VSCodium, VS Code,
  Mousepad…), con su ícono, incluidos los de Flatpak.
- **Atajos de teclado** configurables, **Lugares** editables y la carpeta se
  actualiza sola cuando algo cambia.
- **Se actualiza desde GitHub** sin desinstalar, y su logo sale en la barra
  de tareas (también en Wayland).

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="capturas"></a>

## <img src="docs/readme/carpetas/capturas.svg" width="34" align="top" alt=""> Capturas

<sub>⌂ Solar › <b>Capturas</b></sub>

**En un escritorio de verdad:** Gentoo con Niri, fondo transparente,
carpetas con color y logotipo, una SSD en *Dispositivos* y el logo en el dock.

<p align="center">
  <img src="docs/capturas/escritorio-niri.png" width="100%" alt="Solar en Gentoo con Niri, con fondo transparente y su ícono en el dock">
</p>

<p align="center">
  <img src="docs/capturas/principal.png" width="100%" alt="Solar mostrando carpetas de colores con logotipos">
</p>

**Vista Órbita:** la carpeta personal va en el centro; sus carpetas giran
más cerca y los archivos más afuera, sobre órbitas eléctricas.

<p align="center">
  <img src="docs/capturas/orbita.png" width="100%" alt="Vista Órbita: la carpeta personal en el centro, con sus carpetas y archivos girando a su alrededor en órbitas eléctricas">
</p>

<table>
<tr>
<td width="50%"><img src="docs/capturas/espacios.png" alt="El menú de espacios de trabajo, con la carpeta Juegos en Órbita y el tema Venus"><p align="center"><sub><b>Espacios de trabajo</b>, cada uno con su tema</sub></p></td>
<td width="50%"><img src="docs/capturas/menu-vista.png" alt="El menú Vista, como el de Windows"><p align="center"><sub>El menú <b>Vista</b>: tamaños de íconos, lista, mosaicos y órbita</sub></p></td>
</tr>
<tr>
<td width="50%"><img src="docs/capturas/clic-derecho.png" alt="Clic derecho con los colores de carpeta"><p align="center"><sub>Clic derecho → <b>Color de carpeta</b></sub></p></td>
<td width="50%"><img src="docs/capturas/logotipos.png" alt="Los 54 logotipos"><p align="center"><sub>Clic derecho → <b>Logotipo</b></sub></p></td>
</tr>
<tr>
<td><img src="docs/capturas/personalizar.png" alt="Personalizar la carpeta Juegos"><p align="center"><sub><b>Personalizar carpeta</b>: verde con un mando verde oscuro</sub></p></td>
<td><img src="docs/capturas/abrir-con.png" alt="Abrir con en un archivo .cpp"><p align="center"><sub><b>Abrir con</b> en un <code>.cpp</code></sub></p></td>
</tr>
<tr>
<td><img src="docs/capturas/lista-clara.png" alt="Vista de lista con el tema Aurora"><p align="center"><sub>Vista de lista con el tema claro <b>Aurora</b></sub></p></td>
<td><img src="docs/capturas/transparencia.png" alt="Fondo transparente con compositor"><p align="center"><sub><b>Fondo transparente</b> al 75 % con un compositor</sub></p></td>
</tr>
<tr>
<td><img src="docs/capturas/ajustes-temas.png" alt="Ajustes: temas"><p align="center"><sub>Ajustes → <b>Temas</b></sub></p></td>
<td><img src="docs/capturas/ajustes-ventana.png" alt="Ajustes: ventana, sonido y animaciones"><p align="center"><sub>Ajustes → <b>Ventana, sonido y animaciones</b></sub></p></td>
</tr>
<tr>
<td><img src="docs/capturas/ajustes-distribucion.png" alt="Ajustes: distribución"><p align="center"><sub>Ajustes → <b>Distribución</b></sub></p></td>
<td><img src="docs/capturas/ajustes-atajos.png" alt="Ajustes: atajos"><p align="center"><sub>Ajustes → <b>Atajos</b></sub></p></td>
</tr>
<tr>
<td><img src="docs/capturas/actualizaciones.png" alt="Ajustes: actualizaciones"><p align="center"><sub>Ajustes → <b>Actualizaciones</b></sub></p></td>
<td><img src="docs/capturas/mosaicos.png" alt="Vista de mosaicos"><p align="center"><sub>Vista de <b>mosaicos</b>, con el tipo y el tamaño</sub></p></td>
</tr>
</table>

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="instalar"></a>

## <img src="docs/readme/carpetas/instalar.svg" width="34" align="top" alt=""> Instalar

<sub>⌂ Solar › <b>Instalar</b></sub>

Necesitas **Rust 1.95 o más nuevo**, un compilador de C (gcc o clang) y
`git`. Para usarlo al 100 %: **udisks2** (memorias USB sin root),
**shared-mime-info** (Abrir con), **curl** (actualizaciones), PipeWire,
PulseAudio o ALSA (sonidos) y una terminal. Usa OpenGL 3: Mesa sirve,
incluso en GPUs viejitas.

**1. Lo necesario según tu distro**

<details open>
<summary><b>Gentoo</b></summary>

```sh
sudo emerge --ask --noreplace virtual/rust dev-vcs/git sys-fs/udisks x11-misc/xdg-utils \
    net-misc/curl x11-misc/shared-mime-info
rustc --version     # tiene que decir 1.95 o más nuevo
```

`virtual/rust` se conforma con `dev-lang/rust` o con `dev-lang/rust-bin`
(el precompilado, mucho más rápido de instalar).
</details>

<details>
<summary><b>Arch / Manjaro / EndeavourOS</b></summary>

```sh
sudo pacman -S --needed rust base-devel git udisks2 xdg-utils curl
```
</details>

<details>
<summary><b>Debian / Ubuntu / Linux Mint</b> (el Rust de los repos suele ser viejo: usa rustup)</summary>

```sh
sudo apt install build-essential git curl udisks2 xdg-utils libgl1
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```
</details>

<details>
<summary><b>Fedora</b></summary>

```sh
sudo dnf install rust cargo gcc git udisks2 xdg-utils curl
```
</details>

<details>
<summary><b>Otras</b> (Alpine, Void…)</summary>

Instala Rust (≥ 1.95), un compilador de C, git, udisks2, xdg-utils y curl
con tu gestor de paquetes. En musl todavía no está probado.
</details>

**2. Clona y compila**

```sh
git clone https://github.com/CapyIsInsano/Solar.git
cd Solar
make                      # = cargo build --release (la primera vez tarda unos minutos)
./target/release/solar    # pruébalo sin instalar
```

**3. Instala**

```sh
sudo make instalar                  # en /usr/local, para todos
make instalar PREFIX=~/.local       # o solo para ti (con ~/.local/bin en el PATH)
```

Instala el programa, el lanzador y el ícono, así que Solar aparece en tu menú
de aplicaciones y con su logo en el dock.

**4. Hazlo tu gestor de archivos** (opcional)

```sh
xdg-mime default solar.desktop inode/directory
```

<details>
<summary>¿Quieres usar el Lua de tu distro en vez del incluido?</summary>

```sh
cargo build --release --no-default-features --features lua-sistema
```

Busca `lua5.4`, `lua54` o `lua` con pkg-config (en Gentoo: `dev-lang/lua:5.4`).
</details>

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="uso"></a>

## <img src="docs/readme/carpetas/uso.svg" width="34" align="top" alt=""> Uso rápido

<sub>⌂ Solar › <b>Uso rápido</b></sub>

| Hacer | Cómo |
|---|---|
| Personalizar una carpeta | Clic derecho → <i>Color de carpeta</i> / <i>Logotipo</i>, o <kbd>Ctrl</kbd> + <kbd>E</kbd> |
| Abrir con otro programa | Clic derecho → <i>Abrir con</i> |
| Escribir una ruta | <kbd>Ctrl</kbd> + <kbd>L</kbd> o clic en el espacio vacío de la barra |
| Buscar en la carpeta | Empieza a escribir, o <kbd>Ctrl</kbd> + <kbd>F</kbd> |
| Ver ocultos | <kbd>Ctrl</kbd> + <kbd>H</kbd> |
| Íconos / lista / mosaicos / órbita | <kbd>Ctrl</kbd> + <kbd>1</kbd> / <kbd>2</kbd> / <kbd>3</kbd> / <kbd>4</kbd>, o el menú *Vista* de la barra |
| Íconos más grandes o chicos | <kbd>Ctrl</kbd> + rueda, <kbd>Ctrl</kbd> + <kbd>Plus</kbd> / <kbd>Ctrl</kbd> + <kbd>Minus</kbd> |
| Nueva carpeta / archivo | <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>N</kbd> / <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>N</kbd> |
| Renombrar | <kbd>F2</kbd> |
| Papelera / eliminar para siempre | <kbd>Supr</kbd> / <kbd>Shift</kbd> + <kbd>Supr</kbd> |
| Copiar, cortar, pegar, duplicar | <kbd>Ctrl</kbd> + <kbd>C</kbd> <kbd>Ctrl</kbd> + <kbd>X</kbd> <kbd>Ctrl</kbd> + <kbd>V</kbd> <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>D</kbd> |
| Propiedades | <kbd>Alt</kbd> + <kbd>Enter</kbd> |
| Añadir a Lugares | <kbd>Ctrl</kbd> + <kbd>D</kbd> |
| Pestaña nueva / cerrar / reabrir la cerrada | <kbd>Ctrl</kbd> + <kbd>T</kbd> / <kbd>Ctrl</kbd> + <kbd>W</kbd> / <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>T</kbd> |
| Pestaña siguiente / anterior | <kbd>Ctrl</kbd> + <kbd>Tab</kbd> / <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>Tab</kbd> |
| Abrir una carpeta en otra pestaña | Clic con la rueda |
| Espacios de trabajo | <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>E</kbd>, o el botón a la izquierda de las pestañas |
| Nueva ventana | <kbd>Ctrl</kbd> + <kbd>N</kbd> |
| Terminal aquí | <kbd>F4</kbd> |
| Barra lateral | <kbd>F9</kbd> |
| Ajustes | <kbd>Ctrl</kbd> + <kbd>,</kbd> |
| Atrás / adelante / subir | <kbd>Alt</kbd> + <kbd>←</kbd> <kbd>Alt</kbd> + <kbd>→</kbd> <kbd>Alt</kbd> + <kbd>↑</kbd> (o <kbd>Retroceso</kbd> y los botones del mouse) |
| Quitar la selección, cerrar menús o Ajustes | <kbd>Esc</kbd> |

Arrastra archivos sobre una carpeta, un lugar, una memoria USB o un pedazo de
la barra de ubicación para **moverlos**; con <kbd>Ctrl</kbd> se **copian**.
Suéltalos en *Papelera* para tirarlos. Al copiar, Solar también deja las
rutas como texto en el portapapeles: puedes pegarlas en una terminal.

**Pestañas.** Arrástralas para cambiarlas de lugar; el clic con la rueda
cierra una. Si arrastras archivos sobre una pestaña y los sueltas, se mueven a
esa carpeta; si esperas un momento encima, la pestaña se abre y puedes
soltarlos donde quieras. Con el clic derecho: duplicarla, abrirla en una
ventana nueva, cerrar las demás o mandarla a otro espacio de trabajo.

**Espacios de trabajo.** Con el botón de la izquierda de las pestañas creas,
renombras y eliminas espacios, y cambias de uno a otro. Cada espacio guarda
sus pestañas, la vista de cada una y, si quieres, su propio tema y sus propios
ajustes (*Configurar este espacio…*). Un espacio solo se abre en una ventana a
la vez: si abres otra ventana, usa un espacio **temporal** (con borde
punteado) que no se guarda, salvo que elijas *Guardar como espacio…*.

**Órbita.** Al pasar el mouse por un archivo todo se detiene. La rueda acerca
y aleja, el botón de la rueda mueve la vista y la mira la vuelve a centrar.
La barrita de arriba cambia el giro, la velocidad, el acomodo y la
electricidad (también en *Ajustes → Archivos*). Con el teclado, <kbd>←</kbd>
<kbd>→</kbd> recorren la órbita y <kbd>↑</kbd> <kbd>↓</kbd> saltan de anillo.
Doble clic en la carpeta del centro sube a la de arriba.

**Entre ventanas.** Abre dos Solar, uno donde está el archivo y otro donde lo
quieres, y arrástralo de uno a otro: se **mueve** a la carpeta abierta (o a la
carpeta o lugar donde lo sueltes); si va a otro disco, se copia. Si lo
sueltas en una **terminal**, se escribe su ruta, lista para usar
(`'/home/capy/Mis cosas/nota.txt'`). También funciona con otros gestores de
archivos y programas que acepten archivos, en Wayland y en X11.

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="configuracion"></a>

## <img src="docs/readme/carpetas/configuracion.svg" width="34" align="top" alt=""> Configuración

<sub>⌂ Solar › <b>Configuración</b></sub>

| | Archivo | Para qué |
|:-:|---|---|
| <img src="docs/readme/archivos/lua.svg" width="20" alt=""> | `~/.config/solar/config.lua` | Todos los ajustes. Solar lo crea la primera vez y lo reescribe cuando cambias algo en Ajustes. |
| <img src="docs/readme/archivos/lua.svg" width="20" alt=""> | `~/.config/solar/espacios.lua` | Tus espacios de trabajo con sus pestañas. Solar lo reescribe solo; edítalo con Solar cerrado. |
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | `~/.config/solar/temas/` | Tus temas de colores (Solar nunca los reescribe). |
| <img src="docs/readme/archivos/texto.svg" width="20" alt=""> | `~/.local/share/solar/carpetas.tsv` | El color y logotipo de cada carpeta. |
| <img src="docs/readme/archivos/texto.svg" width="20" alt=""> | `~/.config/mimeapps.list` | El programa predeterminado de cada tipo de archivo (el mismo de los demás programas). |
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | `~/.cache/solar/` | Descargas y compilaciones de actualizaciones. Se puede borrar sin miedo. |

`config.lua` es Lua de verdad, en un entorno seguro: sin `io`, `os.execute`
ni `require`, y con límite de memoria y de tiempo para que un bucle infinito
no congele Solar. Si algo no se entiende, Solar te dice qué fue y usa el valor
por defecto. Todas las opciones están explicadas en
[`config/config.lua`](config/config.lua).

```lua
return {
  tema = "nebulosa",
  transparencia = true,
  opacidad = 0.8,
  barra_ubicacion = "abajo",
  barra_lateral = "derecha",
  sonidos = true,
  vista = "orbita",              -- cuadricula, lista, mosaicos u orbita
  orbita_velocidad = 0.5,        -- que la órbita gire más despacio
  atajos = { terminal = "Ctrl+Alt+T", eliminar = false },
}
```

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="temas"></a>

## <img src="docs/readme/carpetas/temas.svg" width="34" align="top" alt=""> Temas

<sub>⌂ Solar › <b>Temas</b></sub>

Crea un archivo en `~/.config/solar/temas/`, por ejemplo `bosque.lua`:

```lua
return {
  nombre = "Bosque",
  base = "venus",               -- hereda todo lo que no pongas
  colores = {
    acento  = "#22C55E",
    carpeta = "#15803D",        -- color de las carpetas sin personalizar
  },
}
```

Colores: `fondo`, `superficie`, `elevado`, `borde`, `texto`, `texto_tenue`,
`acento`, `texto_acento`, `seleccion`, `peligro`, `exito`, `aviso` y
`carpeta`. También puedes hacer uno desde *Ajustes → Apariencia → Crear un
tema a partir de este…*. Ejemplo completo en
[`config/temas/ejemplo.lua`](config/temas/ejemplo.lua).

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="actualizar"></a>

## <img src="docs/readme/carpetas/actualizar.svg" width="34" align="top" alt=""> Actualizar

<sub>⌂ Solar › <b>Actualizar</b></sub>

Desde Solar: *Ajustes → Actualizaciones → Buscar actualizaciones →
Actualizar ahora*, y luego *Reiniciar Solar*. No hay que desinstalar nada.
Solar compila la versión nueva con tu `cargo` (lo compilado se guarda en
`~/.cache/solar`, así que las siguientes son más rápidas). Si lo instalaste
con `sudo`, te pide la contraseña con pkexec. *Borrar caché* libera ese
espacio.

A mano también se puede: `git pull && make && sudo make instalar`.

**¿Cómo se entera Solar de que hay algo nuevo?**

- **Commits nuevos:** con solo hacer `git push` a `main`. Solar sabe de qué
  commit salió (`solar --version` lo muestra) y te ofrece *«Hay 3 commits
  nuevos en main»* con sus mensajes. Si lo compilaste con cambios tuyos que
  no están en GitHub, no te ofrece nada.
- **Versiones publicadas:**
  1. Sube la versión en `Cargo.toml` y en `nucleo/include/solar.h`, y
     escribe qué trae en `CHANGELOG.md`.
  2. `git commit -am "Solar v0.2.0-alpha" && git push`
  3. `git tag v0.2.0-alpha && git push origin v0.2.0-alpha`
  4. En GitHub → *Releases → Draft a new release*, elige esa etiqueta y pega
     las notas: Solar las muestra en *Qué trae*.

  Orden de versiones: `0.2.0-alpha < 0.2.0-beta < 0.2.0-rc1 < 0.2.0`.

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="desinstalar"></a>

## <img src="docs/readme/carpetas/desinstalar.svg" width="34" align="top" alt=""> Desinstalar

<sub>⌂ Solar › <b>Desinstalar</b></sub>

Desde la carpeta del código (usa el mismo `PREFIX` con el que instalaste):

```sh
sudo make desinstalar               # o: make desinstalar PREFIX=~/.local
```

Eso quita el programa, el lanzador, los íconos y los ejemplos. **Tu
configuración se queda**, por si vuelves. Para borrar todo rastro:

```sh
rm -rf ~/.config/solar ~/.local/share/solar ~/.cache/solar
rm -f ~/.local/share/applications/solar.desktop \
      ~/.local/share/icons/hicolor/*/apps/solar.png \
      ~/.local/share/icons/hicolor/scalable/apps/solar.svg
```

<details>
<summary>¿Ya no tienes la carpeta del código?</summary>

```sh
sudo rm -f /usr/local/bin/solar /usr/local/share/applications/solar.desktop
sudo rm -f /usr/local/share/icons/hicolor/*/apps/solar.png \
           /usr/local/share/icons/hicolor/scalable/apps/solar.svg
sudo rm -rf /usr/local/share/solar
```
</details>

Si era tu gestor de archivos predeterminado, regresa al de antes, por ejemplo
`xdg-mime default thunar.desktop inode/directory`.

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="como-esta-hecho"></a>

## <img src="docs/readme/carpetas/como-esta-hecho.svg" width="34" align="top" alt=""> Cómo está hecho

<sub>⌂ Solar › <b>Cómo está hecho</b></sub>

| Parte | Lenguaje | Qué hace |
|---|---|---|
| `nucleo/` | **C11** | Listar y ordenar, copiar/mover/eliminar con progreso, papelera, memorias USB (sysfs + mountinfo + udisks2), vigilar cambios (inotify), colores y logotipos, tipos MIME y programas (Abrir con), sonidos (minimp3 + PulseAudio/PipeWire o ALSA), y la configuración con **Lua 5.4** embebido. |
| `src/` | **Rust** (egui) | Toda la interfaz y el dibujo vectorial de carpetas y archivos. Llama al núcleo por FFI. |
| `~/.config/solar/*.lua` | **Lua** | Tu configuración y tus temas. |

`cargo build` compila todo junto: `build.rs` compila el núcleo en C y Lua
(viene incluido) y los enlaza con la interfaz.

**Hecho para gastar poco** (pensando en laptops como la X220):

- Sin hacer nada, Solar no redibuja ni revisa nada: los cambios de carpetas,
  de la papelera, de montajes y de discos (`/dev`) llegan por eventos
  (inotify, mountinfo, eventfd). Lo único que despierta al programa es la
  biblioteca de accesibilidad, 2 veces por segundo.
- Leer, ordenar e indexar las carpetas se hace en otro hilo; soltar una
  carpeta enorme también, y la memoria se le devuelve al sistema.
- Lo que se dibuja en cada cuadro no copia nada: la barra de estado y el
  estilo se recalculan solo cuando cambian.
- Los sonidos se mezclan en un solo hilo con una sola conexión de audio, que
  se cierra tras 2 s de silencio.

**Así se ve la carpeta del proyecto:**

| | Nombre | Tipo | Qué hay |
|:-:|---|---|---|
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | **`nucleo/`** | Carpeta | El núcleo en C: listado, copiar/mover, papelera, USB, MIME y programas, sonido, Lua |
| <img src="docs/readme/archivos/c.svg" width="20" alt=""> | **`nucleo/include/solar.h`** | Encabezado C | La API pública del núcleo |
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | **`nucleo/pruebas/`** | Carpeta | 251 revisiones con AddressSanitizer + UBSan |
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | **`nucleo/terceros/minimp3/`** | Carpeta | Decodificador de MP3 (CC0) |
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | **`src/`** | Carpeta | La interfaz en Rust (egui): ventana, vistas, menús, diálogos, Ajustes |
| <img src="docs/readme/archivos/rust.svg" width="20" alt=""> | **`src/app.rs`** | Código Rust | Estado, navegación y acciones |
| <img src="docs/readme/archivos/rust.svg" width="20" alt=""> | **`src/vista.rs`** | Código Rust | Lo común a todas las vistas: selección, banda, arrastrar, teclado y animaciones |
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | **`src/vista/`** | Carpeta | Cada vista (íconos, lista, mosaicos, órbita) y los menús de clic derecho |
| <img src="docs/readme/archivos/rust.svg" width="20" alt=""> | **`src/pestanas.rs`** | Código Rust | Las pestañas y su barra |
| <img src="docs/readme/archivos/rust.svg" width="20" alt=""> | **`src/espacios.rs`** | Código Rust | Espacios de trabajo: guardar, cambiar, un espacio por ventana |
| <img src="docs/readme/archivos/rust.svg" width="20" alt=""> | **`src/iconos.rs`** | Código Rust | Carpetas y archivos vectoriales con la forma del logo |
| <img src="docs/readme/archivos/rust.svg" width="20" alt=""> | **`src/actualizar.rs`** | Código Rust | Buscar en GitHub, compilar y reemplazar el programa |
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | **`src/arrastre_os/`** | Carpeta | Arrastrar entre ventanas: Wayland (`wl_data_device`) y X11 (XDND) |
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | **`config/`** | Carpeta | Ejemplos de <code>config.lua</code> y de un tema |
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | **`recursos/`** | Carpeta | Logo, íconos, sonidos y <code>solar.desktop</code> |
| <img src="docs/readme/carpetas/carpeta.svg" width="20" alt=""> | **`docs/`** | Carpeta | Capturas y las imágenes de este README |
| <img src="docs/readme/archivos/rust.svg" width="20" alt=""> | **`build.rs`** | Código Rust | Compila el núcleo en C y Lua junto con la interfaz |
| <img src="docs/readme/archivos/toml.svg" width="20" alt=""> | **`Cargo.toml`** | TOML | Versión y dependencias de Rust |
| <img src="docs/readme/archivos/make.svg" width="20" alt=""> | **`Makefile`** | Makefile | <code>make</code>, <code>make instalar</code>, <code>make pruebas</code> |
| <img src="docs/readme/archivos/md.svg" width="20" alt=""> | **`CHANGELOG.md`** | Markdown | Qué trae cada versión |
| <img src="docs/readme/archivos/texto.svg" width="20" alt=""> | **`LICENSE`** | Texto | GPL-3.0 |

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="pruebas"></a>

## <img src="docs/readme/carpetas/pruebas.svg" width="34" align="top" alt=""> Pruebas

<sub>⌂ Solar › <b>Pruebas</b></sub>

```sh
make pruebas                 # pruebas de Rust (también ejercitan el núcleo)
make pruebas-nucleo          # núcleo en C con AddressSanitizer + UBSan
```

**Medir el rendimiento:** `SOLAR_PERFIL=1 solar` escribe en la terminal cuánto
tarda cada parte de la ventana por cuadro, el arranque, cada listado de
carpeta y cada revisión de discos. Apagado no cuesta nada.

**Limitaciones conocidas** (es una alpha):

- Solo Linux.
- Todavía no hay miniaturas ni deshacer.
- La transparencia necesita un compositor (Niri, Hyprland, KWin, picom…).
- Si `config.lua` tiene lógica (if, variables), Ajustes la convierte en
  valores fijos la próxima vez que cambies algo desde la ventana.

**Ideas para las siguientes versiones:** miniaturas, deshacer, búsqueda en
subcarpetas, panel doble, comprimir/extraer y logotipos con imágenes propias.

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="licencia"></a>

## <img src="docs/readme/carpetas/licencia.svg" width="34" align="top" alt=""> Licencia

<sub>⌂ Solar › <b>Licencia</b></sub>

Solar es software libre bajo la **GPL-3.0 o posterior** ([LICENSE](LICENSE)):
puedes usarlo, estudiarlo, cambiarlo y compartirlo, y quien distribuya una
versión modificada tiene que compartir también su código bajo la misma
licencia.

<p align="right"><sub><a href="#inicio">↑ Subir</a></sub></p>

<img src="docs/readme/separador.svg" width="100%" alt="">

<a id="creditos"></a>

## <img src="docs/readme/carpetas/creditos.svg" width="34" align="top" alt=""> Créditos

<sub>⌂ Solar › <b>Créditos</b></sub>

Hecho por **Arturo ([CapyIsInsano](https://github.com/CapyIsInsano))**, el
mismo de [Capyro](https://github.com/CapyIsInsano/Capyro), de donde vienen
los efectos de sonido.

Usa [egui/eframe](https://github.com/emilk/egui) (MIT/Apache-2.0),
[Lua 5.4](https://www.lua.org) (MIT), íconos [Phosphor](https://phosphoricons.com)
(MIT), [minimp3](https://github.com/lieff/minimp3) (CC0) y
[resvg](https://github.com/linebender/resvg) (Apache-2.0/MIT). Las imágenes de
este README salen de [`docs/readme/generar.py`](docs/readme/generar.py).

<p align="center"><br><img src="docs/readme/carpetas/solar.svg" width="48" alt=""><br><sub>☀ Solar · v0.1.2-alpha</sub></p>
