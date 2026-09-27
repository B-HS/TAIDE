# LSP 설치 부모 선종료의 그룹 소유권 보완

## 대상 파일

- `crates/taide-infra/src/owned_child.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`
- `crates/taide-runtime/src/lsp_install_toolchain.rs`
- `docs/architecture.md`, `docs/PROCESS.md`, 연결된 설치/종료 QA

## 리포트

부모가 먼저 종료한 뒤 stdout/stderr를 보유한 자손이 남는 자기 생성 fixture를 추가했습니다. 기존 구현은 reader timeout 뒤 성공과 슬롯 해제를 반환했지만 자손은 살아 있어 exit 101로 실패했습니다. 부모 PID를 회수하지 않고 종료를 관찰한 뒤 그룹에 KILL을 전달하고, 마지막에 부모를 회수하도록 수정했습니다.

## 상세

1. Unix infra 경계는 `waitid(P_PID, WEXITED | WNOHANG | WNOWAIT)`로 자기 direct child만 관찰합니다. 반환 PID를 검사하고 EINTR만 재시도하며 다른 오류를 반환합니다. 공개 계약은 단독 소유·회수 전 호출이며 이미 회수한 Child 캐시를 재사용하지 않습니다.
2. runtime은 child mutex 안에서 관찰·그룹 종료·wait·회수 플래그 설정을 직렬화합니다. 늦은 취소/Drop은 회수 플래그를 확인해 숫자 PID를 다시 그룹 대상으로 쓰지 않습니다. 원래 종료 코드와 기존 출력/tail·취소 정책은 보존합니다.
3. macOS 커널은 그룹 시그널에서 좀비를 제외하며 대상이 없으면 EPERM을 반환할 수 있습니다. 정상 종료 2건이 이 조건으로 실패한 뒤, 부모가 종료했고 자기 PGID 조회에 그 부모 하나만 남았을 때만 빈 그룹으로 처리했습니다. 다른 구성원/조회 실패/권한 오류를 성공으로 바꾸지 않습니다. 조회 버퍼는 두 PID이며 전체 프로세스 목록·사용자 프로세스를 조회하지 않습니다.
4. 실패 fixture는 부모 종료 전 직접 만든 anchor child를 같은 그룹에 넣습니다. 실패 시에도 살아 있는 자기 anchor가 그룹 소유권을 유지할 때만 그룹을 종료하며 anchor를 회수하고 자기 UUID marker를 삭제합니다. 수정된 제품 경로가 anchor를 종료하면 fixture는 그룹을 재신호하지 않습니다.
5. 안정된 std API에 회수 없는 관찰이 없어 기존 전이 의존성이자 최신으로 확인한 libc 0.2.189를 Unix 직접 의존성으로 재사용했습니다. 패키지 버전/체크섬을 바꾸지 않았고 lockfile은 infra 의존 목록의 libc 한 줄만 추가됐습니다. C ABI 구조체는 libc 원본에서 사용합니다.

## 공식 근거와 검증

[Rust Child](https://doc.rust-lang.org/std/process/struct.Child.html)의 try_wait 회수 계약, [libc waitid](https://docs.rs/libc/0.2.189/libc/fn.waitid.html), Apple 원천의 [waitid WNOWAIT](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_exit.c), [그룹 시그널](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_sig.c), [PGID 한정 조회](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/proc_info.c)와 [상수](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info.h)를 대조했습니다. unsafe의 메모리 경계는 유효한 zero 초기화 siginfo_t·독점 mutable child·크기가 검사된 PID 배열 포인터입니다.

실제 명령·결과와 환경 실패는 [부모 선종료 QA](../quality-assurance/2026-09-27-lsp-install-parent-exit-lifecycle.md)에 기록합니다.

## 미완료 경계

그룹 시그널 전달과 direct child/reader 회수는 모든 자손의 wait/join을 뜻하지 않습니다. fixture는 직접 만든 자손 PID가 사라짐까지 관찰했지만 모든 OS/스케줄링에서 자손 종료 시간을 보장하지 않습니다. 그룹을 벗어난 자손·다중 좀비/권한 변경·Windows process tree·native 직접 Exit·감독되지 않은 nested worker/LSP wait/PTY·M6 body 전수·M7/M8·Phase 0 실기는 별도 gate입니다. 전체 M6 완료 전 push/UI 실행은 하지 않습니다.
