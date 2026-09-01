//! The names a repl session has in scope, and the imports that put them in
//! front of a generated module.

use std::{
    collections::{BTreeMap, HashSet},
    fmt::Write,
    rc::Rc,
};

use ecow::EcoString;
use gleam_core::{
    ast::{AssignName, Import, SrcSpan, UnqualifiedImport},
    build::Module,
    parse,
    type_::printer::Names,
};

use crate::{source::Source, swriteln};

/// Where a name in scope comes from: the module that defines it and the name
/// it has there. The repl reaches every name this way, a saved value
/// included.
#[derive(Clone)]
pub struct NameEntry {
    module: String,
    original: String,
    origin: Origin,
}

/// How the name entered the session, which decides when a generated module
/// imports it and what a const may do with it.
#[derive(Clone)]
pub enum Origin {
    /// An import brought it, or the user's module seeded it.
    Import,
    /// An input defined it.
    Def,
    /// A `let` bound it, and a binding at the top of every body that names it
    /// reads it back. A const cannot read this kind of value.
    Binding,
}

impl NameEntry {
    pub fn new(module: &str, original: impl AsRef<str>, origin: Origin) -> NameEntry {
        NameEntry {
            module: module.into(),
            original: original.as_ref().into(),
            origin,
        }
    }

    pub fn is_binding(&self) -> bool {
        matches!(self.origin, Origin::Binding)
    }
}

/// The import the input just wrote and everything it brings, so the repl can
/// write the input's own line instead of one of its own.
#[derive(Clone)]
pub struct OwnImport {
    input: Rc<str>,
    span: SrcSpan,
    path: String,
    alias: Option<String>,
    values: Vec<String>,
    types: Vec<String>,
}

/// What an input defines itself, which its own module imports from nowhere.
/// One list per namespace, as a type and a value of the same name are two
/// names.
#[derive(Default)]
pub struct Defined {
    pub types: Vec<String>,
    pub values: Vec<String>,
}

/// The names the session has in scope, and the modules that hold them.
#[derive(Clone, Default)]
pub struct Scope {
    // One map per Gleam namespace, so `import gleam/list`, `type list` and
    // `fn list()` all coexist. BTreeMap and not HashMap so the generated
    // imports come out in a stable order, and so do the diagnostics about them.
    /// `import gleam/int as i` → "i" → "gleam/int"
    pub modules: BTreeMap<String, String>,
    pub types: BTreeMap<String, NameEntry>,
    pub values: BTreeMap<String, NameEntry>,
    /// The modules this session wrote that hold names: one for each input
    /// that defines a name, one for each item that binds one.
    pub own_modules: Vec<String>,
}

impl Scope {
    /// Seeds the project module's public names one by one, instead of a single
    /// blanket import, so that later definitions can shadow them.
    pub fn seed_module(&mut self, module: &Module) {
        let path = module.name.to_string();
        let interface = &module.ast.type_info;

        self.modules
            .insert(short_name(&path).to_string(), path.clone());

        for type_ in interface.public_type_names() {
            self.types.insert(
                type_.to_string(),
                NameEntry::new(&path, type_, Origin::Import),
            );
        }

        for value in interface.public_value_names() {
            self.values.insert(
                value.to_string(),
                NameEntry::new(&path, value, Origin::Import),
            );
        }
    }

    /// Registers everything `import` brings, and hands back the input's own
    /// line, which goes into the modules the repl builds while it checks the
    /// import.
    pub fn register_import(
        &mut self,
        import: &Import<()>,
        input: &Rc<str>,
        span: SrcSpan,
    ) -> OwnImport {
        let module = import.module.to_string();
        let mut own = OwnImport {
            input: input.clone(),
            span,
            path: module.clone(),
            alias: None,
            values: vec![],
            types: vec![],
        };

        match &import.as_name {
            Some((AssignName::Variable(name), _)) => {
                self.modules.insert(name.to_string(), module.clone());
                own.alias = Some(name.to_string());
            }
            // `as _` brings in the unqualified names only.
            Some((AssignName::Discard(_), _)) => {}
            None => {
                self.modules
                    .insert(short_name(&module).to_string(), module.clone());
                own.alias = Some(short_name(&module).to_string());
            }
        }

        register_unqualified(
            &import.unqualified_values,
            &module,
            &mut self.values,
            &mut own.values,
        );
        register_unqualified(
            &import.unqualified_types,
            &module,
            &mut self.types,
            &mut own.types,
        );

        own
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.values
            .keys()
            .chain(self.types.keys())
            .chain(self.modules.keys())
            .map(String::as_str)
    }

    /// Writes what the input has in scope: the modules and the names taken
    /// from them, `skip` aside — the names the module defines itself. Nothing
    /// here carries an annotation, so a later redefinition cannot change what
    /// the module reads.
    ///
    /// Only the names `code` mentions come in, so an input does not pay for
    /// the whole scope. `None` writes them all, which is what checks an import.
    ///
    /// `own` is the import the input just wrote, if it wrote one.
    pub fn write_imports(
        &self,
        src: &mut Source,
        code: Option<&str>,
        skip: &Defined,
        own: Option<&OwnImport>,
    ) {
        let mentioned = code.map(mentioned);
        let mentions = |name: &str| mentioned.as_ref().is_some_and(|names| names.contains(name));
        let wanted = |name: &str| mentioned.is_none() || mentions(name);
        // The input's own import goes in as the input wrote it, so a diagnostic
        // about it is about a copy and not about a line the repl rebuilt.
        if let Some(own) = own {
            src.copy(&own.input, own.span);
            src.write("\n");
        }
        for (name, path) in &self.modules {
            if own.is_some_and(|own| own.alias.as_deref() == Some(name) && &own.path == path) {
                continue;
            }
            if !wanted(name) {
                continue;
            }
            if short_name(path) == name {
                swriteln!(src, "import {path}");
            } else {
                swriteln!(src, "import {path} as {name}");
            }
        }
        // A module of this session comes in when the input names it, unless an
        // import already writes the line — under that name, or of that module.
        for module in &self.own_modules {
            if mentions(module)
                && !self
                    .modules
                    .iter()
                    .any(|(name, path)| name == module || path == module)
            {
                swriteln!(src, "import {module}");
            }
        }
        for (kind, values, entries, skip) in [
            ("", true, &self.values, &skip.values),
            ("type ", false, &self.types, &skip.types),
        ] {
            for (name, entry) in entries {
                let NameEntry {
                    module, original, ..
                } = entry;
                let needed = wanted(name);
                let written = own.is_some_and(|own| {
                    &own.path == module
                        && if values { &own.values } else { &own.types }.contains(name)
                });
                if skip.contains(name) || !needed || written {
                    continue;
                }
                if name == original {
                    swriteln!(src, "import {module}.{{{kind}{original}}} as _");
                } else {
                    swriteln!(src, "import {module}.{{{kind}{original} as {name}}} as _");
                }
            }
        }
    }

    /// The bindings a body reads, written ahead of the user's text. A `let` is
    /// a function of its companion module, and reading it back is what makes
    /// the name mean the value. At the first statement the scope holds the
    /// module level names and the parameters, so leaving those out — and what
    /// this input defines — is all that keeps a binding from shadowing.
    pub fn injections(&self, code: &str, defined: &[String], params: &[String]) -> String {
        let mentioned = mentioned(code);
        let mut src = String::new();
        for (name, entry) in &self.values {
            if entry.is_binding()
                && mentioned.contains(name.as_str())
                && !defined.contains(name)
                && !params.contains(name)
            {
                swriteln!(src, "let {name} = {name}()");
            }
        }
        src
    }

    /// Registers the session's types into `names`, for printing. Registering
    /// takes the plain name away from whatever held it, so the plain name lands
    /// on the newest definition, which is where the user expects it.
    pub fn register_types(&self, names: &mut Names) {
        for (name, entry) in &self.types {
            names.named_type_in_scope(
                entry.module.as_str().into(),
                entry.original.as_str().into(),
                name.into(),
            );
        }
    }
}

/// The names the source mentions, as the lexer reads them, a label and a local
/// included. Over-approximate on purpose, as a missing import is an error.
fn mentioned(code: &str) -> HashSet<EcoString> {
    parse::lexer::make_tokenizer(code)
        .filter_map(|token| match token {
            Ok((
                _,
                parse::token::Token::Name { name } | parse::token::Token::UpName { name },
                _,
            )) => Some(name),
            _ => None,
        })
        .collect()
}

/// Registers the unqualified names an import brings, each under the name the
/// input gave it.
fn register_unqualified(
    imported: &[UnqualifiedImport],
    module: &str,
    entries: &mut BTreeMap<String, NameEntry>,
    own: &mut Vec<String>,
) {
    for import in imported {
        let name = import.used_name().to_string();
        own.push(name.clone());
        entries.insert(name, NameEntry::new(module, &import.name, Origin::Import));
    }
}

fn short_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{self, ReplItem};
    use gleam_core::ast::Definition;

    fn imports(scope: &Scope, code: Option<&str>) -> String {
        imports_skipping(scope, code, &Defined::default())
    }

    fn imports_skipping(scope: &Scope, code: Option<&str>, skip: &Defined) -> String {
        imports_owning(scope, code, skip, None)
    }

    fn imports_owning(
        scope: &Scope,
        code: Option<&str>,
        skip: &Defined,
        own: Option<&OwnImport>,
    ) -> String {
        let mut src = Source::new();
        scope.write_imports(&mut src, code, skip, own);
        src.as_str().into()
    }

    fn def(module: &str, name: &str) -> NameEntry {
        NameEntry::new(module, name, Origin::Def)
    }

    #[test]
    fn a_definition_comes_in_only_when_mentioned() {
        let mut scope = Scope::default();
        scope.values.insert("f".into(), def("repl1", "f"));
        assert_eq!(imports(&scope, Some("f() + 1")), "import repl1.{f} as _\n");
        assert_eq!(imports(&scope, Some("g() + 1")), "");
        assert_eq!(imports(&scope, None), "import repl1.{f} as _\n");
    }

    #[test]
    fn an_imported_name_comes_in_only_when_mentioned() {
        let mut scope = Scope::default();
        scope.values.insert(
            "max".into(),
            NameEntry::new("gleam/int", "max", Origin::Import),
        );
        scope.types.insert(
            "Order".into(),
            NameEntry::new("gleam/order", "Order", Origin::Import),
        );
        assert_eq!(imports(&scope, Some("1 + 1")), "");
        assert_eq!(
            imports(&scope, Some("max(1, 2)")),
            "import gleam/int.{max} as _\n"
        );
        assert_eq!(
            imports(&scope, Some("Order")),
            "import gleam/order.{type Order} as _\n"
        );
    }

    #[test]
    fn a_renamed_name_keeps_its_original_beside_it() {
        let mut scope = Scope::default();
        scope.values.insert(
            "m".into(),
            NameEntry::new("gleam/int", "max", Origin::Import),
        );
        assert_eq!(imports(&scope, None), "import gleam/int.{max as m} as _\n");
    }

    #[test]
    fn what_the_input_defines_is_left_out_by_namespace() {
        let mut scope = Scope::default();
        scope.types.insert("T".into(), def("repl1", "T"));
        scope.values.insert("T".into(), def("repl1", "T"));
        let skip = Defined {
            types: vec!["T".into()],
            values: vec![],
        };
        assert_eq!(
            imports_skipping(&scope, Some("T"), &skip),
            "import repl1.{T} as _\n"
        );
    }

    #[test]
    fn an_own_module_comes_in_by_name_unless_an_import_writes_it() {
        let mut scope = Scope::default();
        scope.own_modules.push("repl1".into());
        assert_eq!(imports(&scope, Some("repl1.x()")), "import repl1\n");
        assert_eq!(imports(&scope, Some("1 + 1")), "");
        scope.modules.insert("r".into(), "repl1".into());
        assert_eq!(
            imports(&scope, Some("repl1.x() + r.x()")),
            "import repl1 as r\n"
        );
    }

    #[test]
    fn the_inputs_own_import_goes_in_as_its_own_line() {
        let input: Rc<str> = "import gleam/int.{max} as i".into();
        let ReplItem::ReplDefinition(targeted, _) = parser::parse_repl(&input)
            .expect("parse the import")
            .swap_remove(0)
        else {
            panic!("an import");
        };
        let Definition::Import(import) = &targeted.definition else {
            panic!("an import");
        };
        let mut scope = Scope::default();
        let own = scope.register_import(import, &input, SrcSpan::new(0, input.len() as u32));
        assert_eq!(
            imports_owning(&scope, None, &Defined::default(), Some(&own)),
            "import gleam/int.{max} as i\n"
        );
        // The input that wrote the import is over; a later one writes the
        // names it brought.
        assert_eq!(
            imports(&scope, None),
            "import gleam/int as i\nimport gleam/int.{max} as _\n"
        );
    }

    #[test]
    fn a_binding_is_read_back_unless_something_closer_names_it() {
        let mut scope = Scope::default();
        scope
            .values
            .insert("x".into(), NameEntry::new("repl1", "x", Origin::Binding));
        scope.values.insert("y".into(), def("repl2", "y"));
        assert_eq!(scope.injections("x + y", &[], &[]), "let x = x()\n");
        assert_eq!(scope.injections("y", &[], &[]), "");
        assert_eq!(scope.injections("x", &["x".into()], &[]), "");
        assert_eq!(scope.injections("x", &[], &["x".into()]), "");
    }

    #[test]
    fn mentioned_reads_every_name_the_lexer_sees() {
        let names = mentioned("let x = Foo(bar) // baz");
        assert!(names.contains("x"));
        assert!(names.contains("Foo"));
        assert!(names.contains("bar"));
        assert!(!names.contains("let"));
        assert!(!names.contains("baz"));
    }
}
