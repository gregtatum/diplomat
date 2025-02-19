use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::Write;

use diplomat_core::ast;
use indenter::indented;

#[cfg(test)]
#[macro_use]
mod test_util;

mod types;

mod structs;
use structs::*;

mod conversions;

pub mod docs;

mod util;

static RUNTIME_HPP: &str = include_str!("runtime.hpp");

pub fn gen_bindings(
    env: &HashMap<ast::Path, HashMap<String, ast::ModSymbol>>,
    outs: &mut HashMap<String, String>,
) -> fmt::Result {
    super::c::gen_bindings(env, outs)?;

    let diplomat_runtime_out = outs
        .entry("diplomat_runtime.hpp".to_string())
        .or_insert_with(String::new);
    write!(diplomat_runtime_out, "{}", RUNTIME_HPP)?;

    let all_types = crate::util::get_all_custom_types(env);

    for (in_path, typ) in &all_types {
        let out = outs
            .entry(format!("{}.hpp", typ.name()))
            .or_insert_with(String::new);

        writeln!(out, "#ifndef {}_HPP", typ.name())?;
        writeln!(out, "#define {}_HPP", typ.name())?;

        writeln!(out, "#include <stdint.h>")?;
        writeln!(out, "#include <stddef.h>")?;
        writeln!(out, "#include <stdbool.h>")?;
        writeln!(out, "#include <algorithm>")?;
        writeln!(out, "#include <memory>")?;
        writeln!(out, "#include <optional>")?;
        writeln!(out, "#include <variant>")?;
        writeln!(out, "#include \"diplomat_runtime.hpp\"")?;
        writeln!(out)?;
        writeln!(out, "namespace capi {{")?;
        writeln!(out, "#include \"{}.h\"", typ.name())?;
        writeln!(out, "}}")?;

        writeln!(out)?;

        let mut seen_includes = HashSet::new();
        seen_includes.insert(format!("#include \"{}.hpp\"", typ.name()));

        match typ {
            ast::CustomType::Opaque(_) => {}

            ast::CustomType::Enum(enm) => {
                writeln!(out)?;
                writeln!(out, "enum struct {} {{", enm.name)?;
                let mut enm_indent = indented(out).with_str("  ");
                for (name, discriminant, _) in enm.variants.iter() {
                    writeln!(&mut enm_indent, "{} = {},", name, discriminant)?;
                }
                writeln!(out, "}};")?;
            }

            ast::CustomType::Struct(strct) => {
                for (_, typ, _) in &strct.fields {
                    gen_includes(
                        typ,
                        in_path,
                        true,
                        false,
                        true,
                        env,
                        &mut seen_includes,
                        out,
                    )?;
                }
            }
        }

        for method in typ.methods() {
            for param in &method.params {
                gen_includes(
                    &param.ty,
                    in_path,
                    true,
                    false,
                    false,
                    env,
                    &mut seen_includes,
                    out,
                )?;
            }

            if let Some(return_type) = method.return_type.as_ref() {
                gen_includes(
                    return_type,
                    in_path,
                    true,
                    false,
                    false,
                    env,
                    &mut seen_includes,
                    out,
                )?;
            }
        }

        match typ {
            ast::CustomType::Opaque(_) => {
                writeln!(out)?;
                gen_struct(typ, in_path, true, env, out)?;
            }

            ast::CustomType::Enum(_) => {}

            ast::CustomType::Struct(_) => {
                writeln!(out)?;
                gen_struct(typ, in_path, true, env, out)?;
            }
        }

        writeln!(out)?;

        for method in typ.methods() {
            for param in &method.params {
                gen_includes(
                    &param.ty,
                    in_path,
                    false,
                    false,
                    false,
                    env,
                    &mut seen_includes,
                    out,
                )?;
            }

            if let Some(return_type) = method.return_type.as_ref() {
                gen_includes(
                    return_type,
                    in_path,
                    false,
                    false,
                    false,
                    env,
                    &mut seen_includes,
                    out,
                )?;
            }
        }

        match typ {
            ast::CustomType::Opaque(_) => {
                writeln!(out)?;
                gen_struct(typ, in_path, false, env, out)?;
            }

            ast::CustomType::Enum(_) => {}

            ast::CustomType::Struct(_) => {
                writeln!(out)?;
                gen_struct(typ, in_path, false, env, out)?;
            }
        }

        writeln!(out, "#endif")?
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn gen_includes<W: fmt::Write>(
    typ: &ast::TypeName,
    in_path: &ast::Path,
    pre_struct: bool,
    behind_ref: bool,
    for_field: bool,
    env: &HashMap<ast::Path, HashMap<String, ast::ModSymbol>>,
    seen_includes: &mut HashSet<String>,
    out: &mut W,
) -> fmt::Result {
    match typ {
        ast::TypeName::Named(_) => {
            let (_, custom_typ) = typ.resolve_with_path(in_path, env);
            match custom_typ {
                ast::CustomType::Opaque(_) => {
                    if pre_struct {
                        let decl = format!("class {};", custom_typ.name());
                        if !seen_includes.contains(&decl) {
                            writeln!(out, "{}", decl)?;
                            seen_includes.insert(decl);
                        }
                    } else {
                        let include = format!("#include \"{}.hpp\"", custom_typ.name());
                        if !seen_includes.contains(&include) {
                            writeln!(out, "{}", include)?;
                            seen_includes.insert(include);
                        }
                    }
                }

                ast::CustomType::Struct(_) => {
                    if pre_struct && (!for_field || behind_ref) {
                        let decl = format!("struct {};", custom_typ.name());
                        if !seen_includes.contains(&decl) {
                            writeln!(out, "{}", decl)?;
                            seen_includes.insert(decl);
                        }
                    } else {
                        let include = format!("#include \"{}.hpp\"", custom_typ.name());
                        if !seen_includes.contains(&include) {
                            writeln!(out, "{}", include)?;
                            seen_includes.insert(include);
                        }
                    }
                }

                ast::CustomType::Enum(_) => {
                    let include = format!("#include \"{}.hpp\"", custom_typ.name());
                    if !seen_includes.contains(&include) {
                        writeln!(out, "{}", include)?;
                        seen_includes.insert(include);
                    }
                }
            }
        }
        ast::TypeName::Box(underlying) => {
            gen_includes(
                underlying,
                in_path,
                pre_struct,
                true,
                for_field,
                env,
                seen_includes,
                out,
            )?;
        }
        ast::TypeName::Reference(underlying, _) => {
            gen_includes(
                underlying,
                in_path,
                pre_struct,
                true,
                for_field,
                env,
                seen_includes,
                out,
            )?;
        }
        ast::TypeName::Primitive(_) => {}
        ast::TypeName::Option(underlying) => {
            gen_includes(
                underlying,
                in_path,
                pre_struct,
                behind_ref,
                for_field,
                env,
                seen_includes,
                out,
            )?;
        }
        ast::TypeName::Result(ok, err) => {
            gen_includes(
                ok.as_ref(),
                in_path,
                pre_struct,
                behind_ref,
                for_field,
                env,
                seen_includes,
                out,
            )?;

            gen_includes(
                err.as_ref(),
                in_path,
                pre_struct,
                behind_ref,
                for_field,
                env,
                seen_includes,
                out,
            )?;
        }
        ast::TypeName::Writeable => {}
        ast::TypeName::StrReference => {}
        ast::TypeName::Unit => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cross_module_struct_fields() {
        test_file! {
            #[diplomat::bridge]
            mod mod1 {
                use super::mod2::Bar;

                struct Foo {
                    x: Bar,
                }
            }

            #[diplomat::bridge]
            mod mod2 {
                use super::mod1::Foo;

                struct Bar {
                    y: Box<Foo>,
                }
            }
        }
    }

    #[test]
    fn test_cross_module_struct_methods() {
        test_file! {
            #[diplomat::bridge]
            mod mod1 {
                use super::mod2::Bar;

                #[diplomat::opaque]
                struct Foo;

                impl Foo {
                    fn to_bar(&self) -> Bar {
                        unimplemented!()
                    }
                }
            }

            #[diplomat::bridge]
            mod mod2 {
                use super::mod1::Foo;

                struct Bar {
                    y: Box<Foo>,
                }
            }
        }
    }
}
