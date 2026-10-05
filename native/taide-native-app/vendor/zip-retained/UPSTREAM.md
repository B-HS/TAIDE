# ZIP retained adapter 출처

설치된 registry zip 8.6.0의 src·LICENSE·README·normalized Cargo.toml·Cargo.toml.orig·.cargo_vcs_info.json을 가져왔습니다. source revision은 `771dfc534d2614158af5497ea3dff4d4208d7db1`, license는 MIT입니다. LICENSE SHA-256은 `58545fed1565e42d687aecec6897d35c6d37ccb71479a137c0deb2203e125c79`, Cargo.toml.orig SHA-256은 `a0d0c22e1a88331b6aade076372ba83d0ec2c5bc96375b8754ccab85e8daaccf`입니다.

원본 normalized manifest의 dependencies/features를 유지하고 local workspace·optional helper·기본 비활성 native-retained feature만 추가했습니다. rhwp의 기존 default-features=false/deflate 선택은 유지합니다. source 변경은 ZipArchive·Config·ArchiveOffset·ZipFileData·System·AesMode·AesVendorVersion·CompressionMethod·DateTime·ExtraField·Ntfs·ExtendedTimestamp의 조건부 derive와 ZipArchiveMetadata의 전체 field adapter입니다. 공개 IndexMap capacity의 entry payload·keys/values·hasher, 원본 Cursor, Arc extra-field·OnceLock을 방문합니다. private hash/index/control bookkeeping은 계산하지 않으며 RSS cap이라고 주장하지 않습니다.

원본 MSRV 1.88은 feature-off 경계이고 optional helper feature는 native app MSRV 1.95를 요구합니다. decompress/read semantics를 변경하지 않았으며 실제 합성 HWPX lazy resolver 경로를 consuming app integration test로 검증했습니다. app lock에는 다른 dependency가 쓰는 registry zip 8.6.0과 local 8.6.0이 함께 존재합니다. 이 중복의 packaging/성능 gate는 미완료이며 preflight zip 2.4.2도 그대로 유지합니다.
