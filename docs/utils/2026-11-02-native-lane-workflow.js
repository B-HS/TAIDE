export const meta = {
  name: "native-lane",
  description:
    "native 전환 영역 lane 하나: 사양 기반 묶음 직렬 구현 → lane 리뷰 → 수정 (자기 worktree 안에서)",
  phases: [
    { title: "Implement", detail: "묶음별 직렬 구현과 검증", model: "opus" },
    { title: "Review", detail: "lane 전체 diff 리뷰", model: "sonnet" },
    { title: "Fix", detail: "차단 항목 수정", model: "opus" },
  ],
};

const MAIN = "/Users/hyunseokbyun/development/TAIDE";
const L = args.lane;
const ROOT = L.root;
const TARGET = L.target;
const BASE = args.base;
const DATE = args.date;
const NA = ROOT + "/native/taide-native-app";
const NU = ROOT + "/native/taide-native-ui";
const RW = ROOT + "/native/taide-remote-web";
const QA = ROOT + "/docs/quality-assurance";
const MATRIX = QA + "/2026-10-09-native-function-matrix.md";
const SPEC = ROOT + "/docs/research/2026-10-28-lane-" + L.spec + "-spec.md";
const CARGO_TAIL = `--locked --offline --target-dir ${TARGET}`;

const BACKGROUND = `
당신은 TAIDE(IDE) Rust-native 전환의 "${L.key}" lane 작업자입니다. 한국어 존댓말, 이모지 금지.

# 작업 위치 (가장 중요)
- 이 lane 의 저장소는 git worktree ${ROOT} (브랜치 lane/${L.key}) 입니다. 모든 읽기·수정·빌드는 이 경로 아래에서만 합니다. ${MAIN} 은 메인 작업 트리이므로 절대 수정하지 않습니다(예외: ${MAIN}/node_modules 는 원본 Monaco·라이브러리 소스 읽기 전용으로 허용, worktree 에는 node_modules 가 없습니다).
- cargo target 은 이 lane 전용 ${TARGET} 만 씁니다.
- 다른 lane(shell, explorer-search, terminal-agent, editor, preview, git, settings)이 동시에 다른 worktree 에서 작업합니다. 공용 파일은 영역 블록 규칙(${ROOT}/docs/research/2026-10-28-native-lane-seams.md 의 공용 파일 규칙)대로 자기 영역 블록 끝에 추가만 합니다.

# 기준
- 원본 TS 화면·상태·상호작용을 재현합니다(TS: ${ROOT}/src, Tauri: ${ROOT}/src-tauri, 코어: ${ROOT}/crates). 새 기능·디자인 추가 금지, 원본 결함은 억지로 재현하지 않습니다.
- 이 lane 의 사양: ${SPEC}. 원본 근거·native 자산·설계·묶음 계획이 이미 정리돼 있으니 그것을 기준으로 하고, 필요한 파일 구간만 확인합니다(원본 전체 재조사 금지). 사양이 실제 코드와 다르면 실제 코드를 따르고 QA 문서에 차이를 적습니다.
- 결정: ${ROOT}/docs/acknowledge/2026-10-28-native-lane-spec-decisions.md(사양 4.1 결정은 추천안, 보류 목록은 하지 않음)와 ${ROOT}/docs/acknowledge/2026-10-26-native-autonomous-continuation.md.
- 구조: ${ROOT}/docs/research/2026-10-28-native-lane-seams.md(영역 모듈 domains/<영역>.rs, HostCommand::<영역>, ShellIntent 영역 하위 enum).
- ${ROOT}/docs/PROCESS.md 와 HANDOFF.md 는 읽지 않습니다.
${L.note ?? ""}
`;

const RULES = `
# 절대 규칙
- 파일 수정은 Edit·Write 로만. 셸은 절대 경로 한 줄 단순 명령만(grep, rg, ls, wc, sed -n, cat, diff, git -C ${ROOT} status/diff/log/show, cargo check/test/build/fmt). cd·파이프·&&·;·루프·변수 대입·heredoc·python·sed -i·rm·npm 금지. 저장소 밖 탐색 금지(예외: cargo registry 의 crate 소스 읽기).
- git 상태 변경(add, commit, stash, checkout, reset, restore, push, worktree) 금지. 다른 에이전트를 띄우지 않습니다.
- cargo 는 "--manifest-path <${ROOT} 아래 절대경로> ... ${CARGO_TAIL}" 형태, 한 번에 하나, --quiet. Cargo.toml·Cargo.lock 은 사양이 새 의존성 없음으로 정한 범위에서 바꾸지 않습니다. 필요하면 구현하지 말고 unresolved 에 적습니다.
- 다른 lane 이 동시에 빌드해 CPU 부하가 큽니다. 앱 lib 실제 App 검사는 -- --test-threads=2 로 필터 실행하고, 시간 초과로 실패한 검사는 단독으로 1회 재실행해 흔들림과 결함을 구분합니다. 필터 없는 앱 전체 테스트는 메인이 병합 때 실행합니다.
- 앱 lib 의 LSP 검사는 ${TARGET}/debug/examples/native-lsp-mock 이 필요합니다. 없으면 cargo build --quiet --manifest-path ${NA}/Cargo.toml --example native-lsp-mock ${CARGO_TAIL} 를 먼저 실행합니다. 첫 빌드는 캐시가 비어 오래 걸립니다.
- 금지: GUI 실행, 사용자 실제 데이터·OS 설정·클립보드·Keychain·휴지통(${NA}/tests/explorer-delete.rs 실행 금지, 추적 안 된 파일 discard 실검사 금지), 실제 OS 알림, 네트워크, 합성 OS 입력, .env·키 파일. 테스트는 임시 디렉터리·주입 포트·가짜 platform 만.
- 동결 ${RW} 소스·Cargo 파일 변경 금지. UI crate 를 바꾸면 cargo check --quiet --manifest-path ${RW}/Cargo.toml --target wasm32-unknown-unknown --lib ${CARGO_TAIL} 로 확인합니다.
- 코드 주석 새로 쓰지 않음(공개 API 영어 /// 만 예외). 매직넘버는 의미 있는 const. #[allow]·테스트 기대값 조작 같은 우회 금지, 근본 원인 수정. 사용자 문자열은 taide-locale 카탈로그 키(원본과 같은 키) 사용.
- 요청 밖 리팩터링·리네임·재포맷 금지. 범위 밖 수정은 최소로 하고 QA 에 사유를 적습니다.
- 컨텍스트 절약: 큰 파일은 Grep 으로 위치를 찾아 구간만 읽습니다. 같은 파일 반복 읽기 금지. QA 초안을 절반 지점에 씁니다. 너무 크면 완성된 부분을 컴파일·테스트 통과 상태로 마무리하고 남은 것을 unresolved 에 정확히 적습니다.
- 같은 가정의 수정이 세 번 연속 실패하면 멈추고 unresolved 에 원인 가설과 함께 적습니다. 실행하지 않은 검사를 통과했다고 쓰지 않습니다.
- 보류 목록(허락 필요)에 해당하는 일은 하지 않고 needsUserDecision 에 적습니다. 그 밖의 작은 결정은 원본에 가장 가까운 안으로 정하고 QA 에 근거를 적습니다.
`;

const DONE = `
# 검증과 문서
- 각 ID: 실패 테스트 → 구현 → 실제 앱 경로(main.rs → NativeApplication → 해당 경로) 도달 확인 → 해당 동작 자동 검사(UI egui 하네스 --features native-host,inspection, 앱 lib 실제 App, crate 단위).
- 변경 crate 의 cargo check --quiet --all-targets(UI 는 --features native-host,inspection), 관련 테스트, cargo fmt --manifest-path <Cargo.toml> -- --check.
- QA 문서 ${QA}/${DATE}-native-lane-${L.key}-<묶음키>.md: 대상 ID, 원본 동작 근거(사양 참조 가능), 구현, 검증(실제 명령·결과 수), 남은 범위.
- 기능 대응표 ${MATRIX} 에서 이번 묶음 ID 행만 갱신합니다(완료 = 구현 + 실제 앱 경로 + 해당 동작 자동 검사, 일부면 partial 과 남은 것). 상단 합계 표는 건드리지 않습니다. 사양이 정정을 제안한 행(not-applicable 등)도 근거와 함께 갱신합니다.
`;

const IMPL_SCHEMA = {
  type: "object",
  properties: {
    bundle: { type: "string" },
    summary: { type: "string" },
    idStatus: {
      type: "array",
      items: {
        type: "object",
        properties: {
          id: { type: "string" },
          status: { type: "string" },
          remaining: { type: "string" },
        },
        required: ["id", "status", "remaining"],
      },
    },
    changedFiles: { type: "array", items: { type: "string" } },
    verification: {
      type: "array",
      items: {
        type: "object",
        properties: {
          command: { type: "string" },
          exitCode: { type: "integer" },
          result: { type: "string" },
        },
        required: ["command", "exitCode", "result"],
      },
    },
    unresolved: { type: "array", items: { type: "string" } },
    needsUserDecision: { type: "array", items: { type: "string" } },
    qaDocPath: { type: "string" },
  },
  required: [
    "bundle",
    "summary",
    "idStatus",
    "changedFiles",
    "verification",
    "unresolved",
    "needsUserDecision",
    "qaDocPath",
  ],
};

const REVIEW_SCHEMA = {
  type: "object",
  properties: {
    verdict: { type: "string", enum: ["pass", "needs-fix"] },
    blocking: {
      type: "array",
      items: {
        type: "object",
        properties: {
          file: { type: "string" },
          line: { type: "integer" },
          ids: { type: "string" },
          problem: { type: "string" },
          failureScenario: { type: "string" },
          fix: { type: "string" },
        },
        required: ["file", "problem", "failureScenario", "fix"],
      },
    },
    nonBlocking: { type: "array", items: { type: "string" } },
    statusOverclaims: { type: "array", items: { type: "string" } },
  },
  required: ["verdict", "blocking", "nonBlocking", "statusOverclaims"],
};

phase("Implement");
const impls = [];
for (let i = 0; i < L.bundles.length; i++) {
  const b = L.bundles[i];
  const r = await agent(
    `${BACKGROUND}
${RULES}
# 이번 묶음 (${L.key} ${i + 1}/${L.bundles.length}: ${b.key})
대상 ID: ${b.ids.join(", ")}
${b.brief ?? ""}
실행 순서: 1) git -C ${ROOT} status 로 미커밋 변경 확인(앞 묶음 작업자의 변경이면 유지, 이 묶음 범위의 변경이 이미 있으면 이어서 작업) 2) 사양에서 대상 ID 절과 묶음 계획을 읽음 3) 실패 테스트 → 구현 → 검증 4) QA·대응표 갱신 후 반환.
${DONE}
QA 묶음키는 "${b.key}" 입니다.`,
    {
      label: `impl:${L.key}:${b.key}`,
      phase: "Implement",
      schema: IMPL_SCHEMA,
      model: "opus",
      effort: "xhigh",
    },
  );
  impls.push(r);
  log(
    `${L.key}/${b.key}: ${r ? r.idStatus.map((x) => x.id + "=" + x.status).join(", ") : "실패"}`,
  );
}

phase("Review");
const review = await agent(
  `당신은 TAIDE Rust-native 전환 "${L.key}" lane 의 읽기 전용 리뷰어입니다. 한국어. 파일 수정·git 상태 변경·cargo 실행 금지(Read·Grep·Glob 과 git -C ${ROOT} diff/show/log 한 줄 명령만).
대상: worktree ${ROOT} 의 기준 커밋 ${BASE} 대비 전체 변경(git -C ${ROOT} diff ${BASE} --stat, git -C ${ROOT} status 의 새 파일). 사양 ${SPEC}, 결정 ${ROOT}/docs/acknowledge/2026-10-28-native-lane-spec-decisions.md.
작업자 요약: ${JSON.stringify(impls.map((r) => r && { bundle: r.bundle, idStatus: r.idStatus, unresolved: r.unresolved }))}
확인: (1) 원본 TS 동작과 화면·상태·상호작용 일치(빠진 분기: 오류·빈 상태·비활성·취소·수명), 새로 만든 동작, (2) 실제 앱 경로 도달(unwired 아님), (3) 정확성 버그(상태 수명·동시성·오래된 응답·패닉), (4) 테스트가 실제로 동작을 단언하는지, 동결 remote-web·Cargo 변경, 주석·매직넘버·#[allow]·하드코딩 문자열, (5) 대응표 complete 근거 충분성, (6) 공용 파일 영역 블록 규칙 위반(다른 lane 과 병합 충돌을 키우는 변경). blocking 은 실제 결함만(재현 시나리오 포함).`,
  {
    label: `review:${L.key}`,
    phase: "Review",
    schema: REVIEW_SCHEMA,
    model: "sonnet",
    effort: "xhigh",
  },
);

phase("Fix");
let fix = null;
if (
  review &&
  (review.verdict === "needs-fix" || review.statusOverclaims.length > 0)
) {
  fix = await agent(
    `${BACKGROUND}
${RULES}
# 수정 (${L.key} lane 리뷰 반영)
각 차단 항목을 실제 코드로 확인해 실제 결함이면 재현 테스트 → 근본 수정, 리뷰가 틀렸으면 unresolved 에 "리뷰 반박: 근거" 로 적습니다. 과대 판정은 근거 보강 또는 partial 로 낮춥니다.
차단: ${JSON.stringify(review.blocking)}
과대 판정: ${JSON.stringify(review.statusOverclaims)}
비차단(여유 있으면): ${JSON.stringify(review.nonBlocking)}
${DONE}
QA 는 해당 묶음 문서에 수정 절을 추가합니다.`,
    {
      label: `fix:${L.key}`,
      phase: "Fix",
      schema: IMPL_SCHEMA,
      model: "opus",
      effort: "xhigh",
    },
  );
}
return { lane: L.key, impls, review, fix };
