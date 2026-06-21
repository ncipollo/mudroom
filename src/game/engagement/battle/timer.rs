pub fn ticks_to_secs(ticks: u64, tick_rate_ms: u64) -> u64 {
    ticks * tick_rate_ms.max(1) / 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_converts_correctly_at_1000ms_tick_rate() {
        assert_eq!(ticks_to_secs(30, 1000), 30);
    }

    #[test]
    fn timer_converts_correctly_at_500ms_tick_rate() {
        // 60 max ticks * 500ms = 30 000ms = 30s; 30 countdown ticks * 500ms = 15s
        assert_eq!(ticks_to_secs(60, 500), 30);
        assert_eq!(ticks_to_secs(30, 500), 15);
    }
}
