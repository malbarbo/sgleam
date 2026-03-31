use camino::Utf8Path;
use gleam_core::io::{FileSystemReader, memory::InMemoryFileSystem};
use indoc::formatdoc;

use crate::error::SgleamError;

use std::{
    fmt::Write as _,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use rquickjs::{
    Array, CatchResultExt, CaughtError, Context, Ctx, Error, Function, Module, Object, Promise,
    Result, Runtime, Value,
    context::EvalOptions,
    loader::{Loader, Resolver},
    module::Declared,
    qjs::{JS_FreeCString, JS_ToCStringLen},
};

use crate::{
    STACK_SIZE,
    engine::{Engine, MainFunction},
    gleam::Project,
    swriteln,
};

#[derive(Clone)]
pub struct QuickJsEngine {
    context: Context,
    fs: InMemoryFileSystem,
}

impl Engine for QuickJsEngine {
    // Interrupt uses a global AtomicBool, so only one active engine at a time
    // is correctly supported. Clones share the same JS context via refcount.
    fn new(fs: InMemoryFileSystem) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::Once;
            static CTRLC_INIT: Once = Once::new();
            CTRLC_INIT.call_once(|| {
                ctrlc::set_handler(interrupt).expect("Add ctrlc handlers");
            });
        }

        QuickJsEngine {
            context: create_context(fs.clone(), Project::out().into()).unwrap(),
            fs,
        }
    }

    fn run_main(
        &self,
        module: &str,
        main: MainFunction,
        show_output: bool,
    ) -> std::result::Result<(), SgleamError> {
        run_main(&self.context, module, main, show_output)
    }

    fn has_var(&self, index: usize) -> bool {
        self.context.with(|ctx| {
            ctx.globals()
                .get::<_, Array>("repl_vars")
                .map(|a| index < a.len())
                .unwrap_or(false)
        })
    }

    fn run_tests(&self, modules: &[&str]) -> std::result::Result<(), SgleamError> {
        run_tests(&self.context, modules)
    }

    fn interrupt(&self) {
        interrupt();
    }
}

impl QuickJsEngine {
    pub fn dump_module(&self, module: &str) {
        let mut path = String::from("/build/");
        path.push_str(module);
        path.push_str(".mjs");
        let content = self.fs.read(Utf8Path::new(&path)).unwrap();
        println!("{path}\n{content}");
    }
}

static STOP: AtomicBool = AtomicBool::new(false);

pub fn interrupt() {
    STOP.store(true, Ordering::Relaxed);
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    mod ffi {
        #[link(wasm_import_module = "env")]
        unsafe extern "C" {
            pub fn check_interrupt() -> bool;
            pub fn sleep(ms: u64);
            pub fn draw_svg(str: *const u8, len: usize);
            pub fn get_key_event(key: *mut u8, len: usize, modifiers: *mut bool) -> usize;
            pub fn text_width(
                text: *const u8,
                text_len: usize,
                font_css: *const u8,
                font_css_len: usize,
            ) -> f64;
            pub fn text_height(
                text: *const u8,
                text_len: usize,
                font_css: *const u8,
                font_css_len: usize,
            ) -> f64;
            pub fn text_x_offset(
                text: *const u8,
                text_len: usize,
                font_css: *const u8,
                font_css_len: usize,
            ) -> f64;
            pub fn text_y_offset(
                text: *const u8,
                text_len: usize,
                font_css: *const u8,
                font_css_len: usize,
            ) -> f64;
            /// Fetch bitmap, cache it, return data URI length (0 on error).
            pub fn load_bitmap_fetch(path: *const u8, path_len: usize) -> usize;
            /// Read cached width/height.
            pub fn load_bitmap_width() -> f64;
            pub fn load_bitmap_height() -> f64;
            /// Copy cached data URI into buf. Returns bytes written.
            pub fn load_bitmap_data(buf: *mut u8, buf_len: usize) -> usize;
        }
    }

    pub fn check_interrupt() -> bool {
        unsafe { ffi::check_interrupt() }
    }

    pub fn sleep(ms: u64) {
        unsafe { ffi::sleep(ms) };
    }

    pub fn draw_svg(str: String) {
        unsafe { ffi::draw_svg(str.as_ptr(), str.len()) }
    }

    pub fn get_key_event() -> Vec<String> {
        let mut key = [0u8; 32];
        let mut modifiers = [false; 5];
        let result =
            unsafe { ffi::get_key_event(key.as_mut_ptr(), key.len(), modifiers.as_mut_ptr()) };
        if let Some(type_) = ["keypress", "keydown", "keyup"].get(result) {
            let mut ret = vec![
                (*type_).into(),
                String::from_utf8_lossy(&key)
                    .trim_matches(char::from(0))
                    .to_string(),
            ];
            for (on, key) in modifiers
                .iter()
                .zip(&["alt", "ctrl", "shift", "meta", "repeat"])
            {
                if *on {
                    ret.push((*key).into())
                }
            }
            ret
        } else {
            vec![]
        }
    }

    pub fn text_width(text: String, font_css: String) -> f64 {
        unsafe { ffi::text_width(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
    }

    pub fn text_height(text: String, font_css: String) -> f64 {
        unsafe { ffi::text_height(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
    }

    pub fn text_x_offset(text: String, font_css: String) -> f64 {
        unsafe { ffi::text_x_offset(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
    }

    pub fn text_y_offset(text: String, font_css: String) -> f64 {
        unsafe { ffi::text_y_offset(text.as_ptr(), text.len(), font_css.as_ptr(), font_css.len()) }
    }

    pub fn load_bitmap(path: String) -> (f64, f64, String) {
        let data_uri_len = unsafe { ffi::load_bitmap_fetch(path.as_ptr(), path.len()) };
        if data_uri_len == 0 {
            return (0.0, 0.0, String::new());
        }
        let w = unsafe { ffi::load_bitmap_width() };
        let h = unsafe { ffi::load_bitmap_height() };
        let mut buf = vec![0u8; data_uri_len];
        unsafe { ffi::load_bitmap_data(buf.as_mut_ptr(), buf.len()) };
        let data_uri = String::from_utf8_lossy(&buf).into_owned();
        (w, h, data_uri)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::STOP;
    use std::sync::atomic::Ordering;

    pub fn check_interrupt() -> bool {
        STOP.swap(false, Ordering::Relaxed)
    }

    pub fn sleep(ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    #[cfg(feature = "resvg")]
    pub fn text_width(text: String, font_css: String) -> f64 {
        crate::text_metrics::text_width(text, font_css)
    }

    #[cfg(feature = "resvg")]
    pub fn text_height(text: String, font_css: String) -> f64 {
        crate::text_metrics::text_height(text, font_css)
    }

    #[cfg(feature = "resvg")]
    pub fn text_x_offset(text: String, font_css: String) -> f64 {
        crate::text_metrics::text_x_offset(text, font_css)
    }

    #[cfg(feature = "resvg")]
    pub fn text_y_offset(text: String, font_css: String) -> f64 {
        crate::text_metrics::text_y_offset(text, font_css)
    }

    #[cfg(not(feature = "resvg"))]
    fn parse_size(font_css: &str) -> f64 {
        font_css
            .split_whitespace()
            .find_map(|s| s.strip_suffix("px").and_then(|n| n.parse().ok()))
            .unwrap_or(14.0)
    }

    #[cfg(not(feature = "resvg"))]
    pub fn text_width(text: String, font_css: String) -> f64 {
        text.len() as f64 * parse_size(&font_css) * 0.6
    }

    #[cfg(not(feature = "resvg"))]
    pub fn text_height(_text: String, font_css: String) -> f64 {
        parse_size(&font_css)
    }

    #[cfg(not(feature = "resvg"))]
    pub fn text_x_offset(_text: String, _font_css: String) -> f64 {
        0.0
    }

    #[cfg(not(feature = "resvg"))]
    pub fn text_y_offset(_text: String, _font_css: String) -> f64 {
        0.0
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_bitmap(path: String) -> (f64, f64, String) {
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error reading {path}: {e}");
            return (0.0, 0.0, String::new());
        }
    };
    let (w, h) = image_dimensions(&data);
    if w == 0 || h == 0 {
        eprintln!("Error: could not detect image dimensions for {path}");
        return (0.0, 0.0, String::new());
    }
    let mime = if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".gif") {
        "image/gif"
    } else if path.ends_with(".bmp") {
        "image/bmp"
    } else if path.ends_with(".webp") {
        "image/webp"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    };
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
    let data_uri = format!("data:{mime};base64,{b64}");
    (w as f64, h as f64, data_uri)
}

#[cfg(not(target_arch = "wasm32"))]
fn image_dimensions(data: &[u8]) -> (u32, u32) {
    // PNG: bytes 16-23 contain width and height as u32 big-endian
    if data.len() >= 24 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
        let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return (w, h);
    }
    // JPEG: scan for SOF0/SOF2 marker
    if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
        let mut i = 2;
        while i + 9 < data.len() {
            if data[i] != 0xFF {
                i += 1;
                continue;
            }
            let marker = data[i + 1];
            if marker == 0xC0 || marker == 0xC2 {
                let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
                let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
                return (w, h);
            }
            let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
            i += 2 + len;
        }
    }
    // GIF: bytes 6-9 contain width and height as u16 little-endian
    if data.len() >= 10 && &data[0..4] == b"GIF8" {
        let w = u16::from_le_bytes([data[6], data[7]]) as u32;
        let h = u16::from_le_bytes([data[8], data[9]]) as u32;
        return (w, h);
    }
    // BMP: bytes 18-25 contain width and height as i32 little-endian
    if data.len() >= 26 && &data[0..2] == b"BM" {
        let w = i32::from_le_bytes([data[18], data[19], data[20], data[21]]).unsigned_abs();
        let h = i32::from_le_bytes([data[22], data[23], data[24], data[25]]).unsigned_abs();
        return (w, h);
    }
    (0, 0)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod bitmap_tests {
    use super::image_dimensions;

    #[test]
    fn png_dimensions() {
        // Minimal 1x1 PNG header
        let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&[0; 8]); // chunk length + type (IHDR)
        data.extend_from_slice(&10u32.to_be_bytes()); // width
        data.extend_from_slice(&20u32.to_be_bytes()); // height
        assert_eq!(image_dimensions(&data), (10, 20));
    }

    #[test]
    fn gif_dimensions() {
        let mut data = b"GIF89a".to_vec();
        data.extend_from_slice(&30u16.to_le_bytes()); // width
        data.extend_from_slice(&40u16.to_le_bytes()); // height
        assert_eq!(image_dimensions(&data), (30, 40));
    }

    #[test]
    fn bmp_dimensions() {
        let mut data = vec![0; 26];
        data[0] = b'B';
        data[1] = b'M';
        data[18..22].copy_from_slice(&50u32.to_le_bytes()); // width
        data[22..26].copy_from_slice(&60u32.to_le_bytes()); // height
        assert_eq!(image_dimensions(&data), (50, 60));
    }

    #[test]
    fn bmp_negative_height() {
        let mut data = vec![0; 26];
        data[0] = b'B';
        data[1] = b'M';
        data[18..22].copy_from_slice(&50i32.to_le_bytes());
        data[22..26].copy_from_slice(&(-60i32).to_le_bytes()); // top-down
        assert_eq!(image_dimensions(&data), (50, 60));
    }

    #[test]
    fn empty_data() {
        assert_eq!(image_dimensions(&[]), (0, 0));
    }

    #[test]
    fn invalid_data() {
        assert_eq!(image_dimensions(b"not an image"), (0, 0));
    }

    #[test]
    fn truncated_png() {
        let data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        // Only header magic, no IHDR
        assert_eq!(image_dimensions(&data), (0, 0));
    }
}

#[cfg(not(target_arch = "wasm32"))]
use native::{check_interrupt, sleep, text_height, text_width, text_x_offset, text_y_offset};
#[cfg(target_arch = "wasm32")]
use wasm::{check_interrupt, sleep, text_height, text_width, text_x_offset, text_y_offset};

#[cfg(target_arch = "wasm32")]
fn load_bitmap(path: String) -> (f64, f64, String) {
    wasm::load_bitmap(path)
}

pub fn create_context(fs: InMemoryFileSystem, base: PathBuf) -> Result<Context> {
    let runtime = Runtime::new()?;
    runtime.set_max_stack_size(STACK_SIZE - 1024 * 1024);
    runtime.set_interrupt_handler(Some(Box::new(check_interrupt)));
    let context = Context::full(&runtime)?;
    runtime.set_loader(FileResolver { base }, ScriptLoader { fs });
    context
        .with(|ctx| {
            add_console(&ctx)?;
            add_sgleam(&ctx)
        })
        .map(|_| context)
}

pub fn run_main(
    context: &Context,
    module: &str,
    main: MainFunction,
    show_output: bool,
) -> std::result::Result<(), SgleamError> {
    let name = main.name();
    let kind = match &main {
        MainFunction::Main => "Main",
        MainFunction::ReplMain(_) => "ReplMain",
        MainFunction::Smain => "Smain",
        MainFunction::SmainStdin => "SmainStdin",
        MainFunction::SmainStdinLines => "SmainStdinLines",
    };
    let code = formatdoc! {r#"
        import {{ try_main }} from "./sgleam/sgleam_ffi.mjs";
        import {{ {name} }} from "./{module}.mjs";
        try_main({name}, "{kind}", {show_output});
        "#
    };
    run_script(context, code)
}

pub fn run_tests(context: &Context, modules: &[&str]) -> std::result::Result<(), SgleamError> {
    let mut src = String::new();
    swriteln!(
        &mut src,
        r#"import {{ run_tests }} from "./sgleam/sgleam_ffi.mjs";"#
    );
    let mut imports = vec![];
    for module in modules {
        let import = module.replace("/", "_");
        swriteln!(&mut src, r#"import * as {import} from "./{module}.mjs";"#);
        imports.push(import);
    }
    let modules = imports.join(", ");
    swriteln!(&mut src, "run_tests([{modules}]);");
    run_script(context, src)
}

pub fn run_script(context: &Context, source: String) -> std::result::Result<(), SgleamError> {
    context.with(|ctx| {
        let mut options = EvalOptions::default();
        options.global = false;
        let promise = ctx.eval_with_options::<Promise, _>(source, options)?;
        match promise.finish::<Value>().catch(&ctx) {
            Err(CaughtError::Exception(value)) if value.message() == Some("interrupted".into()) => {
                Err(SgleamError::Interrupted)
            }
            Err(CaughtError::Error(err)) => Err(err.into()),
            Err(_) => Err(SgleamError::UserProgramRuntimeError),
            Ok(_) => Ok(()),
        }
    })
}

fn add_console(ctx: &Ctx) -> Result<()> {
    let global = ctx.globals();
    let console = Object::new(ctx.clone())?;
    console.set("log", Function::new(ctx.clone(), log)?.with_name("log")?)?;
    global.set("console", console)?;
    Ok(())
}

fn add_sgleam(ctx: &Ctx) -> Result<()> {
    let global = ctx.globals();
    let sgleam = Object::new(ctx.clone())?;
    sgleam.set(
        "getline",
        Function::new(ctx.clone(), getline)?.with_name("getline")?,
    )?;
    sgleam.set(
        "print",
        Function::new(ctx.clone(), print_no_newline)?.with_name("print")?,
    )?;
    sgleam.set(
        "sleep",
        Function::new(ctx.clone(), sleep)?.with_name("sleep")?,
    )?;
    #[cfg(target_arch = "wasm32")]
    sgleam.set(
        "draw_svg",
        Function::new(ctx.clone(), wasm::draw_svg)?.with_name("draw_svg")?,
    )?;
    #[cfg(target_arch = "wasm32")]
    sgleam.set(
        "get_key_event",
        Function::new(ctx.clone(), wasm::get_key_event)?.with_name("get_key_event")?,
    )?;
    sgleam.set(
        "text_width",
        Function::new(ctx.clone(), text_width)?.with_name("text_width")?,
    )?;
    sgleam.set(
        "text_height",
        Function::new(ctx.clone(), text_height)?.with_name("text_height")?,
    )?;
    sgleam.set(
        "text_x_offset",
        Function::new(ctx.clone(), text_x_offset)?.with_name("text_x_offset")?,
    )?;
    sgleam.set(
        "text_y_offset",
        Function::new(ctx.clone(), text_y_offset)?.with_name("text_y_offset")?,
    )?;
    sgleam.set(
        "load_bitmap",
        Function::new(ctx.clone(), move |path: String| -> Vec<String> {
            let (w, h, data_uri) = load_bitmap(path);
            vec![w.to_string(), h.to_string(), data_uri]
        })?
        .with_name("load_bitmap")?,
    )?;
    global.set("sgleam", sgleam)?;
    Ok(())
}

fn getline() -> Option<String> {
    let mut buffer = String::new();
    let stdin = std::io::stdin();
    match stdin.read_line(&mut buffer) {
        Ok(0) => None,
        Ok(_) => {
            if buffer.ends_with('\n') {
                buffer.pop();
                if buffer.ends_with('\r') {
                    buffer.pop();
                }
            }
            Some(buffer)
        }
        Err(err) => {
            eprintln!("{}", err);
            None
        }
    }
}

fn log(value: Value) {
    let ctx_ptr = value.ctx().as_raw().as_ptr();
    let raw = value.as_raw();
    let mut len = std::mem::MaybeUninit::uninit();
    let ptr = unsafe { JS_ToCStringLen(ctx_ptr, len.as_mut_ptr(), raw) };
    assert!(!ptr.is_null());
    let len = unsafe { len.assume_init() };
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(ptr as _, len as _) };
    let s = std::str::from_utf8(bytes).unwrap_or("");
    println!("{s}");
    unsafe { JS_FreeCString(ctx_ptr, ptr) };
}

fn print_no_newline(s: String) {
    print!("{s}");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

#[derive(Debug)]
struct FileResolver {
    pub(crate) base: PathBuf,
}

impl Resolver for FileResolver {
    fn resolve(&mut self, _ctx: &Ctx, base: &str, name: &str) -> Result<String> {
        let result = if base == "eval_script" {
            self.base.join(name.strip_prefix("./").unwrap_or(name))
        } else {
            resolve_path(
                &Path::new(base)
                    .parent()
                    .ok_or_else(|| {
                        Error::new_resolving_message(base, name, format!("no parent for {base}"))
                    })?
                    .join(name),
            )
        };
        Ok(result.to_string_lossy().into())
    }
}

fn resolve_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            Component::ParentDir => {
                if let Some(Component::Normal(_)) = components.last() {
                    components.pop();
                }
            }
            Component::CurDir => {}
            _ => {
                components.push(component);
            }
        }
    }

    components.iter().collect()
}

struct ScriptLoader {
    fs: InMemoryFileSystem,
}

impl Loader for ScriptLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, path: &str) -> Result<Module<'js, Declared>> {
        tracing::debug!("Loading {path}");
        let src = self
            .fs
            .read(path.into())
            .map_err(|err| Error::new_loading_message(path, err.to_string()))?;
        Module::declare(ctx.clone(), path, src)
    }
}
