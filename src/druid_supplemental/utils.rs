macro_rules! selector {
    ($id: ident: $t: ty) => {
        pub const $id: Selector<$t> = Selector::new(concat!(module_path!(), "::", stringify!($id)));
    };
}
