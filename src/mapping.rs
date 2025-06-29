use std::sync::Arc;

pub trait Mapping {
    fn remap_class<C>(&self, class_name: C) -> Option<Arc<str>>
    where
        C: AsRef<str>;
    
    fn remap_method<C, M, D>(&self, class_name: C, method_name: M, descriptor: D) -> Option<Arc<str>>
    where
        C: AsRef<str>,
        M: AsRef<str>,
        D: AsRef<str>;
    
    fn remap_field<C, F, D>(&self, class_name: C, field_name: F, descriptor: D) -> Option<Arc<str>>
    where 
        C: AsRef<str>,
        F: AsRef<str>,
        D: AsRef<str>;
    
    fn remap_descriptor<D>(&self, descriptor: &D) -> Arc<str>
    where
        D: AsRef<str>;
}