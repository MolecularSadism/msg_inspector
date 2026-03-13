//! Animation finite-state machine utilities.
//!
//! Provides [`AnimationFSM`], a Bevy component that tracks the current
//! animation state and **silently ignores** transitions to the same state.
//! This prevents animations from restarting when the game logic repeatedly
//! confirms the current state every frame (e.g. attack loops).
//!
//! # Example
//!
//! ```
//! use bevy::prelude::*;
//! use msg_inspector::animation::AnimationFSM;
//!
//! #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
//! enum CharacterAnim {
//!     #[default]
//!     Idle,
//!     Run,
//!     Attack,
//! }
//!
//! fn attack_system(mut q: Query<&mut AnimationFSM<CharacterAnim>>) {
//!     for mut fsm in &mut q {
//!         // Only triggers the transition (and thus the animation restart)
//!         // the first time. Subsequent calls while already in `Attack`
//!         // are silently ignored.
//!         fsm.transition_to(CharacterAnim::Attack);
//!     }
//! }
//! ```

use std::fmt::Debug;
use std::hash::Hash;

use bevy::prelude::*;

/// A minimal animation state machine that rejects same-state transitions.
///
/// `S` is the state enum (e.g. `Idle`, `Run`, `Attack`).
///
/// The component tracks the **current** state and a **changed** flag that is
/// set to `true` only when a *real* transition occurs (i.e. old ≠ new).
/// Game systems can read [`changed`](Self::changed) to know whether the
/// underlying animation player needs to be re-started.
///
/// Calling [`transition_to`](Self::transition_to) with the current state is a
/// no-op — the flag stays `false` and no animation restart happens.
#[derive(Component, Debug, Clone)]
pub struct AnimationFSM<S>
where
    S: Clone + PartialEq + Eq + Hash + Debug + Send + Sync + 'static,
{
    current: S,
    /// `true` during the frame in which a real transition happened.
    changed: bool,
}

impl<S> AnimationFSM<S>
where
    S: Clone + PartialEq + Eq + Hash + Debug + Send + Sync + 'static,
{
    /// Create a new FSM starting in the given state.
    ///
    /// The initial `changed` flag is `true` so that the first frame can
    /// set up the animation.
    #[must_use]
    pub fn new(initial: S) -> Self {
        Self {
            current: initial,
            changed: true,
        }
    }

    /// Attempt to transition to `target`.
    ///
    /// If `target` equals the current state the call is **silently ignored**
    /// — `changed` stays `false` and no animation restart should occur.
    ///
    /// Returns `true` when a real transition happened.
    pub fn transition_to(&mut self, target: S) -> bool {
        if self.current == target {
            return false;
        }
        self.current = target;
        self.changed = true;
        true
    }

    /// The current animation state.
    #[must_use]
    pub fn current(&self) -> &S {
        &self.current
    }

    /// Whether the state changed since the last [`clear_changed`](Self::clear_changed) call.
    #[must_use]
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Reset the changed flag.
    ///
    /// Typically called at the end of the animation-apply system so that
    /// `changed` is only `true` for one frame per transition.
    pub fn clear_changed(&mut self) {
        self.changed = false;
    }
}

impl<S> Default for AnimationFSM<S>
where
    S: Clone + PartialEq + Eq + Hash + Debug + Send + Sync + Default + 'static,
{
    fn default() -> Self {
        Self::new(S::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
    enum Anim {
        #[default]
        Idle,
        Attack,
        Hurt,
    }

    #[test]
    fn initial_state_is_marked_changed() {
        let fsm = AnimationFSM::new(Anim::Idle);
        assert_eq!(*fsm.current(), Anim::Idle);
        assert!(fsm.changed(), "initial state should be marked as changed");
    }

    #[test]
    fn same_state_transition_is_rejected() {
        let mut fsm = AnimationFSM::new(Anim::Idle);
        fsm.clear_changed();

        let transitioned = fsm.transition_to(Anim::Idle);
        assert!(!transitioned, "same-state transition must return false");
        assert!(
            !fsm.changed(),
            "changed flag must stay false on same-state transition"
        );
        assert_eq!(*fsm.current(), Anim::Idle);
    }

    #[test]
    fn different_state_transition_is_accepted() {
        let mut fsm = AnimationFSM::new(Anim::Idle);
        fsm.clear_changed();

        let transitioned = fsm.transition_to(Anim::Attack);
        assert!(transitioned, "different-state transition must return true");
        assert!(fsm.changed(), "changed flag must be true after transition");
        assert_eq!(*fsm.current(), Anim::Attack);
    }

    #[test]
    fn clear_changed_resets_flag() {
        let mut fsm = AnimationFSM::new(Anim::Idle);
        assert!(fsm.changed());
        fsm.clear_changed();
        assert!(!fsm.changed());
    }

    #[test]
    fn repeated_same_state_transitions_never_set_changed() {
        let mut fsm = AnimationFSM::new(Anim::Attack);
        fsm.clear_changed();

        for _ in 0..100 {
            let transitioned = fsm.transition_to(Anim::Attack);
            assert!(!transitioned);
            assert!(!fsm.changed());
        }
    }

    #[test]
    fn sequential_different_transitions_work() {
        let mut fsm = AnimationFSM::new(Anim::Idle);
        fsm.clear_changed();

        assert!(fsm.transition_to(Anim::Attack));
        assert!(fsm.changed());
        assert_eq!(*fsm.current(), Anim::Attack);

        fsm.clear_changed();

        assert!(fsm.transition_to(Anim::Hurt));
        assert!(fsm.changed());
        assert_eq!(*fsm.current(), Anim::Hurt);

        fsm.clear_changed();

        // Back to the same state as one step ago — should still succeed
        assert!(fsm.transition_to(Anim::Attack));
        assert!(fsm.changed());
    }

    #[test]
    fn default_impl_uses_default_state() {
        let fsm = AnimationFSM::<Anim>::default();
        assert_eq!(*fsm.current(), Anim::Idle);
        assert!(fsm.changed());
    }
}
