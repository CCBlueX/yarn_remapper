use std::sync::Arc;

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
    /// An `Option<Arc<str>>` containing the obfuscated class name if available, or `None` if no mapping exists.
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
    /// An `Option<Arc<str>>` containing the obfuscated method name if available, or `None` if no mapping exists.
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
    /// An `Option<Arc<str>>` containing the obfuscated field name if available, or `None` if no mapping exists.
    fn remap_field<C, F, D>(&self, class_name: C, field_name: F, descriptor: D) -> Option<Arc<str>>
    where 
        C: AsRef<str>,
        F: AsRef<str>,
        D: AsRef<str>;

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
    /// An `Arc<str>` containing the obfuscated descriptor in official format.
    ///
    /// # Notes
    ///
    /// The input descriptor must be in the named format, and the output descriptor will be in the obfuscated format.
    fn remap_descriptor<D>(&self, descriptor: &D) -> Arc<str>
    where
        D: AsRef<str>;
}
