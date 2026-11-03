use taide_model::git::GitBranch;

use super::{Choice, listing};

fn branch(name: &str, is_head: bool, is_remote: bool) -> GitBranch {
    GitBranch {
        name: name.into(),
        is_head,
        is_remote,
        upstream: None,
    }
}

fn names(branches: &[&GitBranch]) -> Vec<String> {
    branches.iter().map(|branch| branch.name.clone()).collect()
}

fn fixture() -> Vec<GitBranch> {
    vec![
        branch("main", true, false),
        branch("feature", false, false),
        branch("origin/feature-remote", false, true),
        branch("origin/main", false, true),
    ]
}

#[test]
fn 빈_필터는_만들기_항목_없이_로컬과_원격을_나눠_모두_보인다() {
    let branches = fixture();
    let listed = listing(&branches, "");
    assert_eq!(listed.create, None);
    assert_eq!(names(&listed.local), ["main", "feature"]);
    assert_eq!(
        names(&listed.remote),
        ["origin/feature-remote", "origin/main"]
    );
    assert_eq!(
        listed.choices(),
        [
            Choice::Checkout("main".into()),
            Choice::Checkout("feature".into()),
            Choice::CheckoutRemote("origin/feature-remote".into()),
            Choice::CheckoutRemote("origin/main".into()),
        ]
    );
    let blank = listing(&branches, "   ");
    assert_eq!(blank.create, None);
    assert_eq!(blank.local.len() + blank.remote.len(), branches.len());
}

#[test]
fn 필터는_앞뒤_공백을_빼고_대소문자를_무시해_이름에_포함되는지_보고_빈_그룹은_비운다() {
    let branches = fixture();
    let listed = listing(&branches, "  FEAT ");
    assert_eq!(names(&listed.local), ["feature"]);
    assert_eq!(names(&listed.remote), ["origin/feature-remote"]);
    assert_eq!(listed.create, Some("FEAT"));
    assert_eq!(listed.choices()[0], Choice::Create("FEAT".into()));

    let remote_only = listing(&branches, "remote");
    assert!(remote_only.local.is_empty());
    assert_eq!(names(&remote_only.remote), ["origin/feature-remote"]);
}

#[test]
fn 같은_이름의_브랜치가_정확히_있으면_만들기_항목을_내지_않는다() {
    let branches = fixture();
    assert_eq!(listing(&branches, " feature ").create, None);
    assert_eq!(listing(&branches, "origin/main").create, None);
    assert_eq!(listing(&branches, "Feature").create, Some("Feature"));
    let created = listing(&branches, "zzz");
    assert_eq!(created.create, Some("zzz"));
    assert!(created.local.is_empty() && created.remote.is_empty());
    assert_eq!(created.choices(), [Choice::Create("zzz".into())]);
    assert!(listing(&[], "").choices().is_empty());
}
