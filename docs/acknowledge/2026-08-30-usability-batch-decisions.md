# 사용성 배치(Ctrl+G·팔레트 캐럿·빈 폴더) 사용자 결정 (2026-08-30)

> 배경: 2026-08-29~30 사용성 3건 wf(opus·max, `wf_a068d4a0`) 구현 후 미결 결정에 대한 회신.
> 구현 경위·검토는 `docs/PROCESS.md` "사용성 3건" 절, 설계 정본은
> `docs/features/agent-integration.md` §2.3.

## 결정

1. **`VISUAL` 도 `EDITOR` 와 같은 값으로 주입 (B안 채택)**. Claude Code 는 `$VISUAL` 을
   `$EDITOR` 보다 먼저 해석하므로(claude 2.1.251 바이너리 실측), EDITOR 만 주입하면 `VISUAL`
   사용자에게 ctrl+g 가 계속 vim 으로 열린다. 부모 프로세스 환경의 기존 `EDITOR`/`VISUAL` 은
   의도적으로 덮는다(기존 "부모 EDITOR 존중" 규칙 폐지). 최종 우선권은 셸 rc 의 export 에 있다
   — rc 는 스폰 이후 실행되어 주입값을 자연히 덮는다.
   - 반영: `agent::service::build_editor_env_entries(cli_path)` (구 `build_editor_env_entry`
     의 parent_editor 파라미터 제거), `agent::commands::editor_terminal_env`.

## 함께 확정된 사실 (재론 방지)

- **git status 의 빈 폴더는 버그가 아니다**: libgit2 1.9.6 은 `recurse_untracked_dirs(false)`
  에서도 디렉토리 내용을 검사해 빈/ignored-only 디렉토리를 상태에서 제외한다(libgit2
  `diff_generate.c` 소스 + 실측 2회 — git CLI 와 일치). `collect_status_rows` 는 무수정,
  회귀 테스트 4건으로 계약 고정. 사용자가 목격한 "빈 폴더" 증상은 재현 정보(해당 폴더의 실제
  내용) 확보 시 재추적한다 — 유력 가설은 트리 숨김 목록(`IGNORED_DIR_NAMES`)만 든 폴더.
- **EDITOR/VISUAL 값은 셸 인용 금지**: Claude Code 가 공백 split 후 shell 없이 spawn 하므로
  인용 시 ENOENT. 공백 든 경로는 주입 생략.
