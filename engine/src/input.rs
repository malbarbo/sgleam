//! What a keystroke does with the text being typed: whether Enter runs it or
//! opens a line, how far in that line starts, and what a Tab has to offer.
//! The text here is Gleam, so a reader that also takes commands says where the
//! Gleam of an input begins before it asks.

use crate::parser;

/// What the prompt does with the code as it stands and where the cursor is in
/// it: `-1` runs it, and anything else is how far in the line the cursor opens
/// starts, in spaces.
///
/// An input of one line runs from anywhere in it, the way a prompt always has.
/// One of several runs from the end only: with the cursor back in the text,
/// Enter is opening a line inside a block still being written, and running the
/// block because it happens to be finished is not what the key was pressed
/// for.
///
/// An input with nothing open ends at a blank line, finished or not. That is
/// the only way out of an input that will not close. The user can type an open
/// bracket shut, but `let x =` has no bracket to type, and without this rule
/// the user could only erase the line -- while the error the engine gives for
/// it is the answer they want. With a bracket open the rule would cost more
/// than it gives, taking the blank line between two statements of a function
/// for the end of it.
///
/// `cursor` is a byte offset into `src`, as `complete` already takes one. One
/// that falls inside a character moves back to the boundary before it, and one
/// past the end is the end.
pub fn ready_state(src: &str, cursor: usize) -> i32 {
    let cursor = src.floor_char_boundary(cursor);
    // Nothing but whitespace is left after the cursor: the line it opens is
    // the line after the input, which the input alone already answers.
    let at_end = src[cursor..].trim().is_empty();
    let above = if at_end { src } else { &src[..cursor] };
    let depth = parser::nesting_depth(above);
    if at_end || !src.contains('\n') {
        if !parser::is_incomplete(src) {
            return -1;
        }
        if depth == 0
            && let Some((_, last)) = src.rsplit_once('\n')
            && last.trim().is_empty()
        {
            return -1;
        }
    }
    // The brackets open above the cursor say how far in the line goes, and the
    // code written under it says how far in it has to go: a line shallower
    // than that closes a block the code below still needs open. The deeper of
    // the two, so that neither is crossed.
    (depth * INDENT).max(indent_below(src, cursor)) as i32
}

/// How far in the code under the cursor is written, in spaces: the first line
/// below the cursor's own that says something and can be measured. A blank
/// line and a comment say nothing about where the block is written, and a line
/// indented with anything but spaces cannot be measured in them: all three are
/// passed over.
fn indent_below(src: &str, cursor: usize) -> usize {
    let Some((_, below)) = src[cursor..].split_once('\n') else {
        return 0;
    };
    below
        .lines()
        .find_map(|line| {
            let text = line.trim_start_matches(' ');
            (!text.is_empty() && !text.starts_with("//") && !text.starts_with(char::is_whitespace))
                .then(|| line.len() - text.len())
        })
        .unwrap_or(0)
}

/// What one level of indentation is worth, in spaces.
const INDENT: usize = 2;

/// The word around the cursor and where it starts, both in bytes: everything
/// before `cursor`, back to the last char an identifier cannot hold.
pub fn word_at(text: &str, cursor: usize) -> (usize, &str) {
    let before = &text[..text.floor_char_boundary(cursor)];
    let start = before
        .char_indices()
        .rev()
        .find(|(_, c)| is_break_char(*c))
        .map_or(0, |(i, c)| i + c.len_utf8());
    (start, &before[start..])
}

/// Of `names`, the ones that carry on the word around the cursor, and where
/// that word starts. An empty word offers nothing, as every name carries it
/// on.
pub fn complete<'a>(names: &'a [String], text: &str, cursor: usize) -> (usize, Vec<&'a str>) {
    let (start, prefix) = word_at(text, cursor);
    if prefix.is_empty() {
        return (start, vec![]);
    }
    let candidates = names
        .iter()
        .filter(|name| name.starts_with(prefix))
        .map(String::as_str)
        .collect();
    (start, candidates)
}

/// Returns `true` if no name of the language has the char in it, so the word
/// being completed ends at it. `:` and `.` are in a name here, as the commands
/// start with one and the qualified names carry the other.
fn is_break_char(c: char) -> bool {
    !c.is_alphanumeric() && c != '_' && c != ':' && c != '.'
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cursor at the end of the input, which is where the tests written
    /// before there was a cursor put it.
    fn ready_state(src: &str) -> i32 {
        super::ready_state(src, src.len())
    }

    /// The `|` in `marked` is where the cursor is.
    fn at_cursor(marked: &str) -> i32 {
        let (above, below) = marked.split_once('|').expect("the cursor is a `|`");
        super::ready_state(&format!("{above}{below}"), above.len())
    }

    #[test]
    fn the_word_at_the_cursor_is_what_comes_before_it() {
        assert_eq!(word_at("let x = lis", 11), (8, "lis"));
        assert_eq!(word_at("lis", 11), (0, "lis"));
        assert_eq!(word_at("1 + f(x", 7), (6, "x"));
        assert_eq!(word_at(":ty", 3), (0, ":ty"));
        assert_eq!(word_at("list.ma", 7), (0, "list.ma"));
        assert_eq!(word_at("let x = lis", 8), (8, ""));
        assert_eq!(word_at("", 0), (0, ""));
    }

    #[test]
    fn only_the_names_that_carry_the_word_on_are_offered() {
        let names = [
            "list".to_string(),
            "list.map".to_string(),
            "let ".to_string(),
        ];
        assert_eq!(complete(&names, "lis", 3), (0, vec!["list", "list.map"]));
        assert_eq!(complete(&names, "x = list.m", 10), (4, vec!["list.map"]));
        assert_eq!(complete(&names, "nope", 4), (0, vec![]));
        // Every name carries an empty word on, so none is worth offering.
        assert_eq!(complete(&names, "x = ", 4), (4, vec![]));
    }

    #[test]
    fn the_word_survives_the_chars_that_take_more_than_a_byte() {
        // The break char before it is one of them.
        assert_eq!(word_at("x = \"ol\u{e1}\u{2026}foo", 15), (12, "foo"));
        // The cursor is in the middle of one.
        assert_eq!(word_at("ol\u{e1}_bar", 3), (0, "ol"));
        // The word itself is made of them.
        assert_eq!(word_at("ol\u{e1}_bar", 8), (0, "ol\u{e1}_bar"));
    }

    #[test]
    fn a_finished_input_is_run() {
        for input in ["1 + 1", "", "let x = 1", "pub fn f() {\n  1\n}"] {
            assert_eq!(ready_state(input), -1, "{input:?}");
        }
    }

    #[test]
    fn an_unfinished_input_says_how_far_in_the_next_line_starts() {
        // Nothing is open, so the input is waiting on a value, not on a
        // bracket.
        assert_eq!(ready_state("let x ="), 0);
        assert_eq!(ready_state("1 +"), 0);
        // Every kind of bracket is worth a level.
        assert_eq!(ready_state("case x {"), 2);
        assert_eq!(ready_state("io.println("), 2);
        assert_eq!(ready_state("let x = [1,"), 2);
        assert_eq!(ready_state("let x = #(1,"), 2);
        assert_eq!(ready_state("let x = <<1,"), 2);
        assert_eq!(ready_state("pub fn f() {\n  case x {"), 4);
        // And one that is closed is worth none.
        assert_eq!(ready_state("pub fn f() {\n  f(1)\n  ["), 4);
    }

    #[test]
    fn one_line_runs_from_anywhere_in_it_and_several_from_the_end() {
        assert_eq!(at_cursor("let x = |1"), -1);
        // Several lines with the cursor back in them: the key opens a line,
        // finished or not.
        assert_eq!(at_cursor("let x = 1\nlet y = |2"), 0);
    }

    #[test]
    fn the_line_goes_as_deep_as_the_brackets_above_and_the_code_below() {
        // Nothing under the cursor but the closing brace: the depth at the
        // cursor is the whole answer, and it is the canonical one.
        assert_eq!(at_cursor("pub fn f() {\n  let x = 1|\n}"), 2);
        // Code written further in than that is not brought back out by the
        // line the cursor opens: it would leave the line under nothing.
        assert_eq!(
            at_cursor("pub fn f() {\n    let x = 1|\n    let y = 2\n}"),
            4
        );
        // A blank line and a comment say nothing about where the block is.
        assert_eq!(
            at_cursor("pub fn f() {\n    let x = 1|\n\n    // nota\n    let y = 2\n}"),
            4
        );
        // And a cursor past the end is the end.
        assert_eq!(super::ready_state("case x {", 999), 2);
        // A line indented with a tab cannot be measured in spaces, so it is
        // passed over instead of counted as a line written at the margin.
        assert_eq!(
            at_cursor("pub fn f() {\n    let x = 1|\n\tlet y = 2\n    let z = 3\n}"),
            4
        );
    }

    #[test]
    fn a_blank_last_line_ends_an_input_with_nothing_open() {
        // The way out of an input that has no bracket left to type.
        assert_eq!(ready_state("let x =\n"), -1);
        assert_eq!(ready_state("let x = \"abc\n"), -1);
        // A line with something on it is not one of these.
        assert_eq!(ready_state("1 +\n  2 +"), 0);
    }

    #[test]
    fn a_blank_line_inside_brackets_is_part_of_the_input() {
        // A function is written with blank lines between its statements, and
        // reading one as the end of the input cuts it in half.
        assert_eq!(ready_state("pub fn f() {\n  let x = 1\n"), 2);
        assert_eq!(ready_state("case x {\n  "), 2);
        assert_eq!(ready_state("let x = [\n  1,\n\n"), 2);
    }
}
