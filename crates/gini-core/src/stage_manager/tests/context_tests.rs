use crate::stage_manager::context::StageContext;
use std::path::PathBuf;
use std::any::TypeId;

// Helper function to create a dummy path for tests
fn dummy_path() -> PathBuf {
    PathBuf::from("./dummy_context_test_path")
}

#[test]
fn test_context_creation_live() {
    let path = dummy_path();
    let context = StageContext::new_live(path.clone());

    assert!(!context.is_dry_run());
    assert_eq!(context.config_dir(), &path);
    // We can't easily check the internal map size without adding a method,
    // but we can verify get_data returns None for an arbitrary key.
    assert!(context.get_data::<i32>("initial_check").is_none());
}

#[test]
fn test_context_creation_dry_run() {
    let path = dummy_path();
    let context = StageContext::new_dry_run(path.clone());

    assert!(context.is_dry_run());
    assert_eq!(context.config_dir(), &path);
}

#[test]
fn test_context_data_storage_retrieval() {
    let mut context = StageContext::new_live(dummy_path());

    // Store different data types
    context.set_data("my_string", "hello".to_string());
    context.set_data("my_int", 42_u32);
    context.set_data("my_bool", true);

    // Retrieve and check string
    let retrieved_string = context.get_data::<String>("my_string");
    assert!(retrieved_string.is_some());
    // Dereference the Option<&String> to compare with owned String
    assert_eq!(retrieved_string.unwrap(), "hello");

    // Retrieve and check integer
    let retrieved_int = context.get_data::<u32>("my_int");
    assert!(retrieved_int.is_some());
    // Dereference the Option<&u32> to compare with literal
    assert_eq!(*retrieved_int.unwrap(), 42);

    // Retrieve and check boolean
    let retrieved_bool = context.get_data::<bool>("my_bool");
    assert!(retrieved_bool.is_some());
    // Dereference the Option<&bool> to compare with literal
    assert_eq!(*retrieved_bool.unwrap(), true);

    // Retrieve non-existent key
    let non_existent = context.get_data::<f64>("non_existent");
    assert!(non_existent.is_none());

    // Retrieve with wrong type
    let wrong_type = context.get_data::<f32>("my_string");
    assert!(wrong_type.is_none());
}

#[test]
fn test_context_data_overwrite() {
    let mut context = StageContext::new_live(dummy_path());

    context.set_data("my_key", 100_i32);
    let first_val = context.get_data::<i32>("my_key").unwrap();
    // Dereference for comparison
    assert_eq!(*first_val, 100);

    // Overwrite with a different value of the same type
    context.set_data("my_key", 200_i32);
    let second_val = context.get_data::<i32>("my_key").unwrap();
    // Dereference for comparison
    assert_eq!(*second_val, 200);

    // Overwrite with a different type (should work as it's Any)
    context.set_data("my_key", "new_string".to_string());
    let third_val = context.get_data::<String>("my_key").unwrap();
    // Dereference for comparison
    assert_eq!(third_val, "new_string");

    // Check the old type is no longer retrievable
    let old_type_val = context.get_data::<i32>("my_key");
    assert!(old_type_val.is_none());
}

// Removed test_context_data_removal as remove_data is not implemented

#[test]
fn test_context_has_data_check() { // Renamed from test_context_has_data
     let mut context = StageContext::new_live(dummy_path());

     context.set_data("my_key", 123);

     // Use get_data().is_some() to check for existence
     assert!(context.get_data::<i32>("my_key").is_some());
     assert!(context.get_data::<i32>("other_key").is_none());
}

// Removed test_context_get_data_ref as get_data already returns a reference

#[test]
fn test_context_get_data_mut() {
    let mut context = StageContext::new_live(dummy_path());
    let my_struct = MyTestData { value: 10 };
    context.set_data("my_struct", my_struct);

    // Get a mutable reference and modify
    {
        let data_mut = context.get_data_mut::<MyTestData>("my_struct");
        assert!(data_mut.is_some());
        data_mut.unwrap().value += 5;
    } // Mutable borrow ends here

    // Get an immutable reference to check modification
    let data_ref = context.get_data::<MyTestData>("my_struct"); // Use get_data
    assert!(data_ref.is_some());
    assert_eq!(data_ref.unwrap().value, 15);

    // Attempt to get mut ref with wrong type
    let wrong_mut_ref = context.get_data_mut::<String>("my_struct");
    assert!(wrong_mut_ref.is_none());

     // Attempt to get mut ref for non-existent key
    let non_existent_mut_ref = context.get_data_mut::<MyTestData>("non_existent");
    assert!(non_existent_mut_ref.is_none());
}

// Helper struct for testing data storage
#[derive(Debug, Clone, PartialEq)]
struct MyTestData {
    value: i32,
}