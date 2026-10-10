//! Times the cases in `benches/image`. Each case is a Gleam program that
//! times itself, so the compile and the start of the engine stay out of the
//! numbers. `cargo bench --bench image -- scene` runs only the cases whose
//! name holds `scene`.

use camino::Utf8PathBuf;

use engine::{gleam::find_imports, run::run_main};

fn main() {
    let filter: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with('-'))
        .collect();

    // A path names its module, so the cases run from their own directory,
    // where `import bench` finds the helper.
    std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/benches/image"))
        .expect("the directory of the cases");

    let mut cases: Vec<Utf8PathBuf> = std::fs::read_dir(".")
        .expect("the list of the cases")
        .filter_map(|entry| Utf8PathBuf::from_path_buf(entry.ok()?.path()).ok())
        .filter_map(|path| Some(Utf8PathBuf::from(path.file_name()?)))
        .filter(|path| path.extension() == Some("gleam") && path != "bench.gleam")
        .filter(|path| filter.is_empty() || filter.iter().any(|f| path.as_str().contains(f)))
        .collect();
    cases.sort();

    // The engine expects the stack of the thread that the binary runs it on.
    let thread = std::thread::Builder::new()
        .stack_size(engine::STACK_SIZE)
        .spawn(move || {
            for case in cases {
                println!("# {case}");
                let paths = find_imports(vec![case.clone()]).expect("the imports of the case");
                if let Err(err) = run_main(&paths) {
                    println!("{case} failed: {err}");
                }
            }
        })
        .expect("a thread for the engine");
    thread.join().expect("the cases to finish");
}
