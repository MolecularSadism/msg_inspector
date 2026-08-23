//! Generic card list with per-card numeric steppers.
//!
//! A [`Card`] is a backend-agnostic description of one adjustable thing (a
//! prefab, a resource, an item). [`draw_cards`] renders a scrollable, grouped
//! list of cards — each showing an icon, a name, the current amount `N`, and
//! the `-5 / -1 / N / +1 / +5` adjustment controls — and reports which buttons
//! were clicked so the caller can apply the matching change.
//!
//! Cards are drawn in slice order and grouped by consecutive equal
//! [`Card::category_label`] values, so callers sort the slice however they
//! want the groups to appear.

use bevy_egui::egui;

/// One entry rendered as a card.
///
/// # Example
///
/// ```
/// use msg_inspector::widgets::Card;
///
/// let card = Card {
///     key: "torch".to_string(),
///     name: "Torch".to_string(),
///     ..Default::default()
/// };
/// assert_eq!(card.count, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Card {
    /// Stable string identifier, shown under the name. The caller uses this to
    /// map reported actions back to its own concrete ids.
    pub key: String,
    /// Human-readable display name.
    pub name: String,
    /// Tier used for the icon tint and shown next to the key.
    pub tier: u8,
    /// Heading shown above each run of cards sharing a label.
    pub category_label: String,
    /// Current amount `N` shown between the stepper buttons.
    pub count: u32,
    /// Whether the entry is flagged hidden (shown dimmed, never filtered out —
    /// a dev panel surfaces everything).
    pub hidden: bool,
}

/// A button press reported back from [`draw_cards`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardAction {
    /// Decrease the amount by five, clamping at zero.
    ///
    /// Reported at any non-zero amount, including amounts below five, so
    /// callers must apply it with [`u32::saturating_sub`] (or an equivalent
    /// clamp to zero), never a bare subtraction.
    Sub5,
    /// Decrease the amount by one, clamping at zero.
    ///
    /// Only reported at non-zero amounts, but applying it with
    /// [`u32::saturating_sub`] keeps callers robust if their own state has
    /// drifted from the drawn `count`.
    Sub1,
    /// Increase the amount by one.
    Add1,
    /// Increase the amount by five.
    Add5,
    /// Arm cursor placement: create one at the next world click.
    Place,
}

/// Render a scrollable, grouped list of cards. Returns one `(index, action)`
/// pair per button clicked this frame, where `index` refers into `cards`.
///
/// `show_place` adds a placement button to each card for hosts that support
/// click-to-place.
///
/// The `-1` and `-5` buttons are enabled at any non-zero amount, so
/// [`CardAction::Sub5`] can be reported at amounts below five; apply
/// decrements with [`u32::saturating_sub`] so they clamp to zero, as the
/// example does.
///
/// The scroll area uses a fixed id; use [`draw_cards_with_salt`] to draw more
/// than one card list in the same panel.
///
/// # Example
///
/// ```
/// use msg_inspector::widgets::{Card, CardAction, draw_cards};
///
/// fn tab_ui(ui: &mut msg_inspector::egui::Ui, cards: &[Card], counts: &mut [u32]) {
///     for (index, action) in draw_cards(ui, cards, false) {
///         counts[index] = match action {
///             CardAction::Sub5 => counts[index].saturating_sub(5),
///             CardAction::Sub1 => counts[index].saturating_sub(1),
///             CardAction::Add1 => counts[index] + 1,
///             CardAction::Add5 => counts[index] + 5,
///             CardAction::Place => counts[index],
///         };
///     }
/// }
/// ```
#[must_use = "the returned actions must be applied by the caller for the buttons to do anything"]
pub fn draw_cards(ui: &mut egui::Ui, cards: &[Card], show_place: bool) -> Vec<(usize, CardAction)> {
    draw_cards_with_salt(ui, cards, show_place, "msg_inspector_cards")
}

/// Like [`draw_cards`], with a caller-chosen scroll-area id salt so several
/// card lists can share one panel without their scroll offsets clashing.
///
/// # Example
///
/// ```
/// use msg_inspector::widgets::{Card, draw_cards_with_salt};
///
/// fn tab_ui(ui: &mut msg_inspector::egui::Ui, primary: &[Card], secondary: &[Card]) {
///     let primary_actions = draw_cards_with_salt(ui, primary, false, "primary");
///     let secondary_actions = draw_cards_with_salt(ui, secondary, false, "secondary");
///     let _ = (primary_actions, secondary_actions);
/// }
/// ```
#[must_use = "the returned actions must be applied by the caller for the buttons to do anything"]
pub fn draw_cards_with_salt(
    ui: &mut egui::Ui,
    cards: &[Card],
    show_place: bool,
    id_salt: impl std::hash::Hash,
) -> Vec<(usize, CardAction)> {
    let mut actions = Vec::new();

    if cards.is_empty() {
        ui.weak("No entries to show.");
        return actions;
    }

    egui::ScrollArea::vertical()
        .id_salt(id_salt)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut current_group: Option<&str> = None;
            for (idx, card) in cards.iter().enumerate() {
                if current_group != Some(card.category_label.as_str()) {
                    if current_group.is_some() {
                        ui.add_space(8.0);
                    }
                    ui.label(
                        egui::RichText::new(&card.category_label)
                            .strong()
                            .underline(),
                    );
                    current_group = Some(card.category_label.as_str());
                }
                if let Some(action) = draw_one_card(ui, card, show_place) {
                    actions.push((idx, action));
                }
            }
        });

    actions
}

/// Render a single card and report the button clicked, if any.
fn draw_one_card(ui: &mut egui::Ui, card: &Card, show_place: bool) -> Option<CardAction> {
    let mut action = None;

    ui.group(|ui| {
        ui.horizontal(|ui| {
            draw_icon(ui, card.tier, &card.name);
            ui.add_space(6.0);

            ui.vertical(|ui| {
                let mut title = egui::RichText::new(&card.name).strong();
                if card.hidden {
                    title = title.italics().color(ui.visuals().weak_text_color());
                }
                ui.label(title);
                ui.weak(format!("{}  ·  tier {}", card.key, card.tier));
            });

            // Lay the controls out from the right edge inward. Widgets added
            // first land furthest right, so the additions below read as the
            // reverse of the on-screen order `-5 -1 N +1 +5 [place]`.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if show_place {
                    if ui
                        .add(egui::Button::new("place").small())
                        .on_hover_text("Click, then click in the world to place one at the cursor")
                        .clicked()
                    {
                        action = Some(CardAction::Place);
                    }
                    ui.add_space(6.0);
                }

                if ui.add(egui::Button::new("+5").small()).clicked() {
                    action = Some(CardAction::Add5);
                }
                if ui.add(egui::Button::new("+1").small()).clicked() {
                    action = Some(CardAction::Add1);
                }
                ui.label(
                    egui::RichText::new(format!("{:>3}", card.count))
                        .monospace()
                        .strong(),
                );
                let can_sub = card.count > 0;
                if ui
                    .add_enabled(can_sub, egui::Button::new("-1").small())
                    .clicked()
                {
                    action = Some(CardAction::Sub1);
                }
                if ui
                    .add_enabled(can_sub, egui::Button::new("-5").small())
                    .clicked()
                {
                    action = Some(CardAction::Sub5);
                }
            });
        });
    });

    action
}

/// Draw the card icon: a tier-tinted disc carrying the entry's initials.
fn draw_icon(ui: &mut egui::Ui, tier: u8, name: &str) {
    let size = egui::vec2(34.0, 34.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    painter.circle_filled(rect.center(), 16.0, tier_color(tier));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        initials_of(name),
        egui::FontId::proportional(13.0),
        egui::Color32::from_gray(20),
    );
}

/// Up to two uppercase initials taken from the name's words, falling back to the
/// first characters when the name is a single word.
fn initials_of(name: &str) -> String {
    let from_words: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect();
    let initials = if from_words.is_empty() {
        name.chars().take(2).collect::<String>()
    } else {
        from_words
    };
    initials.to_uppercase()
}

/// Colour ramp keyed by tier, from muted grey upward.
fn tier_color(tier: u8) -> egui::Color32 {
    match tier {
        0 => egui::Color32::from_rgb(130, 130, 130),
        1 => egui::Color32::from_rgb(120, 200, 120),
        2 => egui::Color32::from_rgb(110, 170, 230),
        3 => egui::Color32::from_rgb(180, 140, 230),
        4 => egui::Color32::from_rgb(230, 170, 90),
        _ => egui::Color32::from_rgb(230, 110, 110),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str, category: &str, count: u32) -> Card {
        Card {
            key: name.to_lowercase(),
            name: name.to_string(),
            tier: 1,
            category_label: category.to_string(),
            count,
            hidden: false,
        }
    }

    /// Runs one egui frame and returns what `draw_cards` reported.
    fn run_frame(cards: &[Card], show_place: bool) -> Vec<(usize, CardAction)> {
        let ctx = egui::Context::default();
        let mut actions = Vec::new();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                actions = draw_cards(ui, cards, show_place);
            });
        });
        actions
    }

    #[test]
    fn builds_without_input_and_reports_no_actions() {
        let cards = [
            card("Alpha Beast", "Creatures", 2),
            card("Beta Beast", "Creatures", 0),
            card("Torch", "Items", 5),
        ];
        assert!(run_frame(&cards, false).is_empty());
        assert!(run_frame(&cards, true).is_empty());
    }

    #[test]
    fn empty_list_renders_placeholder() {
        assert!(run_frame(&[], false).is_empty());
    }

    #[test]
    fn salted_lists_share_a_panel() {
        let ctx = egui::Context::default();
        let cards = [card("Alpha Beast", "Creatures", 2)];
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(draw_cards_with_salt(ui, &cards, false, "primary").is_empty());
                assert!(draw_cards_with_salt(ui, &cards, false, "secondary").is_empty());
            });
        });
    }

    #[test]
    fn initials_use_word_starts_then_fall_back_to_prefix() {
        assert_eq!(initials_of("Alpha Beast"), "AB");
        assert_eq!(initials_of("torch"), "T");
        assert_eq!(initials_of("Grand Old Duke"), "GO");
        assert_eq!(initials_of(""), "");
    }

    #[test]
    fn tier_colors_are_distinct_up_the_ramp() {
        let colors: Vec<_> = (0..=5).map(tier_color).collect();
        for (i, a) in colors.iter().enumerate() {
            for b in &colors[i + 1..] {
                assert_ne!(a, b);
            }
        }
        // The ramp saturates: everything past tier 5 shares tier 5's colour.
        assert_eq!(tier_color(6), tier_color(200));
    }
}
