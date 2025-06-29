use std::{fs, io};
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
pub enum MappingFile {
    /// Mapping file located at a filesystem path.
    Path(Box<Path>),

    /// Mapping file provided as raw bytes in memory.
    Bytes(Arc<[u8]>)
}

impl MappingFile {
    /// Returns the mapping data as a byte slice.
    ///
    /// If the `MappingFile` is already bytes, returns them directly.
    /// If it is a path, reads the file contents and returns the bytes.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if reading from the file path fails.
    pub fn as_bytes(&self) -> Result<Arc<[u8]>, io::Error> {
        match self {
            MappingFile::Bytes(data) => Ok(data.clone()),
            MappingFile::Path(path) => {
                let bytes = fs::read(path)?;
                Ok(bytes.into())
            }
        }
    }

    /// Returns the mapping data as an `Arc<str>`.
    ///
    /// This method converts the internal byte representation of the mapping file into a
    /// shared string slice (`Arc<str>`). If the data is already in memory, it is reused;
    /// if it comes from a file path, the file is read and validated as UTF-8.
    ///
    /// To avoid unnecessary allocations, this method performs a zero-copy conversion from
    /// `Arc<[u8]>` to `Arc<str>` after validating that the bytes are valid UTF-8. This is done
    /// using `Arc::from_raw` on the validated pointer, ensuring that the resulting string shares
    /// ownership of the original data without cloning it.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] in the following cases:
    ///
    /// - The file cannot be read
    /// - The byte content is not valid UTF-8
    ///
    /// # Safety
    ///
    /// Internally uses an unsafe block to perform a zero-copy cast from `Arc<[u8]>` to `Arc<str>`.
    /// This is sound only because the bytes are validated using [`std::str::from_utf8`] beforehand.
    pub fn as_str(&self) -> Result<Arc<str>, io::Error> {
        let bytes = self.as_bytes()?;

        // Validate UTF-8 without allocating
        let _ = std::str::from_utf8(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // SAFETY: We just verified that the bytes are valid UTF-8
        let ptr = Arc::into_raw(bytes);
        let len = unsafe { (&*ptr).len() };
        let str_ptr = unsafe {
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            let str_ref = std::str::from_utf8_unchecked(slice);
            Arc::from_raw(str_ref)
        };
        Ok(str_ptr)
    }
}

/// Errors that can occur during loading or processing mapping files.
///
/// This enum aggregates possible error types, such as I/O errors,
/// parsing errors, or invalid mapping formats.
/// Implementors can extend this to support additional error kinds as needed.
#[derive(Debug, Error)]
pub enum MappingError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// A trait for loading and parsing mapping data.
///
/// Implementors provide logic to convert raw mapping data into a concrete type
/// that implements the [`Mapping`] trait.
///
/// The input data source is abstracted by [`MappingFile`], allowing flexibility
/// in how the mapping data is provided (e.g., files, byte slices, etc.).
pub trait MappingLoader: Sized + Mapping {
    /// Loads a mapping from the given input source.
    ///
    /// # Parameters
    ///
    /// - `file`: A data source convertible into [`MappingFile`].
    ///
    /// # Returns
    ///
    /// Returns the loaded mapping instance on success, or a [`MappingError`] on failure.
    ///
    /// # Errors
    ///
    /// Returns a [`MappingError`] variant if loading or parsing the input fails.
    fn load<F>(file: F) -> Result<Self, MappingError>
    where
        F: Into<MappingFile>;
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