use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub struct IconNames {
    pub component: String,
    pub module: String,
    pub module_ref: String,
}

pub fn icon_names(stem: &str) -> Result<IconNames> {
    if let Some(component) = override_component_name(stem) {
        return names_from_component(component);
    }

    let parts = ordered_parts(stem)?;
    let component = parts
        .iter()
        .map(|part| pascal_segment(part))
        .collect::<String>();

    if !is_valid_rust_ident(&component) {
        bail!("{stem}.svg produced invalid Rust component identifier `{component}`");
    }

    let module = parts.join("_").replace('-', "_");
    if !is_valid_rust_module_ident(&module) {
        bail!("{stem}.svg produced invalid Rust module identifier `{module}`");
    }

    let module_ref = if is_rust_keyword(&module) {
        format!("r#{module}")
    } else {
        module.clone()
    };

    Ok(IconNames {
        component,
        module,
        module_ref,
    })
}

fn names_from_component(component: &str) -> Result<IconNames> {
    if !is_valid_rust_ident(component) {
        bail!("override produced invalid Rust component identifier `{component}`");
    }

    let module = component_to_module(component);
    let module_ref = if is_rust_keyword(&module) {
        format!("r#{module}")
    } else {
        module.clone()
    };

    Ok(IconNames {
        component: component.to_owned(),
        module,
        module_ref,
    })
}

fn ordered_parts(stem: &str) -> Result<Vec<String>> {
    let mut parts = stem
        .split('-')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();

    if parts.is_empty() {
        bail!("empty Lucide icon name");
    }

    let mut leading_digit_parts = Vec::new();
    while parts
        .first()
        .is_some_and(|part| part.starts_with(|ch: char| ch.is_ascii_digit()))
    {
        leading_digit_parts.push(parts.remove(0));
    }

    parts.extend(leading_digit_parts);
    Ok(parts)
}

fn pascal_segment(segment: &str) -> String {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    if first.is_ascii_digit() {
        segment.to_owned()
    } else {
        let mut output = String::new();
        output.push(first.to_ascii_uppercase());
        output.extend(chars);
        output
    }
}

fn component_to_module(component: &str) -> String {
    let mut output = String::new();
    let mut previous_was_lower_or_digit = false;

    for ch in component.chars() {
        if ch.is_ascii_uppercase() {
            if previous_was_lower_or_digit {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
            previous_was_lower_or_digit = false;
        } else if ch.is_ascii_digit() {
            if previous_was_lower_or_digit {
                output.push('_');
            }
            output.push(ch);
            previous_was_lower_or_digit = true;
        } else {
            output.push(ch);
            previous_was_lower_or_digit = true;
        }
    }

    output
}

fn override_component_name(stem: &str) -> Option<&'static str> {
    // Kept explicit so Lucide-React divergences can be audited and pinned here.
    const OVERRIDES: &[(&str, &str)] = &[];

    OVERRIDES
        .iter()
        .find_map(|(source, component)| (*source == stem).then_some(*component))
}

fn is_valid_rust_ident(ident: &str) -> bool {
    let mut chars = ident.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_valid_rust_module_ident(ident: &str) -> bool {
    is_valid_rust_ident(ident)
}

fn is_rust_keyword(ident: &str) -> bool {
    matches!(
        ident,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_lucide_names() {
        assert_eq!(icon_names("trash").unwrap().component, "Trash");
        assert_eq!(icon_names("arrow-down").unwrap().component, "ArrowDown");
        assert_eq!(
            icon_names("chevrons-up-down").unwrap().component,
            "ChevronsUpDown"
        );
        assert_eq!(icon_names("1-circle").unwrap().component, "Circle1");
        assert_eq!(icon_names("circle-2").unwrap().component, "Circle2");
        assert_eq!(icon_names("box").unwrap().module_ref, "r#box");
    }
}
