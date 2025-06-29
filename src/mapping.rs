use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Represents the source of a mapping input file.
///
/// This enum abstracts over different ways to provide mapping data:
/// - a filesystem path (`Path`),
/// - or raw bytes (`Bytes`).
///
/// The loader can accept either to flexibly support different use cases
/// (e.g., reading from disk or in-memory data).
pub enum MappingFile<'a> {
    /// Mapping file located at a filesystem path.
    Path(&'a Path),

    /// Mapping file provided as raw bytes in memory.
    Bytes(&'a [u8])
}

/// Errors that can occur during loading or processing mapping files.
///
/// This enum aggregates possible error types, such as I/O errors,
/// parsing errors, or invalid mapping formats.
/// Implementors can extend this to support additional error kinds as needed.
#[derive(Debug, Error)]
pub enum MappingError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Loads a mapping from a file or bytes.
///
/// Implementors of this trait provide loading/parsing logic that converts
/// raw mapping data into an instance implementing the [`Mapping`] trait.
///
/// The `load` function takes a generic `file` argument that can be converted
/// into [`MappingFile`], enabling flexible inputs like file paths or byte slices.
pub trait MappingLoader {
    /// The concrete type of the mapping produced by this loader.
    type MappingType: Mapping;

    /// Loads a mapping from the given input.
    ///
    /// # Parameters
    ///
    /// * `file` - Input mapping data source, convertible into [`MappingFile`].
    ///
    /// # Returns
    ///
    /// Returns the loaded mapping on success, or a [`MappingError`] on failure.
    ///
    /// # Errors
    ///
    /// Returns `MappingError::Io` if reading from a file fails, or
    /// other error variants depending on implementation.
    fn load<'a, F>(file: F) -> Result<Self::MappingType, MappingError>
    where
        F: Into<MappingFile<'a>>;
}

/// Trait representing a mapping between named identifiers and their obfuscated counterparts.
///
/// This trait provides methods to remap class names, method names, field names, and descriptors
/// from a "named" format (human-readable, e.g. `net/minecraft/client/MinecraftClient`) to an
/// obfuscated or official format (e.g. `a`, `b`, `Lev;`).
///
/// The remapping is essential in contexts such as Minecraft modding or any environment
/// where code is obfuscated and the original names need to be translated back and forth
/// for analysis, debugging, or transformation.
///
/// Implementors of this trait provide the logic and data to perform these remappings,
/// typically based on external mapping files or databases.
pub trait Mapping {
    /// Remaps the named class name to its obfuscated counterpart from the mapping data.
    ///
    /// # Arguments
    ///
    /// * `class_name` - A class name in named format (e.g. `"net/minecraft/client/MinecraftClient"`).
    ///
    /// # Returns
    ///
    /// An [`Option<Arc<str>>`] containing the obfuscated class name if available, or [`None`] if no mapping exists.
    fn remap_class<C>(&self, class_name: C) -> Option<Arc<str>>
    where
        C: AsRef<str>;

    /// Remaps the named method name to its obfuscated counterpart from the mapping data, given the descriptor.
    ///
    /// # Arguments
    ///
    /// * `class_name` - The class name owning the method.
    /// * `method_name` - The method name in named format.
    /// * `descriptor` - The method descriptor in named format (e.g. `"()Ljava/lang/String;"`).
    ///
    /// # Returns
    ///
    /// An [`Option<Arc<str>>`] containing the obfuscated method name if available, or [`None`] if no mapping exists.
    fn remap_method<C, M, D>(&self, class_name: C, method_name: M, descriptor: D) -> Option<Arc<str>>
    where
        C: AsRef<str>,
        M: AsRef<str>,
        D: AsRef<str>;

    /// Remaps the named field name to its obfuscated counterpart from the mapping data, given the descriptor.
    ///
    /// # Arguments
    ///
    /// * `class_name` - The class name owning the field.
    /// * `field_name` - The field name in named format.
    /// * `descriptor` - The field descriptor in named format (e.g. `"()Ljava/lang/String;"`).
    ///
    /// # Returns
    ///
    /// An [`Option<Arc<str>>`] containing the obfuscated field name if available, or [`None`] if no mapping exists.
    fn remap_field<C, F, D>(&self, class_name: C, field_name: F, descriptor: D) -> Option<Arc<str>>
    where 
        C: AsRef<str>,
        F: AsRef<str>,
        D: AsRef<str>;
}

/// Extension trait for [`Mapping`] providing additional utility methods.
///
/// This trait offers higher-level helper functions such as recursive
/// remapping of descriptors, which are commonly needed when working
/// with Java bytecode descriptors in the context of obfuscated code.
pub trait MappingExt: Mapping {
    /// Remaps the named descriptor to its obfuscated counterpart from the mapping data.
    ///
    /// This function is recursive and will remap the descriptor recursively.
    ///
    /// # Arguments
    ///
    /// * `descriptor` - The descriptor in named format. Supports class descriptors (e.g. `"Lnet/minecraft/client/MinecraftClient;"`),
    ///   array descriptors (e.g. `"[Lnet/minecraft/client/MinecraftClient;"`), and method descriptors
    ///   (e.g. `"(Lnet/minecraft/client/MinecraftClient;)V"`).
    ///
    /// # Returns
    ///
    /// An [`Arc<str>`] containing the obfuscated descriptor in official format.
    ///
    /// # Notes
    ///
    /// The input descriptor must be in the named format, and the output descriptor will be in the obfuscated format.
    fn remap_descriptor<D>(&self, descriptor: &D) -> Arc<str>
    where
        D: AsRef<str> + ?Sized;
}

impl<T: Mapping> MappingExt for T {
    fn remap_descriptor<D>(&self, descriptor: &D) -> Arc<str>
    where
        D: AsRef<str> + ?Sized
    {
        let descriptor = descriptor.as_ref();

        // Remap L class descriptor from named to official
        if descriptor.starts_with('L') {
            // Format: Lnet/minecraft/client/MinecraftClient;
            let class_name = &descriptor[1..descriptor.len()-1];
            let remapped_class_name = self.remap_class(class_name)
                .unwrap_or_else(|| class_name.into());

            return format!(":{remapped_class_name};").into();
        }

        // Remap [ array descriptor
        if let Some(stripped) = descriptor.strip_prefix('[') {
            // Format: [Lnet/minecraft/client/MinecraftClient;
            let remapped_descriptor = self.remap_descriptor(stripped);
            return format!("[{remapped_descriptor}").into();
        }

        // Remap ( method descriptor
        if descriptor.starts_with('(') {
            // Remap method descriptor recursively
            // Format: (Lnet/minecraft/client/MinecraftClient;Lnet/minecraft/client/MinecraftClient;)Lnet/minecraft/client/MinecraftClient;

            let mut remapped_descriptor = String::new();
            let mut current_descriptor = String::new();

            for ch in descriptor.chars() {
                match ch {
                    '(' | ')' => remapped_descriptor.push(ch),
                    'L' => current_descriptor.push('L'),
                    ';' => {
                        current_descriptor.push(';');
                        let remapped = self.remap_descriptor(&current_descriptor);
                        remapped_descriptor.push_str(&remapped);
                        current_descriptor.clear();
                    }
                    _ => {
                        if current_descriptor.is_empty() {
                            remapped_descriptor.push(ch);
                        } else {
                            current_descriptor.push(ch);
                        }
                    }
                }
            }

            return remapped_descriptor.into();
        }

        descriptor.into()
    }
}