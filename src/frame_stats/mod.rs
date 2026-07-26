//! Per-frame timing capture for render-stutter diagnosis.
//!
//! Probes in the render hot paths accumulate into global atomics so that no
//! signature between the UI loop, the renderer thread, and the terminal writer
//! has to thread instrumentation state through. The recorder snapshots the
//! counters once per frame and writes the deltas as one CSV row.

use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static TERMINAL_BYTES:   AtomicU64 = AtomicU64::new(0);
static TERMINAL_WRITE_NS: AtomicU64 = AtomicU64::new(0);
static SPECTRUM_NS:      AtomicU64 = AtomicU64::new(0);
static REBUILD_NS:    [AtomicU64; 3] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];
static REBUILD_COUNT: [AtomicU64; 3] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];

pub(crate) fn note_spectrum(elapsed: Duration) {
    SPECTRUM_NS.fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
}

pub(crate) fn note_rebuild(slot: usize, elapsed: Duration) {
    REBUILD_NS[slot].fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
    REBUILD_COUNT[slot].fetch_add(1, Ordering::Relaxed);
}

/// Stdout wrapper under the terminal backend: meters bytes and time spent in
/// write/flush syscalls. With metering off it is a plain pass-through — and it
/// is the seam where a buffered writer slots in for the A/B experiment.
pub(crate) struct MeteredStdout {
    out: io::Stdout,
    metering: bool,
}

impl MeteredStdout {
    pub(crate) fn new(metering: bool) -> Self {
        Self { out: io::stdout(), metering }
    }
}

impl Write for MeteredStdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.metering { return self.out.write(buf); }
        let start = Instant::now();
        let written = self.out.write(buf)?;
        TERMINAL_BYTES.fetch_add(written as u64, Ordering::Relaxed);
        TERMINAL_WRITE_NS.fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.metering { return self.out.flush(); }
        let start = Instant::now();
        self.out.flush()?;
        TERMINAL_WRITE_NS.fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
        Ok(())
    }
}

/// Writes one CSV row per frame to `frame-stats.csv` in the working directory.
pub(crate) struct Recorder {
    file: io::BufWriter<std::fs::File>,
    capture_start: Instant,
    prev_bytes: u64,
    prev_write_ns: u64,
    prev_spectrum_ns: u64,
    prev_rebuild_ns: [u64; 3],
    prev_rebuild_count: [u64; 3],
}

pub(crate) const CAPTURE_FILENAME: &str = "frame-stats.csv";

impl Recorder {
    pub(crate) fn create() -> io::Result<Self> {
        let mut file = io::BufWriter::new(std::fs::File::create(CAPTURE_FILENAME)?);
        writeln!(file, "# deck {} capture", env!("CARGO_PKG_VERSION"))?;
        writeln!(file, "t_us,frame_us,service_us,spectrum_us,draw_us,write_us,bytes,budget_us,sleep_us,rebuild0_us,rebuild1_us,rebuild2_us,rebuild0_n,rebuild1_n,rebuild2_n")?;
        Ok(Self {
            file,
            capture_start: Instant::now(),
            prev_bytes: 0,
            prev_write_ns: 0,
            prev_spectrum_ns: 0,
            prev_rebuild_ns: [0; 3],
            prev_rebuild_count: [0; 3],
        })
    }

    /// Called once per frame, after the end-of-frame sleep. `frame_start` is
    /// the frame's own timestamp, so `frame_start.elapsed()` is the full frame.
    pub(crate) fn record_frame(
        &mut self,
        frame_start: Instant,
        service: Duration,
        draw: Duration,
        budget: Duration,
        sleep: Duration,
    ) {
        let bytes       = TERMINAL_BYTES.load(Ordering::Relaxed);
        let write_ns    = TERMINAL_WRITE_NS.load(Ordering::Relaxed);
        let spectrum_ns = SPECTRUM_NS.load(Ordering::Relaxed);
        let rebuild_ns:    [u64; 3] = std::array::from_fn(|s| REBUILD_NS[s].load(Ordering::Relaxed));
        let rebuild_count: [u64; 3] = std::array::from_fn(|s| REBUILD_COUNT[s].load(Ordering::Relaxed));

        let row = format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            (frame_start - self.capture_start).as_micros(),
            frame_start.elapsed().as_micros(),
            service.as_micros(),
            (spectrum_ns - self.prev_spectrum_ns) / 1_000,
            draw.as_micros(),
            (write_ns - self.prev_write_ns) / 1_000,
            bytes - self.prev_bytes,
            budget.as_micros(),
            sleep.as_micros(),
            (rebuild_ns[0] - self.prev_rebuild_ns[0]) / 1_000,
            (rebuild_ns[1] - self.prev_rebuild_ns[1]) / 1_000,
            (rebuild_ns[2] - self.prev_rebuild_ns[2]) / 1_000,
            rebuild_count[0] - self.prev_rebuild_count[0],
            rebuild_count[1] - self.prev_rebuild_count[1],
            rebuild_count[2] - self.prev_rebuild_count[2],
        );
        let _ = writeln!(self.file, "{row}");

        self.prev_bytes = bytes;
        self.prev_write_ns = write_ns;
        self.prev_spectrum_ns = spectrum_ns;
        self.prev_rebuild_ns = rebuild_ns;
        self.prev_rebuild_count = rebuild_count;
    }
}
