//! Wide-bitmask editor for `u32`/bitflags-style values.
//!
//! Default reflection-based inspectors render a bare `u32` bitmask in a narrow
//! `DragValue` — masks with high bits set get clipped and the value can't be
//! selected for copy/paste. [`bitmask_field`] renders a selectable text field
//! that scales with the available panel width instead, plus one toggle per
//! named bit and select-all/none buttons.
//!
//! Bit labels come from a slice of names ([`bitmask_field`]) or a closure
//! ([`bitmask_field_with`]), so any bitflags-style value works: convert to its
//! raw `u32` bits, edit, convert back (bitflags-style types offer
//! `from_bits_truncate`).
//!
//! [`bitmask_field_layers`] is a variant that shows one checkbox per layer under
//! a collapsing header folded by default: labelled by bit index when given no
//! names, or by a [`BitmaskLayers`] set otherwise. Register a reflected enum as
//! such a set through [`BitmaskRegistry`](crate::BitmaskRegistry).

use std::collections::BTreeMap;

use bevy_egui::egui;

/// Comfortable minimum width for a full 32-bit decimal value (up to 10 digits).
const MIN_WIDTH: f32 = 160.0;

/// A named set of bitmask layers: a display label for the bit at each named
/// position.
///
/// [`bitmask_field_layers`] draws one checkbox per named layer instead of a bare
/// bit index. Unnamed positions are omitted from the checkbox list; the raw bits
/// stay editable through the widget's text field regardless.
///
/// Build one from an ordered list of names ([`BitmaskLayers::from_names`], where
/// bit `i` is `names[i]`), from explicit `(bit, label)` pairs
/// ([`BitmaskLayers::from_labels`]), or incrementally
/// ([`BitmaskLayers::with_label`]). Only bits `0..32` are stored; any position at
/// or past 32 is ignored. Registering a reflected enum through
/// [`BitmaskRegistry`](crate::BitmaskRegistry) uses `from_names`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BitmaskLayers {
    name: String,
    labels: BTreeMap<u32, String>,
}

impl BitmaskLayers {
    /// An empty layer set with the given display name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            labels: BTreeMap::new(),
        }
    }

    /// A layer set naming bit `i` after `names[i]`, in order.
    ///
    /// Names past bit 31 are dropped. This is the mapping used for a
    /// bitflags-style enum whose variants are declared in bit order.
    #[must_use]
    pub fn from_names<S: Into<String>>(
        name: impl Into<String>,
        names: impl IntoIterator<Item = S>,
    ) -> Self {
        let labels = names
            .into_iter()
            .take(u32::BITS as usize)
            .enumerate()
            .map(|(bit, label)| (bit as u32, label.into()))
            .collect();
        Self {
            name: name.into(),
            labels,
        }
    }

    /// A layer set from explicit `(bit, label)` pairs.
    ///
    /// Later pairs overwrite earlier ones for the same bit; positions at or past
    /// bit 32 are dropped. Use this when the bit positions are not a contiguous
    /// `0..n` run.
    #[must_use]
    pub fn from_labels<S: Into<String>>(
        name: impl Into<String>,
        labels: impl IntoIterator<Item = (u32, S)>,
    ) -> Self {
        let labels = labels
            .into_iter()
            .filter(|(bit, _)| *bit < u32::BITS)
            .map(|(bit, label)| (bit, label.into()))
            .collect();
        Self {
            name: name.into(),
            labels,
        }
    }

    /// Name bit `bit` after `label`, replacing any existing label for it.
    ///
    /// A `bit` at or past 32 is ignored.
    #[must_use]
    pub fn with_label(mut self, bit: u32, label: impl Into<String>) -> Self {
        if bit < u32::BITS {
            self.labels.insert(bit, label.into());
        }
        self
    }

    /// The display name of the layer set.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The label for `bit`, if it is named.
    #[must_use]
    pub fn label(&self, bit: u32) -> Option<&str> {
        self.labels.get(&bit).map(String::as_str)
    }

    /// Every named bit paired with its label, in ascending bit order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &str)> {
        self.labels
            .iter()
            .map(|(&bit, label)| (bit, label.as_str()))
    }

    /// The union of every named bit.
    #[must_use]
    pub fn mask(&self) -> u32 {
        self.labels.keys().fold(0, |mask, &bit| mask | (1 << bit))
    }

    /// The number of named layers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// Whether no layer is named.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }
}

/// Edit a `u32` bitmask with per-bit toggles labelled from a slice of names.
///
/// `names[i]` labels bit `i`; bits at or past `names.len()` are unnamed. See
/// [`bitmask_field_with`] for the full behavior.
///
/// # Example
///
/// ```
/// use msg_inspector::widgets::bitmask_field;
///
/// fn field_ui(ui: &mut msg_inspector::egui::Ui, bits: &mut u32) {
///     let changed = bitmask_field(ui, bits, &["Ground", "Water", "Air"]);
///     let _ = changed;
/// }
/// ```
pub fn bitmask_field(ui: &mut egui::Ui, bits: &mut u32, names: &[&str]) -> bool {
    bitmask_field_with(ui, bits, |bit| names.get(bit as usize).copied())
}

/// Edit a `u32` bitmask with per-bit toggles labelled by a closure.
///
/// Renders, top to bottom:
///
/// - a selectable decimal text field for the raw value, sized to the available
///   panel width (never narrower than a full 10-digit `u32`), applied when the
///   edited text parses as a `u32`;
/// - `All` / `None` buttons that set/clear every *named* bit, leaving unnamed
///   bits untouched;
/// - one checkbox per bit for which `label` returns `Some`, in ascending bit
///   order. `label` is called at most once per bit per frame, and may return
///   any [`egui::WidgetText`] source (`&str`, `String`, [`egui::RichText`]).
///
/// While the raw-value field has keyboard focus, in-progress text that does
/// not parse as a `u32` (an emptied field, a stray character) is kept as
/// typed, so the value can be cleared and retyped; the field re-syncs with
/// `bits` when focus is lost. The field's id is derived from the surrounding
/// [`egui::Ui`], so hosts drawing more than one bitmask field in the same
/// panel should wrap each call in [`egui::Ui::push_id`].
///
/// Returns `true` when the mask changed this frame.
pub fn bitmask_field_with<T: Into<egui::WidgetText>>(
    ui: &mut egui::Ui,
    bits: &mut u32,
    label: impl Fn(u32) -> Option<T>,
) -> bool {
    let mut changed = raw_value_field(ui, bits);

    let named = named_bits(&label);
    changed |= all_none_row(ui, bits, named_mask(&named));

    for (bit, name) in named {
        changed |= bit_checkbox(ui, bits, bit, name);
    }

    changed
}

/// Edit a `u32` bitmask with one checkbox per layer, the checkboxes under a
/// header folded by default to save panel space.
///
/// The raw-value text field and `All` / `None` buttons render first and stay
/// visible; only the per-layer checkbox list collapses. Layers come from
/// `layers`:
///
/// - `Some(_)` — one checkbox per named layer, labelled by name; `All` / `None`
///   cover exactly those layers. The header shows the set's
///   [`name`](BitmaskLayers::name).
/// - `None` — one checkbox for every bit `0..32`, labelled by its index;
///   `All` / `None` cover the whole mask.
///
/// The raw-value field behaves as in [`bitmask_field_with`]. The header and
/// field ids derive from the surrounding [`egui::Ui`], so hosts drawing more
/// than one bitmask field in a panel should wrap each call in
/// [`egui::Ui::push_id`].
///
/// Returns `true` when the mask changed this frame.
///
/// # Example
///
/// ```
/// use msg_inspector::widgets::{BitmaskLayers, bitmask_field_layers};
///
/// fn field_ui(ui: &mut msg_inspector::egui::Ui, bits: &mut u32) {
///     let layers = BitmaskLayers::from_names("Physics", ["Ground", "Water", "Air"]);
///     let changed = bitmask_field_layers(ui, bits, Some(&layers));
///     let _ = changed;
/// }
/// ```
pub fn bitmask_field_layers(
    ui: &mut egui::Ui,
    bits: &mut u32,
    layers: Option<&BitmaskLayers>,
) -> bool {
    let mut changed = raw_value_field(ui, bits);

    let active_mask = layers.map_or(u32::MAX, BitmaskLayers::mask);
    changed |= all_none_row(ui, bits, active_mask);

    let heading = layers
        .map(BitmaskLayers::name)
        .filter(|name| !name.is_empty())
        .unwrap_or("Layers");

    egui::CollapsingHeader::new(heading)
        .default_open(false)
        .show(ui, |ui| match layers {
            Some(layers) => {
                for (bit, name) in layers.iter() {
                    changed |= bit_checkbox(ui, bits, bit, name);
                }
            }
            None => {
                for bit in 0..u32::BITS {
                    changed |= bit_checkbox(ui, bits, bit, bit.to_string());
                }
            }
        });

    changed
}

/// Draw the selectable raw-value text field and apply any parsed edit to `bits`.
///
/// Returns `true` when the mask changed this frame. While the field has focus,
/// in-progress text that does not parse as a `u32` is kept as typed; the field
/// re-syncs with `bits` when focus is lost.
fn raw_value_field(ui: &mut egui::Ui, bits: &mut u32) -> bool {
    let field_id = ui.make_persistent_id("bitmask_raw_value");
    let mut text = ui
        .data_mut(|data| data.get_temp::<String>(field_id))
        .unwrap_or_else(|| bits.to_string());
    let response = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id(field_id)
            .desired_width(ui.available_width().max(MIN_WIDTH)),
    );

    let mut changed = false;
    if response.changed()
        && let Some(parsed) = parse_raw_value(&text, *bits)
    {
        *bits = parsed;
        changed = true;
    }

    if response.has_focus() {
        ui.data_mut(|data| data.insert_temp(field_id, text));
    } else {
        ui.data_mut(|data| data.remove_temp::<String>(field_id));
    }

    changed
}

/// Draw the `All` / `None` buttons that set or clear `mask` within `bits`.
///
/// Returns `true` when the mask changed this frame.
fn all_none_row(ui: &mut egui::Ui, bits: &mut u32, mask: u32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        if ui.button("All").clicked() && *bits | mask != *bits {
            *bits |= mask;
            changed = true;
        }
        if ui.button("None").clicked() && *bits & !mask != *bits {
            *bits &= !mask;
            changed = true;
        }
    });
    changed
}

/// Draw one labelled checkbox for `bit` and toggle it in `bits`.
///
/// Returns `true` when the bit changed this frame. `bit` must be `< 32`.
fn bit_checkbox<T: Into<egui::WidgetText>>(
    ui: &mut egui::Ui,
    bits: &mut u32,
    bit: u32,
    label: T,
) -> bool {
    let mut set = *bits & (1 << bit) != 0;
    if ui.checkbox(&mut set, label).changed() {
        if set {
            *bits |= 1 << bit;
        } else {
            *bits &= !(1 << bit);
        }
        true
    } else {
        false
    }
}

/// Parse an edited raw-value string into a replacement mask.
///
/// Returns `Some` only when the text is a valid `u32` that differs from the
/// current bits, so in-progress edits that don't parse (an emptied field,
/// stray characters, out-of-range values) and no-op edits leave the mask
/// untouched.
fn parse_raw_value(text: &str, bits: u32) -> Option<u32> {
    text.parse::<u32>().ok().filter(|&parsed| parsed != bits)
}

/// Every named bit paired with its label, in ascending bit order; `label` is
/// invoked exactly once per bit.
///
/// [`bitmask_field_with`] uses this to collect its labels once per frame;
/// hosts building their own bit UIs can reuse it with [`named_mask`].
///
/// # Example
///
/// ```
/// use msg_inspector::widgets::bitmask::named_bits;
///
/// let names = ["Ground", "Water"];
/// let named = named_bits(&|bit| names.get(bit as usize).copied());
/// assert_eq!(named, vec![(0, "Ground"), (1, "Water")]);
/// ```
pub fn named_bits<T>(label: &impl Fn(u32) -> Option<T>) -> Vec<(u32, T)> {
    (0..u32::BITS)
        .filter_map(|bit| label(bit).map(|name| (bit, name)))
        .collect()
}

/// The union of the bits collected by [`named_bits`].
///
/// # Example
///
/// ```
/// use msg_inspector::widgets::bitmask::named_mask;
///
/// assert_eq!(named_mask(&[(0, "Ground"), (3, "Air")]), 0b1001);
/// ```
pub fn named_mask<T>(named: &[(u32, T)]) -> u32 {
    named.iter().fold(0, |mask, (bit, _)| mask | (1 << bit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// Runs one egui frame of the widget and returns (changed, bits after).
    fn run_frame(bits: u32, names: &[&str]) -> (bool, u32) {
        let ctx = egui::Context::default();
        let mut bits = bits;
        let mut changed = false;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                changed = bitmask_field(ui, &mut bits, names);
            });
        });
        (changed, bits)
    }

    #[test]
    fn builds_without_input_and_leaves_bits_untouched() {
        let (changed, bits) = run_frame(0b101, &["Ground", "Water", "Air"]);
        assert!(!changed);
        assert_eq!(bits, 0b101);
    }

    #[test]
    fn builds_with_no_named_bits() {
        let (changed, bits) = run_frame(u32::MAX, &[]);
        assert!(!changed);
        assert_eq!(bits, u32::MAX);
    }

    #[test]
    fn closure_labels_render_sparse_bits() {
        let ctx = egui::Context::default();
        let mut bits = 1 << 31;
        let mut changed = false;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                changed = bitmask_field_with(ui, &mut bits, |bit| match bit {
                    0 => Some("First"),
                    31 => Some("Last"),
                    _ => None,
                });
            });
        });
        assert!(!changed);
        assert_eq!(bits, 1 << 31);
    }

    // Focus-driven editing (clear the field, retype, lose focus) is not
    // simulated here: headless `RawInput` carries no reliable way to focus the
    // text field and stream key events without an input-injection test
    // harness, so the parse decision is covered as a pure function instead.
    #[test]
    fn raw_value_parse_accepts_only_changed_valid_u32() {
        assert_eq!(parse_raw_value("42", 7), Some(42));
        assert_eq!(parse_raw_value("4294967295", 0), Some(u32::MAX));
        assert_eq!(parse_raw_value("007", 0), Some(7));

        // In-progress states that must not clobber the mask.
        assert_eq!(parse_raw_value("", 7), None);
        assert_eq!(parse_raw_value("-", 7), None);
        assert_eq!(parse_raw_value("abc", 7), None);
        assert_eq!(parse_raw_value("4294967296", 7), None);

        // No-op edits report no change.
        assert_eq!(parse_raw_value("7", 7), None);
    }

    #[test]
    fn label_is_called_once_per_bit_per_frame() {
        let ctx = egui::Context::default();
        let calls = Cell::new(0u32);
        let mut bits = 0b11;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                bitmask_field_with(ui, &mut bits, |bit| {
                    calls.set(calls.get() + 1);
                    (bit < 2).then_some("named")
                });
            });
        });
        assert_eq!(calls.get(), u32::BITS);
    }

    #[test]
    fn named_bits_collects_and_masks_labelled_positions() {
        let named = named_bits(&|bit| match bit {
            0 | 2 | 31 => Some("on"),
            _ => None,
        });
        assert_eq!(
            named.iter().map(|(bit, _)| *bit).collect::<Vec<_>>(),
            vec![0, 2, 31]
        );
        assert_eq!(named_mask(&named), 0b101 | (1 << 31));

        let names = ["a", "b", "c"];
        let named = named_bits(&|bit| names.get(bit as usize).copied());
        assert_eq!(named_mask(&named), 0b111);

        assert_eq!(named_mask(&named_bits(&|_| None::<&str>)), 0);
    }

    /// Runs one egui frame of the layers widget and returns (changed, bits after).
    fn run_layers_frame(bits: u32, layers: Option<&BitmaskLayers>) -> (bool, u32) {
        let ctx = egui::Context::default();
        let mut bits = bits;
        let mut changed = false;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                changed = bitmask_field_layers(ui, &mut bits, layers);
            });
        });
        (changed, bits)
    }

    #[test]
    fn bitmask_layers_from_names_maps_index_to_bit() {
        let layers = BitmaskLayers::from_names("Physics", ["Ground", "Water", "Air"]);
        assert_eq!(layers.name(), "Physics");
        assert_eq!(layers.label(0), Some("Ground"));
        assert_eq!(layers.label(2), Some("Air"));
        assert_eq!(layers.label(3), None);
        assert_eq!(layers.mask(), 0b111);
        assert_eq!(layers.len(), 3);
        assert!(!layers.is_empty());
        assert_eq!(
            layers.iter().collect::<Vec<_>>(),
            vec![(0, "Ground"), (1, "Water"), (2, "Air")]
        );
    }

    #[test]
    fn bitmask_layers_from_labels_allow_sparse_and_drop_out_of_range() {
        let layers = BitmaskLayers::from_labels("Sparse", [(0, "a"), (5, "b"), (40, "gone")]);
        assert_eq!(layers.label(0), Some("a"));
        assert_eq!(layers.label(5), Some("b"));
        assert_eq!(layers.label(40), None);
        assert_eq!(layers.mask(), (1 << 0) | (1 << 5));
        assert_eq!(layers.len(), 2);

        // A 33rd-plus name and an out-of-range `with_label` are both dropped, so
        // `mask()` never shifts past the word width.
        let capped = BitmaskLayers::from_names("Capped", (0..40).map(|i| i.to_string()))
            .with_label(32, "past-the-end");
        assert_eq!(capped.len(), u32::BITS as usize);
        assert_eq!(capped.mask(), u32::MAX);
        assert_eq!(capped.label(32), None);
    }

    #[test]
    fn bitmask_layers_default_is_empty() {
        let layers = BitmaskLayers::default();
        assert!(layers.is_empty());
        assert_eq!(layers.mask(), 0);
        assert_eq!(layers.name(), "");
    }

    #[test]
    fn bitmask_field_layers_numbers_builds_without_input() {
        let (changed, bits) = run_layers_frame(0xDEAD_BEEF, None);
        assert!(!changed);
        assert_eq!(bits, 0xDEAD_BEEF);
    }

    #[test]
    fn bitmask_field_layers_named_builds_without_input() {
        let layers = BitmaskLayers::from_names("Physics", ["Ground", "Water", "Air"]);
        let (changed, bits) = run_layers_frame(0b101, Some(&layers));
        assert!(!changed);
        assert_eq!(bits, 0b101);
    }

    #[test]
    fn bitmask_field_layers_handles_empty_layers() {
        let layers = BitmaskLayers::new("Empty");
        let (changed, bits) = run_layers_frame(0, Some(&layers));
        assert!(!changed);
        assert_eq!(bits, 0);
    }
}
