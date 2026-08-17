use gleam_core::io::{FileSystemReader, memory::InMemoryFileSystem};
use indoc::formatdoc;
use path_clean::clean;

use crate::error::SgleamError;

use std::{
    fmt::Write as _,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use rquickjs::{
    CatchResultExt, CaughtError, Coerced, Context, Ctx, Error, Exception, Function, Module, Object,
    Promise, Result, Runtime, Value,
    context::EvalOptions,
    function::IntoJsFunc,
    loader::{ImportAttributes, Loader, Resolver},
    module::Declared,
    qjs::{JS_GetRuntime, JS_SetMaxStackSize},
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
}

impl Engine for QuickJsEngine {
    // Interrupt uses a global AtomicBool, so only one active engine at a time
    // is correctly supported. Clones share the same JS context via refcount.
    fn new(fs: InMemoryFileSystem) -> std::result::Result<Self, SgleamError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::OnceLock;
            // Installed once for the process, and what came of it kept, so
            // that a second engine is told the same thing as the first and
            // not silently left without a way to be stopped.
            static CTRLC: OnceLock<std::result::Result<(), String>> = OnceLock::new();
            let installed =
                CTRLC.get_or_init(|| ctrlc::set_handler(interrupt).map_err(|err| err.to_string()));
            // Shown as it came: what ctrlc says already names its subject.
            if let Err(err) = installed {
                return Err(SgleamError::Other(err.clone().into()));
            }
        }

        Ok(QuickJsEngine {
            context: create_context(fs)?,
        })
    }

    fn run_main(
        &self,
        module: &str,
        main: MainFunction,
        show_output: bool,
    ) -> std::result::Result<(), SgleamError> {
        run_main(&self.context, module, main, show_output)
    }

    fn has_var(&self, key: &str) -> bool {
        self.context.with(|ctx| {
            ctx.globals()
                .get::<_, Object>("repl_vars")
                .and_then(|vars| vars.contains_key(key))
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
            pub fn now_ms() -> u64;
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

    pub fn now_ms() -> u64 {
        unsafe { ffi::now_ms() }
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
        let filled = unsafe { ffi::load_bitmap_data(buf.as_mut_ptr(), buf.len()) };
        buf.truncate(filled);
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

    pub fn now_ms() -> u64 {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
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
    let extension = Path::new(&path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    let mime = match extension.as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
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
    // JPEG: walk the marker segments up to a start of frame, which is where
    // the dimensions are
    if data.len() >= 2 && data[0..2] == [0xFF, 0xD8] {
        let mut i = 2;
        while i + 1 < data.len() && data[i] == 0xFF {
            match data[i + 1] {
                // A marker may be padded with extra 0xFF bytes.
                0xFF => i += 1,
                // TEM, RST0-7, SOI and EOI carry no segment.
                0x01 | 0xD0..=0xD9 => i += 2,
                // Every kind of frame says its size the same way: length,
                // precision, height, width.
                0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                    if i + 9 > data.len() {
                        break;
                    }
                    let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
                    let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
                    return (w, h);
                }
                // The frame comes before the scan, so there is nothing ahead
                // but entropy-coded data.
                0xDA => break,
                _ => {
                    if i + 4 > data.len() {
                        break;
                    }
                    i += 2 + u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                }
            }
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
    fn jpeg_dimensions() {
        let mut data = vec![0xFF, 0xD8]; // SOI
        data.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x04, b'J', b'F']); // APP0
        data.extend_from_slice(&[0xFF, 0xFF]); // fill bytes before the marker
        data.extend_from_slice(&[0xFF, 0xC1, 0x00, 0x0B, 8]); // SOF1
        data.extend_from_slice(&70u16.to_be_bytes()); // height
        data.extend_from_slice(&80u16.to_be_bytes()); // width
        assert_eq!(image_dimensions(&data), (80, 70));
    }

    #[test]
    fn jpeg_marker_inside_a_segment_is_data() {
        let mut data = vec![0xFF, 0xD8]; // SOI
        // An APP0 whose payload happens to spell a SOF0
        data.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x06, 0xFF, 0xC0, 0x00, 0x00]);
        data.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, 8]); // SOF0
        data.extend_from_slice(&10u16.to_be_bytes()); // height
        data.extend_from_slice(&20u16.to_be_bytes()); // width
        assert_eq!(image_dimensions(&data), (20, 10));
    }

    #[test]
    fn truncated_jpeg() {
        let data = [0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B];
        assert_eq!(image_dimensions(&data), (0, 0));
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
use native::{
    check_interrupt, now_ms, sleep, text_height, text_width, text_x_offset, text_y_offset,
};
#[cfg(target_arch = "wasm32")]
use wasm::{check_interrupt, now_ms, sleep, text_height, text_width, text_x_offset, text_y_offset};

#[cfg(target_arch = "wasm32")]
fn load_bitmap(path: String) -> (f64, f64, String) {
    wasm::load_bitmap(path)
}

pub fn create_context(fs: InMemoryFileSystem) -> Result<Context> {
    let runtime = Runtime::new()?;
    runtime.set_interrupt_handler(Some(Box::new(check_interrupt)));
    let context = Context::full(&runtime)?;
    // Not `Runtime::set_max_stack_size`: since rquickjs 0.12 it disables the
    // check above 16 MiB, and student recursion needs the whole thread.
    context.with(|ctx| unsafe {
        JS_SetMaxStackSize(
            JS_GetRuntime(ctx.as_raw().as_ptr()),
            (STACK_SIZE - 1024 * 1024) as _,
        );
    });
    runtime.set_loader(FileResolver, ScriptLoader { fs });
    context
        .with(|ctx| {
            seed_bigint_flag(&ctx)?;
            add_console(&ctx)?;
            add_sgleam(&ctx)
        })
        .map(|_| context)
}

fn seed_bigint_flag(ctx: &Ctx) -> Result<()> {
    let flag = gleam_core::javascript::is_bigint_enabled();
    ctx.globals().set("__sgleam_bigint", flag)
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
        MainFunction::ReplMain { .. } => "ReplMain",
        MainFunction::Smain => "Smain",
        MainFunction::SmainStdin => "SmainStdin",
        MainFunction::SmainStdinLines => "SmainStdinLines",
    };
    // A place in a generated module is read against the input it was copied
    // from, which is what the lines of each file say. They are declared here
    // and not as each is compiled, so a module that ran nothing is still known
    // by the time an input reaches into it.
    let repl_files = match &main {
        MainFunction::ReplMain { files, .. } => files
            .iter()
            .map(|file| {
                let lines = file
                    .lines
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                format!(r#"["src/{}", [{lines}]]"#, file.path)
            })
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    };
    let code = formatdoc! {r#"
        import {{ try_main }} from "./sgleam/sgleam_ffi.mjs";
        import {{ {name} }} from "./{module}.mjs";
        try_main({name}, "{kind}", {show_output}, [{repl_files}]);
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
        // The script imports its neighbors, so it is named as one of them —
        // without the extension only a loadable module has, which is what
        // keeps an import from ever finding it.
        options.filename = Some(Project::out().join("eval_script").into_string());
        // Caught here, and not handed on as it comes: the error left from an
        // exception only says that there was one. The top level of every module
        // the script imports runs inside this call, and none of it reaches
        // `try_main`, so nothing else is going to say what failed.
        let promise = ctx
            .eval_with_options::<Promise, _>(source, options)
            .catch(&ctx)
            .map_err(script_error)?;
        match promise.finish::<Value>().catch(&ctx) {
            Err(CaughtError::Exception(value)) if is_interrupt(&value) => {
                Err(SgleamError::Interrupted)
            }
            Err(CaughtError::Error(err)) => Err(err.into()),
            Err(_) => Err(SgleamError::UserProgramRuntimeError),
            Ok(_) => Ok(()),
        }
    })
}

/// Read while it can still be read: the message of a caught exception comes
/// from the context it was caught in, so it is turned into text here.
fn script_error(err: CaughtError<'_>) -> SgleamError {
    match &err {
        CaughtError::Exception(exception) if is_interrupt(exception) => SgleamError::Interrupted,
        // Its `Display` ends in a newline, which the one printing it adds.
        _ => SgleamError::Script(err.to_string().trim_end().to_string()),
    }
}

/// What an interruption throws is QuickJS's own InternalError, which is not
/// what a panic saying "interrupted" is.
fn is_interrupt(exception: &Exception) -> bool {
    exception.message() == Some("interrupted".into())
        && exception
            .get("name")
            .is_ok_and(|name: String| name == "InternalError")
}

fn add_console(ctx: &Ctx) -> Result<()> {
    let console = Object::new(ctx.clone())?;
    set_fn(&console, "log", log)?;
    ctx.globals().set("console", console)
}

fn add_sgleam(ctx: &Ctx) -> Result<()> {
    let sgleam = Object::new(ctx.clone())?;
    set_fn(&sgleam, "getline", getline)?;
    set_fn(&sgleam, "print", print_no_newline)?;
    set_fn(&sgleam, "sleep", sleep)?;
    set_fn(&sgleam, "now_ms", now_ms)?;
    #[cfg(target_arch = "wasm32")]
    set_fn(&sgleam, "draw_svg", wasm::draw_svg)?;
    #[cfg(target_arch = "wasm32")]
    set_fn(&sgleam, "get_key_event", wasm::get_key_event)?;
    set_fn(&sgleam, "text_width", text_width)?;
    set_fn(&sgleam, "text_height", text_height)?;
    set_fn(&sgleam, "text_x_offset", text_x_offset)?;
    set_fn(&sgleam, "text_y_offset", text_y_offset)?;
    set_fn(&sgleam, "load_bitmap", |path: String| -> Vec<String> {
        let (w, h, data_uri) = load_bitmap(path);
        vec![w.to_string(), h.to_string(), data_uri]
    })?;
    ctx.globals().set("sgleam", sgleam)
}

/// The property, and also what a stack trace calls the function.
fn set_fn<'js, F, P>(object: &Object<'js>, name: &str, f: F) -> Result<()>
where
    F: IntoJsFunc<'js, P> + 'js,
{
    object.set(
        name,
        Function::new(object.ctx().clone(), f)?.with_name(name)?,
    )
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

fn log(value: Coerced<String>) {
    println!("{}", value.0);
}

fn print_no_newline(s: String) {
    print!("{s}");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

#[derive(Debug)]
struct FileResolver;

impl Resolver for FileResolver {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> Result<String> {
        let dir = Path::new(base).parent().ok_or_else(|| {
            Error::new_resolving_message(base, name, format!("no parent for {base}"))
        })?;
        // The generated modules import each other through `..`, which the file
        // system does not resolve: it looks a path up as the components it is
        // written with.
        Ok(clean(dir.join(name)).to_string_lossy().into())
    }
}

struct ScriptLoader {
    fs: InMemoryFileSystem,
}

impl Loader for ScriptLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        path: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> Result<Module<'js, Declared>> {
        tracing::debug!("Loading {path}");
        let src = self
            .fs
            .read(path.into())
            .map_err(|err| Error::new_loading_message(path, err.to_string()))?;
        Module::declare(ctx.clone(), path, src)
    }
}
