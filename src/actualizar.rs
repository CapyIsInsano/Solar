//! Actualizaciones desde GitHub, sin desinstalar ni reinstalar.
//!
//! 1. Se piden las versiones publicadas (releases; si no hay, las etiquetas)
//!    del repo de `config.lua` (por defecto CapyIsInsano/Solar).
//! 2. Si hay una más nueva:
//!    - si la versión trae un binario para Linux de esta arquitectura
//!      (un archivo con "linux" y "x86_64" en el nombre), se descarga;
//!    - si no, se descarga el código y se compila con cargo (la compilación
//!      se guarda en ~/.cache/solar, así la siguiente tarda mucho menos).
//! 3. El ejecutable nuevo reemplaza al actual en su lugar (rename atómico).
//!    Si está en una carpeta del sistema se pide permiso con pkexec; si no
//!    hay pkexec, Solar te da el comando para copiarlo tú.
//!
//! Para todo esto se usan `curl`, `tar` y `cargo` del sistema.

use crate::nucleo;
use eframe::egui;
use std::cmp::Ordering as Orden;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ----------------------------------------------------------------------
//  Versiones
// ----------------------------------------------------------------------

/// "v0.2.0-beta.1" → ([0, 2, 0], rango del sufijo). Sin sufijo es la final.
#[derive(Clone, Debug)]
pub struct NumeroVersion {
    numeros: Vec<u64>,
    sufijo: Option<(u8, u64)>, // (alpha=0, beta=1, rc=2, otro=0), número
}

impl NumeroVersion {
    pub fn leer(texto: &str) -> Option<Self> {
        let t = texto.trim().trim_start_matches(['v', 'V']);
        let (base, sufijo) = match t.split_once(['-', '+']) {
            Some((b, s)) => (b, Some(s)),
            None => (t, None),
        };
        let numeros: Vec<u64> = base.split('.').map(|n| n.parse().ok()).collect::<Option<_>>()?;
        if numeros.is_empty() {
            return None;
        }
        let sufijo = sufijo.map(|s| {
            let s = s.to_ascii_lowercase();
            let rango = if s.starts_with("rc") {
                2
            } else if s.starts_with("beta") || s.starts_with('b') {
                1
            } else {
                0
            };
            let numero = s.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0);
            (rango, numero)
        });
        Some(Self { numeros, sufijo })
    }
}

impl PartialEq for NumeroVersion {
    fn eq(&self, otro: &Self) -> bool {
        self.cmp(otro) == Orden::Equal
    }
}

impl Eq for NumeroVersion {}

impl PartialOrd for NumeroVersion {
    fn partial_cmp(&self, otro: &Self) -> Option<Orden> {
        Some(self.cmp(otro))
    }
}

impl Ord for NumeroVersion {
    fn cmp(&self, otro: &Self) -> Orden {
        let n = self.numeros.len().max(otro.numeros.len());
        for i in 0..n {
            let a = self.numeros.get(i).copied().unwrap_or(0);
            let b = otro.numeros.get(i).copied().unwrap_or(0);
            if a != b {
                return a.cmp(&b);
            }
        }
        match (&self.sufijo, &otro.sufijo) {
            (None, None) => Orden::Equal,
            (None, Some(_)) => Orden::Greater, // 1.0 > 1.0-rc
            (Some(_), None) => Orden::Less,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Version {
    pub etiqueta: String,
    pub nombre: String,
    pub notas: String,
    pub fecha: String,
    pub previa: bool,
    /// URL de un binario para esta arquitectura, si la versión lo trae
    pub binario: Option<(String, String)>, // (nombre, url)
    pub fuente: String,
}

#[derive(Clone, Debug)]
pub enum Estado {
    Nada,
    Buscando,
    AlDia,
    Disponible(Box<Version>),
    Trabajando { paso: String, fraccion: Option<f32>, detalle: String },
    /// Ya quedó; falta reiniciar Solar
    Listo { version: String },
    /// No pudimos escribir el ejecutable; el usuario puede copiar el comando
    Permiso { version: String, comando: String },
    Error(String),
}

pub struct Actualizador {
    pub estado: Arc<Mutex<Estado>>,
    cancelar: Arc<AtomicBool>,
    hijo: Arc<Mutex<Option<Child>>>,
}

impl Default for Actualizador {
    fn default() -> Self {
        Self { estado: Arc::new(Mutex::new(Estado::Nada)), cancelar: Arc::new(AtomicBool::new(false)), hijo: Arc::default() }
    }
}

pub fn dir_cache() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| nucleo::dir_hogar().join(".cache"))
        .join("solar")
}

fn ahora() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// ¿Ya toca revisar? (una vez al día)
pub fn toca_revisar() -> bool {
    let marca = dir_cache().join("ultima-revision");
    let antes = std::fs::read_to_string(&marca).ok().and_then(|t| t.trim().parse::<u64>().ok()).unwrap_or(0);
    ahora().saturating_sub(antes) > 20 * 3600
}

fn marcar_revision() {
    let dir = dir_cache();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("ultima-revision"), ahora().to_string());
}

fn existe_programa(nombre: &str) -> bool {
    Command::new(nombre)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Ejecuta y devuelve stdout, o el error en español.
fn correr(cmd: &mut Command) -> Result<Vec<u8>, String> {
    let salida = cmd.stdin(Stdio::null()).output().map_err(|e| format!("No se pudo ejecutar {:?}: {e}", cmd.get_program()))?;
    if salida.status.success() {
        Ok(salida.stdout)
    } else {
        let err = String::from_utf8_lossy(&salida.stderr).trim().to_string();
        Err(if err.is_empty() { format!("{:?} terminó con error ({})", cmd.get_program(), salida.status) } else { err })
    }
}

fn curl_json(url: &str) -> Result<serde_json::Value, String> {
    let datos = correr(
        Command::new("curl")
            .args(["-fsSL", "--max-time", "20", "-H", "Accept: application/vnd.github+json"])
            .args(["-A", &format!("Solar/{VERSION}")])
            .arg(url),
    )
    .map_err(|e| {
        if e.contains("404") {
            "No encontré el repositorio en GitHub (¿el nombre está bien y es público?).".to_string()
        } else if e.contains("403") || e.contains("429") {
            "GitHub dice que hicimos demasiadas preguntas; intenta en un rato.".to_string()
        } else if e.contains("resolve host") || e.contains("Could not resolve") || e.contains("connect") {
            "No hay conexión con GitHub. Revisa tu internet.".to_string()
        } else {
            format!("No se pudo consultar GitHub: {e}")
        }
    })?;
    serde_json::from_slice(&datos).map_err(|e| format!("GitHub respondió algo que no entiendo: {e}"))
}

fn arquitectura_coincide(nombre: &str) -> bool {
    let n = nombre.to_ascii_lowercase();
    let arch = std::env::consts::ARCH;
    let alias: &[&str] = match arch {
        "x86_64" => &["x86_64", "amd64", "x64"],
        "aarch64" => &["aarch64", "arm64"],
        otra => &[otra][..],
    };
    n.contains("linux") && alias.iter().any(|a| n.contains(a)) && !n.ends_with(".sha256") && !n.ends_with(".sig") && !n.ends_with(".asc")
}

/// Dirección de la API de GitHub (SOLAR_API_GITHUB la cambia, para pruebas
/// o para un espejo).
fn api() -> String {
    std::env::var("SOLAR_API_GITHUB")
        .ok()
        .filter(|u| u.starts_with("http"))
        .map(|u| u.trim_end_matches('/').to_string())
        .unwrap_or_else(|| "https://api.github.com".into())
}

/// Pide las versiones a GitHub y devuelve la más nueva (si hay alguna).
pub fn version_mas_nueva(repo: &str) -> Result<Option<Version>, String> {
    if !repo.contains('/') || repo.contains(char::is_whitespace) {
        return Err(format!("«{repo}» no parece un repo de GitHub (debe ser usuario/repo)."));
    }
    if !existe_programa("curl") {
        return Err("Para buscar actualizaciones necesito curl (net-misc/curl en Gentoo).".into());
    }
    let mut versiones = Vec::new();
    let lista = curl_json(&format!("{}/repos/{repo}/releases?per_page=30", api()))?;
    for r in lista.as_array().into_iter().flatten() {
        if r["draft"].as_bool().unwrap_or(false) {
            continue;
        }
        let etiqueta = r["tag_name"].as_str().unwrap_or_default().to_string();
        if etiqueta.is_empty() {
            continue;
        }
        let binario = r["assets"].as_array().into_iter().flatten().find_map(|a| {
            let nombre = a["name"].as_str()?;
            arquitectura_coincide(nombre).then(|| (nombre.to_string(), a["browser_download_url"].as_str().unwrap_or_default().to_string()))
        });
        versiones.push(Version {
            nombre: r["name"].as_str().filter(|n| !n.trim().is_empty()).unwrap_or(&etiqueta).to_string(),
            notas: r["body"].as_str().unwrap_or_default().replace("\r\n", "\n"),
            fecha: r["published_at"].as_str().unwrap_or_default().chars().take(10).collect(),
            previa: r["prerelease"].as_bool().unwrap_or(false),
            binario,
            fuente: r["tarball_url"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| format!("https://github.com/{repo}/archive/refs/tags/{etiqueta}.tar.gz")),
            etiqueta,
        });
    }
    if versiones.is_empty() {
        // Sin releases: miramos las etiquetas (git tag vX.Y.Z && git push --tags)
        let tags = curl_json(&format!("{}/repos/{repo}/tags?per_page=50", api()))?;
        for t in tags.as_array().into_iter().flatten() {
            let Some(etiqueta) = t["name"].as_str() else { continue };
            versiones.push(Version {
                nombre: etiqueta.to_string(),
                notas: String::new(),
                fecha: String::new(),
                previa: etiqueta.contains('-'),
                binario: None,
                fuente: t["tarball_url"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("https://github.com/{repo}/archive/refs/tags/{etiqueta}.tar.gz")),
                etiqueta: etiqueta.to_string(),
            });
        }
    }
    let actual = NumeroVersion::leer(VERSION).expect("la versión de Cargo.toml es válida");
    Ok(versiones
        .into_iter()
        .filter_map(|v| NumeroVersion::leer(&v.etiqueta).map(|n| (n, v)))
        .filter(|(n, _)| *n > actual)
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, v)| v))
}

// ----------------------------------------------------------------------
//  Aplicar
// ----------------------------------------------------------------------

impl Actualizador {
    pub fn estado(&self) -> Estado {
        self.estado.lock().map(|e| e.clone()).unwrap_or(Estado::Nada)
    }

    fn poner(estado: &Arc<Mutex<Estado>>, ctx: &egui::Context, nuevo: Estado) {
        if let Ok(mut e) = estado.lock() {
            *e = nuevo;
        }
        ctx.request_repaint();
    }

    pub fn ocupado(&self) -> bool {
        matches!(self.estado(), Estado::Buscando | Estado::Trabajando { .. })
    }

    /// Busca en segundo plano. `al_terminar(hay_nueva)` corre en el hilo.
    pub fn buscar(&self, repo: String, ctx: egui::Context) {
        if self.ocupado() {
            return;
        }
        let estado = self.estado.clone();
        Self::poner(&estado, &ctx, Estado::Buscando);
        std::thread::Builder::new()
            .name("solar-actualizar".into())
            .spawn(move || {
                let nuevo = match version_mas_nueva(&repo) {
                    Ok(Some(v)) => Estado::Disponible(Box::new(v)),
                    Ok(None) => Estado::AlDia,
                    Err(e) => Estado::Error(e),
                };
                marcar_revision();
                Self::poner(&estado, &ctx, nuevo);
            })
            .ok();
    }

    pub fn cancelar(&self) {
        self.cancelar.store(true, Ordering::Relaxed);
        if let Ok(mut h) = self.hijo.lock()
            && let Some(c) = h.as_mut()
        {
            let _ = c.kill();
        }
    }

    pub fn aplicar(&self, version: Version, ctx: egui::Context) {
        if self.ocupado() {
            return;
        }
        self.cancelar.store(false, Ordering::Relaxed);
        let estado = self.estado.clone();
        let cancelar = self.cancelar.clone();
        let hijo = self.hijo.clone();
        Self::poner(&estado, &ctx, Estado::Trabajando { paso: "Preparando…".into(), fraccion: None, detalle: String::new() });
        std::thread::Builder::new()
            .name("solar-actualizar".into())
            .spawn(move || {
                let paso = |p: &str, f: Option<f32>, d: &str| {
                    Self::poner(&estado, &ctx, Estado::Trabajando { paso: p.into(), fraccion: f, detalle: d.into() })
                };
                let resultado = conseguir_binario(&version, &paso, &cancelar, &hijo)
                    .and_then(|nuevo| reemplazar(&nuevo, &version.etiqueta));
                let final_ = match resultado {
                    _ if cancelar.load(Ordering::Relaxed) => Estado::Disponible(Box::new(version)),
                    Ok(None) => Estado::Listo { version: version.etiqueta.clone() },
                    Ok(Some(comando)) => Estado::Permiso { version: version.etiqueta.clone(), comando },
                    Err(e) => Estado::Error(e),
                };
                Self::poner(&estado, &ctx, final_);
            })
            .ok();
    }
}

fn descargar(url: &str, destino: &Path, hijo: &Arc<Mutex<Option<Child>>>) -> Result<(), String> {
    if let Some(d) = destino.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let c = Command::new("curl")
        .args(["-fsSL", "--max-time", "600", "-o"])
        .arg(destino)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("No se pudo ejecutar curl: {e}"))?;
    esperar(c, hijo).map(|_| ()).map_err(|e| format!("No se pudo descargar: {e}"))
}

/// Espera a un proceso guardándolo para poder cancelarlo.
fn esperar(c: Child, hijo: &Arc<Mutex<Option<Child>>>) -> Result<String, String> {
    if let Ok(mut h) = hijo.lock() {
        *h = Some(c);
    }
    loop {
        let terminado = hijo.lock().ok().and_then(|mut h| h.as_mut().and_then(|c| c.try_wait().ok().flatten()));
        if let Some(st) = terminado {
            let mut err = String::new();
            if let Ok(mut h) = hijo.lock()
                && let Some(mut c) = h.take()
                && let Some(mut e) = c.stderr.take()
            {
                use std::io::Read;
                let _ = e.read_to_string(&mut err);
            }
            return if st.success() { Ok(err) } else { Err(if err.trim().is_empty() { st.to_string() } else { err.trim().to_string() }) };
        }
        std::thread::sleep(std::time::Duration::from_millis(80));
    }
}

fn verificar(bin: &Path) -> Result<(), String> {
    let salida = Command::new(bin).arg("--version").stdin(Stdio::null()).output().map_err(|e| format!("El programa nuevo no arranca: {e}"))?;
    let texto = String::from_utf8_lossy(&salida.stdout);
    if salida.status.success() && texto.starts_with("Solar ") {
        Ok(())
    } else {
        Err("El programa descargado no parece ser Solar; no lo instalé.".into())
    }
}

fn conseguir_binario(
    v: &Version,
    paso: &dyn Fn(&str, Option<f32>, &str),
    cancelar: &AtomicBool,
    hijo: &Arc<Mutex<Option<Child>>>,
) -> Result<PathBuf, String> {
    let cache = dir_cache();
    let descargas = cache.join("descargas");
    let _ = std::fs::create_dir_all(&descargas);
    if !existe_programa("curl") {
        return Err("Necesito curl para descargar la actualización.".into());
    }

    // a) La versión trae un binario listo
    if let Some((nombre, url)) = &v.binario {
        paso(&format!("Descargando {nombre}…"), None, "");
        let archivo = descargas.join(nombre);
        descargar(url, &archivo, hijo)?;
        if cancelar.load(Ordering::Relaxed) {
            return Err("Cancelado".into());
        }
        let bin = if nombre.ends_with(".tar.gz") || nombre.ends_with(".tgz") || nombre.ends_with(".tar.xz") {
            let dir = descargas.join(format!("binario-{}", v.etiqueta));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            correr(Command::new("tar").arg("xf").arg(&archivo).arg("-C").arg(&dir))?;
            buscar_ejecutable(&dir).ok_or("El paquete descargado no trae el ejecutable «solar».")?
        } else {
            archivo
        };
        set_ejecutable(&bin)?;
        verificar(&bin)?;
        return Ok(bin);
    }

    // b) Compilar desde el código
    if !existe_programa("cargo") {
        return Err("Esta versión no trae binario y para compilarla necesito cargo (Rust 1.95 o más nuevo).".into());
    }
    paso("Descargando el código…", None, &v.fuente);
    let tar = descargas.join(format!("solar-{}.tar.gz", v.etiqueta));
    descargar(&v.fuente, &tar, hijo)?;
    let fuente = cache.join("fuente");
    let _ = std::fs::remove_dir_all(&fuente);
    std::fs::create_dir_all(&fuente).map_err(|e| e.to_string())?;
    correr(Command::new("tar").arg("xzf").arg(&tar).arg("-C").arg(&fuente))?;
    let raiz = std::fs::read_dir(&fuente)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.join("Cargo.toml").is_file())
        .ok_or("El código descargado no tiene Cargo.toml.")?;
    if cancelar.load(Ordering::Relaxed) {
        return Err("Cancelado".into());
    }

    let destino = cache.join("compilacion");
    let total_previo = std::fs::read_to_string(cache.join("crates")).ok().and_then(|t| t.trim().parse::<f32>().ok()).unwrap_or(260.0);
    paso("Compilando… (la primera vez tarda unos minutos)", Some(0.0), "");
    let mut c = Command::new("cargo")
        .args(["build", "--release", "--color", "never"])
        .current_dir(&raiz)
        .env("CARGO_TARGET_DIR", &destino)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("No se pudo ejecutar cargo: {e}"))?;
    let stderr = c.stderr.take();
    if let Ok(mut h) = hijo.lock() {
        *h = Some(c);
    }
    let mut ultimas: Vec<String> = Vec::new();
    let mut compilados = 0.0f32;
    if let Some(e) = stderr {
        for linea in BufReader::new(e).lines().map_while(Result::ok) {
            let l = linea.trim().to_string();
            if l.starts_with("Compiling ") {
                compilados += 1.0;
                let que = l.trim_start_matches("Compiling ").split(' ').next().unwrap_or_default().to_string();
                paso("Compilando…", Some((compilados / total_previo).min(0.97)), &que);
            }
            ultimas.push(l);
            if ultimas.len() > 12 {
                ultimas.remove(0);
            }
        }
    }
    let estado = hijo.lock().ok().and_then(|mut h| h.take()).and_then(|mut c| c.wait().ok());
    if cancelar.load(Ordering::Relaxed) {
        return Err("Cancelado".into());
    }
    if !estado.is_some_and(|s| s.success()) {
        return Err(format!("No compiló. Lo último que dijo cargo:\n{}", ultimas.join("\n")));
    }
    if compilados > 20.0 {
        let _ = std::fs::write(cache.join("crates"), (compilados as u32).to_string());
    }
    let bin = destino.join("release/solar");
    verificar(&bin)?;
    Ok(bin)
}

fn buscar_ejecutable(dir: &Path) -> Option<PathBuf> {
    walk(dir).into_iter().find(|e| e.file_name().is_some_and(|n| n == "solar") && e.is_file())
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                v.extend(walk(&p));
            } else {
                v.push(p);
            }
        }
    }
    v
}

fn set_ejecutable(p: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())
}

/// El ejecutable que está corriendo (aunque ya lo hayamos reemplazado).
pub fn ejecutable_actual() -> Option<PathBuf> {
    let e = std::env::current_exe().ok()?;
    Some(PathBuf::from(e.to_string_lossy().trim_end_matches(" (deleted)").to_string()))
}

/// Pone el binario nuevo en lugar del actual. Ok(None) si quedó; Ok(Some(cmd))
/// si hace falta que el usuario lo copie con permisos.
fn reemplazar(nuevo: &Path, etiqueta: &str) -> Result<Option<String>, String> {
    let actual = ejecutable_actual().ok_or("No sé dónde está instalado Solar.")?;
    let dir = actual.parent().ok_or("No sé dónde está instalado Solar.")?;
    let tmp = dir.join(format!(".solar-{}.nuevo", std::process::id()));
    let copia = std::fs::copy(nuevo, &tmp).and_then(|_| {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
        std::fs::rename(&tmp, &actual)
    });
    match copia {
        Ok(()) => Ok(None),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(30) => {
            let _ = std::fs::remove_file(&tmp);
            // Guardamos una copia fuera del caché de compilación y pedimos permiso
            let listo = dir_cache().join(format!("solar-{etiqueta}"));
            std::fs::copy(nuevo, &listo).map_err(|e| e.to_string())?;
            set_ejecutable(&listo)?;
            let comando = format!("sudo install -m 755 '{}' '{}'", listo.display(), actual.display());
            if existe_programa("pkexec") || Command::new("sh").args(["-c", "command -v pkexec"]).output().is_ok_and(|o| o.status.success()) {
                let ok = Command::new("pkexec")
                    .arg("install")
                    .args(["-m", "755"])
                    .arg(&listo)
                    .arg(&actual)
                    .stdin(Stdio::null())
                    .status()
                    .is_ok_and(|s| s.success());
                if ok {
                    return Ok(None);
                }
            }
            Ok(Some(comando))
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(format!("No se pudo reemplazar {}: {e}", actual.display()))
        }
    }
}

/// Abre la versión nueva (en la misma carpeta) y cierra esta.
pub fn reiniciar(ctx: &egui::Context, carpeta: Option<&Path>) -> Result<(), String> {
    let exe = ejecutable_actual().ok_or("No sé dónde está Solar.")?;
    let mut c = Command::new(&exe);
    if let Some(d) = carpeta {
        c.arg(d);
    }
    c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    Ok(())
}

// ----------------------------------------------------------------------
//  Caché
// ----------------------------------------------------------------------

pub fn tamano_cache() -> u64 {
    fn medir(p: &Path) -> u64 {
        let Ok(m) = std::fs::symlink_metadata(p) else { return 0 };
        if m.is_dir() {
            std::fs::read_dir(p).map(|rd| rd.flatten().map(|e| medir(&e.path())).sum()).unwrap_or(0)
        } else {
            m.len()
        }
    }
    medir(&dir_cache())
}

/// Borra ~/.cache/solar (descargas y compilaciones de actualizaciones).
pub fn borrar_cache() -> Result<u64, String> {
    let dir = dir_cache();
    let antes = tamano_cache();
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("No se pudo borrar {}: {e}", dir.display()))?;
    }
    // La marca de "ya revisé hoy" no es caché de verdad: la dejamos
    marcar_revision();
    Ok(antes)
}

#[cfg(test)]
mod pruebas {
    use super::NumeroVersion as N;

    #[test]
    fn comparar_versiones() {
        let v = |t: &str| N::leer(t).unwrap();
        assert!(v("v0.2.0") > v("0.1.0-alpha"));
        assert!(v("0.1.0") > v("0.1.0-alpha"));
        assert!(v("0.1.0-beta") > v("0.1.0-alpha"));
        assert!(v("0.1.0-rc1") > v("0.1.0-beta.3"));
        assert!(v("0.1.0-alpha.2") > v("0.1.0-alpha"));
        assert!(v("0.1.1-alpha") > v("0.1.0"));
        assert!(v("1.0") == v("1.0.0"));
        assert!(N::leer("nightly").is_none());
    }

    #[test]
    fn binarios_por_arquitectura() {
        if std::env::consts::ARCH == "x86_64" {
            assert!(super::arquitectura_coincide("solar-0.2.0-linux-x86_64.tar.gz"));
            assert!(super::arquitectura_coincide("Solar-linux-amd64"));
            assert!(!super::arquitectura_coincide("solar-linux-x86_64.tar.gz.sha256"));
            assert!(!super::arquitectura_coincide("solar-windows-x86_64.exe"));
        }
    }
}
