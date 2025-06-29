use std::collections::HashMap;
use std::sync::Arc;
use derive_getters::Getters;
use derive_new::new;
use crate::mapping::{Mapping, MappingError, MappingExt, MappingFile, MappingLoader};

// Header struct that parses and stores header information of TinyV2 mapping.
#[derive(Debug, new, Getters)]
pub struct Header {
    pub major_version: usize,
    pub minor_version: usize,
    pub namespaces: Vec<Arc<str>>,
}

// ClassMapping struct that stores obfuscated class name and its members' mappings.
#[derive(Debug, Default, new, Getters)]
pub struct ClassMapping {
    official_name: Option<Arc<str>>,
    intermediary_name: Option<Arc<str>>,
    methods: HashMap<(Arc<str>, Arc<str>), MethodMapping>,  // Use (name, descriptor) as key
    fields: HashMap<(Arc<str>, Arc<str>), FieldMapping>,    // Use (name, descriptor) as key
}

// MethodMapping struct that stores method descriptor mapping.
#[derive(Debug, new, Getters)]
pub struct MethodMapping {
    official_name: Option<Arc<str>>,
    intermediary_name: Option<Arc<str>>,
}

// FieldMapping struct that stores field descriptor mapping.
#[derive(Debug, new, Getters)]
pub struct FieldMapping {
    official_name: Option<Arc<str>>,
    intermediary_name: Option<Arc<str>>,
}

// Mapping struct that includes the entire TinyV2 mapping with classes and header.
#[derive(Debug, new, Getters)]
pub struct TinyV2Mapping {
    header: Header,
    #[new(default)]
    classes: HashMap<Arc<str>, ClassMapping>,
}

fn parse_header(header_line: &str) -> Option<Header> {
    let header_parts: Vec<&str> = header_line.split('\t').collect();
    if header_parts[0] != "tiny" || header_parts.len() < 5 {
        return None;
    }

    let major_version: usize = header_parts[1].parse().ok()?;
    let minor_version: usize = header_parts[2].parse().ok()?;
    let namespaces: Vec<Arc<str>> = header_parts[3..].iter().map(|s| Arc::from(&**s)).collect();

    Some(Header::new(major_version, minor_version, namespaces))
}

impl MappingLoader for TinyV2Mapping {
    fn load<F>(file: F) -> Result<Self, MappingError>
    where
        F: Into<MappingFile>
    {
        let contents = file.into().as_str()?;
        let mut lines = contents.lines();
        
        let header_line = lines.next().ok_or(MappingError::InvalidHeader)?;
        let header = parse_header(header_line).ok_or(MappingError::InvalidHeader)?;
        let mut mapping = TinyV2Mapping::new(header);

        let namespace_named_index = mapping.header.namespaces.iter().position(|ns| &**ns == "named")
            .ok_or(MappingError::MissingNamespace("named".into()))?;
        let namespace_intermediary_index = mapping.header.namespaces.iter().position(|ns| &**ns == "intermediary")
            .ok_or(MappingError::MissingNamespace("intermediary".into()))?;
        let namespace_official_index = mapping.header.namespaces.iter().position(|ns| &**ns == "official")
            .ok_or(MappingError::MissingNamespace("official".into()))?;
        
        let mut current_class_name: Arc<str> = "".into();
        
        for line in lines {
            if line.is_empty() || line.starts_with('#') {
                continue
            }
            
            let parts: Vec<&str> = line.split('\t').collect();
            match parts[0] {
                "c" => {
                    let class_name: Arc<str> = parts.get(1 + namespace_named_index)
                        .map(|s| Arc::from(&**s))
                        .ok_or(MappingError::MissingClassName)?;
                    
                    let official_name: Option<Arc<str>> = parts.get(1 + namespace_official_index)
                        .map(|s| Arc::from(&**s));
                    
                    let intermediary_name: Option<Arc<str>> = parts.get(1 + namespace_intermediary_index)
                        .map(|s| Arc::from(&**s));
                    
                    current_class_name = class_name.clone();
                    mapping.classes.insert(class_name, ClassMapping::new(official_name, intermediary_name, HashMap::new(), HashMap::new()));
                },
                _ if parts[0].is_empty() && !parts[1].is_empty() => {
                    // Method or field section, tab indicates a subsection.
                    if let Some(class_mapping) = mapping.classes.get_mut(&current_class_name) {
                        let subsection_type = &parts[1];
                        let descriptor = parts[2].into();
                        
                        match *subsection_type {
                            "m" | "f" => {
                                let named_name: Arc<str> = parts.get(3 + namespace_named_index)
                                    .map(|s| Arc::from(&**s))
                                    .ok_or(MappingError::MissingFieldOrMethodName)?;
                                
                                let official_name: Option<Arc<str>> = parts.get(3 + namespace_official_index)
                                    .map(|s| Arc::from(&**s));
                                
                                let intermediary_name: Option<Arc<str>> = parts.get(3 + namespace_intermediary_index)
                                    .map(|s| Arc::from(&**s));

                                if *subsection_type == "m" {
                                    // Method section
                                    class_mapping.methods.insert((named_name, descriptor), MethodMapping::new(official_name, intermediary_name));
                                } else {
                                    // Field section
                                    class_mapping.fields.insert((named_name, descriptor), FieldMapping::new(official_name, intermediary_name));
                                }
                            },
                            "c" => {
                                // Comment section
                                // Not relevant for remapping.
                            }
                            _ => {
                                return Err(MappingError::UnknownSubsectionType)
                            }
                        }
                    }
                },
                _ => {}
            }
        }
        
        Ok(mapping)
    }
}

impl Mapping for TinyV2Mapping {
    fn remap_class<C>(&self, class_name: C) -> Option<Arc<str>>
    where
        C: AsRef<str>
    {
        self.classes.get(class_name.as_ref())
            .map(|class| class.official_name.clone().unwrap_or_else(|| Arc::from(class_name.as_ref())))
    }

    fn remap_method<C, M, D>(&self, class_name: C, method_name: M, descriptor: D) -> Option<Arc<str>>
    where
        C: AsRef<str>,
        M: AsRef<str>,
        D: AsRef<str>
    {
        let remapped_descriptor = self.remap_descriptor(descriptor.as_ref());
        
        self.classes.get(class_name.as_ref())
            .and_then(|class| class.methods.get(&(method_name.as_ref().into(), remapped_descriptor)))
            .map(|method| method.official_name.clone().unwrap_or_else(|| Arc::from(method_name.as_ref())))
    }

    fn remap_field<C, F, D>(&self, class_name: C, field_name: F, descriptor: D) -> Option<Arc<str>>
    where
        C: AsRef<str>,
        F: AsRef<str>,
        D: AsRef<str>
    {
        let remapped_descriptor = self.remap_descriptor(descriptor.as_ref());

        self.classes.get(class_name.as_ref())
            .and_then(|class| class.fields.get(&(field_name.as_ref().into(), remapped_descriptor)))
            .map(|method| method.official_name.clone().unwrap_or_else(|| Arc::from(field_name.as_ref())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_mapping() -> impl Mapping {
        TinyV2Mapping::load("test/1.21.1.tiny").unwrap()
    }

    #[test]
    fn test_class_remap() {
        let mapping = get_mapping();

        assert_eq!(mapping.remap_class("net/minecraft/client/MinecraftClient"), Some("evi".into()));
    }

    #[test]
    fn test_method_remap() {
        let mapping = get_mapping();
        assert_eq!(mapping.remap_method("net/minecraft/client/MinecraftClient", "getWindowTitle", "()Ljava/lang/String;"), Some("be".into()));
    }

    #[test]
    fn test_method_remap_2() {
        let mapping = get_mapping();
        assert_eq!(mapping.remap_method("net/minecraft/client/world/ClientWorld", "addParticle", "(DDDDDLnet/minecraft/particle/ParticleEffect;)V"), Some("a".into()));
    }

    #[test]
    fn test_field_remap() {
        let mapping = get_mapping();
        assert_eq!(mapping.remap_field("net/minecraft/client/MinecraftClient", "inGameHud", "Lnet/minecraft/client/gui/hud/InGameHud;"), Some("l".into()));
    }
}
