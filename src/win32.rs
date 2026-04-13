//! Win32 platform code for game window Z-order management.
//!
//! Sets the game window as "owned" by the inspector window using
//! `SetWindowLongPtrW(GWLP_HWNDPARENT)`. This makes the game window
//! always stay in front of the inspector window without being globally
//! "always on top".

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::GameWindowEntity;

const GWLP_HWNDPARENT: i32 = -8;

unsafe extern "system" {
    fn SetWindowLongPtrW(hwnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
}

/// Resource that tracks whether the owner relationship has been established.
#[derive(Resource, Default)]
pub struct OwnerWindowEstablished(pub bool);

/// System that sets the game window as "owned" by the inspector (primary) window.
///
/// Runs every frame but only acts once, after both windows have been created
/// by winit. Uses Win32 `SetWindowLongPtrW(GWLP_HWNDPARENT)` to establish
/// the owner relationship.
#[allow(clippy::needless_pass_by_value)]
pub fn setup_owner_window(
    game_window_entity: Res<GameWindowEntity>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
    mut established: ResMut<OwnerWindowEstablished>,
) {
    if established.0 {
        return;
    }

    let inspector_entity = *primary_window;
    let game_entity = game_window_entity.0;

    WINIT_WINDOWS.with_borrow(|winit_windows| {
        let Some(inspector_winit_id) = winit_windows.entity_to_winit.get(&inspector_entity) else {
            return;
        };
        let Some(game_winit_id) = winit_windows.entity_to_winit.get(&game_entity) else {
            return;
        };

        let Some(inspector_winit) = winit_windows.windows.get(inspector_winit_id) else {
            return;
        };
        let Some(game_winit) = winit_windows.windows.get(game_winit_id) else {
            return;
        };

        let Ok(inspector_handle) = inspector_winit.window_handle() else {
            return;
        };
        let Ok(game_handle) = game_winit.window_handle() else {
            return;
        };

        if let (RawWindowHandle::Win32(inspector_h), RawWindowHandle::Win32(game_h)) =
            (inspector_handle.as_raw(), game_handle.as_raw())
        {
            // SAFETY: Both HWNDs are valid handles from winit-managed windows.
            // Setting GWLP_HWNDPARENT establishes an owner relationship:
            // the game window will always appear in front of the inspector window.
            unsafe {
                SetWindowLongPtrW(
                    game_h.hwnd.get(),
                    GWLP_HWNDPARENT,
                    inspector_h.hwnd.get(),
                );
            }
            established.0 = true;
        }
    });
}
