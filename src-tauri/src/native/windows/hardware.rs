//! Opt-in tests create their own windows and quiet audio processes. No user app is rerouted.
use super::*;
use crate::engine::decision::{decide, select_monitor, Decision};
use std::{
    io::{BufRead, Write},
    process::{Child, Command, Stdio},
    time::Duration,
};

#[test]
#[ignore = "Helper process for the opt-in two-output hardware test"]
fn audio_child() {
    use windows::{
        core::w,
        Media::{Core::MediaSource, Playback::MediaPlayer},
        Storage::StorageFile,
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let ui = std::thread::spawn(move || unsafe {
        let hwnd = CreateWindowExW(
            WS_EX_APPWINDOW,
            w!("STATIC"),
            w!("AudioFlow isolated audio test"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            100,
            100,
            600,
            400,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        tx.send((hwnd.0 as isize, GetCurrentThreadId())).unwrap();
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = DestroyWindow(hwnd);
    });
    let (handle, thread_id) = rx.recv().unwrap();
    let samples = 48000u32;
    let data_size = samples * 2;
    let mut wave = Vec::new();
    wave.extend_from_slice(b"RIFF");
    wave.extend_from_slice(&(36 + data_size).to_le_bytes());
    wave.extend_from_slice(b"WAVEfmt ");
    wave.extend_from_slice(&16u32.to_le_bytes());
    wave.extend_from_slice(&1u16.to_le_bytes());
    wave.extend_from_slice(&1u16.to_le_bytes());
    wave.extend_from_slice(&48000u32.to_le_bytes());
    wave.extend_from_slice(&96000u32.to_le_bytes());
    wave.extend_from_slice(&2u16.to_le_bytes());
    wave.extend_from_slice(&16u16.to_le_bytes());
    wave.extend_from_slice(b"data");
    wave.extend_from_slice(&data_size.to_le_bytes());
    for i in 0..samples {
        let t = i as f32 / 48000.0;
        let envelope = (t / 0.02).min(1.0) * ((1.0 - t) / 0.02).min(1.0);
        let sample = (t * 330.0 * std::f32::consts::TAU).sin() * 0.006 * envelope;
        wave.extend_from_slice(&((sample * i16::MAX as f32) as i16).to_le_bytes());
    }
    let _apartment = ComApartment::new().unwrap();
    let wav = std::env::current_exe().unwrap().with_extension("wav");
    std::fs::write(&wav, &wave).unwrap();
    let file = StorageFile::GetFileFromPathAsync(&windows::core::HSTRING::from(
        wav.to_string_lossy().as_ref(),
    ))
    .unwrap()
    .join()
    .unwrap();
    let source = MediaSource::CreateFromStorageFile(&file).unwrap();
    let play = || {
        let player = MediaPlayer::new().unwrap();
        player.SetSource(&source).unwrap();
        player.SetIsLoopingEnabled(true).unwrap();
        player.Play().unwrap();
        player
    };
    let mut player = play();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line == "stop" {
            break;
        }
        if line == "restart" {
            player.Close().unwrap();
            player = play();
        }
        if let Some(coords) = line.strip_prefix("move ") {
            let v: Vec<i32> = coords
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if v.len() == 2 {
                unsafe {
                    let _ = SetWindowPos(
                        HWND(handle as *mut _),
                        None,
                        v[0],
                        v[1],
                        600,
                        400,
                        SWP_NOZORDER,
                    );
                }
            }
        }
    }
    player.Close().unwrap();
    unsafe {
        let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    let _ = ui.join();
}
struct TestProcess(Child);
impl TestProcess {
    fn command(&mut self, line: &str) {
        writeln!(self.0.stdin.as_mut().unwrap(), "{line}").unwrap();
    }
}
impl Drop for TestProcess {
    fn drop(&mut self) {
        let _ = writeln!(self.0.stdin.as_mut().unwrap(), "stop");
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn wait_for(platform: &WindowsPlatform, pid: u32, target: &str) -> bool {
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(100));
        let devices = platform.list_output_devices().unwrap();
        let sessions = platform.list_audio_sessions(&devices).unwrap();
        if sessions
            .iter()
            .any(|s| s.pid == pid && s.active && s.endpoint_id == target)
            && !sessions
                .iter()
                .any(|s| s.pid == pid && s.active && s.endpoint_id != target)
        {
            return true;
        }
    }
    false
}
fn route_for_window(
    platform: &WindowsPlatform,
    config: &Config,
    pid: u32,
    devices: &[Device],
    monitors: &[Monitor],
) -> String {
    let windows = platform.windows().unwrap();
    let window = windows
        .iter()
        .find(|w| w.pid == pid)
        .expect("Test window not found");
    let monitor = select_monitor(window.bounds, monitors, None)
        .expect("Test window is not on a connected display");
    let Decision::Route(Some(target)) = decide(
        config,
        "isolated-test.exe",
        Some(&monitor.id),
        devices,
        false,
    ) else {
        panic!("No route for test window");
    };
    platform.set_process_output(pid, Some(&target)).unwrap();
    target
}
#[test]
#[ignore = "Requires two real displays and two audio outputs; plays quiet test tones"]
fn two_output_routing() {
    let _apartment = ComApartment::new().unwrap();
    let platform = WindowsPlatform::new().unwrap();
    let monitors = platform.monitors().unwrap();
    let devices = platform.list_output_devices().unwrap();
    assert!(
        monitors.len() >= 2 && devices.len() >= 2,
        "Connect two monitors and two outputs before running this test"
    );
    let root = std::env::temp_dir().join(format!("audioflow-hardware-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let paths = [
        root.join("audioflow-test-a.exe"),
        root.join("audioflow-test-b.exe"),
    ];
    for path in &paths {
        std::fs::copy(std::env::current_exe().unwrap(), path).unwrap();
    }
    let spawn = |path: &std::path::Path| {
        TestProcess(
            Command::new(path)
                .args([
                    "native::windows::hardware::audio_child",
                    "--exact",
                    "--ignored",
                    "--nocapture",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        )
    };
    let mut a = spawn(&paths[0]);
    let mut b = spawn(&paths[1]);
    std::thread::sleep(Duration::from_secs(2));
    let original_a = platform.get_current_route(a.0.id()).unwrap();
    let original_b = platform.get_current_route(b.0.id()).unwrap();
    let mut config = Config::default();
    config.settings.routing_enabled = true;
    config
        .assignments
        .insert(monitors[0].id.clone(), devices[0].id.clone());
    config
        .assignments
        .insert(monitors[1].id.clone(), devices[1].id.clone());
    let move_to = |process: &mut TestProcess, monitor: &Monitor| {
        process.command(&format!(
            "move {} {}",
            monitor.bounds.x + 50,
            monitor.bounds.y + 50
        ))
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        move_to(&mut a, &monitors[0]);
        move_to(&mut b, &monitors[1]);
        std::thread::sleep(Duration::from_millis(300));
        let target_a = route_for_window(&platform, &config, a.0.id(), &devices, &monitors);
        let target_b = route_for_window(&platform, &config, b.0.id(), &devices, &monitors);
        let mut live_a = wait_for(&platform, a.0.id(), &target_a);
        let mut live_b = wait_for(&platform, b.0.id(), &target_b);
        println!("Live route after policy change: app A={live_a}, app B={live_b}");
        if !live_a {
            a.command("restart");
            live_a = wait_for(&platform, a.0.id(), &target_a);
        }
        if !live_b {
            b.command("restart");
            live_b = wait_for(&platform, b.0.id(), &target_b);
        }
        assert!(
            live_a && live_b,
            "Windows did not expose both test streams on their selected outputs"
        );
        println!("Verified simultaneous independent test streams on two real endpoints.");
        move_to(&mut a, &monitors[1]);
        std::thread::sleep(Duration::from_millis(300));
        let target = route_for_window(&platform, &config, a.0.id(), &devices, &monitors);
        if !wait_for(&platform, a.0.id(), &target) {
            a.command("restart");
            assert!(
                wait_for(&platform, a.0.id(), &target),
                "Stream did not move after recreation"
            );
            println!("Moving A required stream recreation.");
        }
        println!("Verified moving A to the second display's output.");
        move_to(&mut a, &monitors[0]);
        std::thread::sleep(Duration::from_millis(300));
        let target = route_for_window(&platform, &config, a.0.id(), &devices, &monitors);
        if !wait_for(&platform, a.0.id(), &target) {
            a.command("restart");
            assert!(wait_for(&platform, a.0.id(), &target));
        }
        println!("Verified moving A back to the first display's output.");
    }));
    let restore_a = platform.restore_routes(a.0.id(), &original_a);
    let restore_b = platform.restore_routes(b.0.id(), &original_b);
    drop(a);
    drop(b);
    std::fs::remove_dir_all(root).unwrap();
    restore_a.unwrap();
    restore_b.unwrap();
    let final_devices = platform.list_output_devices().unwrap();
    assert_eq!(
        devices.iter().find(|d| d.is_default).map(|d| &d.id),
        final_devices.iter().find(|d| d.is_default).map(|d| &d.id),
        "The Windows global default must remain unchanged"
    );
    println!("Verified that the Windows global default output is unchanged.");
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}
