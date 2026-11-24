//! The paths are relative to `engine/`, where cargo runs the tests, since a
//! path names the module the compiler makes of it.

use camino::Utf8PathBuf;

use engine::{
    coverage::{self, RegionKind},
    engine::Engine,
    error::SgleamError,
    gleam::{Project, copy_files_and_build, find_imports},
    host::HostOptions,
    quickjs::QuickJsEngine,
    run::run_test_coverage,
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

/// Each region as its kind, its first line and whether it ran.
fn ran(files: &[coverage::File]) -> Vec<(RegionKind, String, bool)> {
    files
        .iter()
        .flat_map(|file| {
            file.regions.iter().map(|region| {
                let text = &file.source[region.span.start as usize..region.span.end as usize];
                let line = text.lines().next().unwrap_or_default().to_string();
                (region.kind, line, region.ran)
            })
        })
        .collect()
}

#[test]
fn a_test_run_says_which_regions_ran() {
    let user_files = vec![Utf8PathBuf::from("tests/coverage/sign.gleam")];
    let paths = find_imports(user_files.clone()).expect("find the imports");
    let (files, result) = run_test_coverage(&user_files, &paths, HostOptions::default());
    result.expect("run the examples");
    assert_eq!(
        ran(&files),
        vec![
            (
                RegionKind::Function,
                "pub fn sign(n: Int) -> String {".into(),
                true
            ),
            (RegionKind::Clause, "True -> words.negative()".into(), true),
            (RegionKind::Clause, "False -> \"positive\"".into(), false),
            (
                RegionKind::Function,
                "pub fn sign_examples() {".into(),
                true
            ),
            (
                RegionKind::Function,
                "pub fn negative() -> String {".into(),
                true
            ),
            (
                RegionKind::Function,
                "pub fn zero() -> String {".into(),
                false
            ),
        ]
    );
}

#[test]
fn a_failed_example_and_a_panic_keep_the_regions() {
    let user_files = vec![Utf8PathBuf::from("tests/coverage/half.gleam")];
    let (files, result) = run_test_coverage(&user_files, &user_files, HostOptions::default());
    assert!(result.is_err());
    assert_eq!(
        ran(&files),
        vec![
            (
                RegionKind::Function,
                "pub fn half(n: Int) -> Int {".into(),
                true
            ),
            (RegionKind::Clause, "True -> n / 2".into(), true),
            (RegionKind::Clause, "False -> panic as \"odd\"".into(), true),
            (
                RegionKind::Function,
                "pub fn half_examples() {".into(),
                true
            ),
            (RegionKind::Function, "pub fn odd_examples() {".into(), true),
        ]
    );
}

#[test]
fn a_program_that_does_not_compile_has_no_regions() {
    let user_files = vec![Utf8PathBuf::from("tests/coverage/broken.gleam")];
    let (files, result) = run_test_coverage(&user_files, &user_files, HostOptions::default());
    assert!(matches!(result, Err(SgleamError::Gleam(_))));
    assert!(files.is_empty());
}
