export const meta = {
    name: 'hand-qa-fix-review',
    description: '손 QA 6건 수정 Phase E — 4렌즈 검토 → 적대적 검증 → 수정',
    phases: [
        { title: 'Lenses', detail: 'opus+xhigh 4렌즈 병렬' },
        { title: 'Verify', detail: 'opus+high 적대적 검증 (critical/major)' },
        { title: 'Fix', detail: 'sonnet+xhigh confirmed 수정' },
    ],
}

const FINDINGS_SCHEMA = {
    type: 'object',
    required: ['findings'],
    properties: {
        findings: {
            type: 'array',
            items: {
                type: 'object',
                required: ['severity', 'title', 'file', 'claim', 'evidence', 'suggestedFix'],
                properties: {
                    severity: { type: 'string', enum: ['critical', 'major', 'minor'] },
                    title: { type: 'string' },
                    file: { type: 'string' },
                    line: { type: 'number' },
                    claim: { type: 'string' },
                    evidence: { type: 'string' },
                    suggestedFix: { type: 'string' },
                },
            },
        },
    },
}

const VERDICT_SCHEMA = {
    type: 'object',
    required: ['confirmed', 'severity', 'reasoning'],
    properties: {
        confirmed: { type: 'boolean' },
        severity: { type: 'string', enum: ['critical', 'major', 'minor', 'invalid'] },
        reasoning: { type: 'string' },
    },
}

const COMMON = `TAIDE 레포(/Users/hyunseokbyun/TAIDE) 검토 에이전트(읽기 전용 — 코드·문서 수정 금지, 앱 실행 금지. bun test·cargo test 단발 실행은 허용).
검토 대상: 커밋 b00c192("fix: 손 QA 1차 6건 수정")의 diff 전체 — git show --stat b00c192 로 46파일 확인 후 git show b00c192 로 정독(신규 파일 포함). 이후 수정 커밋이 그 위에 더 있다면 b00c192..HEAD 범위도 함께 본다.
계약 정본: docs/acknowledge/2026-08-18-hand-qa-fix-contract.md (§1 확정 사실·§2 확정 설계·§3 기각·§4 완료 조건). 6건: ①터미널 링크(system_open_external_url+WebLinksAddon 핸들러) ②부트 테마(reveal 게이트+실패 배너+follow_system_theme 자동 해제) ③allowed hosts 와일드카드 ④LSP 블로킹(lsp_stop 가드 축소+조기 반환+프론트 dispose 유예 5s) ⑤peek 트위스티 CSS 1규칙 ⑥커밋 diff 에디터 탭 승격(TabKind::Diff rev 3필드).
구현 부기(에이전트 자진 신고 — 진위 검증 대상): lsp_restart 에 스토어 재확인 가드 추가(재구성이 새로 연 stop/restart 경합 대응) / set_theme 위치를 service 로 / Phase D 가 flush 배선 누락을 발견해 lsp-session-flush-registry.ts 레지스트리 패턴으로 수정.
발견은 실증 가능해야 한다: 파일:라인 인용 + 구체적 실패 시나리오(입력→잘못된 결과). 추측·스타일 취향은 제외. 이미 계약 §3 에서 기각된 대안 재론 금지.`

phase('Lenses')

const LENSES = [
    {
        key: 'contract',
        prompt: `렌즈: 계약 정합. 계약 §2.1~§2.7 의 문언 전부를 구현과 1:1 대조하라 — 누락된 요구(예: 수식어 게이트 ⌘·⌥, window.open 선시도 폴백, altClickMovesCursor false, 와일드카드 의미론 6규칙, 스토어 선제거, 조기 반환 상한, 유예 확정 정리 3경로, TabKind serde default, 패널 인라인 diff 완전 제거, 단축 해시 상수화, locale 4곳·en⊆required, 문서 6종+bug 2건+qa6 절), 계약과 다른 구현(자진 신고 3건의 타당성 포함), 계약이 요구한 테스트의 실재. §4 완료 조건 충족 여부.`,
    },
    {
        key: 'correctness',
        prompt: `렌즈: 정확성(동시성·수명주기 집중). 최우선: (a) lsp_stop/lsp_restart 가드 축소 — 언가드 shutdown 중 lsp_spawn/lsp_send/lsp_stop 인터리빙 전 조합, lsp_restart 의 스토어 재확인 가드가 실제로 leak 을 막는지·새 구멍(재확인 전 spawn 이 같은 키로 신규 세션 생성 후 restart 가 NotFound 로 죽는 경로 등)은 없는지, wait_for_process_exit 폴링과 stopping 플래그·handle_process_exit(자동 재시작 경로)의 상호작용 — shutdown 중 크래시 감지가 오발동해 재스폰하지 않는지. (b) 프론트 dispose 유예 — 유예 중 세션에 didClose 가 이미 전송됐는지(재획득 시 didOpen/didClose 시퀀스 정합), disposeTimer 와 flushLspSessionsForProject/flushAllLspSessions 경합, hot-exit flush 중 유예 발화, StrictMode 이중 마운트. (c) reveal 게이트 — 두 쿼리 에러 시 수렴 논증 검증, 보조 창(visible false 동일 경로) 영향. (d) 커밋 diff 탭 — 구 layout.json 복원·preview 탭 교체·중복 판정(PartialEq 에 rev 포함)·최초 커밋 parentRev null·rename beforePath. (e) 터미널 — onOpenLinkRef 수명, window.open 폴백의 실패 모드.`,
    },
    {
        key: 'security',
        prompt: `렌즈: 보안. (a) system_open_external_url — 스킴 화이트리스트 우회(대소문자·유니코드 유사문자· "http://" 접두 후 javascript: 중첩 불가 논증·제어문자 검사의 완전성·trim 의 함정), 원격 deny arm 실재+파리티, opener 호출의 인자 주입 면. (b) 와일드카드 — host_matches_allowed_entry 의 유사 도메인 함정 전수(evil-suffix·빈 레이블·트레일링 닷·포트 포함 hostname·대소문자), is_insecure_connection 통일이 Secure 쿠키 판정을 바꾸는 경계, sanitize 우회(공백·유니코드·IDN), 원격 게이트 3중 스트립 비상호작용 재확인, format_issue_link_url 의 URL 주입. (c) app 레이어 flush 배선이 원격 세션에서 오발동해 데스크톱 세션을 정리하는 경로는 없는지. (d) TabKind rev 필드로 임의 rev 문자열이 git_show_file 에 흘러갈 때의 injection 면(git2 native 라 셸 무관 — 논증 확인).`,
    },
    {
        key: 'design',
        prompt: `렌즈: 설계·추상화·컨벤션. FSD 배치(terminal-link-opener 위치·lsp-session-flush-registry 레지스트리 패턴의 정당성 vs 기존 mirror-flush-registry 선례 대조·shouldActivateTerminalLink 의 features 내 export·isValidAllowedHost export)·타입 유도(수동 타입 재작성 없는지, bindings 파생)·상수화(COMMIT_SHORT_HASH_LENGTH 위치·LSP_SESSION_DISPOSE_GRACE_MS 5000 근거·LSP_SHUTDOWN_POLL_INTERVAL_MS)·주석 규율(JSDoc/러스트 doc 만인지, 한국어 에러 메시지의 기존 관행 정합)·조기/누락 추상화·네이밍 정확성·dead code(제거된 commit-detail-panel 인라인 경로의 잔재·미사용 import)·문서 6종 갱신의 실코드 정합(ipc-contract 179 카운트·거부 12종 표).`,
    },
]

const lensResults = await parallel(
    LENSES.map((lens) => () =>
        agent(`${COMMON}\n\n${lens.prompt}\n\n발견 전부를 findings 배열로 반환하라(발견 없으면 빈 배열). severity 는 실사용 피해 기준으로 보수적으로.`, {
            label: `lens:${lens.key}`,
            phase: 'Lenses',
            model: 'opus',
            effort: 'xhigh',
            schema: FINDINGS_SCHEMA,
        }),
    ),
)

const allFindings = lensResults
    .filter(Boolean)
    .flatMap((r, lensIndex) => r.findings.map((f) => ({ ...f, lens: LENSES[lensIndex].key })))
const majorPlus = allFindings.filter((f) => f.severity === 'critical' || f.severity === 'major')
const minors = allFindings.filter((f) => f.severity === 'minor')
log(`렌즈 발견: 총 ${allFindings.length} (critical/major ${majorPlus.length} · minor ${minors.length})`)

phase('Verify')

const verified = await parallel(
    majorPlus.map((f) => () =>
        agent(
            `${COMMON}\n\n적대적 검증: 아래 발견을 **반증하려고 시도**하라. 실코드를 직접 열어 주장 라인·시나리오를 재현 논증하고, 성립하지 않으면 confirmed=false. 불확실하면 confirmed=false(보수).\n\n[${f.lens}/${f.severity}] ${f.title}\n파일: ${f.file}${f.line ? ':' + f.line : ''}\n주장: ${f.claim}\n근거: ${f.evidence}\n제안: ${f.suggestedFix}`,
            { label: `verify:${f.title.slice(0, 40)}`, phase: 'Verify', model: 'opus', effort: 'high', schema: VERDICT_SCHEMA },
        ).then((v) => ({ finding: f, verdict: v })),
    ),
)
const confirmed = verified.filter(Boolean).filter((v) => v.verdict?.confirmed)
log(`적대적 검증: ${majorPlus.length}건 중 confirmed ${confirmed.length}`)

phase('Fix')

const fixResult =
    confirmed.length + minors.length === 0
        ? 'confirmed/minor 0건 — 수정 불필요'
        : await agent(
              `TAIDE 레포(/Users/hyunseokbyun/TAIDE) 수정 에이전트. 계약 docs/acknowledge/2026-08-18-hand-qa-fix-contract.md 준수. 앱 실행·커밋 금지. HACK·검사기 끄기 금지. 컨벤션: arrow fn·주석 금지(JSDoc/러스트 doc 만)·매직넘버 금지·타입 유도·minimal diff. cargo 환경: export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"\n\n아래 confirmed(적대적 검증 통과) 발견을 전부 근본 수정하고, minor 는 타당하면 수정·아니면 사유와 함께 기각 목록으로 보고하라. 수정마다 회귀 테스트 동반(기존 테스트 파일 관행). 완료 전: bunx tsc --noEmit·bun test 전체·cargo fmt·cargo clippy -D warnings·cargo test --workspace 전부 exit 0.\n\n## confirmed (${confirmed.length}건 — 전건 수정 필수)\n${JSON.stringify(confirmed, null, 1).slice(0, 30000)}\n\n## minor (${minors.length}건 — 판단 수정)\n${JSON.stringify(minors, null, 1).slice(0, 20000)}\n\n보고: 수정/기각 각각의 목록·근거·검증 결과.`,
              { label: 'fixer', phase: 'Fix', model: 'sonnet', effort: 'xhigh' },
          )

return {
    totals: { all: allFindings.length, majorPlus: majorPlus.length, confirmed: confirmed.length, minors: minors.length },
    confirmed,
    rejectedByVerification: verified.filter(Boolean).filter((v) => !v.verdict?.confirmed).map((v) => ({ title: v.finding.title, reasoning: v.verdict?.reasoning })),
    minors,
    fixResult,
}