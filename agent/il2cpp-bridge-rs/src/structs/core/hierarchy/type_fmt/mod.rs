//! Type name formatting helpers for IL2CPP type display.

mod parse;
use parse::{csharp_alias, generic_arity, split_generic_type};

fn display_simple_name(name: &str) -> String {
    let simple = name.rfind('.').map(|p| &name[p + 1..]).unwrap_or(name);
    let nested_simple = simple.rsplit('+').next().unwrap_or(simple);
    csharp_alias(nested_simple)
        .unwrap_or(nested_simple)
        .to_string()
}

pub(super) fn type_name_matches(raw: &str, expected: &str) -> bool {
    let raw = raw.trim();
    let expected = expected.trim();

    if raw == expected {
        return true;
    }

    if csharp_alias(raw).is_some_and(|alias| alias == expected) {
        return true;
    }

    let simple = raw.rfind('.').map(|p| &raw[p + 1..]).unwrap_or(raw);
    let nested_simple = simple.rsplit('+').next().unwrap_or(simple);
    nested_simple == expected || csharp_alias(nested_simple).is_some_and(|alias| alias == expected)
}

pub(super) fn format_class_name(raw: &str) -> String {
    if let Some(a) = csharp_alias(raw) {
        return a.to_string();
    }
    if let Some(el) = raw.strip_suffix("[]") {
        return format!("{}[]", csharp_alias(el).unwrap_or(el));
    }
    if let Some(p) = raw.rfind('[') {
        let s = &raw[p..];
        if s.ends_with(']') && s[1..s.len() - 1].chars().all(|c| c == ',') {
            return format!("{}{s}", csharp_alias(&raw[..p]).unwrap_or(&raw[..p]));
        }
    }
    if let Some(bt) = raw.find('`') {
        let base = &raw[..bt];
        let arity: usize = raw[bt + 1..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0);
        if arity > 0 {
            let p: Vec<_> = (0..arity).map(|i| format!("T{i}")).collect();
            return format!("{}<{}>", display_simple_name(base), p.join(", "));
        }
        return display_simple_name(base);
    }
    display_simple_name(raw)
}

pub(super) fn format_type_name_str(name: &str) -> String {
    let name = name.trim();
    if let Some(a) = csharp_alias(name) {
        return a.to_string();
    }
    if let Some(el) = name.strip_suffix("[]") {
        return format!("{}[]", format_type_name_str(el));
    }
    if let Some(p) = name.rfind('[') {
        let s = &name[p..];
        if s.ends_with(']') && s[1..s.len() - 1].chars().all(|c| c == ',') {
            return format!("{}{s}", format_type_name_str(&name[..p]));
        }
    }
    if let Some((base, args)) = split_generic_type(name) {
        let simple = display_simple_name(base);
        if args.is_empty() {
            let arity = generic_arity(name);
            if arity > 0 {
                let p: Vec<_> = (0..arity).map(|i| format!("T{i}")).collect();
                return format!("{}<{}>", simple, p.join(", "));
            }
            return simple;
        }
        let a = args
            .iter()
            .map(|a| format_type_name_str(a))
            .collect::<Vec<_>>()
            .join(", ");
        return format!("{simple}<{a}>");
    }
    display_simple_name(name)
}

/// Strips assembly qualifiers from an IL2CPP assembly-qualified name.
///
/// Converts e.g. `Dictionary\`2[[System.String, mscorlib],[System.Int32, mscorlib]], mscorlib`
/// into `System.Collections.Generic.Dictionary\`2[System.String,System.Int32]` which
/// `format_type_name_str` can then convert to `Dictionary<string, int>`.
pub(super) fn strip_assembly_qualifiers(aqn: &str) -> String {
    let aqn = aqn.trim();
    let type_part = {
        let mut depth = 0i32;
        let mut end = aqn.len();
        for (i, c) in aqn.char_indices() {
            match c {
                '[' => depth += 1,
                ']' => depth -= 1,
                ',' if depth == 0 => {
                    end = i;
                    break;
                }
                _ => {}
            }
        }
        &aqn[..end]
    };
    let Some(dbl) = type_part.find("[[") else {
        return type_part.to_string();
    };
    let (base, section) = (&type_part[..dbl], &type_part[dbl..]);
    let mut args = Vec::new();
    let mut start = None;
    let mut d = 0i32;
    for (i, c) in section.char_indices() {
        match c {
            '[' => {
                d += 1;
                if d == 2 {
                    start = Some(i + 1);
                }
            }
            ']' => {
                d -= 1;
                if d == 1 {
                    if let Some(s) = start {
                        args.push(strip_assembly_qualifiers(&section[s..i]));
                        start = None;
                    }
                }
            }
            _ => {}
        }
    }
    format!("{}[{}]", base, args.join(","))
}
