//! Low-level parsing helpers for IL2CPP type name strings.

pub(super) fn generic_arity(name: &str) -> usize {
    name.find('`').map_or(0, |bt| {
        let rest = &name[bt + 1..];
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        if end == 0 {
            0
        } else {
            rest[..end].parse().unwrap_or(0)
        }
    })
}

pub(super) fn split_generic_type(name: &str) -> Option<(&str, Vec<&str>)> {
    let bt = name.find('`')?;
    let base = &name[..bt];
    let after = &name[bt + 1..];
    let digits = after.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let rest = after[digits..].trim();
    if rest.is_empty() {
        return Some((base, Vec::new()));
    }
    if let Some(inner) = rest.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        return Some((base, split_top_level(inner)));
    }
    if let Some(inner) = rest.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        return Some((base, split_top_level(inner)));
    }
    Some((base, Vec::new()))
}

fn split_top_level(s: &str) -> Vec<&str> {
    let (mut parts, mut start, mut angle, mut square) = (Vec::new(), 0, 0i32, 0i32);
    for (i, c) in s.char_indices() {
        match c {
            '<' => angle += 1,
            '>' => angle -= 1,
            '[' => square += 1,
            ']' => square -= 1,
            ',' if angle == 0 && square == 0 => {
                parts.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    let tail = s[start..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }
    parts
}

pub(super) fn csharp_alias(name: &str) -> Option<&'static str> {
    Some(match name {
        "System.Void" | "Void" => "void",
        "System.Boolean" | "Boolean" => "bool",
        "System.Byte" | "Byte" => "byte",
        "System.SByte" | "SByte" => "sbyte",
        "System.Int16" | "Int16" => "short",
        "System.UInt16" | "UInt16" => "ushort",
        "System.Int32" | "Int32" => "int",
        "System.UInt32" | "UInt32" => "uint",
        "System.Int64" | "Int64" => "long",
        "System.UInt64" | "UInt64" => "ulong",
        "System.Single" | "Single" => "float",
        "System.Double" | "Double" => "double",
        "System.Decimal" | "Decimal" => "decimal",
        "System.Char" | "Char" => "char",
        "System.String" | "String" => "string",
        "System.Object" | "Object" => "object",
        _ => return None,
    })
}
