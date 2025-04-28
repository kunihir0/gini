// crates/gini-core/src/plugin_system/tests/adapter_tests.rs
#![cfg(test)]

// Import macro from crate root, and other items from the adapter module
use crate::define_adapter; // Use the macro from the crate root
use crate::plugin_system::adapter::{Adapter, AdapterRegistry};
use std::any::{Any, TypeId};

// --- Mock Setup ---

// 1. Define a mock trait
trait MockAdapterTrait: Send + Sync {
    fn greet(&self) -> String;
    fn set_greeting(&mut self, new_greeting: &str);
}

// 2. Define mock implementations
#[derive(Clone)]
struct MockAdapterImpl {
    name: String,
    greeting: String,
}

impl MockAdapterImpl {
    fn new(name: &str, greeting: &str) -> Self {
        Self { name: name.to_string(), greeting: greeting.to_string() }
    }
}

impl MockAdapterTrait for MockAdapterImpl {
    fn greet(&self) -> String {
        format!("{} says: {}", self.name, self.greeting)
    }
    fn set_greeting(&mut self, new_greeting: &str) {
        self.greeting = new_greeting.to_string();
    }
}

// 3. Use define_adapter! macro (without crate:: prefix as it's in scope via `use`)
define_adapter!(MockAdapterImplAdapter, MockAdapterImpl, MockAdapterTrait);

// Another mock for testing duplicates/different types
trait AnotherMockTrait: Send + Sync {
    fn value(&self) -> i32;
}
#[derive(Clone)]
struct AnotherMockImpl { value: i32 }
impl AnotherMockTrait for AnotherMockImpl {
    fn value(&self) -> i32 { self.value }
}
// Use define_adapter! macro (without crate:: prefix)
define_adapter!(AnotherMockImplAdapter, AnotherMockImpl, AnotherMockTrait);


// --- Tests ---

#[test]
fn test_adapter_registry_new_default() {
    let registry_new = AdapterRegistry::new();
    let registry_default = AdapterRegistry::default();

    assert_eq!(registry_new.count(), 0);
    assert!(registry_new.names().is_empty());
    assert_eq!(registry_default.count(), 0);
    assert!(registry_default.names().is_empty());
}

#[test]
fn test_adapter_registry_register_success() {
    let mut registry = AdapterRegistry::new();
    // Create the implementation first
    let implementation = MockAdapterImpl::new("Adapter1Impl", "Hello");
    // Create the adapter wrapper using the macro-generated struct and its new method
    let adapter_wrapper = MockAdapterImplAdapter::new("Adapter1", implementation);
    let adapter_name = adapter_wrapper.name().to_string(); // Get name from wrapper

    // Register the wrapper instance directly
    let result = registry.register(adapter_wrapper);
    assert!(result.is_ok());
    assert_eq!(registry.count(), 1);
    // Use the wrapper type for `has`
    assert!(registry.has::<MockAdapterImplAdapter<MockAdapterImpl>>());
    assert!(registry.has_name(&adapter_name));
    assert_eq!(registry.names(), vec![adapter_name]);
}

#[test]
fn test_adapter_registry_register_duplicate() {
    let mut registry = AdapterRegistry::new();
    let impl1 = MockAdapterImpl::new("Impl1", "Hello");
    let adapter1 = MockAdapterImplAdapter::new("Adapter1", impl1); // Name: Adapter1, Type: MockAdapterImplAdapter<MockAdapterImpl>

    let impl2 = MockAdapterImpl::new("Impl2", "Hi");
    let adapter2 = MockAdapterImplAdapter::new("Adapter1", impl2); // Same name as adapter1

    let impl3 = MockAdapterImpl::new("Impl3", "Yo");
    let adapter3 = MockAdapterImplAdapter::new("Adapter3", impl3); // Different name, same type as adapter1

    let another_impl = AnotherMockImpl { value: 100 };
    let adapter4 = AnotherMockImplAdapter::new("Adapter4", another_impl.clone()); // Different name, different type

    // Register first one successfully
    assert!(registry.register(adapter1).is_ok());
    assert_eq!(registry.count(), 1);

    // Attempt duplicate name (adapter2 has name "Adapter1")
    let result_dup_name = registry.register(adapter2); // adapter2 has same name AND same type as adapter1
    assert!(result_dup_name.is_err());
    // The registry checks TypeId *before* name, so the TypeId error occurs first.
    assert!(result_dup_name.unwrap_err().to_string().contains("Adapter already registered for type ID"));
    assert_eq!(registry.count(), 1); // Count unchanged

    // Attempt duplicate TypeId (adapter3 has same type as adapter1, but different name)
    let result_dup_type = registry.register(adapter3);
     assert!(result_dup_type.is_err());
     // The error message checks TypeId, which is unique per generated adapter struct instance type
     // So registering another MockAdapterImplAdapter<MockAdapterImpl> fails the TypeId check.
     assert!(result_dup_type.unwrap_err().to_string().contains("Adapter already registered for type ID"));
     assert_eq!(registry.count(), 1); // Count unchanged


    // Register adapter4 (different name and type) - should succeed
    assert!(registry.register(adapter4).is_ok());
    assert_eq!(registry.count(), 2);

    // Attempt duplicate name with different TypeId (adapter5 has name "Adapter4")
    let another_impl_2 = AnotherMockImpl { value: 200 };
    let adapter5 = AnotherMockImplAdapter::new("Adapter4", another_impl_2); // Same name as adapter4
    let result_dup_name_diff_type = registry.register(adapter5); // adapter5 has name "Adapter4"
    assert!(result_dup_name_diff_type.is_err());
    // The registry checks TypeId *before* name. Since adapter4 (type AnotherMockImplAdapter) is already registered,
    // registering adapter5 (also type AnotherMockImplAdapter) fails the TypeId check first.
    assert!(result_dup_name_diff_type.unwrap_err().to_string().contains("Adapter already registered for type ID"));
    assert_eq!(registry.count(), 2); // Count unchanged
}


#[test]
fn test_adapter_registry_get_by_type() {
    let mut registry = AdapterRegistry::new();
    let implementation = MockAdapterImpl::new("GetterImpl", "Get me");
    let adapter_wrapper = MockAdapterImplAdapter::new("Getter", implementation);
    registry.register(adapter_wrapper).unwrap();

    // Get existing (immutable) - use the wrapper type
    let retrieved_wrapper = registry.get::<MockAdapterImplAdapter<MockAdapterImpl>>();
    assert!(retrieved_wrapper.is_some());
    // Access the implementation to call the original trait method
    assert_eq!(retrieved_wrapper.unwrap().implementation().greet(), "GetterImpl says: Get me");

    // Get existing (mutable) - use the wrapper type
    let retrieved_mut_wrapper = registry.get_mut::<MockAdapterImplAdapter<MockAdapterImpl>>();
    assert!(retrieved_mut_wrapper.is_some());
    // Access the mutable implementation
    retrieved_mut_wrapper.unwrap().implementation_mut().set_greeting("Gotten!");

    // Verify mutation
    let retrieved_after_mut_wrapper = registry.get::<MockAdapterImplAdapter<MockAdapterImpl>>();
    assert_eq!(retrieved_after_mut_wrapper.unwrap().implementation().greet(), "GetterImpl says: Gotten!");

    // Get non-existent type
    let non_existent = registry.get::<AnotherMockImplAdapter<AnotherMockImpl>>();
    assert!(non_existent.is_none());
}

#[test]
fn test_adapter_registry_get_by_name() {
    let mut registry = AdapterRegistry::new();
    let implementation = MockAdapterImpl::new("NameGetterImpl", "By name");
    let adapter_wrapper = MockAdapterImplAdapter::new("NameGetter", implementation);
    let adapter_name = adapter_wrapper.name().to_string();
    registry.register(adapter_wrapper).unwrap();

    // Get existing by name (immutable) - use the wrapper type
    let retrieved = registry.get_by_name::<MockAdapterImplAdapter<MockAdapterImpl>>(&adapter_name);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().implementation().greet(), "NameGetterImpl says: By name");

    // Get existing by name (mutable) - use the wrapper type
    let retrieved_mut = registry.get_by_name_mut::<MockAdapterImplAdapter<MockAdapterImpl>>(&adapter_name);
    assert!(retrieved_mut.is_some());
    retrieved_mut.unwrap().implementation_mut().set_greeting("Mutated by name!");

    // Verify mutation
    let retrieved_after_mut = registry.get_by_name::<MockAdapterImplAdapter<MockAdapterImpl>>(&adapter_name);
    assert_eq!(retrieved_after_mut.unwrap().implementation().greet(), "NameGetterImpl says: Mutated by name!");

    // Get non-existent name
    let non_existent = registry.get_by_name::<MockAdapterImplAdapter<MockAdapterImpl>>("non_existent_name");
    assert!(non_existent.is_none());

    // Get existing name with wrong type cast
    let wrong_type = registry.get_by_name::<AnotherMockImplAdapter<AnotherMockImpl>>(&adapter_name);
    assert!(wrong_type.is_none());
}

#[test]
fn test_adapter_registry_has() {
    let mut registry = AdapterRegistry::new();
    let implementation = MockAdapterImpl::new("CheckerImpl", "Check me");
    let adapter_wrapper = MockAdapterImplAdapter::new("Checker", implementation);
    let adapter_name = adapter_wrapper.name().to_string();

    // Use the wrapper type for `has`
    assert!(!registry.has::<MockAdapterImplAdapter<MockAdapterImpl>>());
    assert!(!registry.has_name(&adapter_name));

    registry.register(adapter_wrapper).unwrap();

    assert!(registry.has::<MockAdapterImplAdapter<MockAdapterImpl>>());
    assert!(registry.has_name(&adapter_name));
    // Check non-existent type using its wrapper
    assert!(!registry.has::<AnotherMockImplAdapter<AnotherMockImpl>>());
    assert!(!registry.has_name("wrong_name")); // Check non-existent name
}

#[test]
fn test_adapter_registry_remove() {
    let mut registry = AdapterRegistry::new();
    let impl1 = MockAdapterImpl::new("Remover1Impl", "R1");
    let adapter1 = MockAdapterImplAdapter::new("Remover1", impl1);
    let impl2 = AnotherMockImpl { value: 1 };
    let adapter2 = AnotherMockImplAdapter::new("Remover2", impl2);

    let adapter1_name = adapter1.name().to_string();
    let adapter2_name = adapter2.name().to_string();

    registry.register(adapter1).unwrap();
    registry.register(adapter2).unwrap();
    assert_eq!(registry.count(), 2);

    // Remove by type - use the wrapper type
    let removed1 = registry.remove::<MockAdapterImplAdapter<MockAdapterImpl>>();
    assert!(removed1.is_some());
    assert_eq!(registry.count(), 1);
    assert!(!registry.has::<MockAdapterImplAdapter<MockAdapterImpl>>());
    assert!(!registry.has_name(&adapter1_name));
    // Ensure other adapter is still there - use its wrapper type
    assert!(registry.has::<AnotherMockImplAdapter<AnotherMockImpl>>());
    assert!(registry.has_name(&adapter2_name));

    // Attempt remove non-existent type
    let removed_non_existent = registry.remove::<MockAdapterImplAdapter<MockAdapterImpl>>();
    assert!(removed_non_existent.is_none());
    assert_eq!(registry.count(), 1);

    // Remove by name
    let removed2 = registry.remove_by_name(&adapter2_name);
    assert!(removed2.is_some());
    assert_eq!(registry.count(), 0);
    assert!(!registry.has::<AnotherMockImplAdapter<AnotherMockImpl>>());
    assert!(!registry.has_name(&adapter2_name));
    assert!(registry.names().is_empty());

     // Attempt remove non-existent name
    let removed_non_existent_name = registry.remove_by_name(&adapter2_name);
    assert!(removed_non_existent_name.is_none());
    assert_eq!(registry.count(), 0);
}

#[test]
fn test_adapter_registry_count() {
    let mut registry = AdapterRegistry::new();
    assert_eq!(registry.count(), 0);

    let impl1 = MockAdapterImpl::new("c1_impl", "C1");
    let adapter1 = MockAdapterImplAdapter::new("c1", impl1);
    registry.register(adapter1).unwrap();
    assert_eq!(registry.count(), 1);

    let impl2 = AnotherMockImpl { value: 2 };
    let adapter2 = AnotherMockImplAdapter::new("c2", impl2);
    registry.register(adapter2).unwrap();
    assert_eq!(registry.count(), 2);

    // Remove using wrapper type
    registry.remove::<MockAdapterImplAdapter<MockAdapterImpl>>();
    assert_eq!(registry.count(), 1);

    registry.remove_by_name("c2"); // Use the name given during construction
    assert_eq!(registry.count(), 0);
}

#[test]
fn test_adapter_registry_names() {
    let mut registry = AdapterRegistry::new();
    assert!(registry.names().is_empty());

    let impl1 = MockAdapterImpl::new("Name1Impl", "N1");
    let adapter1 = MockAdapterImplAdapter::new("Name1", impl1);
    let impl2 = AnotherMockImpl { value: 1 };
    let adapter2 = AnotherMockImplAdapter::new("Name2", impl2);

    let name1 = adapter1.name().to_string();
    let name2 = adapter2.name().to_string();

    registry.register(adapter1).unwrap();
    registry.register(adapter2).unwrap();

    let mut names = registry.names();
    names.sort(); // Sort for consistent comparison
    let mut expected = vec![name1.clone(), name2.clone()];
    expected.sort();
    assert_eq!(names, expected);

    registry.remove_by_name(&name1);
    let names_after_remove = registry.names();
    assert_eq!(names_after_remove, vec![name2]); // Only name2 should remain
}

#[test]
fn test_define_adapter_macro() {
    // Define a simple trait and struct specifically for this test
    trait MacroTestTrait: Send + Sync {
        fn get_id(&self) -> u32;
    }
    #[derive(Clone)] // Add Clone if needed by wrapper
    struct MacroTestImpl { id: u32 }
    impl MacroTestTrait for MacroTestImpl {
        fn get_id(&self) -> u32 { self.id }
    }

    // Use the macro - defines MacroTestImplAdapter<MacroTestImpl>
    define_adapter!(MacroTestImplAdapter, MacroTestImpl, MacroTestTrait);

    // Instantiate the implementation
    let implementation = MacroTestImpl { id: 123 };
    // Instantiate the adapter wrapper
    let adapter_wrapper = MacroTestImplAdapter::new("MacroTestAdapter", implementation);

    // Verify Adapter trait methods on the wrapper
    assert_eq!(adapter_wrapper.name(), "MacroTestAdapter");
    // Disambiguate the type_id call by specifying the Adapter trait
    assert_eq!(Adapter::type_id(&adapter_wrapper), TypeId::of::<MacroTestImplAdapter<MacroTestImpl>>());

    // Box the wrapper to test downcasting from Box<dyn Adapter>
    let boxed_adapter: Box<dyn Adapter> = Box::new(adapter_wrapper);

    // Verify downcasting via as_any to the wrapper type
    let any_ref = boxed_adapter.as_any();
    assert!(any_ref.is::<MacroTestImplAdapter<MacroTestImpl>>());
    let downcasted_wrapper: Option<&MacroTestImplAdapter<MacroTestImpl>> = any_ref.downcast_ref::<MacroTestImplAdapter<MacroTestImpl>>();
    assert!(downcasted_wrapper.is_some());

    // Access the original implementation through the wrapper
    let retrieved_impl = downcasted_wrapper.unwrap().implementation();
    assert_eq!(retrieved_impl.id, 123);
    assert_eq!(retrieved_impl.get_id(), 123); // Call original trait method
}