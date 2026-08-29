# 전수조사 후속 사용자 결정 4건 (2026-08-29)

> 질문 근거: `docs/quality-assurance/2026-08-29-full-audit.md` §6. 전 항목 사용자 답변 확정.

## 결정

1. **릴리스 프로필**: `lto = "thin"` + `opt-level = 3` 채택 (추천안). codegen-units=1·panic=abort·
   strip 유지. **실측(2026-08-29)**: dmg 15,019,605 bytes vs v0.1.2 13,644,641 (+10.1%) — 단
   d-50/51/53 코드 순증(+8.4k 줄) 합산치라 순수 프로필 효과는 그 이하. 로컬 cargo release 풀
   컴파일 2m28s. 크기가 문제 되면 opt-level "s" 복귀(thin LTO 만) 대안을 쓴다.
2. **CI 캐시 워밍**: main push 워밍 워크플로 추가 — **paths 필터**(Cargo.lock·Cargo.toml·
   src-tauri·crates·워크플로 자신) 한정 (추천안). shared-key `release`/`tests` 를 release.yml 과
   정확히 일치시켜 태그 빌드가 히트하게 한다.
3. **CI 러너**: release·test-frontend **둘 다 macOS 유지** (추천안 기각 — 사용자 결정).
4. **UX 착수 범위**: 저비용 5건(diff 미변경 접기+이동 감지·브래킷 가이드선·스무스 스크롤/커서·
   suggest preview·rulers) **+ on-save 정리(trim trailing whitespace·insert final newline) +
   EditorConfig 지원**까지 = d-53 스코프. 스무스 계열 기본값은 off(보수적), 전부 설정 토글 노출.

## d-52 즉시 반영분 (이 결정과 함께 적용)

- 루트 Cargo.toml `[profile.release]` 프로필 전환 (결정 1)
- `.github/workflows/cache-warm.yml` 신설 (결정 2)
- release.yml: test-rust rust-cache `save-if` 추가(태그 무효 캐시 542MiB·28s 제거),
  릴리스 노트 존재 검증을 build job 앞단에 복제(실패 조기화 — release job 검증은 방어선 유지),
  timeout 축소(build 75→40m·test-rust 45→30m)
- 러너 전환 없음 (결정 3)
