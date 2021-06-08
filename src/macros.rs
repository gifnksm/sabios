#[macro_export]
macro_rules! byte_getter {
    ($vis:vis $field:ident : [u8; $n:expr]) => {
        $vis fn $field(&self) -> [u8; $n] {
            self.$field
        }
    };
    ($vis:vis $field:ident : $ty:ident) => {
        $vis fn $field(&self) -> $ty {
            $ty::from_le_bytes(self.$field)
        }
    };
}
