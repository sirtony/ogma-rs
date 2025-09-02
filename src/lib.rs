pub mod error;
pub mod store;

pub use store::Store;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
    pub struct Address {
        pub street: String,
        pub apt: Option<String>,
        pub city: String,
        pub state: String,
        pub zip: String,
    }

    #[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
    pub struct Person {
        pub first_name: String,
        pub middle_initial: Option<char>,
        pub last_name: String,
        pub age: u8,
        pub address: Address,
    }

    #[test]
    fn test_memory_store() -> error::Result<()> {
        let mut store: Store<u64, Person> = Store::new(Default::default());
        let x = store.set(5, get_person());
        assert!(x.is_none());

        let first = store.get(&5);
        assert!(first.is_some());

        let person = first.unwrap();
        assert_eq!(person, &get_person());

        Ok(())
    }

    #[test]
    fn test_disk_store() -> error::Result<()> {
        let mut store: Store<u64, Person> = Store::new(Default::default());
        let x = store.set(5, get_person());
        assert!(x.is_none());

        store.save()?;
        store = Store::open(Default::default())?;

        let first = store.get(&5);
        assert!(first.is_some());

        let person = first.unwrap();
        assert_eq!(person, &get_person());

        Ok(())
    }

    fn get_person() -> Person {
        Person {
            first_name: "John".to_string(),
            middle_initial: None,
            last_name: "Smith".to_string(),
            age: 35,
            address: Address {
                street: "123 Main St".to_string(),
                apt: Some("F22".to_string()),
                city: "Chicago".to_string(),
                state: "Illinois".to_string(),
                zip: "60606".to_string(),
            },
        }
    }
}
