use super::PyWorld;

pub trait MiscApi {
    fn count_entities_with_type(&self, type_str: String) -> usize;
}

impl MiscApi for PyWorld {
    fn count_entities_with_type(&self, type_str: String) -> usize {
        let world = self.inner.borrow_mut();
        world.count_entities_with_type(&type_str)
    }
}
