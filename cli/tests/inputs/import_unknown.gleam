/// A module no file provides is for the compiler to report, at the import that
/// named it, and not for the search of the imports to stop on.
import nowhere/at_all

pub fn main() {
  at_all.thing()
}
