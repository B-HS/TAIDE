pub fn normalize_cpu_percent(raw_percent: f32, cpu_count: usize) -> f64 {
    if cpu_count == 0 {
        return 0.0;
    }
    (f64::from(raw_percent) / cpu_count as f64).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 코어_수만큼_cpu_사용률을_정규화한다() {
        assert_eq!(normalize_cpu_percent(400.0, 4), 100.0);
        assert_eq!(normalize_cpu_percent(50.0, 2), 25.0);
    }

    #[test]
    fn 코어_수가_0이면_0을_반환한다() {
        assert_eq!(normalize_cpu_percent(50.0, 0), 0.0);
    }

    #[test]
    fn 측정치가_전체_코어_기준을_넘으면_100으로_고정한다() {
        assert_eq!(normalize_cpu_percent(999.0, 1), 100.0);
    }
}
