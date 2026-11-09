-- ☀ Tema de ejemplo para Solar
--
-- Copia este archivo a ~/.config/solar/temas/ con otro nombre (por ejemplo
-- bosque.lua) y aparecerá en Ajustes → Apariencia. El nombre del archivo es
-- el "id" del tema: solo letras sin acento, números, _ y -.
--
-- Es Lua normal: puedes usar variables y cuentas. Los colores van como
-- "#RRGGBB" (o "#RGB"). Lo que no pongas se hereda del tema «base».
--
-- Temas integrados que puedes usar como base:
--   sol, aurora, eclipse, nebulosa, venus, neptuno, marte, mercurio, luna

local verde        = "#22C55E"
local verde_oscuro = "#15803D"

return {
  nombre = "Bosque",
  base   = "venus",
  oscuro = true,

  colores = {
    -- fondo         = "#0A1514",  -- fondo de la vista de archivos
    -- superficie    = "#0F1F1D",  -- barras (lateral, herramientas, estado)
    -- elevado       = "#162B28",  -- menús, diálogos y botones
    -- borde         = "#22403B",  -- líneas y bordes
    -- texto         = "#E6F4F1",  -- texto principal
    -- texto_tenue   = "#8BAFA9",  -- texto secundario
    acento           = verde,      -- botones, selección, interruptores
    -- texto_acento  = "#042F2E",  -- texto encima del acento
    -- seleccion     = verde,      -- (si no lo pones, sigue al acento)
    -- peligro       = "#F87171",  -- eliminar
    -- exito         = "#4ADE80",
    -- aviso         = "#FACC15",
    carpeta          = verde_oscuro, -- color de las carpetas sin personalizar
  },
}
