//! The paths are relative to `engine/`, where cargo runs the tests, since a
//! path names the module the compiler makes of it.

use camino::Utf8PathBuf;

use engine::{
    coverage::{self, RegionKind},
    engine::Engine,
    gleam::{Project, copy_files_and_build, find_imports},
    host::HostOptions,
    quickjs::QuickJsEngine,
};

#[test]
fn the_second_compile_runs_the_marks() {
    let paths = find_imports(vec!["tests/coverage/sign.gleam".into()]).expect("find the imports");
    let mut project = Project::default();
    let built = copy_files_and_build(&mut project, &paths).expect("compile");
    let files = coverage::compile(&mut project, &paths, &built).expect("compile the marks");

    let regions: Vec<_> = files
        .iter()
        .flat_map(|file| {
            file.regions.iter().map(|region| {
                let text = &file.source[region.span.start as usize..region.span.end as usize];
                (
                    file.path.clone(),
                    region.kind,
                    text.lines().next().map(String::from),
                )
            })
        })
        .collect();
    let sign = Utf8PathBuf::from("tests/coverage/sign.gleam");
    let words = Utf8PathBuf::from("tests/coverage/words.gleam");
    let line = |text: &str| Some(text.to_string());
    assert_eq!(
        regions,
        vec![
            (
                sign.clone(),
                RegionKind::Function,
                line("pub fn sign(n: Int) -> String {")
            ),
            (
                sign.clone(),
                RegionKind::Clause,
                line("True -> words.negative()")
            ),
            (
                sign.clone(),
                RegionKind::Clause,
                line("False -> \"positive\"")
            ),
            (
                sign.clone(),
                RegionKind::Function,
                line("pub fn sign_examples() {")
            ),
            (
                words.clone(),
                RegionKind::Function,
                line("pub fn negative() -> String {")
            ),
            (
                words.clone(),
                RegionKind::Function,
                line("pub fn zero() -> String {")
            ),
        ]
    );

    let engine =
        QuickJsEngine::new(project.fs.clone(), HostOptions::default()).expect("start the engine");
    engine
        .run_tests(&["tests/coverage/sign"])
        .expect("run the examples");
    assert_eq!(engine.coverage().expect("read the marks"), vec![0, 1, 3, 4]);
}
