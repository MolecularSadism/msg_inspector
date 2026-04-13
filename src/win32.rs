//! Win32 platform code for game window Z-order management.
//!
//! Sets the game window as "owned" by the inspector window using
//! `SetWindowLongPtrW(GWLP_HWNDPARENT)`. This makes the game window
//! always stay in front of the inspector window without being globally
//! "always on top".

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapper};
use raw_window_handle::RawWindowHandle;

use crate::panel::GameWindow;
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
/// by winit and have their `RawHandleWrapper` components. Uses Win32
/// `SetWindowLongPtrW(GWLP_HWNDPARENT)` to establish the owner relationship.
pub fn setup_owner_window(
    game_window_entity: Res<GameWindowEntity>,
    inspector_handle: Query<&RawHandleWrapper, With<PrimaryWindow>>,
    game_handle: Query<&RawHandleWrapper, With<GameWindow>>,
    mut established: ResMut<OwnerWindowEstablished>,
) {
    if established.0 {
        return;
    }

    // Both windows need their RawHandleWrapper before we can proceed.
    // These are inserted by bevy_winit after the OS windows are created.
    let (Ok(inspector_raw), Ok(game_raw)) = (
        inspector_handle.single(),
        game_handle.single(),
    ) else {
        return;
    };

    let inspector_hwnd = inspector_raw.get_window_handle();
    let game_hwnd = game_raw.get_window_handle();

    // Extract Win32 HWNDs and set owner relationship
    if let (RawWindowHandle::Win32(inspector_h), RawWindowHandle::Win32(game_h)) =
        (inspector_hwnd, game_hwnd)
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
}
