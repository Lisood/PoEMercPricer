//! Memory budgets for the scan path and the idle GUI. Both are `#[ignore]`
//! like the latency budget in `scan_screenshots.rs`: they measure this
//! machine (GPU driver included), not a shared CI runner. Run them after any
//! renderer, dependency or profile change:
//!
//!   cargo test --release --test resource_budgets -- --ignored --nocapture --test-threads=1
//!
//! Baselines and history: docs/performance.md, "Binary size and memory".

use std::os::windows::io::AsRawHandle;
use std::process::Command;
use std::time::Duration;

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};

fn memory_counters(process: HANDLE) -> PROCESS_MEMORY_COUNTERS {
    let mut counters = PROCESS_MEMORY_COUNTERS::default();
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    unsafe { K32GetProcessMemoryInfo(process, &mut counters, size) }.expect("query process memory");
    counters
}

#[test]
#[ignore = "measures this machine; run with --release -- --ignored --test-threads=1"]
fn warm_fullscreen_scan_peak_working_set_stays_within_budget() {
    let image = image::open("samples/fullscreen_danalla.jpg")
        .expect("decode fullscreen sample")
        .into_rgba8();
    // First call warms Windows OCR and the template cache; the second is the
    // repeat-Scan the user sees. Peak covers both.
    poemercpricer::scan::scan_rgba(&image).expect("warm scan");
    poemercpricer::scan::scan_rgba(&image).expect("second scan");

    // -1 is the current-process pseudo handle (GetCurrentProcess()).
    let current = HANDLE(-1isize as *mut core::ffi::c_void);
    let peak = memory_counters(current).PeakWorkingSetSize;
    let budget = 96usize << 20;
    eprintln!(
        "fullscreen scan peak working set: {:.1} MB (budget {} MB)",
        peak as f64 / 1048576.0,
        budget >> 20
    );
    assert!(
        peak <= budget,
        "scan path peaked at {peak} bytes, above the {budget} byte budget: something now retains a capture or a template set"
    );
}

#[test]
#[ignore = "launches the GUI; run with --release -- --ignored --test-threads=1"]
fn idle_gui_working_set_stays_within_budget() {
    // Cargo builds the bin for integration tests, so this is the exe of the
    // profile under test (target/release with --release).
    let mut child = Command::new(env!("CARGO_BIN_EXE_poemercpricer"))
        .arg("--no-updates")
        .spawn()
        .expect("launch poemercpricer");
    std::thread::sleep(Duration::from_secs(8));
    let counters = memory_counters(HANDLE(child.as_raw_handle()));
    let _ = child.kill();
    let _ = child.wait();

    let working_set = counters.WorkingSetSize;
    let budget = 160usize << 20;
    eprintln!(
        "idle GUI working set: {:.1} MB (budget {} MB)",
        working_set as f64 / 1048576.0,
        budget >> 20
    );
    assert!(
        working_set <= budget,
        "idle GUI working set {working_set} bytes exceeds the {budget} byte budget: check the wgpu backend list and memory hint in src/main.rs"
    );
}
