use anyhow::{Context, Result, bail};
use derive_new::new;
use derive_getters::Getters;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// Header struct that parses and stores header information of TinyV2 mapping.
#[derive(Debug, new, Getters)]
pub struct Header {
    pub major_version: usize,
    pub minor_version: usize,
    pub namespaces: Vec<String>,
}

// ClassMapping struct that stores obfuscated class name and its members' mappings.
#[derive(Debug, Default, new, Getters)]
pub struct ClassMapping {
    official_name: Option<String>,
    intermediary_name: Option<String>,
    methods: HashMap<(String, String), MethodMapping>,  // Use (name, descriptor) as key
    fields: HashMap<(String, String), FieldMapping>,    // Use (name, descriptor) as key
}

// MethodMapping struct that stores method descriptor mapping.
#[derive(Debug, new, Getters)]
pub struct MethodMapping {
    official_name: Option<String>,
    intermediary_name: Option<String>,
}

// FieldMapping struct that stores field descriptor mapping.
#[derive(Debug, new, Getters)]
pub struct FieldMapping {
    official_name: Option<String>,
    intermediary_name: Option<String>,
}

// Mapping struct that includes the entire TinyV2 mapping with classes and header.
#[derive(Debug, new, Getters)]
pub struct Mapping {
    header: Header,
    #[new(default)]
    classes: HashMap<String, ClassMapping>,
}

impl Mapping {

    /// Remaps the named class name to its obfuscated counterpart from the mapping data.
    pub fn remap_class(&self, class_name: &str) -> Option<String> {
        self.classes.get(class_name)
            .map(|c| c.official_name.clone().unwrap_or_else(|| class_name.to_string()))
    }

    /// Remaps the named method name to its obfuscated counterpart from the mapping data, given the descriptor.
    pub fn remap_method(&self, class_name: &str, method_name: &str, descriptor: &str) -> Option<String> {
        let remapped_decriptor = self.remap_descriptor(descriptor);
        
        self.classes.get(class_name)
            .and_then(|class_mapping| class_mapping.methods.get(&(method_name.to_string(), remapped_decriptor)))
            .map(|method_mapping| method_mapping.official_name.clone().unwrap_or_else(|| method_name.to_string()))
    }

    /// Remaps the named field name to its obfuscated counterpart from the mapping data, given the descriptor.
    pub fn remap_field(&self, class_name: &str, field_name: &str, descriptor: &str) -> Option<String> {
        let remapped_decriptor = self.remap_descriptor(descriptor);

        self.classes.get(class_name)
            .and_then(|class_mapping| class_mapping.fields.get(&(field_name.to_string(), remapped_decriptor)))
            .map(|field_mapping| field_mapping.official_name.clone().unwrap_or_else(|| field_name.to_string()))
    }

    ///
    /// Remaps the named descriptor to its obfuscated counterpart from the mapping data.
    /// 
    /// This function is recursive and will remap the descriptor recursively.
    /// Input descriptor must be in named format (e.g. Lnet/minecraft/client/MinecraftClient;)
    /// Output descriptor will be in official format (e.g. Lev;)
    /// 
    /// Method descriptor is also supported (e.g. (Lnet/minecraft/client/MinecraftClient;)V)
    /// 
    pub fn remap_descriptor(&self, descriptor: &str) -> String {
        // Remap L class descriptor from named to official
        if descriptor.starts_with('L') {
            // Format: Lnet/minecraft/client/MinecraftClient;
            let class_name = descriptor[1..descriptor.len()-1].to_string();
            let remapped_class_name = self.remap_class(&class_name).unwrap_or_else(|| class_name.clone());
            return format!("L{};", remapped_class_name);
        }

        // Remap [ array descriptor
        if descriptor.starts_with('[') {
            // Format: [Lnet/minecraft/client/MinecraftClient;
            let remapped_descriptor = self.remap_descriptor(&descriptor[1..]);
            return format!("[{}", remapped_descriptor);
        }

        // Remap ( method descriptor
        if descriptor.starts_with('(') {
            // Remap method descriptor recursively
            // Format: (Lnet/minecraft/client/MinecraftClient;Lnet/minecraft/client/MinecraftClient;)Lnet/minecraft/client/MinecraftClient;

            let mut remapped_descriptor = String::new();
            let mut current_descriptor = String::new();

            for c in descriptor.chars() {
                if c == '(' {
                    remapped_descriptor.push('(');
                    continue;
                }
                if c == ')' {
                    remapped_descriptor.push(')');
                    continue;
                }
                if c == ';' {
                    // Remap descriptor
                    current_descriptor.push(';');
                    let remapped_current_descriptor = self.remap_descriptor(&current_descriptor);
                    remapped_descriptor.push_str(&remapped_current_descriptor);
                    current_descriptor.clear();
                    continue;
                }
                if c == 'L' {
                    // Start of class descriptor
                    current_descriptor.push('L');
                    continue;
                }

                if current_descriptor.is_empty() {
                    remapped_descriptor.push(c);
                } else {
                    current_descriptor.push(c);
                }
            }
            
            return remapped_descriptor;
            
        }
        
        return descriptor.to_string();
    }


}

/// Parses a TinyV2 formatted input into a `Mapping` struct.
pub fn parse_tiny_v2(file_path: &Path) -> Result<Mapping> {
    let contents = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read mapping file {:?}", file_path))?;
    let mut lines = contents.lines();

    let header_line = lines.next().context("Missing header line in mapping file")?;
    let header_parts: Vec<&str> = header_line.split('\t').collect();
    if header_parts[0] != "tiny" || header_parts.len() < 5 {
        bail!("Invalid header format");
    }

    let major_version: usize = header_parts[1].parse()?;
    let minor_version: usize = header_parts[2].parse()?;
    let namespaces: Vec<String> = header_parts[3..].iter().map(|s| s.to_string()).collect();

    let header = Header::new(major_version, minor_version, namespaces);
    let mut mapping = Mapping::new(header);

    let namespace_named_index = mapping.header.namespaces.iter().position(|ns| ns == "named")
        .context("Failed to find namespace named")?;
    let namespace_intermediary_index = mapping.header.namespaces.iter().position(|ns| ns == "intermediary")
        .context("Failed to find namespace intermediary")?;
    let namespace_official_index = mapping.header.namespaces.iter().position(|ns| ns == "official")
        .context("Failed to find namespace official")?;

    let mut current_class_name = String::new();

    // Parse the rest of the lines to populate classes, methods, and fields.
    for line in lines {
        if line.is_empty() || line.starts_with('#') {
            continue; // Skip comments or empty lines.
        }
        let parts: Vec<&str> = line.split('\t').collect();
        
        match parts[0] {
            "c" => {
                // Class section
                let class_name = parts.get(1 + namespace_named_index)
                    .map(|s| s.to_string())
                    .context("Named name not found for class")?;
                let official_name = parts.get(1 + namespace_official_index)
                    .map(|s| s.to_string());
                let intermediary_name = parts.get(1 + namespace_intermediary_index)
                    .map(|s| s.to_string());

                current_class_name = class_name.clone();
                mapping.classes.insert(class_name, ClassMapping::new(official_name, intermediary_name, HashMap::new(), HashMap::new()));
            }
            _ if parts[0].is_empty() && !parts[1].is_empty() => {
                // Method or field section, tab indicates a subsection.
                if let Some(class_mapping) = mapping.classes.get_mut(&current_class_name) {
                    let subsection_type = &parts[1];
                    let descriptor = parts[2].to_string();

                    match *subsection_type {
                        "m" => {
                            let named_name = parts.get(3 + namespace_named_index)
                                .context("Named name not found for method or field")?
                                .to_string();
                            let official_name = parts.get(3 + namespace_official_index)
                                .map(|s| s.to_string());
                            let intermediary_name = parts.get(3 + namespace_intermediary_index)
                                .map(|s| s.to_string());

                            // Method section
                            class_mapping.methods.insert((named_name, descriptor), MethodMapping::new(official_name, intermediary_name));
                        }
                        "f" => {
                            let named_name = parts.get(3 + namespace_named_index)
                                .context("Named name not found for method or field")?
                                .to_string();
                            let official_name = parts.get(3 + namespace_official_index)
                                .map(|s| s.to_string());
                            let intermediary_name = parts.get(3 + namespace_intermediary_index)
                                .map(|s| s.to_string());

                            // Field section
                            class_mapping.fields.insert((named_name, descriptor), FieldMapping::new(official_name, intermediary_name));
                        }
                        "c" => {
                            // Comment section
                            // Not relevant for remapping.
                        }
                        _ => bail!("Unknown subsection type"),
                    }
                }
            }
            _ => {},
        }
    }

    Ok(mapping)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_mapping() -> Mapping {
        parse_tiny_v2(Path::new("mappings.tiny")).unwrap()
    }

    #[test]
    fn test_class_remap() {
        let mapping = get_mapping();

        assert_eq!(mapping.remap_class("net/minecraft/client/MinecraftClient"), Some("evi".to_string()));
    }

    #[test]
    fn test_method_remap() {
        let mapping = get_mapping();
        assert_eq!(mapping.remap_method("net/minecraft/client/MinecraftClient", "getWindowTitle", "()Ljava/lang/String;"), Some("be".to_string()));
    }

    #[test]
    fn test_method_remap_2() {
        let mapping = get_mapping();
        assert_eq!(mapping.remap_method("net/minecraft/client/world/ClientWorld", "addParticle", "(DDDDDLnet/minecraft/particle/ParticleEffect;)V"), Some("a".to_string()));
    }

    #[test]
    fn test_field_remap() {
        let mapping = get_mapping();

        assert_eq!(mapping.remap_field("net/minecraft/client/MinecraftClient", "inGameHud", "Lnet/minecraft/client/gui/hud/InGameHud;"), Some("l".to_string()));
    }

}
