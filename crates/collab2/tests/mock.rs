#![allow(missing_docs)]

#[test]
fn foo() {
    use editor_mock::MockEditor;
    println!("Can depend on {}", core::any::type_name::<MockEditor>());
}
