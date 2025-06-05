// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use inputbot::KeybdKey::*;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use tauri::AppHandle;
use tauri::Manager;
use wooting_analog_wrapper::read_analog;


#[derive(Clone, serde::Serialize)]
struct Payload {
    strafe_type: String,
    duration: u128,
}

fn eval_understrafe(elapsed: Duration, released_time: &mut Option<SystemTime>, app: AppHandle) {
    let time_passed = elapsed.as_micros();
    if time_passed < (200 * 1000) && time_passed > (1600) {
        // println!("Early release");
        // println!("{0}.{1}", time_passed / 1000, time_passed % 1000);
        app.emit_all(
            "strafe",
            Payload {
                strafe_type: "Early".into(),
                duration: time_passed,
            },
        )
        .unwrap();
    } else if time_passed < 1600 {
        // println!("Perfect");
        app.emit_all(
            "strafe",
            Payload {
                strafe_type: "Perfect".into(),
                duration: 0,
            },
        )
        .unwrap();
    }
    *released_time = None;
}

fn eval_overstrafe(elapsed: Duration, both_pressed_time: &mut Option<SystemTime>, app: AppHandle) {
    let time_passed = elapsed.as_micros();
    if time_passed < (200 * 1000) {
        // println!("Late release");
        // println!("{0}.{1}", time_passed / 1000, time_passed % 1000);
        app.emit_all(
            "strafe",
            Payload {
                strafe_type: "Late".into(),
                duration: time_passed,
            },
        )
        .unwrap();
    } else {
        // println!("Ignored overstrafe due to time too large")
    }
    *both_pressed_time = None;
}


pub fn main() {
    tauri::Builder::default()
        .setup(|app| {
            run_strafe_logic(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

pub fn run_strafe_logic(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut left_pressed = false;
        let mut right_pressed = false;
        let mut both_pressed_time: Option<SystemTime> = None;
        let mut right_released_time: Option<SystemTime> = None;
        let mut left_released_time: Option<SystemTime> = None;

        let layout = get_layout();

        loop {
            sleep(Duration::from_millis(1));

            handle_key_releases(
                &mut left_pressed,
                &mut right_pressed,
                &mut left_released_time,
                &mut right_released_time,
                &layout,
                &app,
            );

            handle_key_presses(
                &mut left_pressed,
                &mut right_pressed,
                &mut left_released_time,
                &mut right_released_time,
                &layout,
                &app,
            );

            handle_overlap_eval(
                &mut left_pressed,
                &mut right_pressed,
                &mut both_pressed_time,
                &app,
            );
        }
    });
}

enum KeyboardLayout {
    Qwerty,
    Azerty,
}

impl KeyboardLayout {
    fn left_key_pressed(&self) -> bool {
        match self {
            KeyboardLayout::Qwerty => AKey.is_pressed() || LeftKey.is_pressed(),
            KeyboardLayout::Azerty => QKey.is_pressed() || LeftKey.is_pressed(),
        }
    }

    fn left_key_released(&self) -> bool {
        match self {
            KeyboardLayout::Qwerty => !AKey.is_pressed() && !LeftKey.is_pressed(),
            KeyboardLayout::Azerty => !QKey.is_pressed() && !LeftKey.is_pressed(),
        }
    }

    fn right_key_pressed(&self) -> bool {
        DKey.is_pressed() || RightKey.is_pressed()
    }

    fn right_key_released(&self) -> bool {
        !DKey.is_pressed() && !RightKey.is_pressed()
    }
}

fn get_layout() -> KeyboardLayout {
    unsafe {
        let layout = winapi::um::winuser::GetKeyboardLayout(0);
        let layout_id = layout as u32 & 0xFFFF;
        match layout_id {
            0x040C | 0x080C | 0x140C | 0x180C => KeyboardLayout::Azerty,
            _ => KeyboardLayout::Qwerty,
        }
    }
}

fn handle_key_releases(
    left_pressed: &mut bool,
    right_pressed: &mut bool,
    left_released_time: &mut Option<SystemTime>,
    right_released_time: &mut Option<SystemTime>,
    layout: &KeyboardLayout,
    app: &AppHandle,
) {
    if *right_pressed && layout.right_key_released() {
        *right_pressed = false;
        let _ = app.emit_all("d-released", ());
        *right_released_time = Some(SystemTime::now());
    }
    if *left_pressed && layout.left_key_released() {
        *left_pressed = false;
        let _ = app.emit_all("a-released", ());
        *left_released_time = Some(SystemTime::now());
    }
}

fn handle_key_presses(
    left_pressed: &mut bool,
    right_pressed: &mut bool,
    left_released_time: &mut Option<SystemTime>,
    right_released_time: &mut Option<SystemTime>,
    layout: &KeyboardLayout,
    app: &AppHandle,
) {
    if layout.left_key_pressed() && !*left_pressed {
        *left_pressed = true;
        let _ = app.emit_all("a-pressed", ());
        if let Some(x) = right_released_time {
            if let Ok(elapsed) = x.elapsed() {
                eval_understrafe(elapsed, right_released_time, app.clone());
            }
        }
    }
    if layout.right_key_pressed() && !*right_pressed {
        *right_pressed = true;
        let _ = app.emit_all("d-pressed", ());
        if let Some(x) = left_released_time {
            if let Ok(elapsed) = x.elapsed() {
                eval_understrafe(elapsed, left_released_time, app.clone());
            }
        }
    }
}

fn handle_overlap_eval(
    left_pressed: &mut bool,
    right_pressed: &mut bool,
    both_pressed_time: &mut Option<SystemTime>,
    app: &AppHandle,
) {
    if *left_pressed && *right_pressed && both_pressed_time.is_none() {
        *both_pressed_time = Some(SystemTime::now());
    }
    if (!*left_pressed || !*right_pressed) && both_pressed_time.is_some() {
        if let Some(x) = both_pressed_time {
            if let Ok(elapsed) = x.elapsed() {
                eval_overstrafe(elapsed, both_pressed_time, app.clone());
            }
        }
    }
}
