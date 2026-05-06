use std::sync::{Arc, RwLock};

use crate::macro_engine::MacroEngine;
use crate::types::{AppConfig, MacroStep};

pub struct InputRuntime {
    pub config: Arc<RwLock<AppConfig>>,
    pub engine: MacroEngine,
}

pub fn resolve_macro(cfg: &AppConfig, macro_id: &Option<String>) -> Option<Vec<MacroStep>> {
    let id = macro_id.as_ref()?;
    cfg.macros.iter().find(|m| &m.id == id).map(|m| m.steps.clone())
}

pub fn spawn(runtime: Arc<InputRuntime>) {
    #[cfg(windows)]
    win::spawn(runtime);
    #[cfg(not(windows))]
    {
        let _ = runtime;
        log::warn!("Global hotkeys are only active on Windows builds.");
    }
}

#[cfg(windows)]
mod win {
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex, OnceLock};

    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL,
        WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
        WM_XBUTTONDOWN, WM_XBUTTONUP,
    };

    use crate::hotkey_util;
    use crate::input_listener::InputRuntime;
    use crate::types::{AppConfig, ExecutionMode, RunMode};

    static RT: OnceLock<Arc<InputRuntime>> = OnceLock::new();
    static PRESSED: OnceLock<Mutex<HashSet<(u32, u32)>>> = OnceLock::new();

    fn pressed() -> &'static Mutex<HashSet<(u32, u32)>> {
        PRESSED.get_or_init(|| Mutex::new(HashSet::new()))
    }

    pub fn spawn(runtime: Arc<InputRuntime>) {
        let _ = RT.set(runtime);
        std::thread::spawn(|| {
            if let Err(e) = run_loop() {
                log::error!("input loop exited: {e}");
            }
        });
    }

    fn resolve_steps(cfg: &AppConfig, macro_id: &Option<String>) -> Option<Vec<crate::types::MacroStep>> {
        super::resolve_macro(cfg, macro_id)
    }

    fn dispatch(rt: &InputRuntime, vk: u32, down: bool, up: bool) {
        let mods = hotkey_util::async_modifiers();
        let key = (vk, mods);

        if down {
            let mut g = match pressed().lock() {
                Ok(x) => x,
                Err(_) => return,
            };
            if g.contains(&key) {
                return;
            }
            g.insert(key);
        } else if up {
            if let Ok(mut g) = pressed().lock() {
                g.remove(&key);
            }
        }

        let cfg = match rt.config.read() {
            Ok(c) => c,
            Err(_) => return,
        };
        if !cfg.master_enabled {
            return;
        }
        let game = match cfg.game_profiles.iter().find(|p| p.id == cfg.active_game) {
            Some(g) => g,
            None => return,
        };

        let mods = hotkey_util::async_modifiers();

        for b in &game.bindings {
            if !b.enabled {
                continue;
            }
            let Some(hk) = &b.hotkey else { continue };
            if hk.vk != vk || hk.modifiers != mods {
                continue;
            }
            let Some(steps) = resolve_steps(&cfg, &b.macro_id) else {
                continue;
            };
            let jitter = cfg.jitter_ms;
            match b.mode {
                ExecutionMode::Tap => {
                    if down {
                        rt.engine.spawn_run(steps, RunMode::Once, jitter);
                    }
                }
                ExecutionMode::Toggle => {
                    if down {
                        if rt.engine.is_running() {
                            rt.engine.interrupt();
                        } else {
                            rt.engine.spawn_run(steps, RunMode::LoopUntilCancel, jitter);
                        }
                    }
                }
                ExecutionMode::Hold => {
                    if down {
                        rt.engine.spawn_run(steps, RunMode::LoopUntilCancel, jitter);
                    } else if up {
                        rt.engine.interrupt();
                    }
                }
            }
            break;
        }
    }

    fn run_loop() -> Result<(), String> {
        unsafe {
            let hmod = GetModuleHandleW(None).map_err(|e| format!("GetModuleHandle: {e}"))?;
            let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), hmod, 0)
                .map_err(|e| format!("keyboard hook: {e}"))?;
            let ms = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), hmod, 0)
                .map_err(|e| format!("mouse hook: {e}"))?;

            let mut msg = MSG::default();
            loop {
                let r = GetMessageW(&mut msg, None, 0, 0);
                if r.0 == 0 {
                    break;
                }
                if r.0 < 0 {
                    let _ = UnhookWindowsHookEx(kb);
                    let _ = UnhookWindowsHookEx(ms);
                    return Err("GetMessage failed".into());
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnhookWindowsHookEx(kb);
            let _ = UnhookWindowsHookEx(ms);
        }
        Ok(())
    }

    unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            if let Some(rt) = RT.get() {
                let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                let wp = wparam.0 as u32;
                let down = wp == WM_KEYDOWN as u32 || wp == WM_SYSKEYDOWN as u32;
                let up = wp == WM_KEYUP as u32 || wp == WM_SYSKEYUP as u32;
                if down || up {
                    dispatch(rt.as_ref(), info.vkCode as u32, down, up);
                }
            }
        }
        unsafe { CallNextHookEx(HHOOK(std::ptr::null_mut()), code, wparam, lparam) }
    }

    unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            if let Some(rt) = RT.get() {
                let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
                let wp = wparam.0 as u32;
                match wp {
                    WM_LBUTTONDOWN => dispatch(rt.as_ref(), 0x01, true, false),
                    WM_LBUTTONUP => dispatch(rt.as_ref(), 0x01, false, true),
                    WM_RBUTTONDOWN => dispatch(rt.as_ref(), 0x02, true, false),
                    WM_RBUTTONUP => dispatch(rt.as_ref(), 0x02, false, true),
                    WM_MBUTTONDOWN => dispatch(rt.as_ref(), 0x04, true, false),
                    WM_MBUTTONUP => dispatch(rt.as_ref(), 0x04, false, true),
                    WM_XBUTTONDOWN | WM_XBUTTONUP => {
                        let btn = (info.mouseData >> 16) & 0xFFFF;
                        let vk = match btn as u16 {
                            1 => 0x05u32,
                            2 => 0x06u32,
                            _ => 0,
                        };
                        if vk != 0 {
                            let down = wp == WM_XBUTTONDOWN;
                            let up = wp == WM_XBUTTONUP;
                            dispatch(rt.as_ref(), vk, down, up);
                        }
                    }
                    _ => {}
                }
            }
        }
        unsafe { CallNextHookEx(HHOOK(std::ptr::null_mut()), code, wparam, lparam) }
    }
}
