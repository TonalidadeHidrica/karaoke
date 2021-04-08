macro_rules! selector {
    ($vis: vis $id: ident: $t: ty) => {
        $vis const $id: Selector<$t> = Selector::new(concat!(module_path!(), "::", stringify!($id)));
    };
}
