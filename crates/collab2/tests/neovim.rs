#![allow(missing_docs)]

#[test]
fn foo() {
    use editor_neovim::Neovim;
    println!("Can depend on {}", core::any::type_name::<Neovim>());
}
