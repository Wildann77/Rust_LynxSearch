use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationErrors};

/// DTO path parameter untuk endpoint dengan single `:id` berupa UUID.
/// Mendukung `ValidatedPath<PathUuid>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Validate)]
pub struct PathUuid {
    pub id: Uuid,
}

impl PathUuid {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }
}

impl From<Uuid> for PathUuid {
    fn from(id: Uuid) -> Self {
        Self { id }
    }
}

impl From<PathUuid> for Uuid {
    fn from(path: PathUuid) -> Self {
        path.id
    }
}

/// Transparent wrapper untuk UUID yang mengimplementasikan `validator::Validate`.
/// Memungkinkan pemakaian langsung `ValidatedPath<ValidatedUuid>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValidatedUuid(pub Uuid);

impl ValidatedUuid {
    pub fn new(id: Uuid) -> Self {
        Self(id)
    }

    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Validate for ValidatedUuid {
    fn validate(&self) -> Result<(), ValidationErrors> {
        Ok(())
    }
}

impl Deref for ValidatedUuid {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Uuid> for ValidatedUuid {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<ValidatedUuid> for Uuid {
    fn from(v: ValidatedUuid) -> Self {
        v.0
    }
}

impl fmt::Display for ValidatedUuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_uuid_and_validated_uuid_conversion() {
        let raw = Uuid::new_v4();
        let path_uuid = PathUuid::from(raw);
        assert_eq!(path_uuid.id, raw);
        assert!(path_uuid.validate().is_ok());

        let val_uuid = ValidatedUuid::from(raw);
        assert_eq!(*val_uuid, raw);
        assert_eq!(val_uuid.to_string(), raw.to_string());
        assert!(val_uuid.validate().is_ok());
    }
}
