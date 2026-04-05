use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin("mdlint").unwrap()
}

fn create_vault(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }
    dir
}

#[test]
fn test_no_issues() {
    let vault = create_vault(&[
        ("index.md", "# Index\n\n- [Guide](./guide.md)\n"),
        ("guide.md", "# Guide\n\nHello world.\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("No issues found"));
}

#[test]
fn test_broken_standard_link() {
    let vault = create_vault(&[
        ("index.md", "# Index\n\n[Missing](./nope.md)\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("link target not found: ./nope.md"));
}

#[test]
fn test_broken_wikilink() {
    let vault = create_vault(&[
        ("index.md", "# Index\n\n[[nonexistent]]\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("link target not found: nonexistent"));
}

#[test]
fn test_valid_wikilink() {
    let vault = create_vault(&[
        ("index.md", "# Index\n\n[[guide]]\n"),
        ("guide.md", "# Guide\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .success();
}

#[test]
fn test_broken_anchor() {
    let vault = create_vault(&[
        ("index.md", "# Index\n\n[Link](./guide.md#nope)\n"),
        ("guide.md", "# Guide\n\n## Real Heading\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("heading anchor not found: #nope"));
}

#[test]
fn test_valid_anchor() {
    let vault = create_vault(&[
        ("index.md", "# Index\n\n[Link](./guide.md#real-heading)\n"),
        ("guide.md", "# Guide\n\n## Real Heading\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .success();
}

#[test]
fn test_self_anchor() {
    let vault = create_vault(&[
        ("index.md", "# Index\n\n## Section\n\n[Link](#section)\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .success();
}

#[test]
fn test_heading_increment() {
    let vault = create_vault(&[
        ("doc.md", "# Title\n\n#### Skipped\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .stderr(predicates::str::contains("heading level skipped: h1 -> h4"));
}

#[test]
fn test_first_heading_not_h1() {
    let vault = create_vault(&[
        ("doc.md", "## Not H1\n\nSome text.\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .stderr(predicates::str::contains("first heading should be h1, found h2"));
}

#[test]
fn test_missing_image() {
    let vault = create_vault(&[
        ("doc.md", "# Doc\n\n![Logo](./missing.png)\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("link target not found: ./missing.png"));
}

#[test]
fn test_require_frontmatter_config() {
    let vault = create_vault(&[
        ("mdlint.toml", "[rules.require-frontmatter]\nlevel = \"error\"\nfields = [\"title\"]\n"),
        ("doc.md", "# No Frontmatter\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("missing frontmatter"));
}

#[test]
fn test_frontmatter_present_passes() {
    let vault = create_vault(&[
        ("mdlint.toml", "[rules.require-frontmatter]\nlevel = \"error\"\nfields = [\"title\"]\n"),
        ("doc.md", "---\ntitle: Hello\n---\n\n# Hello\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .success();
}

#[test]
fn test_quiet_suppresses_warnings() {
    let vault = create_vault(&[
        ("doc.md", "## Not H1\n\n#### Skipped H3\n"),
    ]);

    // Without quiet: warnings should appear
    cmd()
        .arg(vault.path())
        .assert()
        .stderr(predicates::str::contains("heading level skipped"));

    // With quiet: only errors, no warnings
    cmd()
        .arg(vault.path())
        .arg("--quiet")
        .assert()
        .success()
        .stderr(predicates::str::contains("No issues found"));
}

#[test]
fn test_json_output() {
    let vault = create_vault(&[
        ("doc.md", "# Doc\n\n[Missing](./nope.md)\n"),
    ]);

    cmd()
        .arg(vault.path())
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stdout(predicates::str::contains("\"rule\": \"broken-links\""));
}

#[test]
fn test_short_output() {
    let vault = create_vault(&[
        ("doc.md", "# Doc\n\n[Missing](./nope.md)\n"),
    ]);

    cmd()
        .arg(vault.path())
        .arg("--format")
        .arg("short")
        .assert()
        .failure()
        .stderr(predicates::str::contains("error[broken-links]"));
}

#[test]
fn test_external_links_skipped_by_default() {
    let vault = create_vault(&[
        ("doc.md", "# Doc\n\n[Google](https://google.com)\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .success();
}

#[test]
fn test_html_xml_tags_not_treated_as_links() {
    let vault = create_vault(&[
        ("doc.md", "# Doc\n\nSome text with <custom:tag>content</custom:tag> inline.\n\n<my-ns:element attr=\"val\">block</my-ns:element>\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("broken-links").not());
}

#[test]
fn test_links_in_code_blocks_ignored() {
    let vault = create_vault(&[
        ("doc.md", "# Doc\n\n```\n[Not a link](./fake.md)\n```\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .success();
}

#[test]
fn test_rule_disabled_via_config() {
    let vault = create_vault(&[
        ("mdlint.toml", "[rules.first-heading]\nlevel = \"off\"\n"),
        ("doc.md", "## Not H1\n"),
    ]);

    // first-heading should not fire, only h2 so no heading-increment either
    cmd()
        .arg(vault.path())
        .assert()
        .success();
}

#[test]
fn test_wikilink_in_subdirectory() {
    let vault = create_vault(&[
        ("index.md", "# Index\n\n[[sub-page]]\n"),
        ("sub/sub-page.md", "# Sub Page\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .success();
}

#[test]
fn test_exit_code_zero_on_warnings_only() {
    let vault = create_vault(&[
        ("doc.md", "## Not H1\n\nSome text.\n"),
    ]);

    // first-heading and heading warnings, but no errors -> exit 0
    cmd()
        .arg(vault.path())
        .assert()
        .success();
}

#[test]
fn test_orphan_page_detected() {
    let vault = create_vault(&[
        ("mdlint.toml", "[rules.orphan-pages]\nlevel = \"warning\"\n"),
        ("index.md", "# Index\n\n[Guide](./guide.md)\n"),
        ("guide.md", "# Guide\n"),
        ("orphan.md", "# Orphan\n\nNobody links here.\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .stderr(predicates::str::contains("page has no incoming links"));
}

#[test]
fn test_orphan_page_index_excluded_by_default() {
    let vault = create_vault(&[
        ("index.md", "# Index\n"),
    ]);

    // index.md is excluded by default, so no orphan warning
    cmd()
        .arg(vault.path())
        .assert()
        .stderr(predicates::str::contains("orphan").not());
}

#[test]
fn test_orphan_page_custom_exclude() {
    let vault = create_vault(&[
        ("mdlint.toml", "[rules.orphan-pages]\nlevel = \"warning\"\nexclude = [\"index.md\", \"changelog.md\"]\n"),
        ("index.md", "# Index\n"),
        ("changelog.md", "# Changelog\n"),
    ]);

    // Both excluded, no orphan warnings
    cmd()
        .arg(vault.path())
        .assert()
        .stderr(predicates::str::contains("orphan").not());
}

#[test]
fn test_no_orphans_when_all_linked() {
    let vault = create_vault(&[
        ("index.md", "# Index\n\n[A](./a.md)\n[B](./b.md)\n"),
        ("a.md", "# A\n\n[B](./b.md)\n"),
        ("b.md", "# B\n\n[A](./a.md)\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .stderr(predicates::str::contains("orphan").not());
}

#[test]
fn test_orphan_page_disabled() {
    let vault = create_vault(&[
        ("mdlint.toml", "[rules.orphan-pages]\nlevel = \"off\"\n"),
        ("orphan.md", "# Orphan\n"),
    ]);

    cmd()
        .arg(vault.path())
        .assert()
        .stderr(predicates::str::contains("orphan").not());
}
