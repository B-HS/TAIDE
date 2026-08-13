use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::types::{SystemUsageProcess, SystemUsageProcessKind};

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

/// A single sysinfo-refreshed process, reduced to the fields the usage
/// breakdown needs (pid tree shape + cpu/memory readout).
#[derive(Debug, Clone)]
pub struct ProcessRecord {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    /// Whether this pid already had a cpu sample from a prior breakdown refresh. A pid's first
    /// appearance carries a meaningless `cpu_usage` (sysinfo has no prior time window to diff
    /// against yet), so `build_usage_processes` surfaces it as `cpu_percent: None` instead of a
    /// false zero.
    pub has_previous_cpu_sample: bool,
}

const SHELL_PROCESS_NAMES: &[&str] = &[
    "bash",
    "zsh",
    "sh",
    "dash",
    "fish",
    "ksh",
    "tcsh",
    "csh",
    "pwsh",
    "powershell",
    "cmd",
];

const KNOWN_LSP_BINARY_NAMES: &[&str] = &[
    "rust-analyzer",
    "typescript-language-server",
    "vtsls",
    "gopls",
    "pyright-langserver",
    "basedpyright",
    "pyright",
    "clangd",
    "solargraph",
    "lua-language-server",
    "yaml-language-server",
    "vscode-json-language-server",
    "vscode-css-language-server",
    "vscode-html-language-server",
    "vscode-eslint-language-server",
    "tailwindcss-language-server",
    "omnisharp",
    "jdtls",
    "metals",
    "elixir-ls",
    "zls",
];

fn process_basename(raw: &str) -> &str {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or(raw);
    base.strip_suffix(".exe").unwrap_or(base)
}

/// Fallback classification for descendant processes that no domain claimed
/// via PID (ipc-contract.md §기능 확장 2차 계약 확정 추가).
pub fn classify_process_name(name: &str) -> SystemUsageProcessKind {
    let base = process_basename(name);
    if SHELL_PROCESS_NAMES.contains(&base) {
        return SystemUsageProcessKind::Terminal;
    }
    if KNOWN_LSP_BINARY_NAMES.contains(&base) {
        return SystemUsageProcessKind::Lsp;
    }
    SystemUsageProcessKind::Other
}

/// Walks the parent-pid chain to collect every descendant of `root_pid`
/// (excludes `root_pid` itself).
pub fn collect_descendant_pids(records: &[ProcessRecord], root_pid: u32) -> Vec<u32> {
    let mut frontier = vec![root_pid];
    let mut visited: HashSet<u32> = HashSet::new();
    let mut descendants = Vec::new();

    while let Some(pid) = frontier.pop() {
        if !visited.insert(pid) {
            continue;
        }
        for record in records.iter().filter(|record| record.parent_pid == Some(pid)) {
            descendants.push(record.pid);
            frontier.push(record.pid);
        }
    }

    descendants
}

/// Builds the process breakdown for `root_pid` and its full descendant tree.
/// `domain_labels` maps a pid to a `(kind, label)` pair supplied by a domain
/// that already knows what that pid is (terminal foreground pid, LSP server,
/// detected agent); pids missing from that map fall back to
/// [`classify_process_name`]. Sorted by memory descending.
pub fn build_usage_processes(
    records: &[ProcessRecord],
    root_pid: u32,
    app_label: &str,
    domain_labels: &HashMap<u32, (SystemUsageProcessKind, String)>,
    cpu_count: usize,
) -> Vec<SystemUsageProcess> {
    let mut pids = collect_descendant_pids(records, root_pid);
    pids.push(root_pid);

    let mut processes: Vec<SystemUsageProcess> = pids
        .into_iter()
        .filter_map(|pid| {
            let record = records.iter().find(|record| record.pid == pid)?;
            let (kind, label) = if pid == root_pid {
                (SystemUsageProcessKind::App, app_label.to_string())
            } else if let Some((kind, label)) = domain_labels.get(&pid) {
                (*kind, label.clone())
            } else {
                (classify_process_name(&record.name), record.name.clone())
            };
            Some(SystemUsageProcess {
                pid,
                kind,
                label,
                cpu_percent: record
                    .has_previous_cpu_sample
                    .then(|| normalize_cpu_percent(record.cpu_usage, cpu_count)),
                memory_bytes: record.memory as f64,
            })
        })
        .collect();

    processes.sort_by(|a, b| b.memory_bytes.total_cmp(&a.memory_bytes));
    processes
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

    fn record(pid: u32, parent_pid: Option<u32>, name: &str) -> ProcessRecord {
        ProcessRecord {
            pid,
            parent_pid,
            name: name.to_string(),
            cpu_usage: 0.0,
            memory: 0,
            has_previous_cpu_sample: true,
        }
    }

    #[test]
    fn 쉘_프로세스명은_terminal로_분류한다() {
        assert_eq!(classify_process_name("zsh"), SystemUsageProcessKind::Terminal);
        assert_eq!(classify_process_name("/bin/bash"), SystemUsageProcessKind::Terminal);
        assert_eq!(classify_process_name("powershell.exe"), SystemUsageProcessKind::Terminal);
    }

    #[test]
    fn 알려진_lsp_바이너리명은_lsp로_분류한다() {
        assert_eq!(classify_process_name("rust-analyzer"), SystemUsageProcessKind::Lsp);
        assert_eq!(classify_process_name("C:\\tools\\gopls.exe"), SystemUsageProcessKind::Lsp);
    }

    #[test]
    fn 알수없는_프로세스명은_other로_분류한다() {
        assert_eq!(classify_process_name("Finder"), SystemUsageProcessKind::Other);
    }

    #[test]
    fn 자손_pid를_부모체인으로_전부_수집한다() {
        let records = vec![
            record(1, None, "taide"),
            record(2, Some(1), "zsh"),
            record(3, Some(2), "npm"),
            record(4, Some(3), "node"),
            record(5, None, "unrelated"),
        ];
        let mut descendants = collect_descendant_pids(&records, 1);
        descendants.sort_unstable();
        assert_eq!(descendants, vec![2, 3, 4]);
    }

    #[test]
    fn 순환_참조가_있어도_무한루프_없이_종료한다() {
        let records = vec![record(1, Some(2), "a"), record(2, Some(1), "b")];
        let mut descendants = collect_descendant_pids(&records, 1);
        descendants.sort_unstable();
        assert_eq!(descendants, vec![1, 2]);
    }

    #[test]
    fn 도메인_라벨이_있으면_우선_적용하고_없으면_프로세스명으로_분류한다() {
        let records = vec![
            record(1, None, "taide"),
            record(2, Some(1), "zsh"),
            record(3, Some(1), "rust-analyzer"),
            record(4, Some(1), "Finder"),
        ];
        let mut domain_labels = HashMap::new();
        domain_labels.insert(2, (SystemUsageProcessKind::Terminal, "my-project".to_string()));

        let processes = build_usage_processes(&records, 1, "TAIDE", &domain_labels, 1);

        let app = processes.iter().find(|process| process.pid == 1).expect("app row");
        assert_eq!(app.kind, SystemUsageProcessKind::App);
        assert_eq!(app.label, "TAIDE");

        let terminal = processes.iter().find(|process| process.pid == 2).expect("terminal row");
        assert_eq!(terminal.kind, SystemUsageProcessKind::Terminal);
        assert_eq!(terminal.label, "my-project");

        let lsp = processes.iter().find(|process| process.pid == 3).expect("lsp row");
        assert_eq!(lsp.kind, SystemUsageProcessKind::Lsp);
        assert_eq!(lsp.label, "rust-analyzer");

        let other = processes.iter().find(|process| process.pid == 4).expect("other row");
        assert_eq!(other.kind, SystemUsageProcessKind::Other);
        assert_eq!(other.label, "Finder");

        assert_eq!(processes.len(), 4);
    }

    #[test]
    fn 메모리_내림차순으로_정렬한다() {
        let records = vec![record(1, None, "taide"), record(2, Some(1), "a"), record(3, Some(1), "b")];
        let mut records_with_memory = records;
        records_with_memory[0].memory = 100;
        records_with_memory[1].memory = 300;
        records_with_memory[2].memory = 200;

        let processes = build_usage_processes(&records_with_memory, 1, "TAIDE", &HashMap::new(), 1);
        let memories: Vec<f64> = processes.iter().map(|process| process.memory_bytes).collect();
        assert_eq!(memories, vec![300.0, 200.0, 100.0]);
    }

    #[test]
    fn 처음_등장한_pid는_cpu_percent가_none이다() {
        let mut records = vec![record(1, None, "taide"), record(2, Some(1), "zsh")];
        records[1].has_previous_cpu_sample = false;
        records[1].cpu_usage = 0.0;

        let processes = build_usage_processes(&records, 1, "TAIDE", &HashMap::new(), 1);

        let first_seen = processes.iter().find(|process| process.pid == 2).expect("first-seen row");
        assert_eq!(first_seen.cpu_percent, None);
    }

    #[test]
    fn 이전에_샘플링된_pid는_cpu_percent가_정규화된_값이다() {
        let mut records = vec![record(1, None, "taide"), record(2, Some(1), "zsh")];
        records[1].has_previous_cpu_sample = true;
        records[1].cpu_usage = 200.0;

        let processes = build_usage_processes(&records, 1, "TAIDE", &HashMap::new(), 2);

        let sampled = processes.iter().find(|process| process.pid == 2).expect("sampled row");
        assert_eq!(sampled.cpu_percent, Some(100.0));
    }
}
