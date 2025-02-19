use std::fmt::Write;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use diplomat_core::ast;
use indenter::indented;

use super::types::{gen_type, name_for_type};

pub fn collect_results<'a>(
    typ: &'a ast::TypeName,
    in_path: &ast::Path,
    env: &HashMap<ast::Path, HashMap<String, ast::ModSymbol>>,
    seen: &mut HashSet<(ast::Path, &'a ast::TypeName)>,
    results: &mut Vec<(ast::Path, &'a ast::TypeName)>,
) {
    match typ {
        ast::TypeName::Named(_) => {}
        ast::TypeName::Box(underlying) => {
            collect_results(underlying, in_path, env, seen, results);
        }
        ast::TypeName::Reference(underlying, _) => {
            collect_results(underlying, in_path, env, seen, results);
        }
        ast::TypeName::Primitive(_) => {}
        ast::TypeName::Option(underlying) => {
            collect_results(underlying, in_path, env, seen, results);
        }
        ast::TypeName::Result(ok, err) => {
            let seen_key = (in_path.clone(), typ);
            if !seen.contains(&seen_key) {
                seen.insert(seen_key.clone());
                collect_results(ok, in_path, env, seen, results);
                collect_results(err, in_path, env, seen, results);
                results.push(seen_key);
            }
        }
        ast::TypeName::Writeable => {}
        ast::TypeName::StrReference => {}
        ast::TypeName::Unit => {}
    }
}

pub fn gen_result<W: fmt::Write>(
    typ: &ast::TypeName,
    in_path: &ast::Path,
    env: &HashMap<ast::Path, HashMap<String, ast::ModSymbol>>,
    out: &mut W,
) -> fmt::Result {
    if let ast::TypeName::Result(ok, err) = typ {
        let result_name = format!("{}_{}", in_path.elements.join("_"), name_for_type(typ));
        writeln!(out, "typedef struct {} {{", result_name)?;
        let mut result_indent = indented(out).with_str("    ");
        writeln!(&mut result_indent, "union {{")?;
        let mut union_indent = indented(&mut result_indent).with_str("    ");

        if let ast::TypeName::Unit = ok.as_ref() {
            writeln!(&mut union_indent, "uint8_t ok[0];")?;
        } else {
            gen_type(
                ok,
                in_path,
                env,
                &mut ((&mut union_indent) as &mut dyn fmt::Write),
            )?;
            writeln!(&mut union_indent, " ok;")?;
        }

        if let ast::TypeName::Unit = err.as_ref() {
            writeln!(&mut union_indent, "uint8_t err[0];")?;
        } else {
            gen_type(
                err,
                in_path,
                env,
                &mut ((&mut union_indent) as &mut dyn fmt::Write),
            )?;
            writeln!(&mut union_indent, " err;")?;
        }
        writeln!(&mut result_indent, "}};")?;
        writeln!(&mut result_indent, "bool is_ok;")?;
        writeln!(out, "}} {};", result_name)?;

        Ok(())
    } else {
        panic!()
    }
}
