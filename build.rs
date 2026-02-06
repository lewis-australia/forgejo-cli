use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    path::Path,
};

use fluent_syntax::ast::{self, Entry};
use heck::{ToPascalCase, ToSnakeCase};

fn main() {
    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        std::env::var("TARGET").unwrap()
    );

    if let Err(e) = generate_fluent() {
        println!("cargo::error={e}");
    }
}

fn generate_fluent() -> eyre::Result<()> {
    println!("cargo::rerun-if-changed=localization/*/messages.ftl");
    let mut localizations: BTreeMap<String, ast::Resource<String>> = BTreeMap::new();
    for entry in std::fs::read_dir("localization")? {
        let entry = entry?;
        let locale_name = entry.file_name().to_string_lossy().replace("\r\n", "\n");
        let messages_path = entry.path().join("messages.ftl");
        let parsed = load_fluent_file(&messages_path)?;
        localizations.insert(locale_name, parsed);
    }

    #[derive(Default)]
    struct Message<'a> {
        args: BTreeSet<&'a str>,
        locales: BTreeMap<&'a str, &'a ast::Pattern<String>>,
    }
    let mut messages: BTreeMap<&str, Message> = BTreeMap::new();
    let mut terms: BTreeMap<&str, Message> = BTreeMap::new();

    for (locale, resource) in &localizations {
        for entry in &resource.body {
            match entry {
                Entry::Message(message) => {
                    let entry = messages.entry(&*message.id.name).or_default();
                    if let Some(pattern) = &message.value {
                        args_from_pattern(&mut entry.args, pattern);
                        entry.locales.insert(locale, pattern);
                    }
                }
                Entry::Term(term) => {
                    let entry = terms.entry(&*term.id.name).or_default();
                    args_from_pattern(&mut entry.args, &term.value);
                    entry.locales.insert(locale, &term.value);
                }
                _ => (),
            }
        }
    }

    let mut impls: BTreeMap<&str, String> = BTreeMap::new();
    let mut impl_all = String::new();

    for (msg_id, msg) in &messages {
        let mut generics = String::new();
        for arg in &msg.args {
            generics.push_str(&arg.to_pascal_case());
            generics.push_str(", ");
        }
        let mut constraints = String::new();
        if !msg.args.is_empty() {
            constraints.push_str("where ");
            for arg in &msg.args {
                constraints.push_str(&arg.to_pascal_case());
                constraints.push_str(": FluentValue, ");
            }
        }
        let name = msg_id.to_snake_case();
        if name.starts_with("test_") {
            writeln!(&mut impl_all, "#[allow(unused)]")?;
        }
        write!(&mut impl_all, "pub fn {name}<{generics}>(f: &mut std::fmt::Formatter<'_>, args: args::{name}<{generics}>)")?;
        writeln!(&mut impl_all, " -> std::fmt::Result {constraints} {{")?;
        writeln!(&mut impl_all, "match &**LOCALE {{")?;
        for (locale, pattern) in &msg.locales {
            let mut imp = impls.entry(locale).or_default();
            write!(&mut imp, "pub fn {name}<{generics}>(f: &mut std::fmt::Formatter<'_>, args: args::{name}<{generics}>)")?;
            writeln!(&mut imp, " -> std::fmt::Result {constraints} {{")?;
            code_from_pattern(&mut imp, pattern)?;
            writeln!(&mut imp, "Ok(())")?;
            writeln!(&mut imp, "}}")?;
            writeln!(
                &mut impl_all,
                "\"{locale}\" => {}::{name}(f, args),",
                locale.to_snake_case()
            )?;
        }
        writeln!(&mut impl_all, "_ => en_us::{name}(f, args),")?;
        writeln!(&mut impl_all, "}}}}")?;
    }

    let mut impls_joined = String::new();
    for (locale, imp) in impls {
        writeln!(
            &mut impls_joined,
            "#[allow(unused_variables, unused_imports)]"
        )?;
        writeln!(&mut impls_joined, "mod {} {{", locale.to_snake_case())?;
        writeln!(
            &mut impls_joined,
            "use super::{{args, FluentValue, functions::{} as functions}};",
            locale.to_snake_case(),
        )?;
        writeln!(&mut impls_joined, "{imp}")?;
        writeln!(&mut impls_joined, "}}")?;
    }

    let mut args = String::new();

    for (name, msg) in messages {
        write!(&mut args, "pub struct {}<", name.to_snake_case())?;
        for param in &msg.args {
            write!(&mut args, "{}, ", param.to_pascal_case())?;
        }
        writeln!(&mut args, "> {{")?;
        for param in &msg.args {
            writeln!(&mut args, "pub {param}: {},", param.to_pascal_case())?;
        }
        writeln!(&mut args, "}}")?;
    }

    let out = format!(
        "

#[allow(non_camel_case_types)]
pub mod args {{
{args}
}}

{impls_joined}
{impl_all}
    "
    );

    let out_path = Path::new(&std::env::var("OUT_DIR")?).join("locale-generated.rs");
    std::fs::write(&out_path, out.as_bytes())?;
    // To be able to review the generated code. It's gitignored.
    std::fs::write("locale-generated.rs", out.as_bytes())?;

    let _ = std::process::Command::new("rustfmt")
        .arg(&out_path)
        .args(["--edition", "2021"])
        .status();

    let _ = std::process::Command::new("rustfmt")
        .arg("locale-generated.rs")
        .args(["--edition", "2021"])
        .status();

    Ok(())
}

fn code_from_pattern(out: &mut String, pattern: &ast::Pattern<String>) -> eyre::Result<()> {
    for element in &pattern.elements {
        match element {
            ast::PatternElement::TextElement { value } => {
                writeln!(out, "f.write_str(\"{}\")?;", value.escape_default())?;
            }
            ast::PatternElement::Placeable { expression } => code_from_expression(out, expression)?,
        }
    }
    Ok(())
}

fn code_from_expression(out: &mut String, expr: &ast::Expression<String>) -> eyre::Result<()> {
    match expr {
        ast::Expression::Select { selector, variants } => {
            write!(out, "let tested_value = ")?;
            match selector {
                ast::InlineExpression::MessageReference { .. }
                | ast::InlineExpression::TermReference { .. }
                | ast::InlineExpression::FunctionReference { .. } => {
                    render_inline_expression(out, selector)?
                }
                _ => render_inline_expression(out, selector)?,
            }
            writeln!(out, ";")?;
            for variant in variants {
                match &variant.key {
                    ast::VariantKey::Identifier { name } => {
                        writeln!(
                            out,
                            "if tested_value.matches_string(\"{}\") {{",
                            name.escape_default()
                        )?;
                    }
                    ast::VariantKey::NumberLiteral { value } => {
                        if value.contains('.') {
                            writeln!(out, "if tested_value.matches_float({value}) {{")?;
                        } else {
                            writeln!(out, "if tested_value.matches_int({value}) {{")?;
                        }
                    }
                }
                code_from_pattern(out, &variant.value)?;
                write!(out, "}} else ")?;
            }
            // `unwrap` is okay because it would fail to parse without a default variant
            let default_variant = variants.iter().find(|v| v.default).unwrap();
            write!(out, "{{")?;
            code_from_pattern(out, &default_variant.value)?;
            writeln!(out, "}}")?;
        }
        ast::Expression::Inline(expr) => {
            write!(out, "(")?;
            render_inline_expression(out, expr)?;
            write!(out, ").display(f)?;")?;
        }
    }
    Ok(())
}

fn render_inline_expression(
    out: &mut String,
    expr: &ast::InlineExpression<String>,
) -> eyre::Result<()> {
    match expr {
        ast::InlineExpression::StringLiteral { value } => {
            writeln!(out, "\"{}\"", value.escape_default())?;
        }
        ast::InlineExpression::NumberLiteral { value } => {
            if value.contains(".") {
                writeln!(out, "{}f64", value)?;
            } else {
                writeln!(out, "{}i64", value)?;
            }
        }
        ast::InlineExpression::VariableReference { id } => {
            writeln!(out, "&args.{}", id.name)?;
        }
        ast::InlineExpression::Placeable { .. } => eyre::bail!("nested placeables unsupported"),
        ast::InlineExpression::FunctionReference { id, arguments } => {
            write!(out, "functions::{} {{ __positional: (", id.name)?;
            for arg in &arguments.positional {
                render_inline_expression(out, arg)?;
                write!(out, ", ")?;
            }
            write!(out, ")")?;
            for arg in &arguments.named {
                write!(out, "{}: ", arg.name.name)?;
                render_inline_expression(out, &arg.value)?;
                write!(out, ", ")?;
            }
            write!(out, "}}")?;
        }
        ast::InlineExpression::MessageReference { id, attribute } => {
            eyre::bail!("todo: function reference")
        }
        ast::InlineExpression::TermReference {
            id,
            attribute,
            arguments,
        } => eyre::bail!("todo: function reference"),
    }
    Ok(())
}

fn args_from_pattern<'a>(args: &mut BTreeSet<&'a str>, pattern: &'a ast::Pattern<String>) {
    for element in &pattern.elements {
        match element {
            ast::PatternElement::Placeable { expression } => args_from_expression(args, expression),
            _ => (),
        }
    }
}

fn args_from_expression<'a>(args: &mut BTreeSet<&'a str>, expr: &'a ast::Expression<String>) {
    match expr {
        ast::Expression::Select { selector, variants } => {
            args_from_inline_expression(args, selector);
            for variant in variants {
                args_from_pattern(args, &variant.value);
            }
        }
        ast::Expression::Inline(expr) => args_from_inline_expression(args, expr),
    }
}

fn args_from_inline_expression<'a>(
    args: &mut BTreeSet<&'a str>,
    expr: &'a ast::InlineExpression<String>,
) {
    match expr {
        ast::InlineExpression::VariableReference { id } => {
            args.insert(&id.name);
        }
        ast::InlineExpression::Placeable { expression } => args_from_expression(args, expression),
        _ => return,
    }
}

fn load_fluent_file(path: &Path) -> eyre::Result<ast::Resource<String>> {
    let fluent_file = std::fs::read_to_string(path)?;
    let parsed = match fluent_syntax::parser::parse(fluent_file.clone()) {
        Ok(ftl) => ftl,
        Err((ftl, errs)) => {
            let pre = "cargo::warning=";
            let yellow = "\x1b[93m";
            let blue = "\x1b[94m";
            let bold = "\x1b[1m";
            let reset_fg = "\x1b[39m";
            let reset = "\x1b[0m";
            for err in errs {
                println!("{pre}{yellow}{bold}warning{reset_fg}: {}{reset}", err.kind);
                let line_number = &fluent_file[..err.pos.start].lines().count() + 1;
                println!("{pre} {blue}-->{reset} {}:{line_number}", path.display());
                if let Some(slice) = err.slice {
                    let mut current_offset = slice.start;
                    println!("{pre}  {bold}{blue}|{reset}");
                    let mut has_last_line = false;
                    for line in fluent_file[slice].trim().lines() {
                        has_last_line = false;
                        println!("{pre}  {bold}{blue}|{reset} {line}");
                        if err.pos.start < current_offset + line.len()
                            && err.pos.end > current_offset
                        {
                            let spaces = err.pos.start.saturating_sub(current_offset);
                            let arrows = (err.pos.end - current_offset).min(line.len()) - spaces;
                            println!("{pre}  {bold}{blue}|{reset} {:spaces$}{bold}{yellow}{:^<arrows$}{reset}", "", "");
                            has_last_line = true;
                        }
                        current_offset += line.len() + 1;
                    }
                    if !has_last_line {
                        println!("{pre}  {bold}{blue}|{reset}");
                    }
                    println!("{pre}");
                }
            }
            ftl
        }
    };
    Ok(parsed)
}
