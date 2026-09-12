//! Metadata dump helpers that emit C#-like pseudo-code.
use crate::api::{cache, Application};
#[cfg(debug_assertions)]
use crate::logger;
use crate::structs::Assembly;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Writes a single assembly to the provided writer
fn write_assembly(writer: &mut dyn Write, assembly: &Assembly) -> std::io::Result<()> {
    if !assembly.classes.is_empty() {
        for class in &assembly.classes {
            writeln!(writer, "\n{}", class)?;
        }
    }
    Ok(())
}

/// Collects all assemblies from the cache, sorted by their minimum class token
/// so the order matches the IL2CPP metadata image table.
fn sorted_assemblies() -> Vec<std::sync::Arc<Assembly>> {
    let mut assemblies: Vec<_> = cache::CACHE
        .assemblies
        .iter()
        .map(|e| std::sync::Arc::clone(e.value()))
        .collect();

    assemblies.sort_by_key(|a| {
        a.classes
            .iter()
            .map(|c| c.token & 0x00FF_FFFF)
            .min()
            .unwrap_or(u32::MAX)
    });

    assemblies
}

/// Writes the `// Image N: foo.dll - startIndex` header block to any writer.
/// The starting TypeDefIndex for each image is `min_token - 1` (tokens are 1-based).
fn write_image_list(
    writer: &mut dyn Write,
    assemblies: &[std::sync::Arc<Assembly>],
) -> std::io::Result<()> {
    for (idx, assembly) in assemblies.iter().enumerate() {
        let start = assembly
            .classes
            .iter()
            .map(|c| c.token & 0x00FF_FFFF)
            .min()
            .map(|t| t.saturating_sub(1))
            .unwrap_or(0);

        writeln!(writer, "// Image {}: {} - {}", idx, assembly.file, start)?;
    }
    writeln!(writer)
}

/// Resolves the dump directory based on an optional base path
fn get_dump_dir(base_path: Option<&str>) -> Option<PathBuf> {
    let root = if let Some(path) = base_path {
        path.to_string()
    } else {
        Application::data_path().unwrap_or(".".to_string())
    };

    let dump_dir = Path::new(&root).join("dump");

    if let Err(_e) = std::fs::create_dir_all(&dump_dir) {
        #[cfg(debug_assertions)]
        logger::error(&format!("Failed to create dump directory: {}", _e));
        return None;
    }

    Some(dump_dir)
}

/// Dumps assemblies to separate files, or a single file if specified
fn dump_assemblies_impl(base_path: Option<&str>, single_file_name: Option<&str>) -> Option<String> {
    let dump_dir = get_dump_dir(base_path)?;

    #[cfg(debug_assertions)]
    logger::info(&format!("Dumping assemblies to {:?}...", dump_dir));

    let assemblies = sorted_assemblies();

    if let Some(file_name) = single_file_name {
        // Dump all to one file
        let path = dump_dir.join(file_name);
        let file = match File::create(&path) {
            Ok(f) => f,
            Err(_e) => {
                #[cfg(debug_assertions)]
                logger::error(&format!("Failed to create dump file: {}", _e));
                return None;
            }
        };
        let mut writer = BufWriter::new(file);

        // Image index block at the top
        if let Err(_e) = write_image_list(&mut writer, &assemblies) {
            #[cfg(debug_assertions)]
            logger::error(&format!("Failed to write image list: {}", _e));
        }

        // Classes in sorted order
        for assembly in &assemblies {
            if let Err(_e) = write_assembly(&mut writer, assembly) {
                #[cfg(debug_assertions)]
                logger::error(&format!(
                    "Failed to write assembly {}: {}",
                    assembly.name, _e
                ));
            }
        }

        if let Err(_e) = writer.flush() {
            #[cfg(debug_assertions)]
            logger::error(&format!("Failed to flush writer: {}", _e));
            return None;
        }

        #[cfg(debug_assertions)]
        logger::info(&format!("Dumped all assemblies to {:?}", path));
        Some(path.to_string_lossy().into_owned())
    } else {
        // Dump to separate files — each file still gets the full image list at the top
        for assembly in &assemblies {
            let path = dump_dir.join(format!("{}.cs", assembly.name));

            let file = match File::create(&path) {
                Ok(f) => f,
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    logger::error(&format!(
                        "Failed to create dump file for {}: {}",
                        assembly.name, _e
                    ));
                    continue;
                }
            };
            let mut writer = BufWriter::new(file);

            if let Err(_e) = write_image_list(&mut writer, &assemblies) {
                #[cfg(debug_assertions)]
                logger::error(&format!("Failed to write image list: {}", _e));
            }

            if let Err(_e) = write_assembly(&mut writer, assembly) {
                #[cfg(debug_assertions)]
                logger::error(&format!(
                    "Failed to write assembly {}: {}",
                    assembly.name, _e
                ));
            }

            if writer.flush().is_ok() {
                #[cfg(debug_assertions)]
                logger::info(&format!("Successfully dumped assembly {}", assembly.name));
            }
        }

        #[cfg(debug_assertions)]
        logger::info("Dumped all assemblies");
        Some(dump_dir.to_string_lossy().into_owned())
    }
}

/// Dumps a single assembly into a `.cs` file under the default dump directory.
///
/// If `assembly_to_dump` is `None`, this falls back to [`dump_all`] for legacy
/// behavior.
pub fn dump_assembly(assembly_to_dump: Option<&str>) -> Option<()> {
    // If no specific assembly, dump all to "dump.cs" (legacy behavior matches dump_all essentially)
    if assembly_to_dump.is_none() {
        return dump_all().map(|_| ());
    }

    let target_name = assembly_to_dump.unwrap();

    let dump_dir = get_dump_dir(None)?;
    let file_name = format!("{}.cs", target_name);
    let path = dump_dir.join(file_name);

    let file = match File::create(&path) {
        Ok(f) => f,
        Err(_e) => {
            #[cfg(debug_assertions)]
            logger::error(&format!("Failed to create dump file: {}", _e));
            return None;
        }
    };
    let mut writer = BufWriter::new(file);

    #[cfg(debug_assertions)]
    logger::info(&format!("Dumping assembly {}", target_name));

    let assemblies = sorted_assemblies();

    // Image index block at the top
    if let Err(_e) = write_image_list(&mut writer, &assemblies) {
        #[cfg(debug_assertions)]
        logger::error(&format!("Failed to write image list: {}", _e));
    }

    for assembly in &assemblies {
        if assembly.name.contains(target_name) {
            if let Err(_e) = write_assembly(&mut writer, assembly) {
                #[cfg(debug_assertions)]
                logger::error(&format!("Failed to write assembly header: {}", _e));
                return None;
            }
        }
    }

    if let Err(_e) = writer.flush() {
        #[cfg(debug_assertions)]
        logger::error(&format!("Failed to flush writer: {}", _e));
        return None;
    }

    #[cfg(debug_assertions)]
    logger::info(&format!("Dumped assembly to {:?}", path));
    Some(())
}

/// Dumps all loaded assemblies into separate `.cs` files.
///
/// Returns the output directory path on success.
pub fn dump() -> Option<String> {
    dump_assemblies_impl(None, None)
}

/// Dumps all loaded assemblies into a single `dump.cs` file.
///
/// Returns the output file path on success.
pub fn dump_all() -> Option<String> {
    dump_assemblies_impl(None, Some("dump.cs"))
}

/// Dumps all loaded assemblies into separate `.cs` files under `base_path`.
///
/// Returns the output directory path on success.
pub fn dump_to(base_path: &str) -> Option<String> {
    dump_assemblies_impl(Some(base_path), None)
}

/// Dumps all loaded assemblies into a single `dump.cs` file under `base_path`.
///
/// Returns the output file path on success.
pub fn dump_all_to(base_path: &str) -> Option<String> {
    dump_assemblies_impl(Some(base_path), Some("dump.cs"))
}
