<p align="center">
  <img src="recursos/solar.svg" width="128" alt="Logo de Solar">
</p>

<h1 align="center">☀ Solar</h1>

<p align="center">
  <b>Gestor de archivos para Linux</b><br>
  Núcleo en C · Interfaz en Rust · Configuración en Lua<br>
  <sub>v0.1.0-alpha · GPL-3.0</sub>
</p>

<p align="center">
  <img src="docs/capturas/principal.png" width="820" alt="Solar mostrando carpetas de colores con logotipos">
</p>

Solar es un gestor de archivos ligero y bonito, donde **cada carpeta puede tener
su propio color y logotipo**: tu carpeta `Juegos` puede ser verde con un mando
verde oscuro, `Proyectos` azul con `</>`, `Escuela` amarilla con un birrete…
Todas las carpetas se dibujan con la misma forma del logo de Solar.

## Qué trae la v0.1.0 alpha

- **Carpetas personalizadas**: clic derecho → *Color de carpeta* (21 colores
  listos o uno personalizado con selector y código hex) y *Logotipo* (54
  logotipos: mando, música, código, cámara, chip, cohete, sol…, o un texto o
  emoji). El logotipo también tiene color propio. El estilo sigue a la carpeta
  si la renombras o la mueves desde Solar, y viaja con las copias.
- **9 temas de colores** (Solar, Amanecer, Eclipse, Nebulosa, Aurora, Océano,
  Marte, Grafito, Luna) y **temas propios en Lua**, que pueden heredar de otro.
  También puedes crear uno desde Ajustes con selectores de color.
- **Fondo transparente** con opacidad ajustable (necesita un compositor).
- **Barra de ubicación arriba o abajo**, barra lateral a la **izquierda o
  derecha** (o escondida). Clic en un pedazo de la ruta para ir ahí, o en el
  espacio vacío (o `Ctrl+L`) para escribir una ruta.
- **Lugares** editables (nombre, ruta, ícono y orden) y **memorias USB / tarjetas
  SD** en la barra lateral con su espacio libre, montar con un clic y expulsar
  con seguridad. Opcional: discos internos (como la partición de Windows).
- **Atajos de teclado configurables** desde Ajustes (haz clic y presiona la
  combinación) o en `config.lua`. Avisa si un atajo ya lo usa otra acción.
- Lo fundamental de un gestor de archivos: cuadrícula y lista, orden natural
  (`foto2` antes de `foto10`, sin importar acentos), selección con Ctrl, Shift
  y con banda, arrastrar y soltar, copiar/cortar/pegar con detección de
  conflictos (conservar ambos, reemplazar o combinar, omitir), duplicar,
  renombrar, crear carpetas y archivos, **papelera compatible con
  freedesktop** (la misma de Nautilus, Dolphin y Thunar; también en memorias
  USB), restaurar, propiedades con permisos editables y tamaño de carpetas,
  abrir con…, abrir terminal aquí, búsqueda al escribir, archivos ocultos,
  barra de estado con espacio libre, operaciones en segundo plano con
  progreso y cancelar, y la vista se actualiza sola cuando algo cambia.

### Capturas

Clic derecho → *Color de carpeta* y *Logotipo*, el diálogo *Personalizar
carpeta*, la vista de lista con el tema claro Amanecer, los Ajustes (temas,
ventana, distribución y atajos) y el fondo transparente con un compositor:

<p align="center">
  <img src="docs/capturas/clic-derecho.png" width="49%" alt="Menú del clic derecho con los colores de carpeta">
  <img src="docs/capturas/logotipos.png" width="49%" alt="Menú del clic derecho con los 54 logotipos">
  <img src="docs/capturas/personalizar.png" width="49%" alt="Personalizar la carpeta Juegos: verde con un mando verde oscuro">
  <img src="docs/capturas/lista-clara.png" width="49%" alt="Vista de lista con el tema Amanecer">
  <img src="docs/capturas/ajustes-ventana.png" width="49%" alt="Ajustes: fondo transparente, opacidad, tamaño y fuente">
  <img src="docs/capturas/transparencia.png" width="49%" alt="Fondo transparente al 75 % sobre un fondo de pantalla, con compositor">
  <img src="docs/capturas/ajustes-temas.png" width="49%" alt="Ajustes: temas de colores">
  <img src="docs/capturas/ajustes-distribucion.png" width="49%" alt="Ajustes: barra de ubicación arriba o abajo y barra lateral">
  <img src="docs/capturas/ajustes-atajos.png" width="49%" alt="Ajustes: atajos de teclado">
</p>

## Cómo está hecho

| Parte | Lenguaje | Qué hace |
|---|---|---|
| `nucleo/` | **C11** | Listar y ordenar carpetas, tipos de archivo, copiar/mover/eliminar con progreso, papelera, memorias USB (sysfs + mountinfo + udisks2), vigilar cambios (inotify), recordar colores y logotipos, y leer/escribir la configuración y los temas con **Lua 5.4** embebido. |
| `src/` | **Rust** (egui) | Toda la interfaz: ventana, vistas, menús, diálogos, Ajustes y el dibujo vectorial de carpetas y archivos. Llama al núcleo por FFI. |
| `~/.config/solar/*.lua` | **Lua** | Tu configuración (`config.lua`) y tus temas (`temas/*.lua`). |

`cargo build` compila todo junto: `build.rs` compila el núcleo en C (y Lua
5.4, que viene incluido) y lo enlaza con la interfaz. No necesitas tener Lua
instalado.

## Instalación paso a paso

Necesitas **Rust 1.95 o más nuevo**, un compilador de C (gcc o clang) y `git`.
Para usar Solar al 100 % conviene tener también **udisks2** (montar memorias
USB sin ser root), **xdg-utils** (abrir archivos con su programa) y una
terminal. Solar usa OpenGL 3 (Mesa sirve, incluso en GPUs viejitas).

### 1. Instala lo necesario

**Gentoo**

```sh
sudo emerge --ask --noreplace virtual/rust dev-vcs/git sys-fs/udisks x11-misc/xdg-utils
rustc --version     # tiene que decir 1.95 o más nuevo
```

`virtual/rust` se conforma con `dev-lang/rust` o con `dev-lang/rust-bin`
(el precompilado, mucho más rápido de instalar). Si tu versión es más vieja,
actualiza el slot o usa `dev-util/rustup`.

**Arch / Manjaro / EndeavourOS**

```sh
sudo pacman -S --needed rust base-devel git udisks2 xdg-utils
```

**Debian / Ubuntu / Linux Mint** (el Rust de los repos suele ser viejo; usa rustup)

```sh
sudo apt install build-essential git curl udisks2 xdg-utils libgl1
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

**Fedora**

```sh
sudo dnf install rust cargo gcc git udisks2 xdg-utils
```

**Alpine / Void / otras**: instala Rust (≥ 1.95), un compilador de C, git,
udisks2 y xdg-utils con tu gestor de paquetes. (Sin probar todavía en musl.)

### 2. Clona y compila

```sh
git clone https://github.com/CapyIsInsano/Solar.git
cd Solar
make            # es lo mismo que: cargo build --release
```

La primera vez tarda unos minutos (compila egui y Lua). El programa queda en
`target/release/solar`; puedes probarlo sin instalar:

```sh
./target/release/solar
```

### 3. Instala

```sh
sudo make instalar                  # en /usr/local (todos los usuarios)
# o, sin sudo, solo para ti:
make instalar PREFIX=~/.local       # asegúrate de tener ~/.local/bin en el PATH
```

Esto instala el programa, el lanzador (`solar.desktop`) y el ícono, así que
Solar aparece en tu menú de aplicaciones (en Wayland el ícono sale del
lanzador). Para quitarlo: `sudo make desinstalar` (o con el mismo `PREFIX`).

**¿Quieres usar el Lua de tu distro?**

```sh
cargo build --release --no-default-features --features lua-sistema
```

(Busca `lua5.4`, `lua54` o `lua` con pkg-config; en Gentoo es `dev-lang/lua:5.4`.)

### 4. Hacerlo tu gestor de archivos (opcional)

```sh
xdg-mime default solar.desktop inode/directory
```

## Uso rápido

| Hacer | Cómo |
|---|---|
| Personalizar una carpeta | Clic derecho → *Color de carpeta* / *Logotipo* / *Personalizar carpeta…* (`Ctrl+E`) |
| Escribir una ruta | `Ctrl+L` o clic en el espacio vacío de la barra de ubicación |
| Buscar en la carpeta | Empieza a escribir, o `Ctrl+F` |
| Ver ocultos | `Ctrl+H` |
| Cuadrícula / lista | `Ctrl+1` / `Ctrl+2` |
| Íconos más grandes o chicos | `Ctrl` + rueda del mouse, `Ctrl+Plus` / `Ctrl+Minus` |
| Nueva carpeta / archivo | `Ctrl+Shift+N` / `Ctrl+Alt+N` |
| Renombrar | `F2` |
| Papelera / eliminar para siempre | `Supr` / `Shift+Supr` |
| Copiar, cortar, pegar, duplicar | `Ctrl+C`, `Ctrl+X`, `Ctrl+V`, `Ctrl+Shift+D` |
| Propiedades | `Alt+Enter` |
| Añadir a Lugares | `Ctrl+D` |
| Terminal aquí | `F4` |
| Barra lateral | `F9` |
| Ajustes | `Ctrl+,` |
| Atrás / adelante / subir | `Alt+←` / `Alt+→` / `Alt+↑` (también `Retroceso` y los botones laterales del mouse) |
| Quitar la selección, cerrar menús o Ajustes | `Esc` |

Arrastra archivos sobre una carpeta, un lugar, una memoria USB o un pedazo de
la barra de ubicación para **moverlos**; con `Ctrl` se **copian**. Entre
discos distintos se copian siempre. Suéltalos en *Papelera* para tirarlos.

Al copiar, Solar también deja las rutas como texto en el portapapeles: puedes
pegarlas en una terminal. Y al revés: si copias rutas (o archivos desde otro
gestor) y pegas en Solar, las copia ahí.

## Configuración

| Archivo | Para qué |
|---|---|
| `~/.config/solar/config.lua` | Todos los ajustes. Solar lo crea la primera vez y lo reescribe cuando cambias algo en Ajustes. |
| `~/.config/solar/temas/*.lua` | Tus temas de colores (Solar nunca los reescribe). |
| `~/.local/share/solar/carpetas.tsv` | El color y logotipo de cada carpeta. |

`config.lua` es Lua de verdad, en un entorno seguro (sin `io`, `os.execute`
ni `require`, con límite de memoria y de tiempo para que un bucle infinito no
congele Solar). Si algo no se entiende, Solar te dice qué fue al abrir y usa
el valor por defecto. Hay un ejemplo con todas las opciones explicadas en
[`config/config.lua`](config/config.lua).

```lua
return {
  tema = "nebulosa",
  transparencia = true,
  opacidad = 0.8,
  barra_ubicacion = "abajo",
  barra_lateral = "derecha",
  atajos = { terminal = "Ctrl+T", eliminar = false },
}
```

### Temas en Lua

Crea un archivo en `~/.config/solar/temas/`, por ejemplo `bosque.lua`:

```lua
return {
  nombre = "Bosque",
  base = "aurora",              -- hereda todo lo que no pongas
  colores = {
    acento  = "#22C55E",
    carpeta = "#15803D",        -- color de las carpetas sin personalizar
  },
}
```

Colores disponibles: `fondo`, `superficie`, `elevado`, `borde`, `texto`,
`texto_tenue`, `acento`, `texto_acento`, `seleccion`, `peligro`, `exito`,
`aviso` y `carpeta`. Mira [`config/temas/ejemplo.lua`](config/temas/ejemplo.lua).
También puedes hacer uno desde *Ajustes → Apariencia → Crear un tema a partir
de este…*, que lo guarda como `.lua`.

## Estructura del proyecto

```
Solar/
├── nucleo/                  núcleo en C
│   ├── include/solar.h      API pública del núcleo
│   ├── src/                 listado, operaciones, papelera, dispositivos_linux,
│   │                        vigilante_linux, carpetas, config, temas, lua_seguro, util
│   ├── pruebas/pruebas.c    176 revisiones (con AddressSanitizer + UBSan)
│   └── Makefile             para trabajar solo en el núcleo
├── src/                     interfaz en Rust
│   ├── ffi.rs, nucleo.rs    enlaces con el núcleo (crudos y seguros)
│   ├── app.rs               estado, navegación y acciones
│   ├── vista.rs             cuadrícula, lista, selección, menús del clic derecho
│   ├── barras.rs            barra superior, ubicación, estado y lateral
│   ├── dialogos.rs          renombrar, propiedades, personalizar carpeta…
│   ├── ajustes.rs           ventana de Ajustes
│   ├── iconos.rs            carpetas y archivos vectoriales (forma del logo)
│   ├── tema.rs              temas → egui, colores en OKLab
│   └── atajos.rs, tareas.rs, formato.rs, fuentes.rs, ui_util.rs
├── config/                  ejemplos de config.lua y de tema
├── recursos/                logo, ícono y solar.desktop
├── build.rs                 compila el núcleo y Lua
└── Makefile                 make / make instalar / make pruebas
```

## Pruebas

```sh
make pruebas                 # pruebas de Rust (también ejercitan el núcleo)
make pruebas-nucleo          # núcleo en C con ASan + UBSan (usa el Lua del sistema)
make -C nucleo pruebas LUA_DIR=/ruta/a/lua-5.4/src   # o con las fuentes de Lua
```

## Limitaciones conocidas (es una alpha)

- Solo Linux por ahora (el núcleo usa inotify, `/proc` y `/sys`).
- No hay pestañas, miniaturas de imágenes ni deshacer todavía.
- Arrastrar archivos *desde* Solar hacia otros programas no está soportado.
  Soltar archivos de otros programas *en* Solar funciona en X11; en Wayland
  aún no (limitación de winit).
- La transparencia necesita un compositor (Niri, Hyprland, KWin, picom…); en
  X11 sin compositor la ventana se ve sólida, un poco más oscura.
- Si `config.lua` tiene lógica (if, variables), Ajustes la convierte en valores
  fijos la próxima vez que cambies algo desde la ventana.

## Ideas para las siguientes versiones

Pestañas, miniaturas, deshacer, búsqueda en subcarpetas, panel doble,
comprimir/extraer, más formatos de logotipo (imágenes propias), y llevar Solar
a Windows y macOS como Capyro.

## Licencia y créditos

Solar es software libre bajo la **GPL-3.0 o posterior** (ver [LICENSE](LICENSE)).

Hecho por **Arturo (CapyIsInsano)**. Usa [egui/eframe](https://github.com/emilk/egui)
(MIT/Apache-2.0), [Lua 5.4](https://www.lua.org) (MIT) e íconos
[Phosphor](https://phosphoricons.com) (MIT).
