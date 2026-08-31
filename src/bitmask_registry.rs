//! Registry mapping a Rust type to a named set of bitmask layers.
//!
//! [`BitmaskRegistry`] is a resource holding [`BitmaskLayers`] keyed by the type
//! they describe. Register a reflected enum with
//! [`register_enum`](BitmaskRegistry::register_enum) (or the
//! [`register_bitmask_enum`](crate::InspectorExt::register_bitmask_enum) App
//! extension), then look the layers up at a widget call site and pass them to
//! [`bitmask_field_layers`](crate::bitmask_field_layers):
//!
//! ```
//! use bevy::prelude::*;
//! use msg_inspector::{BitmaskRegistry, egui, bitmask_field_layers};
//!
//! #[derive(Reflect)]
//! enum PhysicsLayer {
//!     Ground,
//!     Water,
//!     Air,
//! }
//!
//! let mut registry = BitmaskRegistry::default();
//! registry.register_enum::<PhysicsLayer>().unwrap();
//!
//! fn field_ui(ui: &mut egui::Ui, bits: &mut u32, registry: &BitmaskRegistry) {
//!     bitmask_field_layers(ui, bits, registry.get::<PhysicsLayer>());
//! }
//! ```

use std::any::TypeId;
use std::collections::HashMap;

use bevy::prelude::*;
use bevy::reflect::{TypeInfo, TypePath, Typed};

use crate::widgets::bitmask::BitmaskLayers;

/// A resource storing named bitmask layers, keyed by the type they describe.
///
/// Register enums with [`register_enum`](Self::register_enum) (or the
/// [`register_bitmask_enum`](crate::InspectorExt::register_bitmask_enum) App
/// extension) at app-build time, then read them back at widget call sites with
/// [`get`](Self::get).
#[derive(Resource, Default)]
pub struct BitmaskRegistry {
    entries: HashMap<TypeId, BitmaskLayers>,
}

impl BitmaskRegistry {
    /// Store `layers` under key type `K`, replacing any prior entry.
    ///
    /// `K` is only a lookup key — it need not be the type of the field the
    /// widget edits. A host editing a foreign bitmask type can key its labels by
    /// the enum they come from and look them up with the same type.
    pub fn register<K: 'static>(&mut self, layers: BitmaskLayers) -> &mut Self {
        self.entries.insert(TypeId::of::<K>(), layers);
        self
    }

    /// Register a reflected enum `E` as a bitmask layer set: variant `i` (in
    /// declaration order) names bit `i`.
    ///
    /// This fits a bitflags-style enum whose variants are declared in bit order
    /// (variant 0 → bit 0, and so on). For any other bit mapping, build a
    /// [`BitmaskLayers`] with [`from_labels`](BitmaskLayers::from_labels) and
    /// pass it to [`register`](Self::register). The stored set is keyed by `E`
    /// and named after its short type path.
    ///
    /// # Errors
    ///
    /// Returns [`NotAnEnum`] (and stores nothing) when `E`'s reflected type is
    /// not an enum.
    pub fn register_enum<E: Typed + TypePath>(&mut self) -> Result<(), NotAnEnum> {
        let TypeInfo::Enum(enum_info) = E::type_info() else {
            return Err(NotAnEnum(E::type_path()));
        };
        let names = enum_info.iter().map(|variant| variant.name());
        let layers = BitmaskLayers::from_names(E::short_type_path(), names);
        self.entries.insert(TypeId::of::<E>(), layers);
        Ok(())
    }

    /// The layers registered for key type `K`, if any.
    #[must_use]
    pub fn get<K: 'static>(&self) -> Option<&BitmaskLayers> {
        self.get_by_id(TypeId::of::<K>())
    }

    /// The layers registered for `type_id`, if any.
    #[must_use]
    pub fn get_by_id(&self, type_id: TypeId) -> Option<&BitmaskLayers> {
        self.entries.get(&type_id)
    }

    /// The number of registered layer sets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Error returned by [`BitmaskRegistry::register_enum`] when the reflected type
/// is not an enum. Carries the type's full reflected path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotAnEnum(pub &'static str);

impl std::fmt::Display for NotAnEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "type `{}` is not a reflected enum", self.0)
    }
}

impl std::error::Error for NotAnEnum {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Reflect)]
    enum PhysicsLayer {
        Ground,
        Water,
        Air,
    }

    #[derive(Reflect)]
    struct NotAnEnumType(u32);

    /// A distinct key type used to register labels for a foreign field type.
    struct GroupKey;

    #[test]
    fn register_enum_names_bits_in_declaration_order() {
        let mut registry = BitmaskRegistry::default();
        assert!(registry.is_empty());

        registry.register_enum::<PhysicsLayer>().unwrap();

        let layers = registry.get::<PhysicsLayer>().expect("registered");
        assert_eq!(layers.name(), "PhysicsLayer");
        assert_eq!(layers.label(0), Some("Ground"));
        assert_eq!(layers.label(1), Some("Water"));
        assert_eq!(layers.label(2), Some("Air"));
        assert_eq!(layers.mask(), 0b111);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn register_enum_rejects_non_enum_types() {
        let mut registry = BitmaskRegistry::default();
        let err = registry.register_enum::<NotAnEnumType>().unwrap_err();
        assert!(err.to_string().contains("not a reflected enum"));
        assert!(registry.get::<NotAnEnumType>().is_none());
        assert!(registry.is_empty());
    }

    #[test]
    fn register_stores_explicit_layers_under_key() {
        let mut registry = BitmaskRegistry::default();
        registry.register::<GroupKey>(BitmaskLayers::from_labels("Group", [(0, "a"), (6, "b")]));

        let layers = registry.get::<GroupKey>().expect("registered");
        assert_eq!(layers.label(6), Some("b"));
        assert_eq!(layers.mask(), (1 << 0) | (1 << 6));
        assert!(registry.get::<PhysicsLayer>().is_none());
    }
}
