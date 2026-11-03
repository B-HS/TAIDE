# native lane settings 묶음 S4: Sync·Remote section (s4-sync-remote)

> 작성일: 2026-11-03 · worktree `TAIDE-lanes/settings`(브랜치 `lane/settings`) · 사양 `docs/research/2026-10-28-lane-settings-spec.md` 2.4~2.8, 2F, 2G, 3절 S4
> 결정: 사양 4.1 전부 추천안(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`, 날짜 형식은 결정 5 A). 앞 묶음 S2·S3의 미커밋 변경 위에서 이어 작업했습니다.

## 1. 대상 ID와 판정

| ID | 내용 | 판정 |
| --- | --- | --- |
| settings-45 | Sync GitHub PAT 연결/해제 | complete(실제 GitHub·Keychain은 4.2 허락 항목) |
| settings-46 | Sync 상태(gist id, 마지막 동기화, remoteNewer 배지)·업로드·다운로드 | complete |
| settings-47 | Sync 충돌 대화상자(로컬 유지/원격 당김) | complete |
| settings-48 | Sync 설정 변경 후 동기화 데이터 반영 | partial(shell-93 미병합, 6절) |
| settings-49 | Remote 서버 활성 토글(설명) | complete |
| settings-50 | Remote 실행 상태·포트·클라이언트 수·보안 경고 | complete |
| settings-51 | Remote 비밀번호 설정/해제·최소 길이·password-only 토글 | complete(Keychain 실기는 4.2) |
| settings-52 | Remote 허용 호스트(추가/삭제, RFC 1035 검증, 중복) | complete |
| settings-53 | Remote 접속 링크 발급·클립보드 복사·세션 폐기 | complete(실제 클립보드는 4.2) |

## 2. 원본 동작 근거

- 사양 2F(45~48), 2G(49~53)의 원본 인용을 따릅니다. 이번에 다시 확인한 원본: `src/features/settings/{sync-section,sync-conflict-dialog,remote-section,remote-password-row,remote-allowed-hosts-row}.tsx`, `remote-allowed-hosts-row.test.ts`, `src/widgets/settings-view/{settings-sync-section,settings-remote-section,settings-view}.tsx`(section 순서 Plugins → Sync → Remote, `issuedRemoteUrl`·`isSyncConflictOpen`·sync mutation은 컨테이너), `src/entities/{sync/sync.query,remote/remote.query}.ts`(refetchInterval 5초, 비밀번호 설정·해제 성공 시 무효화·toast, 해제 실패 무음), `src/app/providers/ipc-sync-provider.tsx:413-428,482-487`.
- native 자산: `crates/taide-runtime/src/{sync_actions,remote_actions,sync_gist_http}.rs`, `native/taide-native-app/src/{remote-sync,remote-http,settings-integrations}.rs`.

## 3. 구현

### 3.1 공유 crate

- `crates/taide-model/src/remote.rs`: `REMOTE_PASSWORD_MIN_LEN`, `DNS_HOSTNAME_MAX_LEN`, `DNS_LABEL_MAX_LEN`, `is_valid_dns_label`, `is_valid_allowed_host`(문서 주석 포함 원문 그대로 이동). `crates/taide-remote/src/types.rs`는 최소 길이를 재수출(`ALLOWED_HOST_WILDCARD_PREFIX`와 같은 방식), `crates/taide-settings/src/service.rs`는 비공개 함수·상수를 지우고 옮긴 함수를 부릅니다(sanitize 동작 동일, 기존 검사 통과).

### 3.2 UI crate

- `settings-controls.rs`: `Section::{Sync, Remote}`(`HOSTED` 5개, 제목 `settings.syncSectionTitle`·`remote.title`, Remote 설명 `remote.description`), `HostedChange::{RemoteAccess, RemotePasswordOnly, RemoteAllowedHosts}`, `HostedChange::patch` 공개.
- `settings-sections-data.rs`: `Query::{SyncStatus(60초), RemoteStatus(60초 + 관찰 중 5초 재조회)}`, 행동 `SyncConnect(Secret)`·`SyncDisconnect`·`SyncUpload`·`SyncDownload{force, owner}`·`RemoteIssueLink{owner}`·`RemoteRevoke`·`RemoteSetPassword(Secret)`·`RemoteClearPassword`·`UpdateSettings(HostedChange)`, 진행 상태 `Busy`(행동 종류별 카운터, 응답·제출 실패로 해제), owner 주소 결과 `OwnerEvent::{SyncConflict, RemoteLinkIssued(url)}`(상한 16), 사건 반영 `record_sync_status`·`record_remote_status`, `observe_settings`에서 `remoteAccessEnabled` 변경 시 원격 상태 무효화, 알림 `Notice::SettingsFailed`.
- `settings-sections-view.rs`:
    - `Host { format_time: fn(&str) -> Option<String> }`(앱이 시각 형식 함수를 넘김). `OwnerState`에 section 상태(PAT·비밀번호·호스트 입력, IME 조합, 충돌 대화상자 Focus, 다운로드 버튼 id)와 컨테이너 상태 `Container { owner, issued_remote_url, is_sync_conflict_open }`. `reset_section_state`는 컨테이너 상태를 남깁니다. `OwnerState::receive`가 매 프레임 그 owner의 결과를 받습니다(`View::show`에서 1줄).
    - Sync: 미연결(설명, 범위 힌트 + 선택 가능한 토큰 URL, password 입력 + 연결 버튼), 연결(CircleCheck + medium 제목 + warning 15% 배지, 들여쓴 gist id/없음·마지막 동기화/Never, secret gist 경고, 업로드·다운로드 outline xs, 오른쪽 ghost CircleX 해제), 충돌 AlertDialog sm.
    - Remote: 토글(발급 URL 지움 + 패치), 상태 줄, 보안 경고, 비밀번호 행(테두리 rounded-md px12 py8, 미설정 힌트, trim 길이 1~7 경고, 설정·해제), 설정됨일 때 password-only 토글, 허용 호스트(라벨·설명, 목록 행 monospace + X 아이콘 버튼 tooltip·접근 이름 `remote.allowedHostsRemove{host}`, 무효 경고, 입력 + 추가, Enter·IME), 발급 URL(선택 가능, 줄바꿈), 발급·폐기 버튼.
- `settings-dialogs.rs`: `Alert.alternative: Option<String>`. 있으면 왼쪽 버튼이 그 문구의 outline이고 `Choice::Alternative`를 돌려줍니다(취소 버튼 없는 변형). Escape는 여전히 `Choice::Cancel`.
- 아이콘 버튼 `xs_icon_button`에 variant 인자를 추가했습니다(Plugins 4곳은 Outline 그대로).

### 3.3 앱

- `settings-ports.rs`: 객체 안전 `GistClient` 트레이트(기본 구현 `SyncGistHttpPort`)와 그것을 `SyncGistPort`로 맞추는 `Gist(Arc<dyn GistClient>)`, `Ports::gist`.
- 새 `settings-sync-time.rs`: `format_last_synced(iso, locale)`(chrono `Local`, ko `2026. 10. 28. 오후 2:05:09`, ja `2026/10/28 14:05:09`, 그 밖 en-US `10/28/2026, 2:05:09 PM`), `format_system_time`(시스템 locale을 프로세스당 1회 읽음).
- `domains/settings.rs`: `Events`가 `SyncStateChanged`·`RemoteStateChanged`의 마지막 값을 보관하고 `tick_settings_reads`가 `Data`로 옮깁니다. `execute_sections`에 상태 조회 2종, 연결·해제·업로드(`run_nonabortable_result`), 다운로드(`prepare_sync_download` → `apply_sync_download` + `apply_and_broadcast` + reconcile), 링크 발급 + `write_clipboard`, 폐기, 비밀번호 설정·해제. `UpdateSettings`는 `dispatch` 안에서 바로 실행해 host 명령 순서를 지킵니다(`settings_update` + reconcile). `Notice::SettingsFailed` → 기존 `toasts.settings_failed`. `show_settings_tab`이 `Host { format_time: format_system_time }`을 넘깁니다.
- 공용 파일: `lib.rs` 모듈 1줄(`settings_sync_time`), `domains/settings.rs` 끝 검사 모듈 1줄. `application.rs`·`host.rs`는 고치지 않았습니다.

## 4. 사양·원본과 다른 점(근거)

1. 원격 설정 변경(토글·password-only·허용 호스트)은 `output.changes` → `Command::UpdateSettings` 대신 `Action::UpdateSettings`로 보내고, 호스트 `dispatch`가 spawn 없이 바로 실행합니다. 기존 `UpdateSettings`는 성공 응답이 없어 원본 `isUpdatingSettings`(허용 호스트 X·추가 비활성)를 만들 수 없기 때문이고, 바로 실행하므로 다른 host 명령과 순서가 같습니다. "저장 중"은 이 section의 원격 설정 변경만 셉니다(원본은 컨테이너의 모든 설정 변경). 실패 문구는 기존 경로와 같은 `settings.saveFailed` toast입니다.
2. 사양 2.2의 `Host::last_synced_display` 대신 형식 함수 포인터를 넘깁니다(`Host`가 `Copy`로 남고 UI 검사가 형식을 주입).
3. `Data`가 모든 owner에 공유되므로 충돌 열림과 발급 URL은 요청한 owner의 ticket으로 돌려줍니다(S3 VSIX ticket과 같은 방식). 원본은 컨테이너 mutation 콜백이 해당 창에만 반영합니다.
4. 충돌 대화상자의 첫 포커스는 "로컬 유지"입니다. 원본 Radix AlertDialog는 Cancel이 없으면 첫 포커스를 옮기지 않아 뒤의 다운로드 버튼에 포커스가 남고 Enter로 다운로드가 다시 눌리는 결함이 있어 재현하지 않았습니다. 긴 문구 버튼은 2열 폭보다 넓어질 수 있습니다(원본도 grid track을 넘침).
5. `remoteAccessEnabled` 변경 감지는 사양 2.8의 앱 `settings_revision` 비교 대신 Remote section을 그릴 때 `Data::observe_settings`에서 합니다. Settings가 닫혀 있던 동안의 변경은 다음 mount의 신선도 판정으로 재조회됩니다.
6. 마지막 동기화 시각이 RFC 3339가 아니면 원문을 표시합니다(사양). 원본은 `Invalid Date`.
7. 새 입력 3개(PAT, 원격 비밀번호, 허용 호스트)는 `id_salt`로 id를 고정했습니다. egui 자동 id가 위쪽 경고 줄이 생길 때 바뀌어 첫 글자 입력 직후 포커스를 잃는 문제를 [U] 검사로 확인하고 근본 수정했습니다.
8. 사양 2.7의 `enum Gist { Http, #[cfg(test)] Fake }` 대신 객체 안전 트레이트를 두었습니다. 운영 코드에 검사 전용 분기가 없습니다.
9. 50의 5초 재조회는 [C]에서 주입한 시각으로 검사했고, 화면은 다음 재조회 시각에 `request_repaint_after`를 겁니다. [A]에서 5초를 실제로 기다리지는 않았습니다.
10. S2 App 검사 `Scene::launch(None, ..)`의 기본 자격 증명 저장소를 실제(격리 서비스명) Keychain에서 메모리 가짜로 바꿨습니다. 새 Sync section이 GitHub 계정을 조회해 기존 검사들이 실제 Keychain을 읽게 되는 것을 막기 위함입니다. 재사용을 위해 `Keychain`·`type_into`·`keys`를 `pub(super)`로 열었습니다(S2·S3 검사 통과).

## 5. 검증

실행 명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/settings`입니다.

| 범위 | 명령 | 결과 |
| --- | --- | --- |
| [C] 실패 확인 | `cargo test --manifest-path Cargo.toml -p taide-model --lib -- remote::tests`(구현 전) | 컴파일 실패(E0425 22건, 함수·상수 없음) |
| [C] 공유 crate | `cargo test --manifest-path Cargo.toml -p taide-model -p taide-settings -p taide-remote` | lib 대상 3개 31·79·69 passed(taide-model 새 3개 포함, taide-settings 기존 sanitize 검사 유지) |
| src-tauri | `cargo check --manifest-path Cargo.toml -p taide` | 통과 |
| [C][U] UI sections | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection -- settings_sections sections_view` | 35 passed(새 data 3, view 6) |
| UI lib 전체 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection` | 202 passed |
| UI 통합 | `... --features native-host,inspection --test settings-code-controls --test snippet-editor` | 1 + 18 passed |
| [A][C] 새 App·날짜 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- settings_sync_remote_tests settings_sync_time --test-threads=2` | 6 passed |
| 앱 settings·remote 회귀 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- settings_ theme_ snippet_host_tests app_file_views remote_preferences presentation remote_sync remote_http remote_dispatch --test-threads=2` | 61 passed(S2·S3·S4 App 검사 포함) |
| 앱 통합 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test settings-controls` | 2 passed |
| 컴파일 | `cargo check --all-targets`(UI `--features native-host,inspection`, 앱 기본) | 통과, 경고는 기존 것뿐(`tests/workbench.rs` unused mut, vendor wry, `is_applying`, `zen.rs`, `fixture_with_emmet_peek`) |
| 동결 미러 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 통과, 경고는 기존 `browser-editor.rs`·`browser-application.rs` |
| 형식 | `cargo fmt -- --check`(UI, 앱), `cargo fmt -p taide-model -p taide-settings -p taide-remote -- --check` | UI·앱·taide-settings·taide-remote 차이 없음. taide-model은 기존 `lib.rs`(`ids`/`identifier` 순서) 차이만 있음(범위 밖이라 그대로 둠) |
| lockfile | `git status --short` | `Cargo.toml`·`Cargo.lock` 변경 없음 |

- [C] `taide-model` `remote::tests`: TS 사례 6개(일반, 단일 레이블 와일드카드, 맨몸 `*`, `*.com`, `foo.*.com`, `*foo.com`), 레이블 63/64·앞뒤 하이픈·전체 253/254·와일드카드 253·스킴·포트·userinfo·공백·IPv4, 최소 길이 8.
- [C] `settings-sections-data-tests.rs`: 동기화 응답(연결 상태 교체·`Secret` 가림·진행 카운터, 실패 문구 키 3종, 업로드 성공/고정 실패, 충돌은 요청 owner에만 사건, force 충돌은 성공 문구, applied 상태 교체, 사건 반영), 원격 상태(4초 재조회 없음·남은 1초, 5초 재조회, 사건 교체, 비밀번호 설정 실패 무효화 없음·성공 무효화, 해제 실패 무음·성공 무효화, 토글 변경 무효화), 링크(복사 성공만 toast, 실패 `remote.startFailed`, owner 사건 상한 16), 폐기, 설정 저장 진행·실패 알림, 제출 실패 응답 식별자 6종.
- [C] `settings-sync-time.rs`: ko-KR·ja-JP·en-US·fr-FR(→ en-US)·언어만·밑줄 locale, 자정(오전 12, 0시)·정오, 잘못된 문자열·빈 문자열 → `None`, RFC 3339 → `Local` 변환.
- [U] `settings-sections-view-tests.rs`: 미러에는 Sync·Remote 없음과 호스트 순서(Plugins < Sync < Remote, Remote 설명 1회, 조회 5종), 미연결(설명·힌트·URL foreground, 끌어 선택 + 복사 = URL, 공백 PAT 비활성, trim PAT 요청·입력 비움, 연결 중 비활성, 연결 뒤 미연결 문구 사라짐), 연결(제목 3회 = 목차·카드·본문, 배지 warning, 없음 2종, 시간 형식/원문, 경고, 버튼 순서·진행 중 비활성·요청 3종), 충돌(문구·취소 없음·버튼 순서, 첫 포커스·Tab 순환, 바깥 클릭 무시, section 상태 초기화 뒤에도 열림, Escape 무요청·다운로드로 포커스 복귀, 로컬 유지 → 업로드, 원격 당김 → force 다운로드), 원격(멈춤·경고·발급/폐기 비활성·요소 순서, 실행 중 포트·muted 클라이언트 수, 발급 요청·발급 중 비활성, URL foreground·위치, 폐기, 컨테이너 URL 유지, 토글 → URL 지움·패치), 비밀번호(미설정 badge·힌트, 1글자 입력 뒤 포커스 유지·경고, 7/8 경계, trim 7, 한글 8자 요청·입력 비움, 저장 중 비활성, 설정됨 색·토글 노출·해제 위치, 해제·토글 요청), 허용 호스트(행·X 활성, 무효 경고 색·비활성, 대소문자 무시 중복, IME 조합 중 포커스 유지·Enter 무시, Enter 추가(trim·소문자)·입력 비움, 저장 중 X·추가 비활성, 추가 3개, 삭제).
- [A] `application-settings-sync-remote-tests.rs`(실제 `NativeApplication`, 경로 main.rs → `NativeApplication` → `show_window` → `show_settings_tab` → `Views::show_with_host` → `sections_view::{sync, remote}` → `load_settings_requests` → `domains::settings::dispatch`):
    1. 가짜 gist·keychain: PAT 입력(앞뒤 공백) + 연결 → keychain `PAT`, `syncGistId`, gist id·Never·배지 표시; 업로드 → 성공 toast·형식화된 마지막 동기화; 업로드 거절 → `settings.syncUploadFailed`; 다운로드 applied → 성공 toast·`editorFontSize` 19·settings.json; `SyncStateChanged` 발행 → 새 시각·배지; 해제 → 성공 toast·미연결 화면·keychain·gist id 비움; discover 거절 → 그 문구 toast·keychain 비어 있음.
    2. 원격이 더 최신인 gist: 배지 → 다운로드 → 대화상자 → 로컬 유지 → 업로드 1회·성공 toast; 더 새 원격 → 다운로드 → 원격 당김 → force 적용(`editorFontSize` 21, 마지막 동기화 = 원격 시각)·업로드는 여전히 1회·성공 toast 각 1회.
    3. 가짜 클립보드·가짜 reconcile bridge: 멈춤·경고 → 앱의 실제 remote ports로 loopback 서버 시작 → 사건으로 포트·0 connected → 발급 → `http://127.0.0.1:{port}/?t=` URL이 클립보드 기록·화면과 같고 `remote.linkCopied`; 클립보드 거절 → 새 URL 표시, 기록·성공 toast 수 그대로, 오류 toast 없음; 폐기 → `remote.sessionsRevoked`; 토글 → reconcile `[true]` 1회·URL 사라짐·settings.json `remoteAccessEnabled: true`; 서버 정지 → 멈춤.
    4. 가짜 keychain: "short" → 최소 길이 경고, "-enough" 추가 입력 → 경고 사라짐 → 설정 → 해시 저장(평문 아님)·세션 폐기 toast·설정됨·password-only 토글 → `remotePasswordOnlyLogin: true`; 해제 → toast 2회째·keychain 비움; 허용 호스트 " Tunnel.Example.com " + Enter → `["tunnel.example.com"]`(settings.json)·행 표시; X → `[]`.

## 6. 남은 범위

- settings-48: `SyncStateChanged` 뒤 테마·로케일 재로드와 Settings 테마 목록 무효화는 shell-93(`presentation_refresh::Changes::record`가 `SyncStateChanged`를 기록)의 산출물인데 이 worktree에 없습니다(`presentation-refresh.rs`에 `SyncStateChanged` 처리 없음 확인). 다운로드가 새 테마 파일을 쓰는 시점(`apply_sync_download`는 설정 적용 뒤 테마·로케일 파일을 씀)과 맞물려, 병합 뒤 "새 사용자 테마와 그 테마를 고른 설정 → 테마 목록·앱 presentation·열린 settings.json 탭 갱신" App 검사를 추가하고 행을 complete로 올립니다.
- 실제 GitHub gist(네트워크), Keychain, 실제 클립보드, 원격 서버 실기 기동은 4.2 허락 항목이라 자동 검사에서 부르지 않았습니다(loopback 서버는 검사 프로세스 안에서만 시작·정지).
- `crates/taide-infra/src/external_url.rs:36` 문서 주석이 옛 경로 `settings::service::is_valid_allowed_host`를 가리킵니다. lane 소유 파일 밖이고 문서 주석뿐이라 고치지 않았습니다.
- 대화상자 등장·퇴장 애니메이션은 S3과 같이 넣지 않았습니다.
