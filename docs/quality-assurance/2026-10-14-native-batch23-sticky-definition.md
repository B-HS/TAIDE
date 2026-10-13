# 배치 23 — 고정 줄 실제 앱 정의 이동

현재작업은PROCESS b2/editor-16의잔여실제child 클릭검사입니다. 서브에이전트/workflow 없이메인이기존b1의UI32/앱회귀1성공을재사용합니다. 원본설치Monaco stickyScrollController의rendered node 위치/보조키·hover정의밑줄/GoTo/Aside와native StickyState/DefinitionLink/Consumer::request_at/activate/reveal를읽었습니다. 본문의가려진줄좌표나고정픽셀을사용하지않고실제sticky response band와Text shape Galley cursor의문자위치를입력합니다.

격리서버sticky mode는실제수신position에따라한줄아래같은UTF-16 열의유효범위를돌려줍니다. 이는요청직전에Consumer가설정한출발커서와서버응답소비로얻는도착커서를구분합니다. 원본Doc ID/URI·source/hover Request.position·현재창이동/Alt옆열기의실제뷰·원문/revision/디스크·종료task0을검사하며새기능/의존성/OS입력/사용자데이터를추가하지않습니다. 기존mock모드는그대로입니다.

mock-build.log는23491 handle에서example/lib compile26.49초·exit0입니다. 첫app-first.log는7661 handle에서RawInput에존재하지않는modifiers 필드를사용해compile실패/0검사·exit101입니다. 실제UI fixture와같은Event::ModifiersChanged로수정했습니다. 보조키를놓는RawInput을매poll에넣지않고같은보조키상태를유지하며응답을기다립니다.

app-events.log는20664 handle에서compile20.80초·실제고정줄/문자point·서버hover응답/UTF-16 source위치까지통과했지만두목적selection 검사에서0통과/2실패·53.53초·exit101입니다. native activate/reveal와host요청은target selection의시작에커서를놓으므로한글자를선택해야한다는fixture가정이잘못됐습니다. 시작커서만기다리면request_at가미리설정한출발커서로오검증할수있어서버목적을한줄아래로구분했습니다. 옆열기에서도source는클릭위치에놓이고목적뷰가새줄에독립이동해야합니다. 실패는제품이동버그로보고하지않습니다.

수정patch는포맷된fixture 문맥과달라한번거절됐고실제파일을읽어적용했습니다. mock-destination.log는45149 handle에서compile5.33초·exit0입니다. 현재app-destination 검사는74935 handle에서진행중이며성공은아직선언하지않습니다. 전체303/588(51.5%)·미완료285·editor-16partial·배치23잔여18·목표active/잔여시간미산정이며일반전체/OS/성능·동결경계를유지합니다.

app-destination.log는74935 handle에서compile20.45초·새실제앱2/53.92초·exit0입니다. 실제고정줄Galley의문자클릭→현재문서/source의UTF-16 Request.position→서버Definition응답의한줄아래위치→시작커서 이동을확인했습니다. Alt옆열기는같은Document의독립뷰를만들고출발뷰의클릭위치·원문/revision/디스크를보존합니다. 정상종료task0입니다.

같은helper에서header만실제비BMP 식별자가있는Rust 문장으로바꾸고공개file service로읽은실제OpenedFile의read_only를적용했습니다. 고정픽셀/가짜ID/언어는없고글자index5의실제byte8/UTF-16열6을Galley와rope에서유도합니다. app-unicode-readonly.log는12841 handle에서compile20.85초·새실제앱1/37.26초·exit0이며서버열6/도착커서·Normal read_only/원문/revision/디스크·종료task0입니다. 같은입력/환경인앞선앱2는재사용합니다. 현재직접UI32+실제App3의35서로다른성공이며검사반복/fixture사례수를더하지않습니다.

현재2Rustfmt/소유diff를확인했고mock의신규모드13줄/기존mode 변경0이며제품UI/어댑터동작은변경없습니다. example/lib 성공과현재App test compile을재사용합니다. 기존vendor wry17/linker unwind/lib is_applying 경고가있으며새무경고로보고하지않습니다. source/shared/core·의존성/manifest/lock/frozen은불변입니다.

완료감사에서설치Monaco StickyModelProvider의outline→syntax folding→indentation 우선순위와native editor_symbols::sticky/StickyState::model을대조했습니다. 현재native는외부document symbols/구문접기/indent 경계를연결했지만원본의JSON/CSS/HTML/TS 내장언어서비스(editor-57)가아직없습니다. 따라서b2 클릭/child/readonly/Unicode 경계만닫고editor-16은partial을유지합니다. 내장공급자가없는언어에서Outline/Folding까지원본과같다고현재3검사로주장하지않습니다. 다음은editor-57의실제원본옵션·worker/API·현재native/서버/기존라이브러리·각소비자 의존을읽어소유범위와구현경로를고정합니다. 전체303/588(51.5%)·미완료285·배치23잔여18·목표active·잔여시간미산정이며일반전체App 준비실패/OS/성능/전환게이트를유지합니다.
