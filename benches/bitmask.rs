//! Benches for the bitmask widget's per-frame hot paths: the named-bit
//! collection on its own, and a full [`bitmask_field`] draw through a headless
//! [`egui::Context`] frame.
//!
//! Inputs are fixed (all 32 bits named, a constant mask) so runs are
//! comparable across changes.

use criterion::{Criterion, criterion_group, criterion_main};
use msg_inspector::egui;
use msg_inspector::widgets::bitmask::{named_bits, named_mask};
use msg_inspector::widgets::bitmask_field;
use std::hint::black_box;

/// One static label per bit, so every checkbox row is exercised.
const NAMES: [&str; 32] = [
    "Ground", "Water", "Air", "Fire", "Ice", "Rock", "Metal", "Wood", "Light", "Dark", "Poison",
    "Sound", "Wind", "Sand", "Ash", "Oil", "Steam", "Mud", "Snow", "Crystal", "Void", "Storm",
    "Lava", "Spore", "Rust", "Glass", "Bone", "Thorn", "Frost", "Ember", "Tide", "Star",
];

/// The per-frame label collection: one closure pass over all 32 bits plus the
/// mask union derived from it.
fn named_bit_computation(c: &mut Criterion) {
    c.bench_function("named_bits_32_named", |b| {
        b.iter(|| {
            let names = black_box(&NAMES);
            let named = named_bits(&|bit| names.get(bit as usize).copied());
            black_box(named_mask(&named))
        })
    });
}

/// A full widget draw — text field, All/None row, 32 checkboxes — inside one
/// headless egui frame.
fn full_bitmask_field_draw(c: &mut Criterion) {
    let ctx = egui::Context::default();
    let mut bits = 0xAAAA_5555_u32;
    c.bench_function("bitmask_field_full_draw", |b| {
        b.iter(|| {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    black_box(bitmask_field(ui, black_box(&mut bits), &NAMES));
                });
            });
        })
    });
}

criterion_group!(benches, named_bit_computation, full_bitmask_field_draw);
criterion_main!(benches);
