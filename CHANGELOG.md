# Registro de cambios

## v0.1.0-alpha — 2026-11-08

Primera versión de Solar.

### Carpetas personalizadas
- Color de carpeta: 21 colores listos, selector de color y código hex.
- Logotipo de carpeta: 54 logotipos (Phosphor) o un texto/emoji propio, con
  su propio color (o automático según la carpeta).
- Todas las carpetas se dibujan en vectorial con la forma del logo de Solar.
- El estilo sigue a la carpeta al renombrarla o moverla desde Solar, viaja
  con las copias y se olvida al eliminarla para siempre.

### Apariencia y Ajustes
- 9 temas integrados y temas propios en Lua con herencia (`base = "…"`);
  editor de temas en Ajustes que guarda archivos `.lua`.
- Fondo transparente con opacidad ajustable, escala de la interfaz y
  fuente integrada, del sistema o desde un archivo.
- Barra de ubicación arriba o abajo; barra lateral a la izquierda, a la
  derecha o escondida; barra de título propia con botones de ventana.
- Lugares editables (nombre, ruta, ícono y orden).
- Atajos de teclado configurables con captura de teclas y aviso de choques.

### Archivos
- Vistas de cuadrícula y lista, orden natural sin importar acentos (la ñ va
  después de la n), carpetas primero, ocultos (punto, `~` y `.hidden`).
- Selección con clic, Ctrl, Shift, banda y teclado; búsqueda al escribir.
- Copiar, cortar, pegar, duplicar y mover con progreso, cancelación y
  conflictos (conservar ambos, reemplazar/combinar, omitir).
- Papelera freedesktop (compatible con otros gestores), también en memorias
  USB; restaurar, eliminar para siempre y vaciar.
- Arrastrar y soltar a carpetas, lugares, memorias USB y la barra de ubicación.
- Propiedades con tamaño de carpetas, fechas y permisos editables.
- Abrir, abrir con…, abrir terminal aquí y abrir en ventana nueva.
- Memorias USB y tarjetas SD con espacio libre, montar y expulsar (udisks2).
- La vista se actualiza sola (inotify) y avisa si se desconecta la unidad.

### Núcleo en C
- API en `nucleo/include/solar.h`, Lua 5.4 embebido en un entorno seguro
  (sin io/os.execute/require, con límite de memoria e instrucciones).
- 176 revisiones de pruebas con AddressSanitizer y UBSan.
