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
    bitmask_field_with(ui, bits, |bit| {
        names.get(bit as usize).map(|name| (*name).to_string())
    })
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
///   order.
///
/// Returns `true` when the mask changed this frame.
pub fn bitmask_field_with(
    ui: &mut egui::Ui,
    bits: &mut u32,
    label: impl Fn(u32) -> Option<String>,
) -> bool {
    let mut changed = false;

    let mut text = bits.to_string();
    let response = ui.add(
        egui::TextEdit::singleline(&mut text).desired_width(ui.available_width().max(MIN_WIDTH)),
    );
    if response.changed()
        && let Ok(parsed) = text.parse::<u32>()
        && parsed != *bits
    {
        *bits = parsed;
        changed = true;
    }

    let named_mask = named_bits(&label);

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

    for bit in 0..u32::BITS {
        let Some(name) = label(bit) else {
            continue;
        };
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

/// The union of all bits the label source names.
fn named_bits(label: &impl Fn(u32) -> Option<String>) -> u32 {
    (0..u32::BITS)
        .filter(|&bit| label(bit).is_some())
        .fold(0, |mask, bit| mask | (1 << bit))
}

#[cfg(test)]
mod tests {
    use super::*;

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
                    0 => Some("First".to_string()),
                    31 => Some("Last".to_string()),
                    _ => None,
                });
            });
        });
        assert!(!changed);
        assert_eq!(bits, 1 << 31);
    }

    #[test]
    fn named_bits_unions_labelled_positions() {
        let mask = named_bits(&|bit| match bit {
            0 | 2 | 31 => Some(String::new()),
            _ => None,
        });
        assert_eq!(mask, 0b101 | (1 << 31));

        let names = ["a", "b", "c"];
        let mask = named_bits(&|bit| names.get(bit as usize).map(|n| (*n).to_string()));
        assert_eq!(mask, 0b111);

        assert_eq!(named_bits(&|_| None), 0);
    }
}
