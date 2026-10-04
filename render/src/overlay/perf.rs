//! Frame-timing statistics for the debug overlay.

/// How many recent frames the graph keeps.
pub const HISTORY: usize = 120;

/// A ring buffer of recent frame times (milliseconds), with derived rates.
/// Fixed capacity, so it never allocates.
#[derive(Clone, Copy, Debug)]
pub struct PerfGraph {
    times: [f32; HISTORY],
    head: usize,
    len: usize,
    /// Frame-time budget in milliseconds (16.6 for 60 fps).
    pub budget_ms: f32,
}

impl Default for PerfGraph {
    fn default() -> Self {
        Self::new(1000.0 / 60.0)
    }
}

impl PerfGraph {
    /// A graph with the given per-frame budget in milliseconds.
    pub fn new(budget_ms: f32) -> Self {
        Self {
            times: [0.0; HISTORY],
            head: 0,
            len: 0,
            budget_ms,
        }
    }

    /// Records one frame's wall-clock duration in milliseconds.
    pub fn push(&mut self, ms: f32) {
        let ms = if ms.is_finite() { ms.max(0.0) } else { 0.0 };
        self.times[self.head] = ms;
        self.head = (self.head + 1) % HISTORY;
        self.len = (self.len + 1).min(HISTORY);
    }

    /// Number of samples held (0..=HISTORY).
    pub fn len(&self) -> usize {
        self.len
    }

    /// True if no frames have been recorded.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Samples oldest to newest.
    pub fn samples(&self) -> impl Iterator<Item = f32> + '_ {
        let start = if self.len < HISTORY { 0 } else { self.head };
        (0..self.len).map(move |i| self.times[(start + i) % HISTORY])
    }

    /// Mean frame time in milliseconds (0 if empty).
    pub fn average_ms(&self) -> f32 {
        if self.len == 0 {
            return 0.0;
        }
        self.samples().sum::<f32>() / self.len as f32
    }

    /// Longest recent frame time in milliseconds.
    pub fn max_ms(&self) -> f32 {
        self.samples().fold(0.0_f32, f32::max)
    }

    /// Frames per second from the average frame time (0 if empty).
    pub fn fps(&self) -> f32 {
        let avg = self.average_ms();
        if avg > 0.0 {
            1000.0 / avg
        } else {
            0.0
        }
    }

    /// How many recent frames exceeded the budget.
    pub fn over_budget(&self) -> usize {
        self.samples().filter(|&ms| ms > self.budget_ms).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_reports_zeroes() {
        let g = PerfGraph::default();
        assert!(g.is_empty());
        assert_eq!(g.fps(), 0.0);
        assert_eq!(g.average_ms(), 0.0);
        assert_eq!(g.over_budget(), 0);
    }

    #[test]
    fn steady_sixteen_ms_is_about_sixty_fps() {
        let mut g = PerfGraph::default();
        for _ in 0..60 {
            g.push(1000.0 / 60.0);
        }
        assert!((g.fps() - 60.0).abs() < 0.01);
        assert_eq!(g.over_budget(), 0);
    }

    #[test]
    fn the_ring_keeps_only_the_last_history_frames() {
        let mut g = PerfGraph::new(16.6);
        for i in 0..(HISTORY + 10) {
            g.push(i as f32);
        }
        assert_eq!(g.len(), HISTORY);
        let first = g.samples().next().expect("sample");
        assert_eq!(first, 10.0, "oldest kept sample is frame 10");
        assert_eq!(g.max_ms(), (HISTORY + 9) as f32);
    }

    #[test]
    fn over_budget_counts_slow_frames() {
        let mut g = PerfGraph::new(16.6);
        g.push(10.0);
        g.push(20.0);
        g.push(33.0);
        assert_eq!(g.over_budget(), 2);
    }

    #[test]
    fn non_finite_and_negative_samples_are_clamped() {
        let mut g = PerfGraph::default();
        g.push(f32::NAN);
        g.push(-5.0);
        assert_eq!(g.max_ms(), 0.0);
    }
}
