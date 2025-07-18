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

impl Header {
    fn find_namespace_offset(&self, name: &str) -> Option<usize> {
        self.namespaces
            .iter()
            .position(|ns| &**ns == name)
    }
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

        let (
            namespace_named_index,
            namespace_intermediary_index,
            namespace_official_index
        ) = mapping.extract_namespaces()?;

        let mut current_class_name: Arc<str> = "".into();

        for line in lines {
            if line.is_empty() || line.starts_with('#') {
                continue
            }

            mapping.parse_line(
                namespace_named_index,
                namespace_intermediary_index,
                namespace_official_index,
                &mut current_class_name,
                line
            )?;
        }

        Ok(mapping)
    }
}

const DELIMITER: char = '\t';
const HEADER_MAJOR_OFFSET: usize = 1;
const HEADER_MINOR_OFFSET: usize = 2;
const HEADER_NAMESPACES_OFFSET: usize = 3;

fn parse_header(header_line: &str) -> Option<Header> {
    let header_parts: Vec<&str> = header_line.split(DELIMITER).collect();
    if header_parts[0] != "tiny" || header_parts.len() < 5 {
        return None;
    }

    let major_version: usize = header_parts[HEADER_MAJOR_OFFSET].parse().ok()?;
    let minor_version: usize = header_parts[HEADER_MINOR_OFFSET].parse().ok()?;
    let namespaces: Vec<Arc<str>> = header_parts[HEADER_NAMESPACES_OFFSET..].iter().map(|s| Arc::from(&**s)).collect();

    Some(Header::new(major_version, minor_version, namespaces))
}

const SUBSECTION_OFFSET: usize = 1;
const DESCRIPTOR_OFFSET: usize = 2;

const CLASS_IDENT: &str = "c";
const COMMENT_IDENT: &str = "c";
const METHOD_IDENT: &str = "m";
const FIELD_IDENT: &str = "f";
const BASE_NAMESPACE_OFFSET: usize = 1;
const SECTION_IDENT_OFFSET: usize = 0;

macro_rules! namespace_value {
    (base, $parts:expr, $namespace_index:expr) => {{
        let value: Option<Arc<str>> = $parts.get(BASE_NAMESPACE_OFFSET + $namespace_index)
            .map(|s| Arc::from(&**s));
        
        value
    }};
    (member, $parts:expr, $namespace_index:expr) => {{
        let value: Option<Arc<str>> = $parts.get(BASE_CLASS_MEMBERS_NAMESPACE_OFFSET + $namespace_index)
            .map(|s| Arc::from(&**s));
        
        value
    }};
}

impl TinyV2Mapping {
    fn extract_namespaces(&self) -> Result<(usize, usize, usize), MappingError> {
        macro_rules! find {
            ($namespace:expr) => {{
                self.header.find_namespace_offset($namespace)
                    .ok_or(MappingError::MissingNamespace($namespace.into()))?
            }};
        }

        Ok((find!("named"), find!("intermediary"), find!("official")))
    }

    fn parse_line(
        &mut self,
        namespace_named_index: usize,
        namespace_intermediary_index: usize,
        namespace_official_index: usize,
        current_class_name: &mut Arc<str>,
        line: &str
    ) -> Result<(), MappingError> {
        let parts: Vec<&str> = line.split(DELIMITER).collect();
        match parts[SECTION_IDENT_OFFSET] {
            CLASS_IDENT => self.parse_class(
                namespace_named_index,
                namespace_intermediary_index,
                namespace_official_index,
                current_class_name,
                &parts
            )?,
            _ if parts[SECTION_IDENT_OFFSET].is_empty() && !parts[SUBSECTION_OFFSET].is_empty() => {
                // Method or field section, tab indicates a subsection.
                if let Some(class_mapping) = self.classes.get_mut(current_class_name) {
                    let subsection_type = &parts[SUBSECTION_OFFSET];
                    let descriptor = parts[DESCRIPTOR_OFFSET].into();

                    match *subsection_type {
                        METHOD_IDENT | FIELD_IDENT => class_mapping.parse_class_members(
                            namespace_named_index,
                            namespace_intermediary_index,
                            namespace_official_index,
                            &parts,
                            subsection_type,
                            descriptor
                        )?,
                        COMMENT_IDENT => {
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

        Ok(())
    }

    fn parse_class(
        &mut self,
        namespace_named_index: usize,
        namespace_intermediary_index: usize,
        namespace_official_index: usize,
        current_class_name: &mut Arc<str>,
        parts: &Vec<&str>
    ) -> Result<(), MappingError> {
        let class_name = namespace_value!(base, parts, namespace_named_index)
            .ok_or(MappingError::MissingClassName)?;

        let official_name = namespace_value!(base, parts, namespace_official_index);
        let intermediary_name = namespace_value!(base, parts, namespace_intermediary_index);

        *current_class_name = class_name.clone();
        self.classes.insert(class_name, ClassMapping::new(official_name, intermediary_name, HashMap::new(), HashMap::new()));
        Ok(())
    }
}

const BASE_CLASS_MEMBERS_NAMESPACE_OFFSET: usize = 3;

impl ClassMapping {
    fn parse_class_members(
        &mut self,
        namespace_named_index: usize,
        namespace_intermediary_index: usize,
        namespace_official_index: usize,
        parts: &Vec<&str>,
        subsection_type: &&str,
        descriptor: Arc<str>
    ) -> Result<(), MappingError> {
        let named_name = namespace_value!(member, parts, namespace_named_index)
            .ok_or(MappingError::MissingFieldOrMethodName)?;

        let official_name = namespace_value!(member, parts, namespace_official_index);
        let intermediary_name = namespace_value!(member, parts, namespace_intermediary_index);

        if *subsection_type == METHOD_IDENT {
            // Method section
            self.methods.insert((named_name, descriptor), MethodMapping::new(official_name, intermediary_name));
        } else {
            // Field section
            self.fields.insert((named_name, descriptor), FieldMapping::new(official_name, intermediary_name));
        }
        Ok(())
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
    use std::path::Path;
    use super::*;

    fn get_mapping() -> impl Mapping {
        TinyV2Mapping::load(Path::new("test/mappings.tiny")).unwrap()
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
