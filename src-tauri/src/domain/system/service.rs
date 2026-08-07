use std::path::Path;

const FILE_URL_SCHEME: &str = "file://";

pub fn file_url(path: &Path) -> String {
    format!("{FILE_URL_SCHEME}{}", path.display())
}

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
    fn 파일_경로를_file_url_로_변환한다() {
        assert_eq!(file_url(Path::new("/workspace/a b.html")), "file:///workspace/a b.html");
    }

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
