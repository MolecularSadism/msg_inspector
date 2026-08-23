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

use bevy_egui::egui;

/// Comfortable minimum width for a full 32-bit decimal value (up to 10 digits).
const MIN_WIDTH: f32 = 160.0;

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
    let mut changed = false;

    let field_id = ui.make_persistent_id("bitmask_raw_value");
    let mut text = ui
        .data_mut(|data| data.get_temp::<String>(field_id))
        .unwrap_or_else(|| bits.to_string());
    let response = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id(field_id)
            .desired_width(ui.available_width().max(MIN_WIDTH)),
    );
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

    let named = named_bits(&label);
    let named_mask = named_mask(&named);

    ui.horizontal(|ui| {
        if ui.button("All").clicked() && *bits | named_mask != *bits {
            *bits |= named_mask;
            changed = true;
        }
        if ui.button("None").clicked() && *bits & !named_mask != *bits {
            *bits &= !named_mask;
            changed = true;
        }
    });

    for (bit, name) in named {
        let mut set = *bits & (1 << bit) != 0;
        if ui.checkbox(&mut set, name).changed() {
            if set {
                *bits |= 1 << bit;
            } else {
                *bits &= !(1 << bit);
            }
            changed = true;
        }
    }

    changed
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
fn named_bits<T>(label: &impl Fn(u32) -> Option<T>) -> Vec<(u32, T)> {
    (0..u32::BITS)
        .filter_map(|bit| label(bit).map(|name| (bit, name)))
        .collect()
}

/// The union of the collected named bits.
fn named_mask<T>(named: &[(u32, T)]) -> u32 {
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
}
