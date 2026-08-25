// This file is from gleam project
// compiler-cli/src/main.rs, except the level: gleam takes a whole filter with
// `with_env_filter`, which needs a feature of tracing-subscriber this build
// leaves out. Here `GLEAM_LOG` holds a level, such as `trace`, and anything
// else falls back to the default.

pub fn initialise_logger() {
    let enable_colours = std::env::var("GLEAM_LOG_NOCOLOUR").is_err();
    let level = std::env::var("GLEAM_LOG")
        .ok()
        .and_then(|l| l.parse().ok())
        .unwrap_or(tracing::Level::ERROR);
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(level)
        .with_target(false)
        .with_ansi(enable_colours)
        .without_time()
        .init();
}
