# CFB retained adapter 출처

설치된 registry cfb 0.14.0의 src·LICENSE·README·Cargo.toml.orig·.cargo_vcs_info.json을 그대로 가져왔습니다. source revision은 `45d0d07adc8e95f9c239ac7dcc42761b1347ed3e`이며 MIT입니다. LICENSE SHA-256은 `ebaac853b53fec0ed475796f533ce676912a4df9ad9d8a621fa76d5060591e10`, Cargo.toml.orig SHA-256은 `40fed1df8c069d7897d23fb35264ee3bb825bdf651c0abb1e95efe58e7c3ee7e`입니다.

consumer가 쓰는 기존 runtime dependencies를 유지한 publish=false local manifest와 기본 비활성 native-retained feature를 사용합니다. source 변경은 CompoundFile·MiniAllocator·Directory·Allocator·Sectors·Version·ObjType·Color·Timestamp의 조건부 derive, DirEntry의 전체 field를 방문하는 adapter입니다. Uuid는 실제 16바이트 inline 타입의 as_bytes를 방문합니다. 파일 format/parser/allocator 동작은 수정하지 않았고 vendor 전체 재포맷도 하지 않았습니다.

원본 MSRV 1.74는 feature-off 경계입니다. 새 optional helper feature는 native app MSRV 1.95를 요구합니다. 원본 dev test/assets는 가져오지 않았으며 사용하는 합성 HWP5 lazy source 경로를 consuming app integration test로 검증했습니다. 이 계산은 logical payload이며 RSS/allocator overhead cap이 아닙니다. app preflight가 쓰는 registry cfb 0.15.0은 변경하지 않았습니다.
