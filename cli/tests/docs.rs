//! Replays the REPL transcripts of the docs, so a change the docs do not
//! follow fails here. Native only: what is checked is the doc, not the backend.

use std::path::Path;

const DOCS: &[&str] = &["../docs/en/cli.md", "../docs/pt-br/cli.md"];

/// What the doc leaves out of a transcript, on a line of its own.
const ELISION: &str = "...";

struct Block<'a> {
    info: &'a str,
    body: String,
}

fn blocks(doc: &str) -> Vec<Block<'_>> {
    let mut blocks = Vec::new();
    let mut lines = doc.lines();
    while let Some(line) = lines.next() {
        let Some(info) = line.strip_prefix("```") else {
            continue;
        };
        let mut body = String::new();
        for line in lines.by_ref() {
            if line == "```" {
                break;
            }
            body.push_str(line);
            body.push('\n');
        }
        blocks.push(Block { info, body });
    }
    blocks
}

/// What the user types (`> `) and what the REPL answers.
fn split(body: &str) -> (String, String) {
    let mut input = String::new();
    let mut output = String::new();
    for line in body.lines() {
        let (dst, line) = match line.strip_prefix("> ") {
            Some(typed) => (&mut input, typed),
            None => (&mut output, line),
        };
        dst.push_str(line);
        dst.push('\n');
    }
    (input, output)
}

/// The file a transcript needs on disk: `sgleam repl <file>` right above it,
/// and the source of the file right above that.
fn fixture<'a>(before: &'a [Block]) -> Option<(&'a str, &'a str)> {
    let [.., source, cmd] = before else {
        return None;
    };
    let name = cmd.body.trim().strip_prefix("sgleam repl ")?;
    (cmd.info == "sh" && source.info == "gleam").then_some((name, source.body.as_str()))
}

fn matches(actual: &str, expected: &str) -> bool {
    let mut rest = actual;
    let mut anchored = true;
    for chunk in expected.split(&format!("\n{ELISION}\n")) {
        let found = if anchored {
            rest.starts_with(chunk).then_some(0)
        } else {
            rest.find(chunk)
        };
        let Some(at) = found else {
            return false;
        };
        rest = &rest[at + chunk.len()..];
        anchored = false;
    }
    // The transcript ends where the output does, unless it ends elided.
    rest.is_empty() || expected.ends_with(&format!("\n{ELISION}\n"))
}

fn run_repl(dir: &Path, file: Option<&str>, input: &str) -> (String, String) {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!();
    cmd.current_dir(dir).args(["repl", "-q"]);
    if let Some(file) = file {
        cmd.arg(file);
    }
    cmd.write_stdin(input.to_string());
    let output = cmd.output().expect("run sgleam");
    (
        String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
        String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n"),
    )
}

#[test]
fn docs_repl_transcripts() {
    for doc in DOCS {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(doc);
        let text = std::fs::read_to_string(&path).expect("read the doc");
        let blocks = blocks(&text);
        let mut replayed = 0;
        for (i, block) in blocks.iter().enumerate() {
            if block.info != "gleam-repl" {
                continue;
            }
            let (input, expected) = split(&block.body);
            let dir = tempfile::tempdir().expect("temp dir");
            let before = blocks.get(..i).expect("the blocks before this one");
            let file = fixture(before).map(|(name, source)| {
                std::fs::write(dir.path().join(name), source).expect("write the fixture");
                name
            });
            let (out, err) = run_repl(dir.path(), file, &input);
            assert!(
                matches(&out, &expected),
                "{doc}, transcript at block {i}:\n--- typed\n{input}--- expected\n{expected}--- got\n{out}{err}"
            );
            replayed += 1;
        }
        assert!(replayed > 0, "no transcript replayed from {doc}");
    }
}
